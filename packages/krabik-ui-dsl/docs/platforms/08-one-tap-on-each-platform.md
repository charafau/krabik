# One tap on each platform

This chapter follows a single tap on a native button through the platform code, twice: first on
Android, from the Kotlin click listener to `Session::node_event` and from the resulting
`Op::Update` back to a `setText` call; then the same on iOS. Every function and file on the way
is named. It is a good test of the previous chapters: if each step here makes sense, you
understand how the two renderers are put together.

The part in the middle, from `Session::node_event` to the list of `Op`s, is the same on both
platforms and is traced in
[`../internals/10-one-tap-end-to-end.md`](../internals/10-one-tap-end-to-end.md). This chapter
covers what happens before and after it.

## The example

A counter: a text that shows a number and a button that increases it.

```rust,ignore
impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Add => self.count += 1,
        }
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .child(Text::new(format!("Count: {}", self.count)))
            .child(Button::new("+1").on_click(Msg::Add))
            .into()
    }
}
```

The app has no navigation, so everything is in host 0, the main root. The user taps "+1". The
text must change from "Count: 0" to "Count: 1".

Three numbers identify the button for the whole trip: the session handle, the host id (0) and the
button's node id.

## The shape of the trip

```text
   native button                                      native label
        |                                                  ^
        | (1) platform delivers the tap                    | (5) set the text
        v                                                  |
   platform event code                              platform renderer
        |                                                  ^
        | (2) NodeEvent::Click + three ids                 | (4) Op::Update
        v                                                  |
   Session::node_event --> app.update --> view --> diff ---+
                 (3) the shared core, see ../internals/
```

## Android

### Before the tap: how the button was set up

When the button's node was created, `AndroidState::create_view` (`rust/src/android/host.rs`)
called `create_widget`, which called `button::create`:

```rust,ignore
// rust/src/android/widgets/button.rs
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let view = site.stock(env, &ids.button.class, &ids.button.ctor)?;
    remove_native_chrome(env, &view)?;
    ids.set_on_click_listener
        .call_void(env, &view, &[JValue::Object(site.root)])?;
    Ok(view)
}
```

The view is a stock `android.widget.Button`. Its click listener is `site.root`, the
`RustRootView` of this host. A little later `create_view` called `set_tag`
(`rust/src/android/style.rs`), which stored the node id on the view as a boxed `Long`. Those two
facts are all the tap needs.

### Step 1: Kotlin receives the click

Android delivers the touch on the main thread and calls the listener:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt
override fun onClick(view: View) {
    val nodeId = view.tag as? Long ?: return
    nativeOnClick(handle, hostId, nodeId)
}
```

`handle` and `hostId` are fields of the root view; `nodeId` comes from the tag. `nativeOnClick`
is an `external fun`, so the JVM jumps into the native library.

(A framework widget such as `KrabikSwitch` would instead call a wrapper like
`root.onCheckedChanged(nodeId, isChecked)`, which calls its `external fun`. The button is the one
widget that goes through the view tag.)

### Step 2: the JNI entry point

The JVM looks up the symbol `Java_dev_krabik_ui_RustRootView_nativeOnClick`, which
`native_name!("nativeOnClick")` produced:

```rust,ignore
// rust/src/android/entry.rs
#[unsafe(export_name = native_name!("nativeOnClick"))]
pub extern "system" fn native_on_click<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnClick", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Click)
    });
}
```

Four functions, all in `rust/src/android/`:

1. `run` (`mod.rs`) turns the raw environment into an `Env` and will swallow and log any error.
2. `on_node_event` (`entry.rs`) wraps the ids: `NodeId(node_id as u64)`, `host as u64`.
3. `on_event` (`entry.rs`) calls `lock_states_for_event`, which is `states().try_lock()`. The
   user really tapped, so nothing else holds the lock and it succeeds. `on_event` then finds the
   `AndroidSession` for `handle`.
4. The closure calls `session.core.node_event(env, host, id, NodeEvent::Click)`.

That is the hand-over to the shared core. The `Env` goes along as the platform context `Cx`,
because the core will need it to call back into the renderer.

### Step 3: the core (summary)

`Session::node_event` (`rust/src/host/session.rs`) finds the button's `Actions` with
`actions_for`, turns `NodeEvent::Click` into the app's message with `Actions::message`
(`rust/src/host/events.rs`), calls `app.update(message)`, and then `render_all`. See
`../internals/10-one-tap-end-to-end.md` for each of those steps.

`render_all` asks every host to render. For host 0:

```rust,ignore
// rust/src/host/session.rs
host.runtime_mut().set_color_scheme(self.scheme);
host.render(cx, &element)?;
```

### Step 4: back in the Android renderer

`host.render` is the trait method `Host<Android>::render` (`rust/src/android/controller.rs`),
which forwards to `AndroidState::render` (`rust/src/android/host.rs`):

```rust,ignore
// rust/src/android/host.rs (trimmed)
let update = self.runtime.update(element, self.viewport);
let mut views = std::mem::take(&mut self.views);
let mut frames = FrameBatch::new();
for op in update.ops {
    self.apply_op(env, op, &mut views, self.root.as_obj(), true, Some(&mut frames))?;
}
```

`runtime.update` is the engine. For this tap it returns one property operation, an `Op::Update`
for the text node (its `content` changed), and possibly an `Op::Layout` if "Count: 1" measures
differently from "Count: 0". The button did not change. Its handler was rebuilt by `view`, but
handlers compare equal by presence, so it produces no op.

`apply_op` matches the update:

```rust,ignore
// rust/src/android/host.rs
Op::Update(node) => {
    if let Some(handle) = views.get(&node.id) {
        update_view(env, handle.view.as_obj(), &node, self.density)?;
    } else {
        self.create_and_insert(env, node, register, views, root_host)?;
    }
}
```

`handle.view` is the global reference to the `TextView` created for this node.

### Step 5: down to `setText`

`update_view` (`rust/src/android/widgets/mod.rs`) calls `configure_view`, whose match sends a
`Widget::Text` to `update_text` (`rust/src/android/widgets/text.rs`):

```rust,ignore
// rust/src/android/widgets/text.rs (trimmed)
pub(crate) fn update_text(env: &mut Env<'_>, view: &JObject<'_>, kind: &NodeKind) -> JniResult<()> {
    let Some((content, size, color, weight)) = text_properties(kind) else {
        return Ok(());
    };
    let ids = jni_handles(env)?;
    // ...
    let string = jcall!(env, env.new_string(content))?;
    ids.set_text
        .call_void(env, view, &[JValue::Object(string.as_ref())])?;
    // ... text size, letter spacing, colour, typeface, line limits, gravity
}
```

- `text_properties` reads `"Count: 1"` out of the `TextProps`.
- `env.new_string` creates a Java `String`; `jcall!` checks for a pending exception.
- `ids.set_text` is the cached method id of `TextView.setText(CharSequence)`, resolved once in
  `Jni::load` (`rust/src/android/handles.rs`).
- `call_void` goes to `Method::call`, which calls `env.call_method_unchecked`.

That is the `setText` call. The JVM runs `TextView.setText`, and the view schedules a redraw.

Note that `update_text` then sets every other text property again (size, colour, typeface, and so
on), although only the content changed. An update applies the whole node, not a per-property
delta. Each is one JNI call with a cached id.

`update_view` continues with `apply_padding`, `apply_style` (one call to `KrabikStyle.apply`) and
`apply_accessibility` (`rust/src/android/style.rs`).

### Finishing

If there was a layout op, `apply_op` pushed the frame into the `FrameBatch`, and after the loop
`apply_layouts` sends the batch to `RustRootView.applyLayouts` in one call. `prune_row_actions`
runs. Back in the session, `render_all` calls `sync`, which compares navigation, chrome, dialog
and sheet with what the controller last received; nothing changed, so no controller method is
called.

Then everything returns: `node_event`, the closure, `on_event` (releasing the lock), `run`,
`native_on_click`, and finally Kotlin's `onClick`.

**The whole trip is synchronous.** When `onClick` returns, the `TextView` already holds "Count:
1". Android draws it on the next frame.

### The Android call chain

```text
 Android touch dispatch                       main thread
 RustRootView.onClick(view)                   RustRootView.kt
 nativeOnClick(handle, hostId, nodeId)        RustRootView.kt
 -- JNI --
 native_on_click                              android/entry.rs
   run                                        android/mod.rs
   on_node_event -> on_event                  android/entry.rs
     lock_states_for_event (try_lock)         android/entry.rs
 Session::node_event                          host/session.rs
   actions_for, Actions::message              host/session.rs, events.rs
   app.update(message)
   Session::render_all                        host/session.rs
     Host<Android>::render                    android/controller.rs
     AndroidState::render                     android/host.rs
       Runtime::update                        engine: tree, layout, diff
       AndroidState::apply_op(Op::Update)     android/host.rs
         update_view -> configure_view        android/widgets/mod.rs
           update_text                        android/widgets/text.rs
             Method::call                     android/handles.rs
 -- JNI --
             TextView.setText(CharSequence)   Android
         apply_padding, apply_style,
           apply_accessibility                android/style.rs
       apply_layouts                          android/style.rs
         RustRootView.applyLayouts            RustRootView.kt
     Session::sync                            host/session.rs
```

## iOS

### Before the tap: how the button was set up

`IosState::create_handle` (`rust/src/ios/host.rs`) called `create_widget`, which called
`button::create`:

```rust,ignore
// rust/src/ios/widgets/button.rs
pub(crate) fn create(props: &ButtonProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let button = UIButton::buttonWithType(UIButtonType::System, site.mtm);
    update_button(&button, props);
    wire_click(&button, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(button.clone());
    ViewHandle::new(view, Content::Button(button), node)
}

pub(crate) fn wire_click(button: &UIButton, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.click",
        mtm,
        move |_| schedule(move || dispatch_click(handle, host, id)),
    );
}
```

`add_action` (`rust/src/ios/mod.rs`) wraps the Rust closure in an Objective-C block, builds a
`UIAction` around it and adds it to the button for `TouchUpInside`. The three ids are not stored
on the view; they are captured inside the closure. No Swift code is involved, here or anywhere
else on this trip.

### Step 1: UIKit runs the action

The user lifts a finger inside the button. UIKit, on the main thread, invokes the `UIAction`'s
handler block. The block calls the Rust closure, which does one thing:

```rust,ignore
// rust/src/ios/mod.rs
pub(crate) fn schedule(work: impl FnOnce() + Send + 'static) {
    DispatchQueue::main().exec_async(work);
}
```

It queues `dispatch_click(handle, host, id)` on the main queue and returns to UIKit. Nothing has
touched the session yet.

### Step 2: one main-queue turn later

When the current main-thread work is done, the queue runs the closure:

```rust,ignore
// rust/src/ios/events.rs
pub(crate) fn dispatch_click(handle: u64, host: u64, id: NodeId) {
    dispatch(handle, host, id, NodeEvent::Click);
}

pub(crate) fn dispatch(handle: u64, host: u64, id: NodeId, event: NodeEvent) {
    // SAFETY: events are delivered on the main queue.
    let mut mtm = unsafe { MainThreadMarker::new_unchecked() };
    guard("event", || {
        with_session(handle, |session| {
            infallible(session.node_event(&mut mtm, host, id, event));
        });
    });
}
```

1. `MainThreadMarker::new_unchecked` produces the platform context; the closure is known to run
   on the main queue.
2. `guard` (`rust/src/ios/mod.rs`) will catch an Objective-C exception raised anywhere below.
3. `with_session` locks the state map, blocking, and finds the session.
4. `session.node_event(&mut mtm, host, id, NodeEvent::Click)` is the hand-over to the core.

For a click the deferral was not strictly needed: nothing else was holding the lock. It is
applied to every event so that the cases where it *is* needed (an event fired from inside a
render) need no special handling. Chapter 4 has the argument.

### Step 3: the core (summary)

Identical to Android: `actions_for`, `Actions::message`, `app.update`, `render_all`. The only
difference is the type of the context that is passed through, `MainThreadMarker` instead of `Env`.

### Step 4: back in the iOS renderer

`host.render` is `Host<Ios>::render` (`rust/src/ios/controller.rs`), which forwards to
`IosState::render` (`rust/src/ios/host.rs`):

```rust,ignore
// rust/src/ios/host.rs (trimmed)
let update = self.runtime.update(element, self.viewport);
let Some(root) = self.root.load() else { return; };
let mut views = std::mem::take(&mut self.views);
for op in update.ops {
    self.apply_to(op, &mut views, &root, true);
}
```

The same `Op::Update` for the text node arrives in `apply_to`:

```rust,ignore
// rust/src/ios/host.rs (trimmed)
Op::Update(node) => {
    if let Some(handle) = views.get_mut(&node.id) {
        // ... take a style snapshot if the node has a style animation
        update_content(&mut handle.content, &node, self.handle, self.host, mtm);
        apply_style(&handle.view, &node, &mut handle.gradient);
        apply_accessibility(&handle.view, &node, mtm);
    }
    // ...
}
```

### Step 5: down to `setText`

`update_content` (`rust/src/ios/widgets/mod.rs`) matches the widget against the stored content:

```rust,ignore
// rust/src/ios/widgets/mod.rs
(Widget::Text(props), Content::Label(label)) => update_label(label, props),
```

`label` is the `Retained<UILabel>` kept in the `ViewHandle` since creation. No lookup, no
downcast.

```rust,ignore
// rust/src/ios/widgets/text.rs
pub(crate) fn update_label(label: &UILabel, props: &TextProps) {
    label.setText(Some(&NSString::from_str(&props.content)));
    let font = system_font(props.size, props.weight);
    // SAFETY: the font is retained by the label for as long as it is set.
    unsafe { label.setFont(Some(&font)) };
    // ... colour, number of lines, line break mode, alignment
}
```

`label.setText(...)` is the `objc2-ui-kit` binding for the Objective-C message `setText:` on
`UILabel`. That is the call. As on Android, the rest of the text properties are set again too.

### Finishing

`apply_style` and `apply_accessibility` (`rust/src/ios/style.rs`) run. If there was a layout op,
`IosState::apply_layout` builds a `LayoutFrames` and calls `apply_rect`
(`rust/src/ios/motion.rs`), which sets the frame of the wrapper view and insets the label inside
it. There is no batch. `prune_rows` runs, the session's `sync` finds nothing to tell the
controller, and the calls return: `node_event`, `with_session` (releasing the lock), `guard`,
`dispatch`. The main queue moves on, and Core Animation commits the change at the end of the
run-loop turn.

### The iOS call chain

```text
 UIKit touch handling                         main thread
 UIAction handler block                       ios/mod.rs (add_action)
   closure from wire_click                    ios/widgets/button.rs
     schedule (exec_async on main queue)      ios/mod.rs
 ---- next main-queue turn ----
 dispatch_click -> dispatch                   ios/events.rs
   guard                                      ios/mod.rs
   with_session (blocking lock)               ios/mod.rs
 Session::node_event                          host/session.rs
   actions_for, Actions::message              host/session.rs, events.rs
   app.update(message)
   Session::render_all                        host/session.rs
     Host<Ios>::render                        ios/controller.rs
     IosState::render                         ios/host.rs
       Runtime::update                        engine: tree, layout, diff
       IosState::apply_to(Op::Update)         ios/host.rs
         update_content                       ios/widgets/mod.rs
           update_label                       ios/widgets/text.rs
             UILabel.setText:                 UIKit, via objc2
         apply_style, apply_accessibility     ios/style.rs
       IosState::apply_layout                 ios/host.rs
         LayoutFrames::apply_rect             ios/motion.rs
     Session::sync                            host/session.rs
```

## Side by side

| | Android | iOS |
| --- | --- | --- |
| Where the listener lives | Kotlin: `RustRootView.onClick` | Rust: a closure in a `UIAction` |
| How the node is identified | the view's tag (button) or the widget's own `nodeId` field | captured in the closure |
| Native code on the way in | Kotlin, then JNI | none |
| Timing | synchronous, inside the click callback | one main-queue turn later |
| Protection against re-entry | `try_lock`, drop the event | `schedule`, delay the event |
| Context passed through the core | `Env` | `MainThreadMarker` |
| How the label is reached | global reference + cached method id | typed `Retained<UILabel>` in `Content` |
| The final call | `TextView.setText` through `call_method_unchecked` | `UILabel.setText:` through `objc2` |
| Frames | batched into one `applyLayouts` call | set directly, one view at a time |
| Errors | `JniResult`, logged by `run` | none in the types; exceptions caught by `guard` |

Everything between the two halves, the part that decides *what* changes, is one piece of code
shared by both.

## Variations worth tracing yourself

- **A button inside a `List` row.** The node is not in the host's main tree. `actions_for` falls
  back to `Host::row_actions`, which reads the map filled by `bind_row`
  (`rust/src/android/rows.rs`, `rust/src/ios/rows.rs`). After the update, the list's `revision`
  changes; Android's adapter rebinds its attached rows and iOS's `sync_recycler` rebinds the
  visible cells, each through the unchanged-row fast path.
- **A button on a pushed screen.** The host id is not 0. On Android the `RustRootView` that
  receives the click belongs to a `KrabikScreenFragment` and passes its own `hostId`.
- **A tap that navigates.** `sync` then finds a different navigation stack and calls
  `Controller::push`, which reaches `KrabikHostController.push` (Kotlin) or `krabikPush:` (Swift).
  The new screen's host attaches on a later turn and is rendered then.
- **A switch.** On Android the event starts in `KrabikSwitch`'s listener and the update ends in
  `setCheckedSilently`, which is why the write does not come back as another event.

## Where this lives

- Android in: `RustRootView.kt` (`onClick`), `rust/src/android/entry.rs` (`native_on_click`,
  `on_node_event`, `on_event`).
- Android out: `rust/src/android/host.rs` (`render`, `apply_op`),
  `rust/src/android/widgets/mod.rs` (`update_view`, `configure_view`),
  `rust/src/android/widgets/text.rs` (`update_text`), `rust/src/android/handles.rs`
  (`Method::call`).
- iOS in: `rust/src/ios/widgets/button.rs` (`wire_click`), `rust/src/ios/mod.rs` (`add_action`,
  `schedule`), `rust/src/ios/events.rs` (`dispatch`).
- iOS out: `rust/src/ios/host.rs` (`render`, `apply_to`), `rust/src/ios/widgets/mod.rs`
  (`update_content`), `rust/src/ios/widgets/text.rs` (`update_label`).
- The core between them: `rust/src/host/session.rs` (`node_event`, `render_all`) and
  [`../internals/10-one-tap-end-to-end.md`](../internals/10-one-tap-end-to-end.md).

This is the last chapter of this part. Back to the [index](README.md).
