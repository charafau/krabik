# Debug a render

This recipe is a procedure for finding out why something on screen is wrong: a widget that is
missing, in the wrong place, showing stale content, or not reacting. It walks the pipeline in
order and gives one concrete check per stage, then lists the causes that have actually occurred
in this codebase, each with the code and the source comment that explains it. After it you will
be able to say which stage a bug is in before you open a platform file.

Background: the stages themselves are explained in [the engine chapters](../internals/README.md)
and [the platform chapters](../platforms/README.md). Reminders are given where a term is first
used.

## The pipeline, and where to cut it

```text
   your view()                                       host test, no device
       |
  [1]  Element          is the description what you expect?
       |  tree::build
  [2]  UiNode tree      does the node exist, with which id, and does it own a view?
       |  layout::compute
  [3]  frames           is it where you expect, at the size you expect?
       |  render::diff
  [4]  Vec<Op>          was a Create / Update / Remove / Layout emitted for it?
  -----|-----------------------------------------------------------------------
       |  AndroidState::apply_op / IosState::apply_to        device only
  [5]  native view      did the platform apply the op?
       |
  [6]  event            does the touch come back as your message?
```

Everything above the line runs on your machine with `cargo test`. Always start there. Most
"wrong on screen" bugs are already wrong at stage 1, 2 or 4, and a host test shows them in
seconds. Only when the ops are right is it worth attaching a device.

The fastest way to work is to reproduce the screen in a throwaway test in your app crate, print
the three things, and read them.

## Step 1: is the element what you expect?

Reminder: `view` returns an `Element<Msg>`, a plain description. `tree::build` turns it into the
internal tree of `UiNode`s; each has an `id`, a `kind`, a `style` and `children`. Both `UiNode`
and its parts derive `Debug`.

```rust
use krabik_ui_dsl::tree;
use krabik_ui_dsl::ui::{Button, Column, Element, Row, Text};

fn screen(count: u32) -> Element {
    Column::new()
        .padding(16)
        .child(Text::new(format!("Count: {count}")))
        .child(
            Row::new()
                .background(0xFFEE_EEEE)
                .child(Button::new("+1").on_click(())),
        )
        .into()
}

fn print_tree() {
    let root = tree::build(&screen(0));
    println!("{root:#?}");
}
```

Read the dump for three things:

- **Is the node there at all?** If not, the bug is in your `view` (a condition, an empty list, a
  `match` arm). No framework code has run yet.
- **Are its properties right?** The widget's properties are inside
  `kind: Widget(Text(TextProps { content: "...", .. }))`. A handler prints as `Handler(set)` or
  `Handler(none)`; `Handler(none)` on something that should react means the `.on_...` call is
  missing or was applied to a different builder.
- **Is the nesting what you meant?** The tree always starts with a synthetic `Root`, then your
  top element.

One limit: `tree::build` does not resolve adaptive colours and builds lists as if scrolled to the
top (its doc comment: "Lists are materialized at offset zero"). For colours and scrolled lists use
a `Runtime`, as in step 2.

## Step 2: does the node own a view, and where is it?

Reminder: the `Runtime` keeps the current tree and its layout. `runtime.frames()` maps every node
id to its absolute frame in logical points. Not every node becomes a native view:

```rust,ignore
// rust/src/engine/tree.rs
    /// Whether the node owns a native view.
    ///
    /// Native widgets always do. Layout nodes stay virtual unless they paint
    /// something, in which case they need a decoration view of their own.
    pub fn is_renderable(&self) -> bool {
        self.kind.is_native() || self.style.needs_view()
    }
```

Print both facts for every node:

```rust
use krabik_ui_dsl::geometry::Size;
use krabik_ui_dsl::measure::ApproximateMeasure;
use krabik_ui_dsl::runtime::Runtime;
use krabik_ui_dsl::tree;
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

fn screen(count: u32) -> Element {
    Column::new()
        .padding(16)
        .child(Text::new(format!("Count: {count}")))
        .child(Button::new("+1").on_click(()))
        .into()
}

fn print_frames() {
    let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
    runtime.update(&screen(0), Size::new(360.0, 640.0));

    for node in tree::preorder(runtime.tree()) {
        println!(
            "{:?} view={} frame={:?}",
            node.id,
            node.is_renderable(),
            runtime.frames().get(&node.id),
        );
    }
}
```

What to look for:

- **`view=false` on the node you are looking for.** It has no native view and never will. See
  [A node that has no view](#a-node-that-has-no-view).
- **A width or height of 0.** The node exists but takes no space. Usual reasons: an empty text
  (`ApproximateMeasure` and the native measurers return `Size::ZERO` for it), a remote image with
  no explicit size (`Measure::image_size` returns `None` for URLs, "and keep the current
  explicit-size requirement"), or a child of a container whose own size is 0.
- **A frame outside the viewport.** The parent is not a scroll view, or a sibling with `grow`
  pushed it out.
- **A frame that looks right here but wrong on the device.** `ApproximateMeasure` estimates text
  as `0.55 * size` per character. Real fonts differ, so text-dependent frames will not match the
  device exactly. The *structure* (who is below whom, who fills what) does. If the structure is
  right here, the difference is native measurement; look at `AndroidMeasure::measure_with` or
  `IosMeasure::measure_with`.

The frames here are absolute. The frame a native view receives is relative to its nearest
ancestor that owns a view, minus that ancestor's padding (`relative_frames` in
`rust/src/engine/render.rs`). Keep that in mind when you compare with numbers from a view
inspector.

## Step 3: which ops were emitted?

Reminder: an `Op` is one instruction for the platform: `Create`, `Update`, `Reparent`, `Remove`
or `Layout`. `Runtime::update` returns the ops for one render. `Runtime::render` hands them to a
`Renderer`; `RecordingRenderer` is one that only stores them.

```rust
use krabik_ui_dsl::geometry::Size;
use krabik_ui_dsl::measure::ApproximateMeasure;
use krabik_ui_dsl::render::{Op, RecordingRenderer};
use krabik_ui_dsl::runtime::Runtime;
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

fn screen(count: u32) -> Element {
    Column::new()
        .padding(16)
        .child(Text::new(format!("Count: {count}")))
        .child(Button::new("+1").on_click(()))
        .into()
}

fn print_ops() {
    let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
    let mut renderer = RecordingRenderer::default();
    let viewport = Size::new(360.0, 640.0);

    runtime.render(&screen(0), viewport, &mut renderer); // the "before"
    runtime.render(&screen(1), viewport, &mut renderer); // the change under test

    for op in &renderer.batches[1] {
        match op {
            Op::Create(node) => println!("create {:?} under {:?}", node.id, node.parent),
            Op::Update(node) => println!("update {:?}: {:?}", node.id, node.kind),
            Op::Reparent { id, parent } => println!("reparent {id:?} under {parent:?}"),
            Op::Remove { id, .. } => println!("remove {id:?}"),
            Op::Layout { id, frame, .. } => println!("layout {id:?} to {frame:?}"),
        }
    }
}
```

Reproduce the exact transition that goes wrong: render the state before, then the state after,
and read the second batch. The readings:

| You see | It means |
|---|---|
| an `Update` for the node, with the new properties | The engine is right. The bug is on the platform (step 4). |
| no op for the node, and you expected a change | The two trees are equal there. Either the state did not change, or the change is in something the tree does not compare. Handlers are the main case: they compare by presence only. |
| `Remove` + `Create` where you expected `Update` | The node's id changed. See [A missing key](#a-missing-key). |
| `Update`s for many siblings after inserting one | Same cause: ids are positional. |
| no ops at all | The whole tree is equal. `Runtime::update` then also skips layout. This is normal after a message that changes nothing visible. |
| only `Layout` ops | Nothing changed but positions, for example after a viewport change. |
| `Create` for a container plus `Reparent` for its children | The container gained a decoration (a background, a border) and now owns a view. |

If the ops are what they should be, stop testing on the host. The remaining stages need a device.

## Step 4: did the platform apply them?

Both platforms have one function that every op goes through. They are the place for a temporary
`crate::log::error(..)` line and the first place to read.

**Android**: `AndroidState::apply_op` in `rust/src/android/host.rs`. `Create` goes to
`create_and_insert`, which calls `create_view` (construct, then `configure_view`, padding, style,
accessibility, tag, register). `Update` goes to `update_view` in
`rust/src/android/widgets/mod.rs`, which runs the same `configure_view`. `Layout` ops are
collected into a `FrameBatch` and applied in one JNI call (`apply_layouts`) after the loop.

**iOS**: `IosState::apply_to` in `rust/src/ios/host.rs`. `Create` goes to `create_handle`, which
calls the widget's own `create`. `Update` goes to `update_content` in
`rust/src/ios/widgets/mod.rs`, then `apply_style` and `apply_accessibility`.

Checks, in order:

1. **Did the app start a session at all?** On Android, if the whole screen is blank, look in
   logcat for a Java exception described during `nativeCreate`. All JNI handles are resolved
   there (`Jni::load` in `rust/src/android/handles.rs`); one wrong class, method name or
   signature fails the lot, `run` returns the default handle `0`, and `RustRootView` then does
   nothing because every call is guarded by `if (handle != 0L)`.
2. **Did the op error?** Every Android entry point runs inside `run` in `rust/src/android/mod.rs`.
   On an error it describes and clears the Java exception, reports `"{operation} failed: {error}"`
   through `crate::log::error`, and returns. The render stops at the failing op: `AndroidState::render`
   breaks out of its loop on the first error, so later ops of that batch are not applied. On iOS
   the entry points run inside `guard` in `rust/src/ios/mod.rs`, which catches an Objective-C
   exception and reports `{label} handler raised an exception: ...`.
3. **Does the view exist natively?** Use the Layout Inspector (Android Studio) or the view
   debugger (Xcode). On Android every view Rust creates carries its node id as its tag
   (`set_tag` in `create_view`), and `RustRootView.registeredViewCount()` tells you how many are
   registered. A view that exists with a 0 x 0 size never received its `Layout`.
4. **Is it a property the update function does not apply?** Open the widget's `update_...`
   function and check that the property you changed is in it. See
   [A widget that only configured itself on create](#a-widget-that-only-configured-itself-on-create).

Where the messages go: `crate::log::error` (`rust/src/log.rs`) writes to the Android system log
under the tag `krabik`, so `adb logcat -s krabik` shows the Rust half of an error next to the
Java exception half (`exception_describe`). On iOS and under `cargo test` it writes to standard
error, which Xcode's console shows. A failed JNI lookup also names the class, method or
constructor it was looking for.

## Step 5: does the event come back?

If the view is right but pressing it does nothing:

1. **Does the node have a handler?** Step 1's dump shows `Handler(set)` or `Handler(none)`.
2. **Does `Actions::of` read that handler?** `rust/src/host/events.rs`. A widget in the "no
   actions" arm reports nothing however many handlers its properties hold.
3. **Is the event the widget sends the one its actions answer?** `Actions::message` maps each
   `NodeEvent` to one handler. The session test `a_click_reaches_the_application_and_rerenders`
   shows the consequence of a mismatch: "An event the widget did not ask for changes nothing."
4. **Did the native listener fire and reach Rust?** Android: the widget's Kotlin class calls a
   `RustRootView.on...` method, which calls an `external fun`. iOS: the closure installed by the
   widget's `wire_...` function calls a `dispatch_...` function.
5. **Was it dropped?** See [An Android event that never arrives](#an-android-event-that-never-arrives).
6. **Is the widget in a recycled row?** Then its handlers are not in the runtime's tree.
   `actions_for` in `rust/src/host/session.rs` falls back to `host.row_actions(id)`, which the
   platform fills when it binds the row. See [A recycled row that does not rebind](#a-recycled-row-that-does-not-rebind).

## Causes that have happened here

### A node that has no view

`Column`, `Row` and `Spacer` are layout nodes. They exist in the tree and get a frame, but they
get a native view only when they paint something:

```rust,ignore
// rust/src/style.rs
    pub fn needs_view(&self) -> bool {
        self.background.is_some()
            || self.gradient.is_some()
            || self.corner_radius > 0.0
            || self.corner_radii.is_some()
            || self.border.is_some()
            || self.opacity.is_some_and(|opacity| opacity < 1.0)
            || self.elevation > 0.0
    }
```

The diff skips everything else: the first line of its loop is
`if !node.is_renderable() { continue; }`.

Symptoms that come from this:

- **A style property on a plain container has no effect.** The list above is complete. An
  `.accessibility_label(...)` or `.enabled(false)` on an undecorated `Column` is stored in the
  style and never reaches a platform, because no view is created to carry it. In a UI test, a
  `withContentDescription` or an XCUITest query for that label finds nothing. Put the label on a
  widget inside, or give the container a background.
- **`opacity(1.0)` does not create a view, `opacity(0.99)` does.** So animating a container's
  opacity from 1.0 creates the view at the moment the animation starts.
- **Adding a background changes more than the background.** The node becomes renderable, so the
  diff emits a `Create` for it and a `Reparent` for each child that owns a view (the render tests
  `decorating_a_container_creates_it_and_reparents_its_children` and
  `removing_a_decoration_reparents_children_before_dropping_it` show both directions). Children's
  frames are then relative to the new view. If something jumps when a decoration appears, this is
  why.
- **The root has no view either.** On Android `create_view` returns a null object for
  `NodeKind::Root` with the comment "The root is the host view itself", and `create_and_insert`
  returns early.

Check: `node.is_renderable()` in step 2.

### A missing key

A node's id is a hash of three things:

```rust,ignore
// rust/src/engine/tree.rs
/// The identifier is stable while the node stays at the same position, which is
/// what keeps native widgets alive across rebuilds. Supplying a key keeps the
/// identifier stable when earlier siblings are inserted or removed; keys must
/// therefore be unique among siblings.
fn node_id(parent: NodeId, index: usize, key: Option<&str>, tag: u64) -> NodeId {
    // ...
    mix(parent.0);
    mix(tag);
    match key {
        Some(key) => mix(hash_string(key)),
        None => mix(index as u64),
    }
```

Without a key, identity is the position among siblings. Insert an item at the top of an unkeyed
column and every later sibling "becomes" the one before it: the first native text field now shows
the second item's text, and a new view is created at the end. The content looks right, because
every view was updated to its new content. What is wrong is everything the native view holds that
the tree does not: focus, the cursor and keyboard, a running animation, scroll position, an
image that was already loaded. Typical reports: "the keyboard closes when I add a row", "the
wrong row animates out", "focus jumps to the next field after deleting".

Three ways it shows in step 3:

- inserting or removing one child produces `Update`s for all later siblings instead of one
  `Create` or `Remove`;
- a child that only moved produces `Remove` + `Create` when the kind at its old position differs
  (the tag is part of the id, so a `Text` and a `Button` at the same index never share an id);
- the test `inserting_an_unkeyed_sibling_keeps_keyed_identifiers` in `rust/src/engine/tree.rs`
  shows the fix: with `.key("a")`, the ids survive the insertion.

Rules:

- Give `.key(...)` to every child built in a loop and to anything that appears conditionally
  *before* siblings with native state. A `List` requires a key function for the same reason.
- Keys must be unique among siblings. In a debug build a duplicate panics in `build_children`
  with `duplicate element key ... among the children of ...`; the comment there says why:
  "Duplicate keys would hash two siblings to the same identifier, which silently aliases their
  native views." In a release build the check is compiled out, and the two nodes share one view.
- A conditional child that is last, or a `Spacer` placeholder in the `else` branch, avoids the
  shift without keys.

Check: compare `node.id` for the same logical item before and after the change.

### A recycled row that does not rebind

Reminder: `List::recycled()` hands the rows to a native recycler (`RecyclerView`,
`UICollectionView`), which asks Rust for a row when it needs one. Those rows are not in the main
tree. That creates two ways for a row to go stale.

**The recycler does not know the content changed.** The main tree only contains one `Recycler`
node. If the item count and height are the same, its properties are equal and the diff emits
nothing, although the rows' content may differ. The fix is in the node's properties:

```rust,ignore
// rust/src/engine/tree.rs, in `RecyclerProps`
    /// A counter that changes whenever the application description was rebuilt.
    ///
    /// RecyclerView owns the rows and would otherwise never notice that an app
    /// state change altered the content of visible rows. A new revision asks
    /// the adapter to rebind the attached cells; the renderer diffs each row,
    /// so unchanged widgets are not touched.
    pub revision: u64,
```

`Runtime::update` stamps the revision into every recycler (`tree::stamp_recycler_revision`), the
changed number produces an `Op::Update`, and the platform's update asks the adapter to rebind
(`update_recycler` in `rust/src/android/rows.rs`; `data_source.update(props)` in
`IosState::apply_to`). If visible rows show old content after a state change, check that this
`Update` is emitted and reaches the adapter.

**The row is rebound, but the handlers are old.** A rebind of an unchanged row takes a shortcut:

```rust,ignore
// rust/src/host/rows.rs, in `RowBinder::begin` (shared by both platforms)
        // A render rebinds every attached row. Most of them did not change:
        // keep the widgets and frames, and only take over the new handlers.
        if reapply.is_none()
            && let Some(row) = self.rows.get_mut(key)
            && row.list == list
            && row.index == index
            && row.width == width
            && row.tree == tree
        {
            let height = row.height;
            row.tree = tree;
            self.clear_actions(list, index);
            let actions = actions_of(&self.rows[key].tree);
            self.store_actions(list, index, actions);
            return Bind::Unchanged { height };
        }
```

`state.tree == tree` is true even when the closures inside differ, because handlers compare by
presence. That is why the shortcut still replaces `state.tree` and re-records the actions: the
new tree carries the handlers of this render. iOS has the same branch, with nearly the same
comment, in `rust/src/ios/rows.rs`. If you change the shortcut, or add a second cache of row state, a row
button that sends a message built from old state ("delete item 3" after item 3 became item 2) is
the symptom.

**The holder was reused without notice.** `bind_row` handles a holder it has no state for by
clearing it and diffing against an empty tree ("The holder was recycled without telling us, so
its previous widgets are unknown."). And `nativeUnbindRow` may be dropped during a render; the
comment in `rust/src/android/entry.rs` explains why that is safe: "the next bind of that holder
rebuilds the row from scratch and overwrites the stale entry."

Two things to remember while debugging rows:

- **On the host there are no recyclers.** `build_list` in `rust/src/engine/tree.rs` creates a
  `Recycler` node only under `cfg!(any(target_os = "android", target_os = "ios"))`. In a host
  test a recycled list is an ordinary windowed list. Nothing in this section can be reproduced
  with `cargo test`; use the harness app's recycler tests.
- **Row node ids are in a separate space.** `row_root(list, index)` derives the row's root id
  from the list id and the item index, so the same item keeps its ids across rebinds and a
  holder that switches to another item sees entirely different ids (a full `Remove`/`Create` of
  the row's widgets).

### An Android event that never arrives

Every Android event entry point takes the session lock with `try_lock`, not `lock`:

```rust,ignore
// rust/src/android/entry.rs
/// Locks the state map for event dispatch, or gives up when a render holds it.
///
/// Event entry points and `nativeRender` all run on the Android main thread,
/// so a real user event can never race a render: while the render lock is
/// held, the main thread is inside `nativeRender` itself. Lock contention in
/// an event entry point therefore means the render synchronously triggered a
/// widget listener (for example `setTransformationMethod` calls `setText`,
/// which fires the input's text watcher). That callback re-enters the entry
/// point on the same thread, and `std` mutexes are not re-entrant, so
/// blocking on the lock would deadlock the UI. Such programmatic echoes are
/// dropped.
pub(crate) fn lock_states_for_event() -> Option<MutexGuard<'static, HashMap<jlong, AndroidSession>>>
{
    states().try_lock().ok()
}
```

So: any event that fires *while Rust is rendering* is discarded without a trace. `on_event`
returns `Ok(())` when the lock is taken.

For a listener that fires because Rust set a value, that is the intended behaviour. It becomes a
bug when a native widget delivers a *real* piece of information synchronously inside a render.
Candidates: a view that reports its initial state from its constructor or from `configure`, a
selection callback that fires when Rust changes the number of items, a focus change caused by a
view being removed. If an event is lost only when it coincides with a state change, suspect this.

How the existing widgets stay clear of it:

- They do not rely on the drop. `KrabikSwitch`, `KrabikSeekBar` and `KrabikRadioGroup` set a
  `silent` flag while Rust applies a value, and their listeners return early under it.
- They defer. `KrabikTextInput` reports focus changes with `post { ... }`, and its comment says
  why: "RecyclerView restores focus during a layout pass. Rendering from here would notify its
  adapter mid-layout, so both directions are reported on the next main-loop turn".

If your widget must report something that can happen during a render, `post` it in Kotlin so it
arrives after the lock is released.

iOS solves the same problem the other way round and loses nothing: every widget event goes
through `schedule` in `rust/src/ios/mod.rs`, which defers it to the next main-queue turn, because
"UIKit can deliver another action synchronously while we are rebuilding the tree (for example
`EditingDidEnd` when a focused field is removed)". The cost is that an iOS event is handled one
turn later than it happened, which is why the XCUITests always wait for results.

Check: a temporary `crate::log::error(..)` in the `None` branch of `on_event`.

### A widget that only configured itself on create

This was a real bug, fixed in commit `e000272` ("apply widget updates through the same path as
creation"). Its message:

> Android configured RadioGroup, TabBar, Pager, DatePicker and Spinner when they were created but
> ignored later changes, so for example swiping a TabsView page never moved the tab highlight.

The engine did everything right: the diff emitted `Op::Update` with the new selection. The
Android code that handled `Update` had a match with a catch-all arm, and those five widgets fell
into it. Step 3 would have shown a correct `Update`; step 4 was where it stopped.

The structural fix is `configure_view`:

```rust,ignore
// rust/src/android/widgets/mod.rs
/// Applies a node's own properties to its native view.
///
/// A new view and an updated view go through this same function, and the
/// match is exhaustive, so a widget cannot be configured on creation and then
/// ignore later changes.
pub(crate) fn configure_view(/* ... */) -> JniResult<()> {
    match kind {
        // ... one arm per node kind, no `_ =>`
    }
}
```

What is still possible today:

- **On iOS the two paths are separate.** Each widget's `create` calls its own update function by
  convention, and `update_content` handles later changes. Its match is exhaustive, but it has an
  arm that lists widgets whose stored content does not match and does nothing. A widget listed
  only there is created correctly and never updated. One existing widget is special-cased on
  purpose: `(Widget::Image(_), _) => {}`, with the comment "Images are updated by
  `IosState::load_image`, which starts downloads and therefore needs the state itself."
- **A property missing from the update function.** The match can be complete and the function it
  calls can still skip a field. On iOS, for instance, `update_content` handles the switch with
  `switch.setOn(props.checked)` and nothing else, which is right only as long as `SwitchProps`
  has one visual property.
- **A Kotlin `configure` that returns early.** `KrabikTextInput.configure` compares the incoming
  values with the ones it stored and returns when they are equal. If you add a parameter and
  forget to add it to that comparison, a change of only the new parameter is ignored.

Check: change exactly one property in a two-render test (step 3), confirm the `Update`, then read
the platform update function for that one property.

### The control shows something the application does not hold

Symptom: a switch looks on, the application's state says off; or a slider sits where the user
left it although the application clamped the value.

Native controls change themselves when touched. Rust is then told, the application updates, and
the next render writes the application's value back, whether or not the description changed:

```rust,ignore
// rust/src/host/session.rs, in `node_event`
        let changes_value = matches!(
            event,
            NodeEvent::Toggle(_)
                | NodeEvent::Text(_)
                | NodeEvent::Slider(_)
                | NodeEvent::Select(_)
                | NodeEvent::Picker(_)
        );
        if changes_value && message.is_some() {
            state.runtime_mut().reapply(id);
        }
```

A control with no handler produces no message and is left as the user set it.

`Runtime::reapply` names one node, and the next diff emits an `Update` for it even when its
properties are equal (`render::diff_reapplying`). A control inside a recycled row is not in the
host's tree; the runtime keeps its id until the row is bound again, and that bind skips the
"row unchanged" shortcut. The bottom navigation bar follows the chrome rule instead: the session
records the destination the user tapped, so a different one in the state is sent again.

So if a control still shows a value the application does not hold, the `Update` arrived and the
platform update function did not apply it. Look there: it must compare against what the native
view shows now, not against a value cached from the last render. The iOS pager used to do the
latter and stayed on a page the application had refused.

Check: step 3 shows an `Update` for the control after its own event; `RejectedInputTest` and
`testRejectedInput` in the harness app cover a switch, a tab bar, a slider, a text field and a
switch in a recycled row.

### Two widgets that share a tag

Not known to have happened, but the code allows it and the failure is confusing. Each `Widget`
variant has a numeric tag in the `widgets!` table (`rust/src/widgets/mod.rs`), "mixed into node
identifiers, so a node that changes widget type gets a new identity". Nothing checks that tags
are distinct. With a duplicate, replacing one widget by the other at the same position keeps the
id; the diff compares only the `NodeKind` discriminant, which is `Widget` for both, and emits an
`Update`. iOS ignores it (content mismatch arm). Android calls the new widget's Kotlin method on
the old widget's view.

Check: after adding a widget, read the table once for duplicate numbers.

## A short decision list

1. Print the tree. Node missing or wrong? Fix `view`.
2. Print `is_renderable` and the frame. No view? Zero size? Fix the style or the layout.
3. Print the ops for the transition. Wrong ops? It is identity (keys) or equality (nothing
   changed). Right ops? Go to the device.
4. On the device: blank app means handle loading; one widget wrong means its update function;
   stale list rows mean the revision or the row shortcut.
5. No reaction to touch: handler, `Actions::of`, the event variant, then a dropped event.

When you have found the stage, write the test at that stage before fixing it;
[Write tests](write-tests.md) shows the helper for each.

## Where this lives

- `rust/src/engine/tree.rs`: `build`, `preorder`, `find`, `node_id`, `UiNode::is_renderable`,
  `RecyclerProps`, `build_children`, `build_list`
- `rust/src/style.rs`: `Style::needs_view`
- `rust/src/engine/runtime.rs`: `Runtime::update`, `frames`, `tree`
- `rust/src/engine/render.rs`: `Op`, `diff`, `relative_frames`, `RecordingRenderer`
- `rust/src/host/events.rs`: `Actions::of`, `Actions::message`
- `rust/src/host/session.rs`: `node_event`, `actions_for`
- `rust/src/android/host.rs`: `apply_op`, `create_view`; `rust/src/android/widgets/mod.rs`:
  `configure_view`; `rust/src/android/rows.rs`: `bind_row`
- `rust/src/android/entry.rs`: `lock_states_for_event`, `on_event`; `rust/src/android/mod.rs`:
  `run`
- `rust/src/ios/host.rs`: `apply_to`, `create_handle`; `rust/src/ios/widgets/mod.rs`:
  `update_content`; `rust/src/ios/mod.rs`: `schedule`, `guard`

Back to the [index of recipes](README.md).
