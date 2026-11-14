# 5. Layout and measurement

This chapter covers `rust/src/engine/layout.rs`, `measure.rs` and `geometry.rs`: how the node tree
is handed to Taffy, how the sizes of text and other content get into the calculation, and what
comes out. After reading it you will know how a `Style` field maps to flexbox, when and why the
platform is asked to measure text, and what coordinate space a frame is in at each stage.

## The contract

Layout is one function:

```rust,ignore
// rust/src/engine/layout.rs
pub fn compute(root: &UiNode, viewport: Size, measurer: &dyn Measure) -> LayoutResult

pub struct LayoutResult {
    /// The absolute frame of every node in the tree.
    pub frames: HashMap<NodeId, Rect>,
    /// The scrollable content size of scroll containers, keyed by node.
    pub content_sizes: HashMap<NodeId, Size>,
}
```

In: a node tree, the size of the area to fill, and something that can measure content. Out: a
rectangle for *every* node, virtual ones included, and a content size for every scroll container.
The function keeps no state between calls. All numbers are logical points (dp on Android, points
on iOS).

[Taffy](https://github.com/DioxusLabs/taffy) is a Rust library that implements the CSS flexbox
algorithm. The engine uses it as a pure function: build a Taffy tree, compute, read the results,
drop the Taffy tree.

## The geometry types

`rust/src/engine/geometry.rs` is small:

- `Size { width, height }` and `Rect { x, y, width, height }`, in `f32` points.
- `Constraints { max_width, max_height }`, both `Option<f32>`: the bounds given to a measurement.
  `None` means unbounded.
- `PixelFrame`, the same as `Rect` in `i32` pixels, and `Rect::to_pixels(density)` which produces
  it. Origins are rounded; sizes are rounded and clamped to zero, "because a native view cannot
  have a negative size".

Layout stays fractional throughout. `compute` calls `taffy.disable_rounding()` with the comment
"the platform renderer rounds when it converts to pixels". Rounding once, at the end, with the real
density avoids accumulating half-point errors down a deep tree.

Two members exist but are not used by non-test code today: `Rect::right()` and `Rect::bottom()`
have no callers, and `Constraints::max_height` is never set to `Some`.

## Step 1: building the Taffy tree

```rust,ignore
// rust/src/engine/layout.rs
fn build_taffy(
    taffy: &mut TaffyTree,
    node: &UiNode,
    ids: &mut HashMap<NodeId, TaffyId>,
    scroll_child: bool,
    pager_child: bool,
) -> TaffyId {
    let is_scroll = matches!(node.kind, NodeKind::Scroll(_));
    let is_pager = matches!(node.kind, NodeKind::Pager(_));
    let children: Vec<TaffyId> = node
        .children
        .iter()
        .map(|child| build_taffy(taffy, child, ids, is_scroll, is_pager))
        .collect();

    let taffy_id = taffy
        .new_with_children(taffy_style(node, scroll_child, pager_child), &children)
        .expect("taffy node");

    ids.insert(node.id, taffy_id);
    taffy_id
}
```

One Taffy node per `UiNode`, children first. Two maps connect the worlds: `NodeId -> TaffyId`
(filled here) and its inverse (built in `compute`), which the measure callback needs to find the
node it is asked about.

The two booleans tell a node what its parent is, because a child of a scroll container or pager
gets different flex settings. They are the only context a node's Taffy style depends on besides
the node itself.

### How `Style` maps to flexbox: `taffy_style`

Every node is `display: flex`. The rest:

| Engine | Taffy / CSS | Notes |
|---|---|---|
| `NodeKind::Row`, `Pager`, horizontal `Scroll` | `flex-direction: row` | everything else is `column` |
| `style.width`, `style.height` | `size` | `Auto`, `Points(n)` or `Percent(f)` through `dimension()` |
| `style.aspect_ratio` | `aspect_ratio` | |
| `style.padding`, `style.margin` | `padding`, `margin` | points only |
| `style.gap` | `gap` on both axes | set by `Column::spacing` / `Row::spacing` |
| `style.grow` | `flex-grow` | |
| `style.align_items` | `align-items` | default `Stretch` |
| `style.justify_content` | `justify-content` | default `Start` |

Four settings are not taken from the style. Each fixes a specific problem.

**`flex-shrink` is 1, except for children of a scroll container or pager, where it is 0.**

```rust,ignore
// Scroll containers must let their content keep its natural size in
// the scroll axis instead of shrinking it into the viewport.
flex_shrink: if scroll_child || pager_child { 0.0 } else { 1.0 },
```

Flexbox normally shrinks children to fit their parent. Inside a scroll view that is exactly wrong:
the content is supposed to be taller than the viewport.

**`overflow`.** A vertical scroll node is `overflow-y: scroll, overflow-x: hidden`; horizontal is
the reverse; a pager is hidden on both. In Taffy, as in CSS, a non-visible overflow does two things
the engine needs: the node's automatic minimum size in that axis becomes zero, so it may be smaller
than its content (a scroll view can be 400 points tall with 3,000 points of content), and Taffy
records how far the content overflows, which becomes the content size (step 4).

**Text may shrink below its natural width.**

```rust,ignore
// Taffy's automatic minimum size uses the content's min-content size.
// For text that is the full unwrapped line, which would stop a label
// from shrinking next to a sibling (for example in a list row). A text
// node may shrink to the space its parent leaves and wrap instead.
min_size: TaffySize {
    width: if pager_child || matches!(node.kind, NodeKind::Widget(Widget::Text(_))) {
        LengthPercentageAuto::length(0.0)
    } else {
        LengthPercentageAuto::auto()
    },
    height: LengthPercentageAuto::auto(),
},
```

In CSS, a flex item's default `min-width: auto` means "never narrower than my content". Because
the measure function (below) reports unwrapped text as its minimum, a long label in a `Row` next to
a button would refuse to shrink and push the button off screen. Setting the minimum to zero for
text lets flexbox give it the remaining width, and the measure function is then called again with
that width and returns the wrapped height. The test `a_long_label_shrinks_beside_a_button` pins
this behaviour.

**Pager pages fill the pager.** A child of a pager ignores its own width and height and gets
`100% x 100%`, with shrink 0. The pager is a row, so its pages sit side by side, each exactly one
viewport wide, overflowing to the right. The native pager scrolls between them. See the test
`pager_pages_fill_the_viewport_side_by_side`.

## Step 2: computing, with a measure callback

```rust,ignore
// rust/src/engine/layout.rs, in compute
let available = TaffySize {
    width: AvailableSpace::Definite(viewport.width),
    height: AvailableSpace::Definite(viewport.height),
};

taffy
    .compute_layout_with_measure(
        taffy_root,
        available,
        |input: LayoutInput, taffy_id: TaffyId, _context, style: &TaffyStyle| {
            let Some(node) = by_taffy.get(&taffy_id).and_then(|id| nodes.get(id)) else {
                return LayoutOutput::DEFAULT;
            };

            // `compute_leaf_layout` applies the node's padding and honours the
            // dimensions the parent already resolved. The closure only has to
            // report the size of the content itself.
            taffy::compute_leaf_layout(
                input,
                style,
                |_, _| 0.0,
                |_known, available| {
                    let size = content_size(node, available, measurer);
                    TaffySize { width: size.width, height: size.height }
                },
            )
        },
    )
    .expect("taffy layout");
```

The root node has style `100% x 100%` (chapter 4), and the available space is the viewport, so the
root's frame is always the viewport. The test `the_root_fills_the_viewport` asserts it.

Taffy knows how to lay out containers. It does not know how big a leaf is. For every node without
children it calls the closure. The closure delegates to `taffy::compute_leaf_layout`, a Taffy
helper that handles the box model (padding, an explicit width or height from the style, aspect
ratio, min and max) and asks an inner closure only for the size of the *content*. That inner
closure calls `content_size`.

The point of going through `compute_leaf_layout` is in the comment on `content_size`: "the
available space has already been reduced by the node's padding, so it describes the content box
exactly: text is measured at the width it will actually occupy, no matter whether the parent
stretched it or not." The test `stretched_text_is_measured_inside_its_padding` checks the exact
number: a 200-point column with 10 points of padding, containing a text with 10 points of
horizontal padding of its own, measures the text at 160.

### `content_size`: who answers for each kind

```rust,ignore
// rust/src/engine/layout.rs
fn content_size(node: &UiNode, available: TaffySize<AvailableSpace>, measurer: &dyn Measure) -> Size {
    let max_width = match available.width {
        AvailableSpace::Definite(value) => Some(value),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    };

    match &node.kind {
        // An image is scaled from its natural size to fit the node's style.
        NodeKind::Widget(widget @ Widget::Image(_)) => {
            let natural = widget.intrinsic_size(max_width, measurer);
            if natural == Size::ZERO {
                Size::ZERO
            } else {
                image_content_size(node, natural, max_width)
            }
        }
        NodeKind::Widget(widget) => widget.intrinsic_size(max_width, measurer),
        // Containers are sized by their children.
        NodeKind::Root | NodeKind::Column | NodeKind::Row | NodeKind::Spacer
        | NodeKind::Scroll(_) | NodeKind::Pager(_) | NodeKind::Recycler(_) => Size::ZERO,
    }
}
```

- A **widget** answers through `Widget::intrinsic_size`, which forwards to the `IntrinsicSize`
  implementation in its file (chapter 3).
- A **container** with no children (an empty column, a spacer, a recycler) has zero content. It
  still gets a size from its style: a `Spacer` grows, a recycler has `width: 100%` and `grow: 1`
  from the `List` conversion.
- An **image** gets extra treatment. `ImageProps::intrinsic_size` returns the natural size if the
  platform knows it, otherwise zero. `image_content_size` then fits it to the style: an explicit
  width derives the height from the natural aspect ratio and the reverse; with neither, the
  natural size is used, scaled down to the available width. A remote image whose size is unknown
  is zero by zero unless the style gives it a size; the doc on `Image::new` tells applications to
  set one.

`max_width` is `None` when Taffy is probing ("how wide would you be with unlimited room?" or "how
narrow can you get?"). Taffy may call the measure function several times per leaf with different
available widths while it resolves flex sizes, so a text node can be measured more than once per
layout. That is why the cost of a layout pass is counted in text measurements, and why the
runtime's "skip layout when nothing changed" fast path matters (chapter 7). The test helper
`CountingMeasure` in `rust/src/engine/runtime.rs` counts exactly these calls.

## What `Measure` asks the platform for

```rust,ignore
// rust/src/engine/measure.rs
pub trait Measure: Send + Sync {
    /// Returns the intrinsic size of the text under the given constraints.
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size;

    /// Returns the platform's intrinsic size for a switch.
    fn switch_size(&self) -> Size {
        Size::new(DEFAULT_SWITCH_WIDTH, DEFAULT_SWITCH_HEIGHT)
    }

    /// Returns the natural size of an image, when it is known without
    /// downloading or decoding it.
    fn image_size(&self, _props: &ImageProps) -> Option<Size> {
        None
    }
}
```

Three questions, and only the first is required:

1. **How big is this text at this width?** `TextProps` carries the string, font size, weight,
   alignment, line limit and ellipsis flag. The answer depends on the real font, so only the
   platform can give it. Every widget that shows text funnels through this one method: a button
   builds a temporary `TextProps` from its label and font and measures that, a checkbox does the
   same for its label, a text field for its value or placeholder.
2. **How big is a native switch?** Switches have a fixed platform size that differs between
   Android and iOS. The default (52 by 32) matches the Material track and "keeps host builds
   deterministic".
3. **What is the natural size of this image?** Only answered for bundled assets, which can be
   inspected cheaply. The default is "unknown".

Everything else is assumed in Rust with constants (button padding, slider height, tab bar height,
date picker sizes), as described in chapter 3.

Each `Runtime` owns a `Box<dyn Measure>`, given to `Runtime::new`. The platform implementations
are in `rust/src/android/measure.rs` and `rust/src/ios/measure.rs`; see
[`../platforms/`](../platforms/).

### `ApproximateMeasure`

For tests and as a fallback there is an implementation that needs no platform:

```rust,ignore
// rust/src/engine/measure.rs
let glyph_width = text.size * 0.55;
let line_height = text.size * 1.25;
let natural_width = text.content.chars().count() as f32 * glyph_width;

let width = constraints.max_width.map_or(natural_width, |max| natural_width.min(max));
let lines = if width > 0.0 { (natural_width / width).ceil().max(1.0) } else { 1.0 };
let lines = text.max_lines.map_or(lines, |max| lines.min(max.max(1) as f32));

Size::new(width, lines * line_height)
```

Every glyph is 0.55 of the font size wide, every line 1.25 of it tall, and text wraps by dividing.
It is wrong for any real font and exactly right for tests: "Hello" at 16 points is 44 by 20,
always. Most engine tests use it; the layout tests also define tiny one-off measurers
(`FixedMeasure`, `WrappingMeasure`, `LengthMeasure`) when a test needs a particular answer.

## Step 3: reading frames back, in absolute coordinates

Taffy reports each node's position relative to its parent. `collect_frames` walks the tree adding
up offsets:

```rust,ignore
// rust/src/engine/layout.rs
fn collect_frames(/* taffy, node, ids, */ parent_x: f32, parent_y: f32, /* frames, content_sizes */) {
    let layout = taffy.layout(ids[&node.id]).expect("taffy layout");

    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    frames.insert(node.id, Rect::new(x, y, layout.size.width, layout.size.height));

    if let Some(content_size) = scroll_content_size(node, layout) {
        content_sizes.insert(node.id, content_size);
    }

    for child in &node.children {
        collect_frames(taffy, child, ids, x, y, frames, content_sizes);
    }
}
```

So `LayoutResult::frames` holds, for every node, a rectangle **relative to the root**, that is, to
the host's native root view. "Absolute" in this codebase means that. Content inside a scroll view
is positioned as if the scroll view showed all of it: the sixth 20-point row of a 100-point scroll
view has `y = 100`, below the scroll view's own bottom edge
(`vertical_scroll_reports_its_content_height`). Scroll offsets are not part of layout at all.

### Why absolute, and where it becomes relative

A native view's frame is relative to its parent *view*. But which node is a view's parent is not a
layout question; it depends on which ancestors are renderable (chapter 4), and that can change
between two renders without any frame changing. So layout produces one neutral coordinate space,
and the conversion to parent-relative frames happens later, in the diff:

```rust,ignore
// rust/src/engine/render.rs
fn collect_relative_frames(
    node: &UiNode,
    frames: &HashMap<NodeId, Rect>,
    origin: (f32, f32),
    relative: &mut HashMap<NodeId, Rect>,
) {
    let frame = frames.get(&node.id).copied();
    let mut child_origin = origin;

    if let Some(frame) = frame
        && node.is_renderable()
    {
        relative.insert(
            node.id,
            Rect::new(frame.x - origin.0, frame.y - origin.1, frame.width, frame.height),
        );
        child_origin = (
            frame.x + node.style.padding.left,
            frame.y + node.style.padding.top,
        );
    }

    for child in &node.children {
        collect_relative_frames(child, frames, child_origin, relative);
    }
}
```

The walk carries an origin. A renderable node's frame is stored relative to the current origin,
and then that node becomes the origin for its descendants. A virtual node stores nothing and passes
the origin through, so its children end up relative to the nearest renderable ancestor: the view
they are actually attached to.

Note the padding. The origin handed to the children is the parent's top-left corner *plus its left
and top padding*: its content-box corner, not its border-box corner. A child's frame in an
operation is therefore relative to where the parent's content starts. The renderers must attach
children to something that sits at that corner (a native view with real padding, or a content host
inside a scroll view). That is their half of the contract; see [`../platforms/`](../platforms/).

```text
absolute frames (from layout)          relative frames (in Layout ops)

Root        0,0   320x640              (not renderable: no entry)
 Column     0,0   320x640  virtual     (not renderable: no entry)
  Card     16,16  288x120  bg, pad 12  Card   16,16   relative to root
   Row     28,28  264x40   virtual     (no entry)
    Text   28,28  100x20               Text    0,0    relative to Card's
    Button 136,28 80x40                Button 108,0   content corner
```

## Step 4: scroll content sizes

A native scroll view needs to be told how large its content is. Taffy records, for each node, the
rectangle its content overflows into. `scroll_content_size` turns that into the number the
platform needs:

```rust,ignore
// rust/src/engine/layout.rs
fn scroll_content_size(node: &UiNode, layout: &Layout) -> Option<Size> {
    let NodeKind::Scroll(props) = node.kind else {
        return None;
    };

    let content = layout.content_box_size();
    let overflow = layout.scrollable_overflow_rect;
    // Taffy adds the scroll container's end padding to the overflow rect, but
    // the host sits inside that padding and the native scroll view already
    // applies it, so only the inner content extent is kept.
    let content_width = (overflow.right - layout.padding.left - layout.padding.right).max(0.0);
    let content_height = (overflow.bottom - layout.padding.top - layout.padding.bottom).max(0.0);

    Some(match props.axis {
        Axis::Vertical => Size::new(content.width, content.height.max(content_height)),
        Axis::Horizontal => Size::new(content.width.max(content_width), content.height),
    })
}
```

Only `Scroll` nodes get an entry. In the scroll axis the content size is the larger of the
viewport's content box and the overflow extent with the padding taken out; in the other axis it is
the content box. The padding arithmetic is the fiddly part, and
`scroll_content_accounts_for_padding` holds it in place: a 200 by 100 scroll view with 10 points
of padding and six 20-point rows reports a content size of 180 by 120.

A windowed list is a `Scroll` node whose content is spacers plus rows (chapter 4), so its content
size comes out as the full list height without any list-specific code here.

`Pager` and `Recycler` nodes get no content size. The native pager is told its page count, and the
native recycler computes its own extent from the item count and row heights.

## Cost and limits

- A new `TaffyTree` is built on every call. Nothing is cached between layouts, and Taffy's own
  incremental layout is not used. The engine's answer to cost is coarser: do not call `compute`
  at all when the tree is unchanged (chapter 7). When one label changes, the whole tree is laid
  out again.
- Every text measurement is a call into platform code (through JNI on Android), possibly several
  per text node. The comment in `Runtime::update` names "the native text measurement behind it" as
  the cost that skipping a layout pass saves.
- There is no way to give a node a minimum or maximum size, no absolute positioning, no wrapping
  of flex lines. `taffy_style` leaves those Taffy fields at their defaults.
- Layout does not know about scroll offsets or about which rows of a recycler exist. Recycled rows
  are laid out separately, one small tree per row (`Runtime::layout_tree`, chapter 7).

## Where this lives

- `rust/src/engine/layout.rs`: `compute`, `taffy_style`, `content_size`, `scroll_content_size`.
  The tests are a readable catalogue of layout behaviour, one case each.
- `rust/src/engine/measure.rs`: `Measure` and `ApproximateMeasure`.
- `rust/src/engine/geometry.rs`: the value types.
- `relative_frames` in `rust/src/engine/render.rs`: absolute to parent-relative.
- `impl IntrinsicSize for ...` in each `rust/src/widgets/*.rs`: the content sizes.

Next: [6. Diffing and operations](06-diffing-and-operations.md).
