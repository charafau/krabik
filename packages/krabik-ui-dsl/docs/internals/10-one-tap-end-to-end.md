# 10. One tap, end to end

This chapter follows two user actions through every function on the core side, in call order, and
names the file each function is in. The first is a tap on a button in the showcase app. The second
is one character typed into a text field inside a recycled list row, which takes the row path and
both fast paths. Use it as a recap of chapters 2 to 9, or as a map: each step links back to the
chapter that explains it.

Two markers show where platform code takes over. That code is described in
[`../platforms/`](../platforms/).

```text
[P1]  native listener -> platform entry point -> Session    (events in)
[P2]  Runtime::update returns ops -> native views           (operations out)
```

A third crossing happens in the middle of layout, when the engine asks the platform to measure
text. It is marked `[measure]`.

## Trace 1: tapping "Add one" in the showcase

### The starting state

The showcase (`example/rust/src/`) is a navigation app. The user opened the Button page, so the
application's stack is `["home", "button"]` and the session has three hosts:

```text
host 0   Main               not rendered (the app has navigation)
host 1   Screen("home")     the catalog, covered by the Button page
host 2   Screen("button")   the page on screen
```

The Button page is built by `button_page` in `example/rust/src/pages.rs`:

```rust,ignore
// example/rust/src/pages.rs
fn button_page(app: &ShowcaseApp) -> Element<Msg> {
    page(
        "Button",
        "Clicks dispatch typed events; disabled buttons ignore input.",
        body![
            Text::new(format!("Taps: {}", app.taps)).size(20).color(COLOR_TEXT),
            Button::new("Add one").on_click(Msg::Tap),
            Button::new("Reset").on_click(Msg::ResetTaps),
            Button::new("Disabled").enabled(false),
        ],
    )
}
```

`page` wraps the body in a `ScrollView` containing a padded `Column` with a title and a caption.
The node tree host 2 holds after the last render is therefore:

```text
Root                       virtual
  Scroll  #S               native view
    Column                 virtual (padding 24, no decoration)
      Text  "Button"       native view
      Text  caption        native view
      Text  "Taps: 0"  #T  native view
      Button "Add one" #B  native view   on_click: set
      Button "Reset"       native view   on_click: set
      Button "Disabled"    native view   on_click: none
```

`#T` and `#B` stand for the node ids of the counter text and the tapped button. Both are hashes of
their path: root, then the scroll node, then the column, then "tag 4 at index 2" and "tag 5 at
index 3" (chapter 4).

### Step 1 [P1]: the native event reaches the session

The native button's click listener knows three numbers that were given to it when the view was
created: the session handle, the host id (2) and the node id (`#B`). It calls a platform entry
point:

- Android: the JNI function exported as `nativeOnClick` in `rust/src/android/entry.rs`, which
  calls `on_node_event(env, handle, host, node_id, NodeEvent::Click)`. That goes through
  `on_event`, which takes the session lock with `try_lock` and drops the event if a render holds
  it.
- iOS: `dispatch_click` in `rust/src/ios/events.rs`, which calls `dispatch`.

Both end in the same call, and from here on the code is shared:

```rust,ignore
session.node_event(cx, 2, NodeId(B), NodeEvent::Click)
```

### Step 2: find the handler and make the message

`Session::node_event` in `rust/src/host/session.rs` (chapter 8):

1. `self.hosts.get_mut(&2)` finds the host.
2. The event is neither `Scroll` nor `Focus`, so no engine state changes; `engine_changed` stays
   `false`.
3. `actions_for::<P>(state, id)`:
   - `tree::find(host.runtime().tree(), id)` in `rust/src/engine/tree.rs` walks the tree stored by
     the last render and returns the node `#B`.
   - `Actions::of(node)` in `rust/src/host/events.rs` matches `Widget::Button(props)` and clones
     `props.on_click` into `actions.click`. Something is set, so it returns `Some(actions)`.
4. `Actions::message(&NodeEvent::Click)` calls `self.click.message(())`.
5. `Handler::message` in `rust/src/message.rs` runs the stored closure. That closure was created
   by `Handler::new(move |()| message.clone())` inside the generated `Button::on_click`, during
   the last call to `button_page` (chapter 2). It returns `Box::new(Msg::Tap)` as a
   `Box<dyn Any>`.

For a widget with no handler set, such as the "Disabled" button, `Actions::of` returns `None` and
`node_event` returns without doing anything.

### Step 3: the application's `update`

`node_event` has a message, so it calls `Session::update(cx, message)`:

```rust,ignore
// rust/src/host/session.rs
fn update(&mut self, cx: &mut P::Cx<'_>, message: Message) -> Result<(), P::Error> {
    self.apply(message);
    self.render_all(cx)
}
```

`self.app` is a `Box<dyn AnyApp>`. `AnyApp::update` in `rust/src/app.rs` (the blanket
implementation, chapter 9) downcasts the box to `Msg` and calls `Component::update`:

```rust,ignore
// example/rust/src/lib.rs
Msg::Tap => self.taps += 1,
```

The state is now `taps == 1`. Nothing has been told that anything changed.

### Step 4: `render_all`, host by host

`Session::render_all` in `rust/src/host/session.rs`:

1. `self.app.navigation()` goes through `AnyApp::navigation` to `ShowcaseApp::navigation`, which
   returns the two routes. `has_navigation` is `true`.
2. The loop visits the three hosts in no particular order.

**Host 0 (Main).** `HostKind::Main if has_navigation => None`. Skipped.

**Host 1 (the home screen, covered).**

- `self.app.screen("home")`: `AnyApp::screen` calls `ShowcaseApp::screen`, which builds the body
  with `pages::screen(self, "home")` and an `AppBar`, and returns a `Scaffold<Msg>`.
  `Scaffold::erase` in `rust/src/scaffold.rs` splits it into `(RawElement, Chrome)`. The loop keeps
  the body.
- `host.runtime_mut().set_color_scheme(self.scheme)`.
- `host.render(cx, &element)`. This is the platform's `Host::render`; its first line calls
  `Runtime::update(element, viewport)` in `rust/src/engine/runtime.rs` (chapter 7):
  - The viewport is unchanged.
  - `Runtime::build` calls `tree::build_with_focus`, which runs `build_node` over the whole
    catalog and computes every id with `node_id`; then `tree::resolve_colors`.
  - `tree::stamp_recycler_revision` finds no recyclers.
  - `first == self.tree`: the home page does not depend on `taps`. Every id, property and style
    is equal, and every handler compares equal because both sides are set. **Fast path.**
  - The revision is bumped; focus is checked; the bookkeeping maps are replaced.
  - `render::diff(&self.tree, &next)` returns an empty list. `diff_layout` is not called.
  - `self.tree = next`, so the handlers of this render replace the previous ones.
  - Returns `Update { ops: [] }`.
- The platform loops over zero operations.

Cost for this host: one call to `ShowcaseApp::screen`, one tree build, one tree comparison. No
layout, no text measurement, no native call.

**Host 2 (the Button page).**

- `self.app.screen("button")` runs `button_page` again. The counter text is now `"Taps: 1"`. Two
  new `on_click` closures are created.
- `host.render(cx, &element)` calls `Runtime::update`:

  **Build.** `tree::build_with_focus` produces the same tree shape with the same ids. The only
  difference from the stored tree is inside node `#T`: `TextProps::content` is `"Taps: 1"`.

  **Compare.** `first == self.tree` is `false`.

  **Layout.** `layout::compute` in `rust/src/engine/layout.rs` (chapter 5):
  - `build_taffy` creates one Taffy node per `UiNode`, with `taffy_style` translating each
    `Style`. The children of the scroll node get `flex-shrink: 0`.
  - `compute_layout_with_measure` runs flexbox. For each leaf it calls the closure, which calls
    `taffy::compute_leaf_layout`, which calls `content_size`.
  - `content_size` calls `Widget::intrinsic_size`, which forwards to the widget's
    `IntrinsicSize` implementation in `rust/src/widgets/`. For the three texts that is
    `TextProps::intrinsic_size`, which calls `measurer.measure(text, constraints)`
    **[measure]**: the platform measures the string with the native font. For the three buttons it
    is `ButtonProps::intrinsic_size`, which measures the label the same way and adds the button
    padding.
  - `collect_frames` reads every node's position, adds up parent offsets, and stores absolute
    frames. `scroll_content_size` records the content size of `#S`.

  Every text on the page is measured again, although only one changed. Layout has no partial mode.

  **Heights.** `Runtime::record_heights` has no variable-height rows to look at and returns
  `false`. The loop ends after one pass.

  **Revision and focus.** Bumped; nothing to stamp; nothing focused.

  **Diff.** `render::diff` in `rust/src/engine/render.rs` (chapter 6):
  - It indexes the old tree and computes renderable parents for both trees. `#T`'s parent is `#S`
    in both, because the column between them is virtual.
  - It walks the new tree in pre-order. `#S`: same kind and style, nothing. The column: not
    renderable, skipped. The title and caption: equal, nothing. `#T`: same id, same parent, but
    `previous.kind != node.kind` because the content differs, so one
    `Op::Update` for `#T`, whose `RenderNode` has `parent: Some(S)` and the new `TextProps` with
    `content: "Taps: 1"`.
    The three buttons: equal. Their closures are new, and `Handler`'s `==` does not look at them.
  - No removals.

  **Layout diff.** `render::diff_layout` converts both layouts with `relative_frames` and compares
  each renderable node's frame. The column has `align_items(Align::Start)`, so the counter text is
  as wide as its measured content. If "Taps: 1" measures to a different width than "Taps: 0", `#T`
  gets an `Op::Layout { id: T, frame, content_size: None, animation: None }`; if the widths are
  equal it gets nothing. Its height is unchanged, so nothing below it moves. The scroll content
  size is unchanged.

  **Commit.** `self.layout = layout; self.tree = next`. Returns

  ```text
  Update { ops: [ Update(#T) ] }              or
  Update { ops: [ Update(#T), Layout(#T) ] }
  ```

### Step 5 [P2]: the operations are applied

Back in the platform's `Host::render` (`AndroidState::render` in `rust/src/android/host.rs`, and
its counterpart in `rust/src/ios/host.rs`), a loop applies each op. For `Update(#T)` it looks up
the native view registered for `#T` and sets the label's properties from the `RenderNode`. On
Android a `Layout` op is collected into a `FrameBatch` (chapter 6) and sent after the loop in one
call; on iOS it is applied directly.

The screen now shows "Taps: 1".

### Step 6: `sync` finds nothing to do

`render_all` ends with `Session::sync` (chapter 8):

1. `self.app.navigation()` again: `[home, button]`, equal to the mirror. Not navigated.
2. `sync_chrome(navigated = false)` calls `top_chrome`, which calls `navigation()` a third time
   and `screen("button")` a second time, keeping only the `Chrome`: an app bar titled "Button".
   Equal to the mirror, so `set_chrome` is not called. No snackbar before, none now.
3. `self.app.dialog()` is `None`, as before.
4. `self.app.sheet()` is `None` and there is no sheet host.
5. `self.app.back()` is `Some(Msg::Back)` because the stack has two screens. It was already
   enabled.

No controller command is sent. `sync` returns, `render_all` returns, `node_event` returns, and
control goes back to the platform's event loop.

### The whole trace on one screen

```text
[P1] click listener -> nativeOnClick / dispatch_click
  Session::node_event                          host/session.rs
    actions_for
      tree::find                               engine/tree.rs
      Actions::of                              host/events.rs
    Actions::message -> Handler::message       host/events.rs, message.rs
    Session::update
      AnyApp::update -> downcast               app.rs
        ShowcaseApp::update   (taps += 1)      the application
      Session::render_all
        AnyApp::navigation                     app.rs
        host 1: AnyApp::screen -> Scaffold::erase      app.rs, scaffold.rs
                Host::render                   platform
                  Runtime::update              engine/runtime.rs
                    tree::build_with_focus     engine/tree.rs
                    tree::resolve_colors
                    == previous tree           fast path
                    render::diff -> []         engine/render.rs
        host 2: AnyApp::screen -> Scaffold::erase
                Host::render                   platform
                  Runtime::update
                    tree::build_with_focus
                    tree::resolve_colors
                    layout::compute            engine/layout.rs
                      Widget::intrinsic_size   widgets/*.rs
                        [measure] Measure::measure     platform
                    Runtime::record_heights
                    render::diff -> [Update(#T)]
                    render::diff_layout -> [] or [Layout(#T)]
[P2]              apply ops to native views    platform
        Session::sync                          host/session.rs
          top_chrome -> AnyApp::screen         (chrome unchanged)
```

Counts for one tap: `App::navigation` three times, `App::screen` three times (once per screen host
and once for the chrome), two tree builds, one layout, one or two operations.

## Trace 2: typing one character in a recycled list row

The showcase's recycled list has buttons in its rows, not text fields. The harness app has the
case we want, on its "List (recycled, form)" screen (`harness/rust/src/lib.rs`):

```rust,ignore
// harness/rust/src/lib.rs, in list_recycled_form
let notes = Arc::clone(&self.recycled_notes);
// ...
List::new(Arc::clone(&self.recycled_form_rows))
    .item_height(56.0)
    .recycled()
    .key(|index| index.to_string())
    .row(move |index| {
        let index = *index;
        Row::new()
            .width_percent(1.0)
            .padding_each(EdgeInsets::symmetric(12, 6))
            .align_items(Align::Center)
            .child(
                TextField::new(notes[index].clone())
                    .placeholder(format!("Note {}", index + 1))
                    .on_change(move |text| Msg::Note(index, text))
                    .size(15)
                    .grow(1.0),
            )
            .into()
    })
```

Two hundred notes, one text field per row. The harness has no navigation, so everything renders in
the main host through `Component::view`.

### The starting state

On a device, `tree::build_list` turns the recycled list into one node (chapter 4). The main tree:

```text
Root
  Column                    virtual
    Button "Back"           (and a title, a caption, a "Refresh" button)
    Recycler #L             item_count 200, item_height 56, revision 41
```

The rows are not in this tree. The runtime keeps the list's item source:
`row_sources[#L] = ListProps { rows: <closure>, .. }`.

The native recycler has bound, say, rows 0 to 11. For each bound row the *platform* holds a small
tree whose root id is `tree::row_root(#L, index)`:

```text
row 3:   Root  (id = row_root(#L, 3))    virtual
           Row                           virtual (padding only)
             TextField  #F3              native view   on_change: set
```

and a table from node id to `Actions`, filled when the row was bound, which contains
`#F3 -> Actions { text: set, .. }`.

The user has focus in row 3 and types "h". The runtime's `row_focus` is
`Some(RowFocus { list: #L, index: 3, field: #F3, parked: false })`.

### Step 1 [P1]: the event arrives

The native field's text listener calls the platform's text entry point (`nativeOnTextChanged` on
Android, `dispatch_text` on iOS) with the host id, `#F3` and the new text. Shared code starts at:

```rust,ignore
session.node_event(cx, 0, NodeId(F3), NodeEvent::Text("h".into()))
```

### Step 2: the handler comes from the row table

`Session::node_event`:

1. Not a scroll, not a focus event.
2. `actions_for::<P>(state, id)`:
   - `tree::find(host.runtime().tree(), #F3)` returns `None`. The field is not in the main tree.
   - `.or_else(|| host.row_actions(id).cloned())` asks the platform host, which finds `#F3` in its
     table of recorded row actions.
3. `Actions::message(&NodeEvent::Text(value))` calls `self.text.message(value.clone())`.
4. `Handler::message` runs the closure from `.on_change(move |text| Msg::Note(index, text))`,
   which captured `index == 3`. Result: `Box::new(Msg::Note(3, "h"))`.

This is the only place the row path differs on the way *in*: one extra lookup.

### Step 3: `update`

`Session::update` calls `AnyApp::update`, which downcasts and calls the harness's `update`:

```rust,ignore
// harness/rust/src/lib.rs
Msg::Note(index, text) => {
    if let Some(note) = Arc::make_mut(&mut self.recycled_notes).get_mut(index) {
        *note = text;
    }
}
```

`recycled_notes[3]` is now `"h"`.

### Step 4: the main tree takes the fast path

`Session::render_all` renders the main host: `self.app.view()` builds the screen's element, and
`Host::render` calls `Runtime::update`.

1. **Build.** `tree::build_with_focus` reaches the list. In `build_list`, the mode is `Recycled`
   and the target is a device, so it stores `props.clone()` in `BuildInfo::recycled` and returns a
   childless `Recycler` node with `RecyclerProps::new(200, Some(56.0))`, revision 0. No row closure
   is called.
2. **First stamp.** `tree::stamp_recycler_revision(&mut first, self.revision)` writes 41, the
   revision the stored tree carries.
3. **Compare.** Nothing in the main tree shows the note text. `first == self.tree` is `true`.
   **Fast path: no layout.**
4. **Second stamp.** `self.revision` becomes 42 and is written into the new tree's `Recycler`.
5. **Focus.** `row_focus` points at list `#L`, index 3; `info.recycled` has `#L` with 200 items;
   the focus is kept.
6. **Bookkeeping.** `self.row_sources = info.recycled`. The stored `rows` closure is now the one
   from this render. It captured the new `Arc` of notes, in which note 3 is `"h"`.
7. **Diff.** `render::diff` compares the stored tree (revision 41) with the new one (42). The
   `Recycler` node has the same id and the same `NodeKind` variant, but `previous.kind !=
   node.kind`. One `Op::Update` for `#L`. Everything else is equal. `diff_layout` is skipped.
8. Returns `Update { ops: [ Update(#L) ] }`.

### Step 5 [P2]: the platform rebinds the visible rows

The platform applies `Update(#L)` to the native recycler. The changed revision tells its adapter
to rebind the rows that are attached (chapter 4 explains why this signal exists). For each of the
twelve visible rows the platform's bind function runs (`bind_row` in `rust/src/android/rows.rs`,
and the equivalent in `rust/src/ios/rows.rs`). It is platform code, but each step is a call into
the core:

```text
for index in 0..12:
  source  = Runtime::row_source(#L)              engine/runtime.rs
  element = (source.rows)(index)                 the application's closure
  tree    = Runtime::build_subtree(&element,
                tree::row_root(#L, index))       engine/runtime.rs, tree.rs
              -> tree::build_at + tree::resolve_colors
  compare tree with the tree this holder showed last
```

**Rows 0, 1, 2, 4 .. 11.** The row closure builds a `TextField` with the same note as before. The
tree is equal to the cached one. This is the **row fast path**: the views and frames are kept, the
cached tree is replaced by the new one, and the row's actions are recorded again (`Actions::of`
over the new tree), so the handlers of this render are the ones the next event finds. No layout,
no native call.

**Row 3.** The closure builds `TextField::new("h")`. The tree differs from the cached one in
`#F3`'s `value`. The slow path for one row:

```text
  layout = Runtime::layout_tree(&tree, (width, 100000))  engine/runtime.rs
             -> layout::compute                          engine/layout.rs
                  TextFieldProps::intrinsic_size         widgets/text_field.rs
                    [measure] Measure::measure           platform
  ops    = render::diff(&old_tree, &tree)                engine/render.rs
             -> [ Update(#F3) ]
         + render::diff_layout(&old_tree, &tree,
                               &old_layout, &layout)
             -> []   (normally: the field fills the row, same frame)
  apply ops to the holder's views                        platform
  record the row's actions and nodes                     platform
```

`Update(#F3)` carries `TextFieldProps { value: "h", .. }`. The native field already shows "h",
because the user typed it there. The platform's update for a text field has to recognize that and
leave the text and cursor alone; that logic, and the guard against the echo it would otherwise
cause, are in the platform chapters. (On Android the guard is visible in step 1 of trace 1: an
event that arrives while a render holds the lock is dropped.)

### Step 6: `sync`

The harness has no navigation stack, and nothing else `sync` looks at changed. It compares, finds
no difference and sends nothing.

### What this trace shows

```text
[P1] text listener -> nativeOnTextChanged / dispatch_text
  Session::node_event
    actions_for
      tree::find                    -> None (not in the main tree)
      Host::row_actions             -> the handlers recorded at bind time
    Actions::message -> Handler::message -> Msg::Note(3, "h")
    Session::update
      AnyApp::update -> GalleryApp::update
      Session::render_all
        AnyApp::view
        Host::render
          Runtime::update
            tree::build_with_focus  (list -> one Recycler node)
            stamp(41); == previous tree            main-tree fast path
            stamp(42)
            render::diff -> [ Update(#L) ]
[P2]      apply: recycler rebinds 12 rows
            11 rows: build_subtree; == cached tree    row fast path
             1 row:  build_subtree; layout_tree;
                     diff -> [ Update(#F3) ]; apply
        Session::sync               (nothing)
```

For one keystroke: the application's `view` runs once; the main tree is built and compared but not
laid out; twelve row closures run and twelve small trees are built and compared; one row is laid
out; two operations reach native views (`Update(#L)` and `Update(#F3)`).

That is the design working as intended, and also its current limit. Both fast paths avoid layout
and native calls for everything that did not change. But the amount of Rust work per keystroke is
proportional to the size of the screen plus the number of visible rows, because nothing tells the
engine that this message only touched note 3. `TODO.md` records it under nice-to-haves: "a way for
`update` to say 'nothing visible changed' would avoid that."

## The two traces side by side

| | Button tap (navigation app) | Keystroke in a recycled row |
|---|---|---|
| Handler found in | the host's main tree | the platform's row action table |
| Hosts rendered | 2 (one unchanged) | 1 |
| Main tree | changed, laid out | unchanged, fast path |
| Layout passes | 1 full | 1 for a single row |
| Ops | `Update(text)`, maybe `Layout(text)` | `Update(recycler)`, then `Update(field)` in the row |
| Who runs the row pipeline | not applicable | the platform, calling `build_subtree`, `layout_tree`, `diff` |

## Where this lives

In call order:

- `rust/src/android/entry.rs` (`on_node_event`) and `rust/src/ios/events.rs` (`dispatch`): where
  events enter.
- `rust/src/host/session.rs`: `node_event`, `actions_for`, `update`, `render_all`, `sync`,
  `top_chrome`.
- `rust/src/host/events.rs`: `Actions::of`, `Actions::message`.
- `rust/src/message.rs`: `Handler::message`.
- `rust/src/app.rs`: `AnyApp::update`, `AnyApp::screen`, `AnyApp::view`.
- `rust/src/scaffold.rs`: `Scaffold::erase`.
- `rust/src/engine/runtime.rs`: `Runtime::update`, `build`, `record_heights`, `row_source`,
  `build_subtree`, `layout_tree`.
- `rust/src/engine/tree.rs`: `build_with_focus`, `build_node`, `node_id`, `build_list`,
  `build_at`, `row_root`, `resolve_colors`, `stamp_recycler_revision`, `find`.
- `rust/src/engine/layout.rs`: `compute`, `content_size`.
- `rust/src/engine/render.rs`: `diff`, `diff_layout`, `relative_frames`.
- `rust/src/android/host.rs`, `rust/src/android/rows.rs`, `rust/src/ios/host.rs`,
  `rust/src/ios/rows.rs`: where the operations are applied and rows are bound.

This is the last chapter of the tour. The story continues on the other side of `[P1]` and `[P2]`
in [`../platforms/`](../platforms/). Back to the [index](README.md).
