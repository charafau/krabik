# The Android renderer

This chapter walks through `rust/src/android/` file by file. It explains how a session is created
and found again, why event entry points only *try* to take the lock, how each `Op` becomes calls
on Android `View`s, why JNI ids are cached and where, how one widget's platform code is organised,
and how recycled list rows are bound. Afterwards you can read any file in the directory and place
it in the whole.

A reminder of the terms (details in [`../internals/`](../internals/)): the engine produces a list
of `Op`s (`Create`, `Update`, `Reparent`, `Remove`, `Layout`); a *host* is one native root view
plus its engine `Runtime`; the *session* owns the app and all hosts; the *controller* is the
Kotlin object that owns the chrome.

## The files

```text
rust/src/android/
  mod.rs         macros, class-name constants, checked(), run()
  entry.rs       JNI entry points, the state map, retained apps
  controller.rs  Platform/Host/Controller impls, calls to KrabikHostController
  host.rs        AndroidState, ViewHandle, applying each Op, create_view
  handles.rs     Jni: every cached class, method, field and constructor id
  style.rs       padding, decoration, accessibility, frames
  measure.rs     AndroidMeasure: text, switch and image sizes
  rows.rs        recycled list rows
  widgets/
    mod.rs       update_view, configure_view, Site, create_widget
    button.rs, text.rs, text_field.rs, ...   one file per widget
```

## `mod.rs`: the vocabulary

`mod.rs` holds the pieces every other file uses.

- `framework_class!`, `native_name!` and `ROOT_VIEW_DESCRIPTOR` spell the Kotlin package
  `dev.krabik.ui` in its three JNI forms (chapter 1).
- The class-name constants (`TEXT_VIEW_CLASS`, `SWITCH_CLASS`, ...) and a few Android integer
  constants that JNI cannot read cheaply (`MEASURE_AT_MOST`, `WRAP_CONTENT`, `GRAVITY_CENTER`,
  `COMPLEX_UNIT_SP`). They are copied from the Android SDK; they are part of Android's stable API.
- `checked` and the `jcall!` macro. A JNI call can leave a Java exception pending without
  returning an error. `checked` asks the environment for a pending exception after the call and
  turns it into a Rust `Err`:

  ```rust,ignore
  // rust/src/android/mod.rs
  pub(crate) fn checked<T>(env: &mut Env<'_>, result: JniResult<T>) -> JniResult<T> {
      let value = result?;
      env.exception_catch()?;
      Ok(value)
  }
  ```

- `string_array!` builds a Java `String[]` from string slices, for the controller calls.
- `run` wraps the body of every entry point (chapter 1).

One stale spot: the module comment at the top of `mod.rs` still describes the Kotlin side as "a
thin `ViewGroup`" that "forwards clicks back". That was true before the Kotlin library grew a
controller, a recycler adapter and a class per interactive widget.

## `entry.rs`: sessions, the lock, and the entry points

### The state map

```rust,ignore
// rust/src/android/entry.rs
pub(crate) struct AndroidSession {
    pub(crate) core: Session<Android>,
    pub(crate) context: Global<JObject<'static>>,
    pub(crate) density: f32,
}

pub(crate) static STATES: OnceLock<Mutex<HashMap<jlong, AndroidSession>>> = OnceLock::new();
```

`core` is the shared session. The two extra fields are what Android needs to build *another* host
later: the activity `Context` (to construct views) and the display density (to convert between
the engine's logical points and pixels).

### `nativeCreate`

```rust,ignore
// rust/src/android/entry.rs (trimmed)
run(&mut unenv, "nativeCreate", |env| {
    jni_handles(env)?;
    let main = AndroidState::new(env, &context, &this, density, HostKind::Main)?;
    let app = retained_apps()
        .lock()
        .expect("retained apps")
        .remove(&retained_app)
        .unwrap_or_else(crate::app::create_app);
    let mut core = Session::new(app, main);
    core.set_color_scheme(if night { ColorScheme::Dark } else { ColorScheme::Light });
    // ... insert into the map under a fresh handle, return the handle
})
```

Four things happen, in an order that matters:

1. `jni_handles(env)?` resolves every JNI id (see `handles.rs` below). It must happen here.
2. The main host is created around `this`, the `RustRootView` that called.
3. The app is either taken from the retained-app map or created from the registered factory.
   `create_app` panics with a clear message if no app was registered; `run` turns that panic into
   a Java `RuntimeException`.
4. The colour scheme is set from the `night` flag before the first render.

Note that `nativeCreate` does **not** render. The root view has no size yet. The first render
happens when Android measures the view and Kotlin calls `nativeRender`.

### `nativeRender` and its relatives

`nativeRender` stores the new viewport on one host (pixels divided by density) and then calls
`session.core.render_all(env)`, which re-renders *every* host and synchronises the chrome. The
same pattern, "change one thing, then `render_all`", appears in `nativeAttachHost` and
`nativeSetController`. Rendering everything is cheap when nothing changed: the engine skips
layout for an identical tree (see the runtime chapter in `../internals/`).

`nativeContentHeight` reads `runtime.content_height()` and converts it to pixels. Kotlin uses it
to size the bottom sheet to its content.

### The try-lock rule

This is the most important design decision in the file.

```rust,ignore
// rust/src/android/entry.rs
pub(crate) fn lock_states_for_event() -> Option<MutexGuard<'static, HashMap<jlong, AndroidSession>>>
{
    states().try_lock().ok()
}

pub(crate) fn on_event(
    handle: jlong,
    body: impl FnOnce(&mut AndroidSession) -> JniResult<()>,
) -> JniResult<()> {
    let Some(mut states) = lock_states_for_event() else {
        return Ok(());
    };
    match states.get_mut(&handle) {
        Some(session) => body(session),
        None => Ok(()),
    }
}
```

Every event entry point goes through `on_event`. If the lock is taken, the event is dropped
silently. That sounds dangerous, so here is the argument for it, as the comment in the source
gives it:

1. Entry points and renders all run on the Android main thread.
2. So while a render holds the lock, the main thread is *inside* that render. A real user event
   cannot arrive at that moment; it is queued behind the current main-loop message.
3. Therefore, if an event entry point finds the lock taken, the event was caused synchronously by
   the render itself. The source's example: `setTransformationMethod` (called when a field becomes
   a password field) calls `setText` internally, which fires the text watcher, which calls
   `nativeOnTextChanged`, all on the same stack as the render.
4. Such an event is an echo of a value Rust wrote a moment ago. Dropping it loses nothing. Blocking on it
   would deadlock the UI, because `std::sync::Mutex` is not re-entrant.

The Kotlin "silent" setters (chapter 3) remove most echoes at the source. The try-lock is the
safety net for the ones Android produces behind the widget's back.

The entry points that are *not* events use a blocking `lock()`: `nativeCreate`, `nativeRender`,
`nativeAttachHost`, `nativeSetController`, `nativeBindRow`, `nativeContentHeight` and
`nativeDestroy`. They must not be skipped. The design relies on Android never calling them
synchronously from inside a render. That holds today because fragment transactions and layout
passes are asynchronous (chapter 3), but nothing enforces it. If a future change made Kotlin call
one of these while Rust holds the lock, the UI thread would hang rather than fail loudly.

`nativeUnbindRow` is a deliberate middle case. `RecyclerView` can recycle a holder during a
render; the comment explains that dropping the cleanup is safe because the next bind of that
holder rebuilds the row from scratch.

### The event entry points

They are all the same shape: decode the JNI arguments into a `NodeEvent`
(`rust/src/host/events.rs`) and hand it to `Session::node_event` through `on_node_event`. A few
do a little decoding first:

- `nativeOnScroll` divides the pixel offset by the density.
- `nativeOnPickerChanged` builds a `DatePickerValue` from a kind code and five integers.
- `nativeOnDialogAction` maps a negative button index to `None` ("the platform dismissed it").
- `nativeOnChromeEvent` decodes `(kind, index)` with `ChromeEvent::from_code`.
- `nativeOnBack` returns the session's answer as a boolean.

`nativeRestoreRowFocus` is the one that does real work in this file. When a recycled row is
re-attached, it checks whether the runtime remembers a logical focus for a field in that row and,
if so, calls `requestFocus` on the native field and asks Kotlin to show the keyboard.

### Retained apps

```rust,ignore
// rust/src/android/entry.rs
static RETAINED_APPS: OnceLock<Mutex<HashMap<jlong, Box<dyn AnyApp>>>> = OnceLock::new();
```

Android destroys and recreates an activity for configuration changes the manifest does not
claim, and a switch between light and dark is one of them (chapter 3 explains why that is left
on). The views die with the activity, but the Rust application must not. So `nativeDestroy` takes
a `retain` flag. With it, the session is dismantled, `session.core.into_app()` extracts the
`Box<dyn AnyApp>`, and it is parked in `RETAINED_APPS` under a fresh id, which is returned. The
new activity passes that id to `nativeCreate`, which takes the app back out. If the activity is
finished for good instead, Kotlin calls `nativeDropRetainedApp`.

What survives is the application value: its state, and therefore its navigation stack, dialog
and so on, because Rust owns those. What does not survive is anything native: scroll positions,
text selection, the keyboard.

## `controller.rs`: plugging into the session

This file has three parts.

`Android` is an empty struct that implements `Platform`, naming the four associated types.

`impl Host<Android> for AndroidState` is glue: it forwards `kind`, `runtime`, `row_actions`,
`row_of` and `render` to the fields and methods of `AndroidState`.

`AndroidController` wraps a global reference to the Kotlin `KrabikHostController` and implements
`Controller<Android>`. Each method marshals its arguments and makes one JNI call, except
`set_chrome`, which makes four (`setToolbar`, `setBottomNavigation`, `setFab`, `setDrawer`).
Optional values are flattened because JNI has no `Option`: a missing app bar becomes empty strings
plus `hidden = true`; a missing drawer selection becomes `-1`; a snackbar without an action
becomes an empty action string.

The file ends with its own `Handles` struct, the cached method ids of `KrabikHostController`.
The doc comment on `AndroidController` lists "the screen stack, toolbar, bottom navigation, alert
and sheet"; it predates the floating action button, drawer and snackbar.

## `handles.rs`: cached JNI ids

### Why cache

Calling a Java method through JNI by name takes two lookups, `FindClass` and `GetMethodID`, each
a string search inside the JVM. A render touches many properties of many views. Before the cache,
every property set paid both lookups. Now each id is resolved once per process.

```rust,ignore
// rust/src/android/handles.rs
#[derive(Clone, Copy)]
pub(crate) struct Method {
    pub(crate) id: JMethodID,
    pub(crate) returns: JavaType,
}

impl Method {
    pub(crate) fn call<'local>(
        self,
        env: &mut Env<'local>,
        object: &JObject<'_>,
        args: &[JValue],
    ) -> JniResult<JValueOwned<'local>> {
        let args: Vec<_> = args.iter().map(JValue::as_jni).collect();
        // SAFETY: ...
        let result = unsafe { env.call_method_unchecked(object, self.id, self.returns, &args) };
        checked(env, result)
    }
}
```

`Method`, `StaticMethod`, `StaticField` and `Ctor` are thin wrappers around an id. The calls are
`unsafe` (`call_method_unchecked`) because the JVM does not verify that the id belongs to the
object or that the arguments match. The safety argument is that each id was resolved from a
literal name and signature in this file, and the call sites pass matching arguments. A wrong
signature string therefore shows up as a crash at run time, not as a compile error. This is the
price of the speed.

The macros `method!`, `static_method!`, `static_field!` and `ctor!` make the resolution lines
short:

```rust,ignore
// rust/src/android/handles.rs
set_padding: method!(env, &view, "setPadding", "(IIII)V", VOID),
apply_layouts: method!(env, &rust_root_view, "applyLayouts", "([J[I[I)V", VOID),
```

The second string is a JNI type signature: `I` int, `J` long, `Z` boolean, `F` float, `[I` int
array, `Lpkg/Class;` an object, and the part after `)` is the return type (`V` is void).

### Lifetime

A method id is valid only while its class is loaded. `Jni` therefore keeps a global reference to
every class it resolved ids from. Two of them, `_view_group` and `_rust_root_view`, exist for no
other reason, which is what the underscore says. The whole struct lives in a `static OnceLock`,
so the ids are valid for the life of the process.

An id resolved from a superclass works on subclass instances. That is why `set_text` (resolved
from `TextView`) can be called on a `Button`, a `KrabikCheckBox` or a `KrabikTextInput`.

### Shared handles and per-widget handles

`Jni` has two kinds of fields. The shared ones (`View`, `ViewGroup`, `TextView`, the root view's
methods, a few statics) are plain fields. Handles that only one widget uses live in a `Handles`
struct in that widget's own file, and `Jni` holds one of each:

```rust,ignore
// rust/src/android/handles.rs
pub(crate) struct Jni {
    // ...
    pub(crate) set_text: Method,
    // Handles owned by one widget live in that widget's module.
    pub(crate) controller: crate::android::controller::Handles,
    pub(crate) text_field: crate::android::widgets::text_field::Handles,
    pub(crate) switch: crate::android::widgets::switch::Handles,
    // ...
}
```

This is the "each widget owns its JNI handles" change from the third round of work. Before it,
adding a widget meant editing one large central struct. Now the widget's file declares what it
needs, and the central file gains one field and one line in `Jni::load`.

Every framework widget class has the same constructor, `(Context, RustRootView, long)`.
`Jni::load` builds that signature once (`widget_signature`) and passes it to each `Handles::load`,
which uses `widget_ctor`. The modules whose view is a stock Android class (`button`, `image`,
`progress`, `rows`) take the parameter and ignore it, named `_widget_signature`. That keeps the
`load` functions uniform at the cost of a few unused parameters.

### Why loading happens in `nativeCreate`

`FindClass` resolves a name through the class loader of the *calling* Java method. Called from
`nativeCreate`, which is invoked from the app's own code on the main thread, the loader is the
app's, so `dev.krabik.ui.*` resolves. Called later from a thread Rust attached itself, the loader
would be the system one, which knows `android.widget.TextView` but not the framework's classes.
`get` would still load lazily on first use, so the explicit `jni_handles(env)?` at the top of
`nativeCreate` is what guarantees the right loader.

(The comments in `handles.rs` and `entry.rs` call these "the app's own Kotlin classes". They are
the framework's classes, packaged into the app; the wording is from before the Kotlin moved into
the `dev.krabik.ui` library.)

## `host.rs`: one root view and its operations

### State

```rust,ignore
// rust/src/android/host.rs
pub(crate) struct ViewHandle {
    pub(crate) view: Global<JObject<'static>>,
    pub(crate) children_host: Global<JObject<'static>>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) frame: Option<Rect>,
}

pub(crate) struct AndroidState {
    pub(crate) runtime: Runtime,
    pub(crate) kind: HostKind,
    pub(crate) root: Global<JObject<'static>>,
    pub(crate) context: Global<JObject<'static>>,
    pub(crate) views: HashMap<NodeId, ViewHandle>,
    pub(crate) rows: RowBinder<(NodeId, i64), ViewHandle>,
    pub(crate) density: f32,
    pub(crate) viewport: Size,
}
```

A `ViewHandle` is what Rust remembers about one node's native view.

- `view` draws the node. `children_host` is where the node's children are added. For almost
  every node they are the same object. For a scroll container they differ: `view` is the
  `ScrollView` and `children_host` is the single `FrameLayout` inside it.
- `parent` is the node's nearest ancestor that has a native view, or `None` for the root view.
  Not every node has a view: an undecorated `Column` is pure layout and exists only in the engine.
- `frame` is the last frame applied, kept so a `Reparent` can re-apply it.

The four `row_*` maps belong to recycled lists and are explained under `rows.rs`.

### `render`

```rust,ignore
// rust/src/android/host.rs (trimmed)
pub(crate) fn render(&mut self, env: &mut Env<'_>, element: &RawElement) -> JniResult<()> {
    let update = self.runtime.update(element, self.viewport);

    let mut views = std::mem::take(&mut self.views);
    let mut frames = FrameBatch::new();
    // ... for op in update.ops { self.apply_op(env, op, &mut views, root, true, Some(&mut frames)) }
    self.views = views;

    if !frames.is_empty() {
        apply_layouts(env, self.root.as_obj(), &frames)?;
    }
    self.prune_row_actions();
    Ok(())
}
```

`runtime.update` is the engine: it builds the tree, lays it out, diffs it against the previous
one and returns the `Op`s. Everything after that line is this platform's job.

The view map is taken out of `self` for the duration. `apply_op` takes `&self` plus a separate
`&mut HashMap`, so the same function can apply operations to the main tree's views *or* to the
private view set of one recycled row. Passing the map in avoids borrowing `self` twice.

### Applying each `Op`

`apply_op` takes two flags besides the op: `register` (true for the main tree, false for a row)
and an optional `FrameBatch`.

**`Create`** calls `create_and_insert`: build the view with `create_view`, add it to its parent's
`children_host` (or to the root view), work out its own `children_host`, and store a `ViewHandle`.

**`Update`** calls `update_view` on the existing view. If no view is known for the node, it
creates one instead; the comment says this can happen "after a screen or host swap". (The iOS
renderer ignores an update for an unknown node. The two differ here.)

**`Reparent`** removes the view from its old host and adds it to the new one, then re-applies
its stored frame.

**`Remove`** either removes the view at once or, when the node declared an exit transition and
this is the main tree, calls `RustRootView.startExit`, which animates the view out and removes it
when the animation ends. Rust forgets the handle immediately in both cases.

**`Layout`** is where batching comes in.

### Layout batching with `frame_batch`

A layout pass can move every view on screen. One JNI call per view would be slow, so the frames
are packed into arrays and sent in one call.

```rust,ignore
// rust/src/frame_batch.rs
pub struct FrameBatch {
    ids: Vec<i64>,
    frames: Vec<i32>,
    animations: Vec<i32>,
}
```

`push` appends one node id, four integers (`x, y, width, height`, already in pixels) and six
integers describing the animation (duration in milliseconds, then the five numbers from
`AnimationCurve::packed`). At the end of `render`, `apply_layouts` in `style.rs` copies the three
vectors into a `long[]` and two `int[]`s and calls `RustRootView.applyLayouts` once.

For this to work Kotlin must be able to find a view from a node id without Rust passing the view
object. That is what `register` is for: `create_view` calls `RustRootView.registerView(nodeId,
view)` for main-tree views, and Kotlin keeps a map. Views inside recycled rows are not registered
(the source does not say why; a row is small, and its views live and die with a holder rather
than with the host), so for them `apply_op` is called with `frames = None` and uses `apply_frame`,
one JNI call per view.

A naming trap: the KDoc on `applyLayouts` still says "two integers per id", which dates from
before custom curves. The stride is six, as `ANIMATION_STRIDE` in `RustRootView.kt` and the test
`animations_are_packed_six_integers_per_node` in `frame_batch.rs` say.

### `create_view`

```rust,ignore
// rust/src/android/host.rs (trimmed)
let view = match &node.kind {
    NodeKind::Widget(widget) => create_widget(env, &site, widget)?,
    NodeKind::Pager(_) => pager::create(env, &site)?,
    NodeKind::Scroll(props) => create_scroll(env, site.context, site.root, /* horizontal */)?,
    NodeKind::Recycler(props) => create_recycler(env, site.context, site.root, node.id, props, self.density)?,
    NodeKind::Column | NodeKind::Row => { /* FrameLayout */ }
    NodeKind::Spacer => { /* View */ }
    NodeKind::Root => return Ok(JObject::null()),
};

configure_view(env, &view, &node.kind, self.density)?;
apply_padding(env, &view, node.style.padding, self.density)?;
apply_style(env, &view, node, false)?;
apply_accessibility(env, &view, node)?;
set_tag(env, &view, node.id.0 as jlong)?;
```

Creation is two steps: construct an empty view of the right class, then configure it. The
configuration step is `configure_view`, the same function an update calls. A `Column` or `Row`
only reaches this function when it is decorated (background, border, corner radius); then it
becomes a `FrameLayout` so it can draw behind its children and clip them.

`set_tag` stores the node id on the view as a boxed `Long`. Two things read it in Kotlin: the
root view's click listener (for stock buttons) and the scroll reporter.

If the node declares an enter transition, `create_view` also calls `RustRootView.startEnter`.

## `widgets/`: one file per widget

### `Site`

```rust,ignore
// rust/src/android/widgets/mod.rs
pub(crate) struct Site<'a> {
    pub(crate) context: &'a JObject<'a>,
    pub(crate) root: &'a JObject<'a>,
    pub(crate) node: NodeId,
}
```

A `Site` is "where a view is being created": the context to construct it with, the root view its
events go to, and the node it is for. It has two constructors' worth of help:

- `site.stock(env, class, ctor)` constructs a stock Android view, `View(Context)`.
- `site.framework(env, class, ctor)` constructs a framework widget, `Widget(Context,
  RustRootView, nodeId)`. The widget keeps the root view and its node id and reports its own
  events through them.

### `create` and `update_*`

Each widget file has the same three parts. The slider is a good example because it is short:

```rust,ignore
// rust/src/android/widgets/slider.rs
pub(crate) fn update_slider(env: &mut Env<'_>, view: &JObject<'_>, props: &SliderProps) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let max = props.max.max(props.min + 0.001);
    ids.slider.configure.call_void(env, view, &[
        JValue::Float(props.min),
        JValue::Float(max),
        JValue::Float(props.step.unwrap_or(0.0)),
        JValue::Int(props.color.argb() as jint),
    ])?;
    let value = snap_slider_value(props.value, props.min, max, props.step);
    ids.slider.set_value_silently.call_void(env, view, &[JValue::Float(value)])
}

pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.slider;
    site.framework(env, &ids.class, &ids.ctor)
}

pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
    pub(crate) set_value_silently: Method,
}
```

`create` only constructs. `update_slider` applies every property. `Handles` lists the ids, with a
`load` function that resolves them.

A few widgets deviate, each for a visible reason:

- `button.rs`: the view is a stock `android.widget.Button`, which has no node id of its own. So
  `create` sets the root view as its `OnClickListener`, and the root finds the node through the
  view's tag. `remove_native_chrome` strips the all-caps label, minimum size and padding, because
  the layout engine already reserves the button's padding.
- `text.rs`: `update_text` serves both `Text` and `Button`, which share label properties
  (`text_properties` picks the right fields). It also disables font padding so the rendered
  metrics match what `measure.rs` measured.
- `check.rs`: one file for `Checkbox` and `RadioButton`, with a `radio` flag.
- `progress.rs`: a determinate bar and an indeterminate spinner, both `ProgressBar`.
- `image.rs`: the view is a stock `ImageView`; loading goes through the `KrabikImages` helper.

### `configure_view`, shared by create and update

```rust,ignore
// rust/src/android/widgets/mod.rs
pub(crate) fn configure_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    kind: &NodeKind,
    density: f32,
) -> JniResult<()> {
    match kind {
        NodeKind::Root | NodeKind::Column | NodeKind::Row | NodeKind::Spacer | NodeKind::Scroll(_) => Ok(()),
        NodeKind::Widget(Widget::Text(_)) | NodeKind::Widget(Widget::Button(_)) => update_text(env, view, kind),
        NodeKind::Widget(Widget::TextField(props)) => update_text_field(env, view, props),
        NodeKind::Widget(Widget::Switch(props)) => update_switch(env, view, props),
        // ... one arm per widget, no `_ =>`
        NodeKind::Recycler(props) => update_recycler(env, view, props, density),
    }
}
```

This function is the fix for a real bug (commit `e000272`). Earlier, creation and update were two
separate code paths, and the update path ended in a catch-all arm. `RadioGroup`, `TabBar`,
`Pager`, `DatePicker` and `Spinner` were configured when created and then silently ignored every
later change on Android; swiping a pager page never moved the tab highlight, for example. Now:

- there is exactly one function that applies a node's own properties, and both `create_view` and
  `update_view` call it, so a widget cannot be configured one way on creation and another way (or
  not at all) on update;
- the match is exhaustive, with no `_ =>` arm, so adding a variant to `Widget` or `NodeKind` is a
  compile error here until it is handled.

`update_view` is `configure_view` plus the three things every view has: padding, style (with
animation allowed) and accessibility.

There are still two matches per platform, `create_widget` and `configure_view`. Both are
exhaustive, so the compiler lists them when a widget is added, but it is not yet "one line per
widget".

One gap worth knowing: `NodeKind::Scroll(_)` is in the do-nothing arm. A scroll container's axis
is fixed when the view is created (`ScrollView` or `HorizontalScrollView`). The diff recreates a
view only when the node's kind changes variant, and a vertical and a horizontal scroll are the
same variant, so changing the axis of an existing scroll node produces an `Update` that changes
nothing. The same is true on iOS.

## `style.rs`: what every view has

- `apply_padding` converts logical points to pixels and calls `View.setPadding`.
- `apply_style` computes the decoration and makes **one** JNI call, `KrabikStyle.apply`, with 23
  arguments: background, gradient, four corner radii, border, opacity, ripple, elevation and the
  animation. Building the drawable in Kotlin keeps this to a single crossing per node. When the
  node is disabled, the colours the DSL paints are dimmed here with `dim_color`; native text and
  controls dim themselves. `animate` is false on creation (nothing to animate from) and true on
  update.
- `apply_accessibility` sets `enabled` and the content description. It always sets `enabled`,
  so removing `.enabled(false)` restores the view.
- `set_tag`, `apply_frame`, `apply_layouts` were covered above.
- `apply_size` and `apply_content_size` size a scroll container's content host. A `ScrollView`
  measures its child with an unspecified constraint and ignores its layout params, so the size is
  also set as the minimum width and height.

## `measure.rs`: sizes the engine cannot compute

The layout engine needs to know how big a piece of text is. Only the platform knows.
`AndroidMeasure` implements the engine's `Measure` trait with three methods.

**Text.** It keeps one off-screen `TextView`, configured like a rendered one (no font padding, no
padding). `measure_with` sets the text, size, typeface and line limits on it, calls
`View.measure` with a width constraint, and reads the measured size. Results are cached in a
`HashMap` keyed by content, size, weight, width, line limit and ellipsis, because Taffy asks for
the same size several times per layout pass.

The `Measure` trait gives no JNI environment, so `measure` obtains one with
`vm.attach_current_thread`. On the main thread, which is where layout runs, that returns the
existing environment.

**Switch.** `measure_switch` constructs a plain `android.widget.Switch`, measures it once, and
caches the result. This is what the `plain_class` and `plain_ctor` handles in
`widgets/switch.rs` are for.

**Image.** `measure_image` asks `KrabikImages.size` for the pixel size of a bundled asset. Kotlin
reads only the image header and packs width and height into one `long`. Remote images have no
intrinsic size; the app must size them.

A failed measurement logs and returns a zero size (or the default switch size) rather than
failing the render.

## `rows.rs`: recycled lists

A `List` in the DSL becomes a `RecyclerView`. The list's rows are not part of the host's main
tree. Instead the runtime keeps a *row source* (the item count and a function from index to
element), and rows are built on demand when the recycler asks for them.

### The round trip

```text
 RecyclerView needs a row
        |
        v
 RowAdapter.onBindViewHolder        (KrabikRecycler.kt)
        |  root.bindRow(node, index, holderId, holder.host, width)
        v
 nativeBindRow                      (entry.rs)
        |
        v
 AndroidState::bind_row             (rows.rs)
   build the row's tree, lay it out, diff against what this
   holder showed, apply the ops into holder.host
        |
        v
 returns the row height in pixels
```

A *holder* is one reusable row container. Kotlin gives each holder a numeric id. What a holder
shows is remembered by a `RowBinder` keyed by `(list node, holder id)`.

### `bind_row` and the shared `RowBinder`

Deciding what a row needs is the same on both platforms, so it lives in one place:
`RowBinder` in `rust/src/host/rows.rs`. Android's `bind_row` asks it and applies the answer:

```rust,ignore
// rust/src/android/rows.rs (trimmed)
let key = (list, holder);
let width = width_px.max(1) as f32 / self.density;
let mut pending = match self.rows.begin(&mut self.runtime, &key, list, index, width) {
    Bind::Missing => return Ok(0),
    Bind::Unchanged { height } => return Ok(self.row_height_px(height)),
    Bind::Changed(pending) => *pending,
};

if pending.fresh {
    // The holder's previous widgets are unknown; the ops build the row from nothing.
    jni_handles(env)?.remove_all_views.call_void(env, host, &[])?;
}
// ... snapshot for the row transition
for op in std::mem::take(&mut pending.ops) {
    self.apply_op(env, op, &mut pending.views, host, false, None)?;
}
// ... fade the snapshot out
let height_px = self.row_height_px(pending.height);
self.rows.finish(key, pending);
```

`RowBinder::begin` does the thinking:

```rust,ignore
// rust/src/host/rows.rs (trimmed)
let tree = runtime.build_subtree(&element, tree::row_root(list, index));
let reapply = runtime.take_row_reapply(&tree);

// A render rebinds every attached row. Most of them did not change:
// keep the widgets and frames, and only take over the new handlers.
if reapply.is_none()
    && let Some(row) = self.rows.get_mut(key)
    && row.list == list && row.index == index && row.width == width && row.tree == tree
{
    row.tree = tree;
    // ... re-record the row's actions
    return Bind::Unchanged { height };
}
```

This is the **unchanged-row fast path**. After every app message the recycler is told to rebind
its attached rows (the Kotlin adapter does this when the list's `revision` changes), because any
row might have changed. Most have not. If the same holder is asked for the same item at the same
width and the new tree equals the old one, `begin` returns `Unchanged`: no layout, no diff, no
JNI calls. The one exception is `reapply`: a control in this row that the user just changed is
updated even when the description is equal (see the session chapter).

Why is `row.tree = tree` assigned when the two are equal? Because handlers compare equal by
presence only (a re-created closure does not count as a change), so two "equal" trees can hold
different closures. The new tree has the current ones. The row's actions are then recorded again
from it, so a tap runs the closure built by the latest `view`.

The slow path, still inside `begin`:

1. Lay the row out at the given width with an effectively unbounded height.
2. Take what the holder showed before. If nothing is known (`fresh`), the holder was recycled
   without Rust hearing about it: the diff runs against an empty tree and the platform removes
   the holder's children first.
3. `render::diff_reapplying` and `render::diff_layout` produce the ops. They are returned in a
   `Pending` together with the holder's views.

The platform then applies the ops with `apply_op` (`register = false`, no frame batch) and calls
`finish`, which records the row's actions and node ids and stores the row. The split into
`begin` and `finish` exists because applying the ops is the one part the binder cannot do.

If the list asked for a row transition, `KrabikRows.snapshot` captures the holder's old
appearance before the ops run and `KrabikRows.fadeOut` fades that overlay away afterwards.

**A holder that is given another item gets new views.** Row node ids are derived from the list
and the item index (`tree::row_root`), so two items never share ids and the diff between them is
"remove the old row, create the new one". Views are reused when the *same* item is bound again,
which is what every render does for the visible rows.

Because the binder is ordinary Rust with no JNI in it, it is tested on the host
(`rust/src/host/rows.rs`, from `a_first_bind_creates_the_row_and_records_its_handlers` on).

### Actions and focus for rows

A widget inside a row is not in the main tree, so the session cannot find its handlers there.
The binder records them instead:

- node id to `Actions`, for every interactive widget in a bound row. The session reads it
  through `Host::row_actions`, which calls `RowBinder::actions`.
- `(list, index)` to the node ids it contributed, so a rebind can clear them.
- node id to `(list, index)`, used to route focus events (`Host::row_of`, `RowBinder::item_of`).

Text-field focus needs care because a focused field can scroll off screen and its holder be
reused. The runtime keeps a *logical* focus (`row_focus`). When a holder is about to show another
item or is unbound while it contains the focused field, the binder calls `park_row_focus` to
remember it. When the row is attached again, `nativeRestoreRowFocus` gives the field focus back.

`prune_row_actions` runs after every render and calls `RowBinder::prune`. It drops actions for
item indexes that no longer exist and drops the rows of lists that left the tree, so their views
can be collected.

### Creating and updating the recycler

`create_recycler` calls `KrabikRecyclers.create` with the item count and the fixed row height in
pixels (zero means "measure each row"). `update_recycler` calls `KrabikRecyclers.update` with the
count, height and the list's `revision`, a counter the engine bumps whenever the app's description
was rebuilt. Chapter 3 shows what the adapter does with it.

## Rough edges in this directory

- The blocking entry points rely on Android never calling them during a render (see above).
- JNI signatures are strings checked only at run time.
- `radio_group.rs` and `tab_bar.rs` build their `String[]` by hand although `string_array!` in
  `mod.rs` does the same thing.
- Several comments describe an older state (noted where they occur).

## Where this lives

- `rust/src/android/entry.rs`: `native_create`, `native_render`, `on_event`,
  `lock_states_for_event`, `native_destroy`.
- `rust/src/android/host.rs`: `AndroidState::render`, `apply_op`, `create_view`.
- `rust/src/android/handles.rs`: `Jni`, `Jni::load`, `get`.
- `rust/src/android/widgets/mod.rs`: `configure_view`, `create_widget`, `Site`.
- `rust/src/android/rows.rs`: `bind_row`, `unbind_row`, `prune_row_actions`.
- `rust/src/frame_batch.rs`: `FrameBatch`.

Next: [The Android shell](03-the-android-shell.md).
