# 6. Diffing and operations

This chapter covers `rust/src/engine/render.rs`: the `Op` enum that is the engine's whole output,
the two functions that produce it, and the order operations arrive in. It ends with the two small
helper modules `rust/src/animation.rs` and `rust/src/frame_batch.rs`. After reading it you will be
able to predict the batch for a given change and you will know which ordering rules a renderer may
rely on.

## The output of the engine: `Op`

```rust,ignore
// rust/src/engine/render.rs
pub enum Op {
    /// Create the native widget of a new node.
    Create(RenderNode),
    /// Update the properties of an existing native widget.
    Update(RenderNode),
    /// Move an existing native widget under another parent view.
    Reparent { id: NodeId, parent: Option<NodeId> },
    /// Remove the native widget of a node.
    Remove { id: NodeId, exit: Option<Transition> },
    /// Move the native widget of a node to its calculated frame.
    Layout {
        id: NodeId,
        /// The frame relative to the node's native parent, in logical points.
        frame: Rect,
        /// The scrollable content size, for scroll containers only.
        content_size: Option<Size>,
        /// Implicit animation to apply, when the node already had a frame.
        animation: Option<Animation>,
    },
}

pub struct RenderNode {
    pub id: NodeId,
    /// The nearest renderable ancestor whose native view hosts this node.
    /// `None` means the node is a direct child of the platform root view.
    pub parent: Option<NodeId>,
    pub kind: NodeKind,
    pub style: Style,
}
```

Five instructions. A platform renderer is, at its core, a loop over a `Vec<Op>` with one arm per
variant and a map from `NodeId` to native view.

Some properties of this design:

- **Operations only mention renderable nodes.** A virtual `Column` never appears. The module
  comment says it: "layout-only nodes never reach the renderer."
- **`Create` and `Update` carry the whole node**, kind and style, not a list of changed fields.
  The engine decides *whether* a node changed; the renderer decides how to apply the new
  properties to its view. This keeps the engine free of per-widget knowledge. It also means an
  update of one field resends them all, and the renderer sets them all.
- **`parent` is a node id, never a view.** `None` is the host's root view.
- **Properties and geometry are separate.** `Create` does not carry a frame. Every position,
  including the first one, arrives as a `Layout` op. This lets the two diffs be independent
  functions and lets a pure move (nothing changed but the frame) skip the property path entirely.

There are two diffs, called one after the other by `Runtime::update`:

```rust,ignore
// rust/src/engine/runtime.rs, in update
let mut ops = render::diff(&self.tree, &next);
if let Some(layout) = layout {
    let mut layout_ops = render::diff_layout(&self.tree, &next, &self.layout, &layout);
    // ...
    ops.extend(layout_ops);
}
```

## `diff`: structure and properties

```rust,ignore
// rust/src/engine/render.rs
pub fn diff(old: &UiNode, new: &UiNode) -> Vec<Op>
```

The algorithm is a lookup by id, not a tree comparison. Stable ids (chapter 4) are what make that
possible: there is no need to guess which old node corresponds to which new one.

**Setup.** Index the old tree by id. Compute the renderable parent of every node in both trees
(`tree::renderable_parents`).

**Pass 1: walk the new tree in pre-order**, parents before children, skipping non-renderable
nodes. For each node, look its id up in the old tree. There are four cases:

```rust,ignore
// rust/src/engine/render.rs, in diff
match previous.get(&node.id) {
    // The node may have existed as a virtual layout node before, in
    // which case no native view exists yet.
    Some(previous) if !previous.is_renderable() => {
        ops.push(Op::Create(RenderNode::new(node, parent)));
    }
    None => ops.push(Op::Create(RenderNode::new(node, parent))),
    // A native widget cannot change class, so the old one is removed
    // before the new one is created under the same identifier.
    Some(previous)
        if std::mem::discriminant(&previous.kind) != std::mem::discriminant(&node.kind) =>
    {
        ops.push(Op::Remove { id: node.id, exit: previous.style.exit.clone() });
        ops.push(Op::Create(RenderNode::new(node, parent)));
        replaced.push(node.id);
    }
    Some(previous) => {
        let old_parent = old_parents.get(&node.id).copied().flatten();
        if old_parent != parent {
            ops.push(Op::Reparent { id: node.id, parent });
        }
        if previous.kind != node.kind || previous.style != node.style {
            ops.push(Op::Update(RenderNode::new(node, parent)));
        }
    }
}
```

1. **Not in the old tree: `Create`.**
2. **In the old tree, but it was virtual there: `Create`.** The id existed, the view did not. This
   is a `Column` that gained a background.
3. **Same id, different `NodeKind` variant: `Remove` then `Create`.** See the note below on when
   this can happen.
4. **Same id, same variant.** Two independent checks: if the nearest renderable ancestor changed,
   `Reparent`; if the kind (which includes all widget properties) or the style is not equal,
   `Update`. A node whose properties and parent are unchanged produces nothing, with one
   exception: the node passed to `diff_reapplying` as `reapply` always gets an `Update`. That is
   the control the user just changed (chapter 7, step 8).

Remember from chapter 2 that handlers compare equal when both are set. A re-created closure is not
a change, so case 4 produces nothing for a button whose `on_click` was rebuilt.

**Pass 2: removals.** Walk the *old* tree in pre-order, keep the renderable nodes whose id is
missing from the new tree or is virtual there, reverse the list, and emit a `Remove` for each
(skipping ids already removed in case 3):

```rust,ignore
// Pre-order lists parents before children; detach children first.
removals.reverse();
```

### When does case 3 fire?

The comment says "a native widget cannot change class". But a node's kind tag is part of its id
(chapter 4), so a `Text` replaced by a `Button` at the same position gets a *different id* and is
handled by cases 1 and pass 2: one removal, one creation. The test
`changing_a_kind_replaces_the_widget` passes through that route.

Also, the check compares `NodeKind` discriminants, and all widgets share the `NodeKind::Widget`
variant, so it could not tell a text from a button anyway.

The one situation where the same id carries a different `NodeKind` variant is a `List` that
switches between windowed and recycled mode. Both come from `ElementKind::List` (tag 11) and so
have the same id, but one becomes `NodeKind::Scroll` and the other `NodeKind::Recycler`. Case 3
exists for that. It is correct, and narrower than its comment suggests.

### Worked example: a column gains a background

```rust,ignore
// before                                // after
Column::new()                            Column::new()
    .child(Text::new("a"))                   .background(0xFF22_2222)
                                             .child(Text::new("a"))
```

The ids are identical in both trees. Before, the column is virtual and the text's renderable
parent is `None`. After, the column is renderable and is the text's parent.

Pass 1 visits the column first: it is in the old tree but was virtual, so case 2, `Create(column)`.
Then the text: same id, same kind, parent changed from `None` to the column, properties equal, so
`Reparent { text, parent: Some(column) }` and no update. Pass 2 finds nothing. The batch:

```text
Create(Column#C, parent: None)
Reparent { Text#T, parent: Some(C) }
```

The native text view is kept and moved. The reverse change gives `Reparent { T, None }` then
`Remove { C }`. Both are tests: `decorating_a_container_creates_it_and_reparents_its_children` and
`removing_a_decoration_reparents_children_before_dropping_it`.

## `diff_layout`: geometry

```rust,ignore
// rust/src/engine/render.rs
pub fn diff_layout(
    old: &UiNode,
    new: &UiNode,
    old_layout: &LayoutResult,
    new_layout: &LayoutResult,
) -> Vec<Op>
```

First, both layouts are converted from absolute frames to frames relative to each node's native
parent with `relative_frames` (explained in chapter 5). The conversion needs the tree, because
"native parent" depends on which nodes are renderable; that is why the function takes both trees
and not only the two frame maps.

Then, for every renderable node of the new tree:

```rust,ignore
// rust/src/engine/render.rs, in diff_layout
let frame = *new_frames.get(&node.id)?;
let content_size = new_layout.content_sizes.get(&node.id).copied();
let previous = old_frames.get(&node.id);
let moved = previous != Some(&frame);
let content_changed = old_layout.content_sizes.get(&node.id).copied() != content_size;
// New nodes appear at their final position; only nodes that already
// had a frame animate. Content-size-only changes are not visual.
let animation = if previous.is_some() && moved {
    node.style.animation.clone()
} else {
    None
};

(moved || content_changed).then_some(Op::Layout { id: node.id, frame, content_size, animation })
```

A `Layout` op is emitted when the relative frame differs from the previous one (which includes
"there was no previous one") or when the scroll content size changed. A node that did not move
produces nothing.

Comparing *relative* frames has a nice effect. If a card moves down by 20 points, the card's frame
changes, but its children's frames relative to the card do not. One `Layout` op moves the whole
subtree, as the native view hierarchy would expect.

It also covers the "column gains a background" change above. The new column view had no relative
frame before (only renderable nodes have one), so it gets a `Layout` op, without animation. The
text's absolute frame is the same, but its relative frame is now measured from the column's
content corner. If the column sits at the root's origin with no padding the numbers are equal and
the text gets no `Layout` op; otherwise it gets one with its coordinates inside the new parent.

## Ordering guarantees

The order of a batch is part of the contract. A renderer applies operations in sequence and may
assume:

**1. Parents are created before their children.** Pass 1 is pre-order. When `Create(child)`
arrives with `parent: Some(p)`, the view for `p` either already existed or was created earlier in
the same batch.

**2. A reparent comes after the creation of its new parent.** Same reason: the new parent is an
ancestor, and ancestors come first in pre-order.

**3. Removals come last among the structural operations, children before parents.** Pass 2 runs
after pass 1 and reverses pre-order. Two reasons:

- *After everything else*, so that a view that survives has already been reparented out of a
  container before the container is removed. In the "background removed" example, the text is
  reparented to the root first; if the column's view were removed first, a renderer that removes a
  view together with its subviews would take the text with it.
- *Deepest first*, so a container is detached after the views inside it. A renderer can release
  each view's resources and listeners without first checking whether an ancestor already took it
  down.

  The exception is case 3 above, where the `Remove` is emitted inline, immediately before the
  `Create` with the same id. It has to be: the renderer's map has one slot per id.

**4. All `Layout` ops come after all other ops.** `Runtime::update` appends the result of
`diff_layout` to the result of `diff`. Reasons:

- A frame is relative to the node's native parent. It can only be applied correctly once the view
  exists and sits under its final parent.
- Frames can be applied as one batch. The Android renderer collects every `Layout` of a batch into
  a `FrameBatch` (below) and sends it across JNI in one call after the loop.

The runtime has a test for this ordering, `layout_operations_come_last_in_every_batch`.

**5. Within the layout ops, parents come before children** (pre-order again).

One thing is *not* guaranteed: there is no `Layout` op for a removed node, and no op tells a
removed node's siblings anything. A renderer learns that a view should move only from its own
`Layout` op.

## Enter, exit and implicit animation

The engine does not animate. It attaches the metadata a renderer needs and makes two timing
decisions. Three style fields are involved (set by `.animate_enter`, `.animate_exit`, `.animate`
from `style_builder!`):

| Style field | Travels in | Decided by the engine |
|---|---|---|
| `enter: Option<Transition>` | `Create(RenderNode)`, inside `style` | nothing; the renderer reads it when it creates the view |
| `exit: Option<Transition>` | `Remove { exit }` | taken from the **old** tree's node |
| `animation: Option<Animation>` | `Layout { animation }` | only when the node had a frame before *and* moved |

**Exit comes from the old tree** because the node is not in the new tree; the last description the
application gave for it is the only place its exit transition is written. The renderer is expected
to keep the view alive for the duration, play the fade and scale, and then really remove it. From
the engine's side the node is gone the moment the op is emitted: its id is free, and if the same
id is created again before the exit finishes, that is the renderer's problem to resolve.

**Layout animation skips first appearances.** "New nodes appear at their final position; only
nodes that already had a frame animate." Without this, every node with `.animate(300)` would slide
in from the origin when its screen first shows. The test
`only_moved_existing_nodes_carry_their_animation` covers both halves.

**Viewport changes suppress it.** One more rule lives in the runtime, not here: when the viewport
size changed since the last update (rotation, split screen, the keyboard), `Runtime::update` clears
the `animation` of every `Layout` op, because "animating that would make a rotation or split-screen
resize look like a deliberate transition".

Decoration changes (a background colour that changes on a node with `.animate`) arrive as a plain
`Update` whose style contains the `animation`. Whether to animate the colour is up to the renderer.

A reminder from chapter 4: `enter`, `exit` and `animation` do not make a container renderable. On
a plain `Column` with no decoration they have nothing to act on.

## `Renderer` and `RecordingRenderer`

```rust,ignore
// rust/src/engine/render.rs
pub trait Renderer {
    /// Applies one batch. All operations of a batch belong to the same frame.
    fn apply(&mut self, ops: &[Op]);
}

pub struct RecordingRenderer {
    pub batches: Vec<Vec<Op>>,
}
```

together with `Runtime::render(element, viewport, &mut dyn Renderer)`, which calls `update` and
then `apply`. This reads like the interface the platforms implement. It is not. Neither `android/`
nor `ios/` implements `Renderer`; both call `Runtime::update` directly and loop over
`update.ops` themselves. (A likely reason: applying an op needs a platform context, such as the
JNI environment, and can fail, and `apply` has room for neither.) The trait and the recorder are
used by one runtime test (`render_applies_one_batch`). Treat them as a test utility, or as a
leftover from an earlier design.

## `animation.rs`: timing curves evaluated in Rust

```rust,ignore
// rust/src/animation.rs
pub(crate) fn ease_curve(curve: &AnimationCurve, t: f32) -> f32
```

Given a curve and a progress `t` in `0..=1`, return the eased progress. `Linear`, `EaseIn`
(`t * t`), `EaseOut` and `EaseInOut` are closed formulas. `CubicBezier` is solved numerically:
eighteen steps of bisection find the curve parameter whose x equals `t`, then the y at that
parameter is returned. The fixed step count means "the cost does not depend on how close the
control points are to the curve". `Spring` is a damped cosine, an approximation that only uses the
`damping` parameter (`response` is ignored here) and is clamped to the range `-0.25..=1.1` "so a
spring can visibly settle".

**Who uses it.** Only iOS. `rust/src/lib.rs` says so next to the module declaration:

```rust,ignore
// Only the iOS renderer evaluates curves from Rust; the module stays compiled
// everywhere so its tests run on the host.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
mod animation;
```

The single caller is `rust/src/ios/motion.rs`, a frame animator driven by a display link: on every
tick it needs the value of the curve at the current time, and UIKit's timing objects cannot be
sampled from Rust. Android does not call this module. It receives the curve as packed integers
(`AnimationCurve::packed`, in the next section) together with the frames.

The module's own doc comment says "shared by the platform renderers", which is no longer accurate.

## `frame_batch.rs`: frames as flat integer arrays

```rust,ignore
// rust/src/frame_batch.rs
pub struct FrameBatch {
    ids: Vec<i64>,
    frames: Vec<i32>,
    animations: Vec<i32>,
}

pub fn push(&mut self, id: NodeId, frame: Rect, density: f32, animation: Option<&Animation>)
```

A `FrameBatch` collects the `Layout` ops of one render into three parallel arrays:

- `ids`: one `i64` per node;
- `frames`: four `i32` per node, `x, y, width, height`, already in pixels (`Rect::to_pixels`);
- `animations`: six `i32` per node: the duration in milliseconds, then the five integers of
  `AnimationCurve::packed()`.

`packed()` is in `rust/src/style.rs`. The first integer is the kind (`0` one of the four standard
curves, `1` cubic Bezier, `2` spring); the rest are parameters, with fractions stored as
ten-thousandths. `EaseIn` is `[0, 2, 0, 0, 0]`; a Bezier `(0.1, 0.2, 0.3, 0.9)` is
`[1, 1000, 2000, 3000, 9000]`. A missing animation is six zeros, and a zero duration tells the
platform to apply the frame directly.

**Why.** Every call from Rust into Java crosses the JNI boundary, and that crossing has a fixed
cost. Positioning views one at a time costs one crossing per view per render. Primitive arrays can
be handed over in one call regardless of length. The module comment puts it as "batching trades
that for a registry keyed by node id and two primitive arrays, which cross the boundary in constant
time". The registry is on the Kotlin side: a map from node id to view, filled when Rust creates a
view. `TODO.md` (section 4, "Batched operations") has the measurements and notes that the gain is
about the shape of the cost: it no longer grows with the number of nodes.

**Who uses it.** Only Android (`rust/src/android/host.rs` fills it during the op loop,
`rust/src/android/style.rs` hands it over). iOS drives UIKit directly from Rust and has no JNI
boundary to economize on, so it does not use this module. `lib.rs` again:

```rust,ignore
// Only the Android renderer consumes the packed frame arrays; the module stays
// compiled everywhere so its tests run on the host.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod frame_batch;
```

Both modules follow the same pattern: logic that only one platform needs, but with no platform
types in it, is kept outside the platform folder so its unit tests run on the host.

Two small inaccuracies in `frame_batch.rs`: the module comment says "two primitive arrays" where
there are now three, and the test `animations_are_packed_two_integers_per_node` asserts six
integers per node.

## Where this lives

- `rust/src/engine/render.rs`: `Op`, `RenderNode`, `diff`, `diff_layout`, `relative_frames`. The
  tests read as a list of "this change gives this batch".
- `Runtime::update` in `rust/src/engine/runtime.rs`: where the two diffs are concatenated and
  where viewport changes clear animations.
- `Style::{animation, enter, exit}`, `Animation`, `Transition`, `AnimationCurve::packed` in
  `rust/src/style.rs`.
- `rust/src/animation.rs` and `rust/src/frame_batch.rs`.
- How a renderer applies each op: [`../platforms/`](../platforms/).

Next: [7. The runtime](07-the-runtime.md).
