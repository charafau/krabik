# 7. The runtime

This chapter covers `rust/src/engine/runtime.rs`. The `Runtime` is the object that remembers: the
previous tree, the previous frames, scroll positions, measured row heights and focus. Its `update`
method ties together everything from chapters 4 to 6. After reading this chapter you will know
exactly what one render does, when it skips work, and what each of the smaller methods is for.

## One runtime per host

A *host* is one native root view that Rust renders into: the main view, one navigation screen, or
the bottom sheet (chapter 8). Each host owns one `Runtime`. A runtime knows nothing about the
application, messages, navigation or the platform. Its interface is:

```text
RawElement + viewport  --update-->  Vec<Op>
scroll / focus reports --------->   "rebuild needed?" (bool)
```

That narrowness is why it can be tested with nothing but an element and a fake text measurer.

## The state it owns

```rust,ignore
// rust/src/engine/runtime.rs
pub struct Runtime {
    tree: UiNode,
    layout: LayoutResult,
    measurer: Box<dyn Measure>,
    /// Scroll offsets in logical points, keyed by the scroll node id.
    scroll_offsets: HashMap<NodeId, f32>,
    /// The extent of every list in the last built tree.
    lists: HashMap<NodeId, ListExtent>,
    /// Measured heights and prefix sums for variable-height lists.
    measurements: HashMap<NodeId, ListMeasurements>,
    /// The node that currently has native focus, if any.
    focused: Option<NodeId>,
    /// For each list, the item index that holds the focused node.
    focused_rows: HashMap<NodeId, usize>,
    /// Focus inside a recycled list, kept across the row's recycling.
    row_focus: Option<RowFocus>,
    /// Item sources of recycled lists, used to build a row on demand.
    row_sources: HashMap<NodeId, ListProps>,
    /// The viewport of the last update, needed to window lists on scroll.
    viewport: Size,
    /// Incremented on every rebuild; stamped into recycled lists so their
    /// adapters rebind the visible rows after an app state change.
    revision: u64,
    /// The appearance colours are resolved for.
    scheme: ColorScheme,
}
```

Grouped by purpose:

| Fields | Purpose |
|---|---|
| `tree`, `layout` | What is on screen now. The next render diffs against these. `tree` is also where events look up handlers. |
| `measurer` | The platform's `Measure` (chapter 5). |
| `viewport` | Detects a size change; used by `scroll` to compute windows. |
| `scroll_offsets`, `lists`, `measurements` | Windowed lists (chapter 4): where each is scrolled to, its extent, its measured row heights. |
| `focused`, `focused_rows` | Keeps the row with the focused field alive in a windowed list. |
| `row_focus`, `row_sources`, `revision` | Recycled lists: logical focus, how to build a row, the "rebind" signal. |
| `scheme` | Light or dark. |

A new runtime starts with `UiNode::empty()` and an empty layout, so the first update diffs against
nothing and creates everything.

## `update`, step by step

```rust,ignore
// rust/src/engine/runtime.rs
pub fn update(&mut self, element: &RawElement, viewport: Size) -> Update

pub struct Update {
    /// The size the root node occupies in the viewport.
    pub size: Size,
    /// The operations the renderer has to apply.
    pub ops: Vec<Op>,
}
```

The function is about a hundred lines. Here it is in nine steps.

### Step 1: note a viewport change

```rust,ignore
// A viewport change relayouts every node; animating that would make a
// rotation or split-screen resize look like a deliberate transition.
let suppress_animation = self.viewport != viewport;
self.viewport = viewport;
```

The flag is used twice below: it rules out the fast path (new viewport means new frames even for an
identical tree) and it strips animations from the layout ops.

### Step 2: build the tree

```rust,ignore
let (mut first, first_info) = self.build(element, viewport);
tree::stamp_recycler_revision(&mut first, self.revision);
```

`self.build` is a private helper:

```rust,ignore
fn build(&mut self, element: &RawElement, viewport: Size) -> (UiNode, BuildInfo) {
    let (mut tree, info) = tree::build_with_focus(
        element,
        &self.scroll_offsets,
        viewport,
        &mut self.measurements,
        &self.focused_rows,
        self.focused,
    );
    tree::resolve_colors(&mut tree, self.scheme);
    (tree, info)
}
```

It passes every piece of list state into the tree builder (chapter 4) and resolves colours for the
current scheme. The result is a complete node tree plus the `BuildInfo` side channel.

The stamp is explained in step 5. For now: `self.revision` is the number that was stamped into
`self.tree` at the end of the previous update, so stamping the same number into `first` makes any
`Recycler` nodes comparable.

### Step 3: the unchanged-tree fast path

```rust,ignore
// Most messages change little or nothing on a given host. When the
// rebuilt tree equals the current one, the frames are still valid, so
// the layout pass (and the native text measurement behind it) is
// skipped. The new tree still replaces the old one: it carries the
// handlers of this render.
let unchanged =
    !suppress_animation && first_info.reset_scroll.is_empty() && first == self.tree;
```

`first == self.tree` is the derived, recursive `PartialEq` on `UiNode`: same ids, same kinds with
all widget properties, same styles, same children. Layout is a pure function of the tree, the
viewport and the measurer. If the tree and the viewport are the same, the frames are the same, so
there is no reason to compute them.

This is the mechanism that pays for the "always call `view` again" design. Think about what
happens after a message in an application with three screens on the navigation stack. The session
re-renders all three hosts (chapter 8). The message probably changed something on the top screen
only. For the two screens underneath, `view` produces the same description, the tree comes out
equal, and the cost is: build the tree, compare it, done. No Taffy, no text measurement, no
operations.

The comparison works only because handlers compare equal by presence (chapter 2). Every render
makes new closures; if they counted as differences, no two trees would ever be equal.

The last sentence of the comment is the other half of that bargain. Even when the trees are equal,
`first` replaces `self.tree` at the end of the function, because `first` holds the closures of
*this* render, which capture this render's state. The test
`an_unchanged_render_still_takes_over_the_new_handlers` renders a button twice with messages 1 and
5: the second update is empty, and a click afterwards produces 5.

What the fast path does *not* save: building the tree (including calling the application's
`view`, row closures, and `Element::map`), resolving colours, and the comparison itself. Those are
linear in the size of the tree and involve no platform calls.

`first_info.reset_scroll.is_empty()` is a guard: a list whose items changed must go through the
slow path, where its scroll offset is dropped.

### Step 4: layout, and the variable-height convergence loop

```rust,ignore
let (mut next, info, layout) = if unchanged {
    (first, first_info, None)
} else {
    // Variable-height lists converge in a couple of passes: the first
    // one measures the rows it materialized, the next uses those
    // heights to recompute the prefix sums and the window. Cap the
    // loop so a row whose height oscillates with the window cannot
    // spin forever.
    let mut iteration = 0;
    let mut built = (first, first_info);
    loop {
        let (next, info) = built;
        for id in &info.reset_scroll {
            self.scroll_offsets.remove(id);
        }
        let layout = layout::compute(&next, viewport, self.measurer.as_ref());
        let changed = self.record_heights(&info, &layout);
        iteration += 1;
        if !changed || iteration >= 3 {
            break (next, info, Some(layout));
        }
        built = self.build(element, viewport);
    }
};
```

In the common case (no variable-height list) this is: lay out once, `record_heights` finds no rows
and returns `false`, leave the loop.

With a variable-height list there is a chicken-and-egg problem. Which rows to materialize depends
on their heights (how many fit in the viewport). Their heights are only known after layout. Layout
needs the rows to exist. The loop resolves it by iteration:

```text
viewport 200 pt tall, buffer 3, estimate 56 pt, real rows 20 pt

pass 1   build:   nothing measured, every row assumed 56 pt
                  -> window = rows 0..7
         layout:  those rows are really 20 pt each
         record:  7 heights changed  -> go again
pass 2   build:   prefix sums now know rows 0..7 are 20 pt
                  -> more rows fit: window = rows 0..12
         layout:  rows 7..12 measured
         record:  5 heights changed  -> go again
pass 3   build:   window = rows 0..13
         layout, record -> stop here whatever the result
```

`record_heights` is the function that feeds measurements back:

```rust,ignore
fn record_heights(&mut self, info: &BuildInfo, layout: &LayoutResult) -> bool {
    let mut changed = false;

    for (list_id, rows) in &info.rows {
        let Some(measurement) = self.measurements.get_mut(list_id) else { continue };

        for (row_id, key_hash, index) in rows {
            let Some(frame) = layout.frames.get(row_id) else { continue };
            let height = frame.height.max(0.0);
            let previous = measurement.heights.get(key_hash).copied().unwrap_or(measurement.estimate);
            if (previous - height).abs() > 0.5 {
                measurement.heights.insert(*key_hash, height);
                // Patch the cached prefix in place: every following item
                // shifts by the height delta. A full rebuild would be
                // O(n) per measured row.
                let start = index.saturating_add(1);
                if let Some(suffix) = measurement.prefix.get_mut(start..) {
                    let delta = height - previous;
                    for value in suffix {
                        *value += delta;
                    }
                }
                changed = true;
            }
        }
    }

    changed
}
```

For each row the build recorded (`BuildInfo::rows`, variable-height lists only), it reads the
row's frame height from the layout. If it differs from what was stored (or from the estimate) by
more than half a point, it stores the new height under the item's key hash and adds the difference
to every prefix sum after that row. The in-place patch keeps the cached prefix array valid, which
is what lets a list with a stable `revision` skip re-keying on the next build (chapter 4).

The half-point tolerance and the cap of three passes both exist to guarantee termination. A row's
height can depend on the window in rare cases, and floating-point noise could otherwise flip a
height back and forth. If heights are still changing after three passes, the third pass's tree and
layout are used as they are. The heights it measured are already stored, so the next update starts
from a better position. In practice lists settle in two passes when the estimate is reasonable, and
most updates after the first settle in one because the heights are cached.

Each extra pass is a full build and a full layout of the whole host, not only of the list.

### Step 5: bump and stamp the recycler revision

```rust,ignore
// RecyclerView does not know that the application description changed;
// a new revision is part of the node properties, so the diff emits an
// update and the adapter rebinds its attached cells.
self.revision = self.revision.wrapping_add(1);
tree::stamp_recycler_revision(&mut next, self.revision);
```

Chapter 4 explained why a `Recycler` node carries a revision: the rows are not in the tree, so
without it the diff could not tell the native recycler that row content may have changed.

Now the two stamps make sense together:

- In step 2 the fresh tree was stamped with the *old* revision, the same one `self.tree` carries.
  So in the equality test of step 3 a recycler compares equal to its previous self, and a screen
  with a recycled list can still take the fast path.
- Here the tree is stamped with the *new* revision. In step 8 the diff compares it with
  `self.tree` (old revision), finds the `Recycler` node's kind unequal, and emits one
  `Op::Update` for it.

The net effect for a screen with a recycled list where nothing in the main tree changed: no
layout, and a batch containing exactly one `Update` for the recycler. The platform reacts by
rebinding the visible rows, and each rebind is diffed against the row's previous tree (see
"Recycled rows" below).

So a host with a recycled list never produces an empty batch. That is the limit recorded in
`TODO.md`: "every message re-renders, which rebinds the visible rows of a recycled list; typing in
a recycled form therefore rebinds per keystroke."

### Step 6: drop focus that no longer exists

```rust,ignore
// A focused node that disappeared (for example a screen swap) cannot
// be pinned anymore.
let focus_lost = self.focused.is_some_and(|id| tree::find(&next, id).is_none());
if focus_lost {
    self.focused = None;
    self.focused_rows.clear();
}

if let Some(focus) = self.row_focus {
    // A screen switch or a shorter list invalidates the logical focus.
    let present = info.recycled.get(&focus.list).is_some_and(|props| focus.index < props.item_count);
    if !present {
        self.row_focus = None;
    }
}
```

Two kinds of focus are checked against the new tree (both are explained below). If the focused
node is gone, forget it. If the recycled list that held the focused row is gone or is now shorter
than the focused index, forget that too.

### Step 7: keep what the next call needs

```rust,ignore
self.lists = info.lists;
self.row_sources = info.recycled;
self.focused_rows = if focus_lost { HashMap::new() } else { info.focused_rows };
```

The `BuildInfo` of the final build becomes runtime state: list extents for `scroll`, item sources
for building recycled rows, and the pinned rows for the next build.

### Step 8: diff

```rust,ignore
let mut ops = render::diff(&self.tree, &next);
if let Some(layout) = layout {
    let mut layout_ops = render::diff_layout(&self.tree, &next, &self.layout, &layout);
    if suppress_animation {
        for op in &mut layout_ops {
            if let render::Op::Layout { animation, .. } = op {
                *animation = None;
            }
        }
    }
    ops.extend(layout_ops);
    self.layout = layout;
}
```

Property operations first, then layout operations (the ordering contract of chapter 6). On the
fast path `layout` is `None`: `diff_layout` is skipped and `self.layout` keeps the previous frames,
which are still correct.

`diff` runs even on the fast path. For equal trees it returns nothing; with a recycler it returns
the one `Update`.

### Step 9: commit

```rust,ignore
let size = self.layout.frames.get(&ROOT_ID)
    .map_or(viewport, |root| Size::new(root.width, root.height));

self.tree = next;

Update { size, ops }
```

The new tree becomes current. `size` is the root's frame, which always equals the viewport
(chapter 4); neither platform reads it.

### The whole thing as a picture

```text
update(element, viewport)
  |
  +- build tree (lists windowed with current offsets, colours resolved)
  |
  +- same viewport and tree == previous tree?
  |     yes ------------------------------+
  |     no                                |
  |      +- layout (Taffy + Measure)      |   fast path:
  |      +- record row heights            |   no layout,
  |      +- heights changed and           |   frames kept
  |         passes < 3?  rebuild, repeat  |
  |                                       |
  +<--------------------------------------+
  +- revision += 1, stamp recyclers
  +- drop stale focus
  +- ops  = diff(previous tree, tree)
  +- ops += diff_layout(...)          (slow path only)
  +- previous tree = tree
  v
Update { ops }
```

## Scrolling

Native scroll views scroll by themselves. Rust does not move content and never sets a native scroll
offset. It only needs to know the offset of *windowed lists*, to decide which rows should exist.

```rust,ignore
// rust/src/engine/runtime.rs
/// Records the scroll offset of a list and reports whether its window
/// changed and the UI has to be rebuilt.
pub fn scroll(&mut self, id: NodeId, offset: f32) -> bool
```

The platform reports every scroll event of every scroll container. `scroll` handles it in this
order:

1. **Is this node a windowed list?** `self.lists` holds only the lists of the last build. A plain
   `ScrollView`, a recycler or an unknown id returns `false` immediately; their offsets are not
   stored.
2. **Clamp** the offset to the list's range (`0 ..= total height - viewport height`), so an
   overscroll reported by the platform cannot produce a window outside the list.
3. **Store** it in `scroll_offsets`. If it equals the stored value, return `false`.
4. **Compare windows.** Compute the window for the old and the new offset and return whether they
   differ. For a fixed-height list that is two calls to `tree::fixed_window`. For a variable-height
   list it is one call to `tree::variable_window` on the cached prefix array, compared with the
   window the last build recorded in `ListMeasurements::window`.

The return value is the point. With 50-point rows and a buffer of 3, the window changes once every
50 points of travel. All the scroll events in between cost a clamp, a map insert and a little
arithmetic, and return `false`; the session then does nothing (chapter 8). Only when a row
boundary is crossed does it return `true`, and the session re-renders. That render builds the tree
with the new offset, the diff creates the rows that entered the window and removes the ones that
left, and layout runs.

The pinned row (below) is passed to both window computations, so scrolling far away from a focused
field does not report a window that would drop it.

**Rough edge.** A `true` from `scroll` causes a full `render_all`: the application's `view` runs
for every host, and the scrolled host is laid out in full. During a fast fling through a windowed
list that happens once per row. This is the cost of windowing in Rust, and the reason the recycled
mode exists: there the native recycler asks for single rows and no full render is involved.

**Rough edge.** `scroll_offsets` and `measurements` are keyed by node id and entries are never
removed when a list disappears from the tree (only on a `reset_scroll`). They live as long as the
host. Because ids are positional, a different list appearing later at the same position inherits
the entry; chapter 4 describes which cases reset it and which do not.

## Focus tracking for windowed lists

```rust,ignore
// rust/src/engine/runtime.rs
/// Records the focus of a native input.
///
/// The runtime keeps the row that holds the focused node materialized, so
/// scrolling does not tear the field down and lose its focus and text.
/// Returns whether the focused node changed.
pub fn set_focus(&mut self, id: NodeId, focused: bool) -> bool
```

The platform reports when a text field gains or loses focus. `set_focus` stores the id in
`self.focused` (or clears it), empties `focused_rows`, and returns whether anything changed.

From there the pinning described in chapter 4 takes over across two builds: the next build
receives `self.focused`, notices which list row contains that node, and reports it in
`BuildInfo::focused_rows`; step 7 stores that; the build after it receives it as the pin and keeps
that row in the window however far the list is scrolled.

The test `a_focused_row_stays_materialized_while_scrolling` walks through it: focus a field in row
2, scroll to offset 900, and the row is still in the tree; release focus, render, and it is gone.

The boolean matters to the session: a changed focus means the set of rows that must be kept may
have changed, so it triggers a render.

## Recycled rows

A recycled list is one `Recycler` node in the main tree (chapter 4). Its rows are built outside
`update`, on demand, when the native recycler asks to bind a row. The binding code is platform
code (`rust/src/android/rows.rs`, `rust/src/ios/rows.rs`), but it is built from four small runtime
methods, all `pub(crate)`:

```rust,ignore
// rust/src/engine/runtime.rs
pub(crate) fn row_source(&self, id: NodeId) -> Option<&ListProps>
pub(crate) fn recycled_item_count(&self, id: NodeId) -> Option<usize>
pub(crate) fn build_subtree(&self, element: &RawElement, root: NodeId) -> UiNode
pub(crate) fn layout_tree(&self, tree: &UiNode, viewport: Size) -> LayoutResult
```

- `row_source(list)` returns the `ListProps` saved in step 7, whose `rows` closure builds the
  element for an index. Because `row_sources` is replaced on every update, the closure always
  captures the application state of the latest render.
- `recycled_item_count(list)` is how the platform notices that a list is gone (`None`) or shorter.
- `build_subtree(element, root)` is `tree::build_at` followed by `tree::resolve_colors`: a small
  tree whose root id is `tree::row_root(list, index)`, with the same colour scheme as the main
  tree.
- `layout_tree(tree, viewport)` is `layout::compute` with the runtime's measurer. The platform
  passes the row's width and a very large height; the row's height is then the root's frame
  height.

The sequence for binding row `index` into a native holder, as the Android code does it:

```text
source  = runtime.row_source(list)                      core
element = (source.rows)(index)                          application closure
tree    = runtime.build_subtree(&element,
              tree::row_root(list, index))              core
if the holder already shows this index at this width
   and tree == the tree it showed:                      (row fast path)
       keep views and frames, re-record the handlers
else:
   layout = runtime.layout_tree(&tree, (width, huge))   core
   ops    = render::diff(&old_tree, &tree)              core
          + render::diff_layout(...)                    core
   apply ops to the holder's views                      platform
```

It is the same pipeline as `update` in miniature, run by the platform, with the row's previous
tree and layout kept by the platform per holder. It has the same fast path: a rebind that produces
an equal tree does no layout and touches no view. That is what makes "rebind every visible row on
every render" (step 5) tolerable: after a keystroke in one row, the other visible rows each cost a
build and a comparison.

The row's handlers are not in `self.tree`, so the session cannot find them there. The platform
records them per node id when it binds a row and exposes them through `Host::row_actions`
(chapter 8).

### Row focus and parking

A focused text field inside a recycled row has a harder version of the focus problem. The native
recycler decides when to detach a row; Rust cannot pin it. When the row scrolls off screen its
holder is reused for another item, the native field loses focus, and a blur event arrives. If the
runtime treated that blur as "the user left the field", the focus would be gone for good.

So the runtime tracks a *logical* focus for recycled rows, separate from `self.focused`:

```rust,ignore
// rust/src/engine/runtime.rs
pub struct RowFocus {
    /// The recycled list that owns the row.
    pub list: NodeId,
    /// The item index the focused row shows.
    pub index: usize,
    /// The field node inside the row that has focus.
    pub field: NodeId,
    /// Whether the row was torn down while the field held focus.
    ///
    /// A blur in this state is a consequence of recycling, not a user action,
    /// so the runtime keeps the focus to restore it when the row comes back.
    pub parked: bool,
}
```

Three methods manage it:

- `set_row_focus(list, index, field, focused)`. With `focused = true` it records the focus
  (unparked) and clears the main-tree focus. With `false` it clears the row focus, *unless it is
  parked*, in which case it does nothing.
- `park_row_focus(field)` sets `parked = true`. It is called when the row is about to go away
  while its field has focus: when a holder is rebound to another index or unbound (platform code),
  or when a blur arrives for a field that is no longer attached (`track_focus` in the session).
- `row_focus()` returns the current value. The platform reads it when it binds a row: if the list
  and index match, it gives native focus back to the field. That produces a focus event, which
  calls `set_row_focus(.., true)` and unparks.

As a state diagram:

```text
            focus event                    row torn down
  none  ------------------>  focused  ------------------>  parked
   ^                           |  ^                           |
   |     blur (user)           |  |    row bound again,       |
   +---------------------------+  +---- focus restored -------+
   ^                                                          |
   +---- list gone / index out of range / main-tree focus ----+
                       (blur while parked: ignored)
```

`set_focus` (a main-tree field took focus) clears the row focus, and step 6 of `update` clears it
when its list disappears. The test `row_focus_survives_recycling_until_the_user_blurs` runs
through every edge.

The limit, recorded in `TODO.md`: focus is *restored* when the row comes back, not kept
continuously. While the row is off screen there is no native field, so the keyboard closes.

## `content_height`

```rust,ignore
// rust/src/engine/runtime.rs
/// The height the content needs, regardless of the viewport's height:
/// the bottom edge of the lowest top-level node.
pub fn content_height(&self) -> f32 {
    self.tree
        .children
        .iter()
        .filter_map(|child| self.layout.frames.get(&child.id))
        .map(|frame| frame.y + frame.height)
        .fold(0.0, f32::max)
}
```

The root always fills the viewport, so its frame says nothing about how tall the content is. A
bottom sheet wants to be as tall as its content. This returns the bottom edge of the root's
child, which, when that child has no explicit height, is its content height. Both platforms expose
it to their shells (`nativeContentHeight` on Android, an entry point in `rust/src/ios/entry.rs`),
and the native sheet sizes itself from it.

## Colour scheme

```rust,ignore
pub fn set_color_scheme(&mut self, scheme: ColorScheme) {
    self.scheme = scheme;
}
```

It only stores the value. The next `update` resolves colours with it in `build`, and the nodes
whose resolved colours differ come out of the diff as `Update`s. Because the tree is not equal to
the previous one, that update takes the slow path and lays out, although no frame changes and the
layout diff then returns nothing. A theme switch is rare enough that this does not matter.

The session calls `set_color_scheme` on every host before every render (chapter 8), so a runtime
never has a stale scheme.

## Accessors and leftovers

- `tree()` returns the current tree. The session uses it to resolve events.
- `frames()`, `content_sizes()`, `scroll_offset(id)` and `render(element, viewport, renderer)` are
  public, and are used only by tests in this crate. The platforms do not call them.

## Where this lives

- `rust/src/engine/runtime.rs`: `Runtime`, `Update`, `RowFocus`. The tests cover the fast path
  (`an_unchanged_render_skips_layout_and_emits_nothing`), scrolling, pinning, row focus and the
  colour scheme, each in isolation.
- `tree::build_with_focus`, `stamp_recycler_revision`, `fixed_window`, `variable_window`,
  `build_at`, `row_root` in `rust/src/engine/tree.rs`.
- `Host::render` implementations in `rust/src/android/host.rs` and `rust/src/ios/host.rs`: the
  callers of `update`. `bind_row` in `rust/src/android/rows.rs` and its iOS counterpart: the
  callers of the row methods. Both are described in [`../platforms/`](../platforms/).

Next: [8. The session and events](08-the-session-and-events.md).
