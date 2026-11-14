# 4. The tree and identity

This chapter covers `rust/src/engine/tree.rs`: how a `RawElement` becomes a tree of `UiNode`s, how
each node gets an identifier that survives a rebuild, which nodes get a native view, and how a
`List` turns into either a windowed scroll container or a native recycler. After reading it you
will be able to predict the node tree and its ids for a given `view()`, and you will know what the
list code caches and when it throws the cache away.

## Why a second tree

The element tree from `view()` is a description with no memory. The engine needs three things it
does not have:

- an **identifier** per node, so the platform can keep a map from id to native view and the diff
  can match "this label" with "that label from the last render";
- lists **expanded** into the rows that are actually on screen;
- colours **resolved** to the current light or dark value.

`tree.rs` adds exactly those and nothing else. Layout and diffing are separate files.

```rust,ignore
// rust/src/engine/tree.rs
pub struct NodeId(pub u64);

pub struct UiNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub style: Style,
    pub children: Vec<UiNode>,
}

pub enum NodeKind {
    Root,
    Column,
    Row,
    Spacer,
    Pager(PagerProps),
    Widget(Widget),
    Scroll(ScrollProps),
    Recycler(RecyclerProps),
}
```

Compare `NodeKind` with `ElementKind` from chapter 2. Two differences:

- There is a `Root`, which no element produces. It is added by the builder.
- There is no `List`. A list element becomes either a `Scroll` node with children or a `Recycler`
  node without. `Recycler` has no element counterpart.

`Widget` is the same enum in both. The node holds a clone of the element's properties.

`UiNode` derives `PartialEq`. Comparing two trees with `==` compares ids, kinds (and so every
widget property), styles and children, recursively. The runtime's fast path (chapter 7) is that one
comparison.

## `Root`

Every tree starts with a synthetic root:

```rust,ignore
// rust/src/engine/tree.rs
pub const ROOT_ID: NodeId = NodeId(0);

pub fn empty() -> Self {
    Self { id: ROOT_ID, kind: NodeKind::Root, style: root_style(), children: Vec::new() }
}

fn root_style() -> Style {
    Style { width: Length::Percent(1.0), height: Length::Percent(1.0), ..Style::default() }
}
```

The root fills the viewport and has exactly one child: the application's top element. It exists so
that the application's top element can be anything (a `Column` with its own size, a bare `Text`)
and still has a parent to be laid out in. The root has no native view; it stands for the platform's
root view. `UiNode::empty()` is also the "previous tree" of the very first render, so the first
diff creates everything.

Recycled list rows are built with a different root (see `build_at` below).

## How node ids are computed

```rust,ignore
// rust/src/engine/tree.rs
fn node_id(parent: NodeId, index: usize, key: Option<&str>, tag: u64) -> NodeId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64; // FNV-1a offset basis

    let mut mix = |value: u64| {
        hash ^= value;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };

    mix(parent.0);
    mix(tag);
    match key {
        Some(key) => mix(hash_string(key)),
        None => mix(index as u64),
    }

    NodeId(hash)
}
```

An id is a 64-bit FNV-1a hash of three things:

1. **The parent's id.** So an id encodes the whole path from the root. Two buttons in different
   columns differ even if both are "child 0".
2. **The kind's tag** (chapter 2). A `Text` and a `Button` at the same position get different ids.
3. **The key, if the element has one; otherwise the position among its siblings.** Note the
   "otherwise": a keyed node's position is not hashed at all.

The function is pure. Building the same description twice gives the same ids, with no state kept
between builds. That is the property everything else relies on, and the tests at the bottom of the
file spell it out one case at a time (`rebuilding_the_same_element_reuses_identifiers`,
`changing_the_content_keeps_the_identifier`, `different_kinds_get_different_identifiers`).

### Why stability matters

The platform maps ids to native views. The diff (chapter 6) says "create" for an id that was not
in the previous tree and "remove" for one that is no longer there. If an id changes between two
renders of what the user sees as the same widget, the native view is destroyed and a new one
created. For a label that is only wasted work. For a text field it loses the cursor, the keyboard
and the focus; for a scroll view, the scroll position; for anything animated, the animation.

### Position, and what keys are for

Without keys, identity is positional. Consider:

```text
render 1                 render 2 (a banner was inserted first)
Column                   Column
  [0] Text "Name"          [0] Text "Offline"    <- same id as old [0]
  [1] TextField            [1] Text "Name"       <- new id (Text at 1)
                           [2] TextField         <- new id (TextField at 2)
```

The diff sees: the text at position 0 changed its content; a text appeared at position 1; the text
field at position 1 disappeared; a text field appeared at position 2. The user's field is
destroyed and recreated, and loses focus.

With keys the position does not enter the hash:

```rust,ignore
Column::new()
    .child(Text::new("Name").key("label"))
    .child(TextField::new(&self.name).key("name"))
```

Now inserting a sibling in front changes nothing for the keyed nodes
(`inserting_an_unkeyed_sibling_keeps_keyed_identifiers`). The rule of thumb: give a key to any
child whose siblings can appear, disappear or reorder, when the child has state worth keeping.

Keys must be unique among siblings, because two siblings with the same key and kind hash to the
same id and would share one native view. Debug builds check this in `build_children` and panic
with `duplicate element key`. Release builds do not check.

Two limits to state plainly:

- Identity never crosses parents. Moving a keyed element from one column to another gives it a new
  id, because the parent's id is in the hash.
- Ids are hashes, and nothing detects a collision between two different paths. With 64 bits and
  the few hundred nodes of a screen the chance is negligible, but it is a probability, not a
  guarantee.

## Building: `build_node`

There are five public entry points (`build`, `build_with`, `build_with_measurements`,
`build_with_focus`, `build_at`). The first four are the same function with more or fewer arguments
filled in with empty defaults; the runtime calls `build_with_focus`, the others exist for tests.
All of them create a root, then call `build_node` on the application's element.

```rust,ignore
// rust/src/engine/tree.rs
fn build_node(element: &RawElement, parent: NodeId, index: usize, cx: &mut BuildContext) -> UiNode {
    let id = node_id(parent, index, element.key(), element.kind.tag());

    if let ElementKind::List(props) = &element.kind {
        return build_list(element, id, props, cx);
    }

    let kind = match &element.kind {
        ElementKind::Column => NodeKind::Column,
        ElementKind::Row => NodeKind::Row,
        ElementKind::Spacer => NodeKind::Spacer,
        ElementKind::Pager(props) => NodeKind::Pager(PagerProps {
            pages: element.children.len(),
            ..props.clone()
        }),
        ElementKind::Widget(widget) => NodeKind::Widget(widget.clone()),
        ElementKind::Scroll(props) => NodeKind::Scroll(*props),
        ElementKind::List(_) => unreachable!("lists are expanded before the node kind match"),
    };

    let children = build_children(element, id, cx);

    UiNode { id, kind, style: element.style.clone(), children }
}
```

For everything except a list this is a one-to-one copy with an id added. The pager's page count is
filled in here from its children. `build_children` maps `build_node` over the children, passing
each child its index.

`BuildContext` carries what the list code needs: the scroll offsets, the viewport, the
measurement cache, the pinned rows, the focused node id, and a `BuildInfo` to write results into.

`BuildInfo` is the build's second return value, "what a build learned about the tree, besides the
tree itself":

| Field | Filled by | Used for |
|---|---|---|
| `lists` | every windowed list | `Runtime::scroll` decides whether a scroll changes the window |
| `rows` | variable-height lists only | `Runtime::record_heights` stores measured row heights |
| `reset_scroll` | variable-height lists whose items changed | the runtime forgets the scroll offset and tells the native view |
| `window_heights` | the height each list was windowed for | the runtime rebuilds when a list turns out taller |
| `recycled` | recycled lists | the platform asks the runtime to build a row on demand |
| `focused_rows` | any windowed list whose row holds the focused node | the next build pins that row |

## Renderable nodes: which nodes get a native view

```rust,ignore
// rust/src/engine/tree.rs
impl NodeKind {
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Widget(_) | Self::Pager(_) | Self::Scroll(_) | Self::Recycler(_))
    }
}

impl UiNode {
    pub fn is_renderable(&self) -> bool {
        self.kind.is_native() || self.style.needs_view()
    }
}
```

and in `rust/src/style.rs`:

```rust,ignore
pub fn needs_view(&self) -> bool {
    self.accessibility_label.is_some()
        || self.animation.is_some()
        || self.enter.is_some()
        || self.exit.is_some()
        || self.background.is_some()
        || self.gradient.is_some()
        || self.corner_radius > 0.0
        || self.corner_radii.is_some()
        || self.border.is_some()
        || self.opacity.is_some_and(|opacity| opacity < 1.0)
        || self.elevation > 0.0
}
```

A widget, pager, scroll container or recycler always has a native view. A `Column`, `Row` or
`Spacer` has one only if it paints something or carries a label or an animation of its own. A plain `Column` used for arranging children exists
for layout and nowhere else: Taffy computes its frame, its children are positioned accordingly,
and no native view is created for it. The children's native views are attached to the nearest
ancestor that does have a view.

**Why.** Deep native view hierarchies are expensive on both platforms (measure and layout passes,
memory, overdraw), and a typical screen has more layout containers than visible widgets. Since
Rust already computes every frame, a container view that draws nothing would add cost and no
function.

**The price.** A node can switch between virtual and real when its style changes (a column gains a
background). The diff has to handle that: create the view, then reparent the children into it
(chapter 6).

**Properties that are not about painting.** An accessibility label and the three animation
properties (`animation`, `enter`, `exit`) only mean something on a view, so setting one of them
makes the container real, just as a background does. `enabled` is handled differently, because
a native container does not pass its disabled state on to its children anyway. The tree carries
it down instead: `build_node` builds every descendant of a disabled node with
`enabled = Some(false)`, whatever the descendant says itself (`a_disabled_container_disables_everything_in_it`
in `rust/src/engine/tree.rs`). The one place this does not reach is the rows of a recycled list,
which are built on demand outside the tree.

`renderable_parents` computes the mapping the diff needs:

```rust,ignore
// rust/src/engine/tree.rs
pub fn renderable_parents(root: &UiNode) -> HashMap<NodeId, Option<NodeId>>
```

For every node it records the nearest renderable ancestor. `None` means "directly under the
platform root view".

```text
node tree                           native view tree
Root                                (platform root view)
  Column         virtual              Text#A
    Text#A       view                 Scroll#S
    Scroll#S     view                   Column#C
      Column#C   view (background)        Button#B
        Row      virtual
          Button#B  view            renderable_parents:
                                      A -> None   S -> None
                                      C -> S      B -> C
```

## Lists

A `List` is the one element that does not map to one node. `build_list` has two outcomes.

### Recycled lists

```rust,ignore
// rust/src/engine/tree.rs, in build_list
if props.mode == ListMode::Recycled && cfg!(any(target_os = "android", target_os = "ios")) {
    cx.info.recycled.insert(id, props.clone());
    return UiNode {
        id,
        kind: NodeKind::Recycler(RecyclerProps::new(props.item_count, props.item_height)),
        style: element.style.clone(),
        children: Vec::new(),
    };
}
```

A recycled list becomes a single childless `Recycler` node carrying only the item count, the
optional fixed row height and a revision number. The rows are not in the tree. The native recycler
(`RecyclerView` on Android, `UICollectionView` on iOS) decides which rows it needs and asks Rust
for each one; the item source (`ListProps`, with its `keys` and `rows` closures) is stored in
`BuildInfo::recycled` so the runtime can answer.

Note the `cfg!`. On any target other than Android and iOS, a `.recycled()` list silently falls
through to the windowed path below. That keeps host tests meaningful (a recycler with no native
recycler behind it would show nothing), but it means the `Recycler` branch of `build_list` is never
executed by `cargo test` on a laptop. `NodeKind::Recycler` is only reached in host tests that
construct the node by hand, such as `stamping_reaches_nested_recyclers`.

### Building one recycled row: `build_at` and `row_root`

When the platform needs row `index` of list `list`, it calls the row closure to get a `RawElement`
and builds it as its own small tree:

```rust,ignore
// rust/src/engine/tree.rs
pub fn build_at(element: &RawElement, root: NodeId) -> (UiNode, BuildInfo)

pub(crate) fn row_root(list: NodeId, index: usize) -> NodeId {
    let mixed = list.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (index as u64);
    NodeId(mixed | 1)
}
```

`build_at` is like the normal build but the root's id is `row_root(list, index)` instead of
`ROOT_ID`, and the root's style is "full width, height from content" instead of "fill the
viewport". Because every id is a hash that starts from the root's id, this gives each row its own
id space:

- ids inside row 3 cannot equal ids in the main tree or in row 4;
- building row 3 again gives the same ids, so a rebind of the same item can be diffed against the
  row's previous tree and only the changed widgets are touched.

The `| 1` makes the root id odd, so it can never be `ROOT_ID` (zero).

One thing follows from deriving the root from the *index*: a recycled row's identity is its
position in the list, not its key. If an item moves from index 3 to index 4, the holder that shows
index 4 diffs the item's row against whatever index 4 showed before.

### The recycler revision stamp

```rust,ignore
// rust/src/engine/tree.rs
pub(crate) fn stamp_recycler_revision(node: &mut UiNode, revision: u64) {
    if let NodeKind::Recycler(props) = &mut node.kind {
        props.revision = revision;
    }
    for child in &mut node.children {
        stamp_recycler_revision(child, revision);
    }
}
```

The native recycler owns its rows. If application state changes in a way that only affects what a
row shows (a counter inside row 5), nothing in the main tree changes: the `Recycler` node has the
same count and height. The diff would emit nothing, and the visible rows would keep their old
content. To avoid that, `RecyclerProps` has a `revision` field, and the runtime writes its own
render counter into it after every build. A new number makes the node unequal to the previous one,
the diff emits an `Update` for it, and the platform's adapter rebinds its visible rows. Each rebind
is then diffed against the row's previous tree, so unchanged rows cost a comparison.

Chapter 7 explains how the runtime stamps twice per update so that this does not defeat the
"nothing changed" fast path.

(The doc comments on `RecyclerProps::revision` and on this function speak of "RecyclerView". The
mechanism is the same on iOS.)

### Windowed lists

The default mode. The list becomes a native scroll container whose content is a column of
`[top spacer, the visible rows, bottom spacer]`:

```text
Scroll (id = the list's id)             content height = all rows
  Column                                 +------------------+
    Spacer  height = rows above          |  (empty)         |
    Column key="k7"  (row 7)             +------------------+
      <row 7 element>                    |  row 7           |  <- window
    Column key="k8"  (row 8)             |  row 8           |
      <row 8 element>                    |  row 9           |
    Column key="k9"  (row 9)             +------------------+
      <row 9 element>                    |  (empty)         |
    Spacer  height = rows below          |                  |
                                         +------------------+
```

The spacers make the scroll extent equal to the full list, so the native scrollbar and fling
physics behave as if every row existed. Only the rows in the window are built (the row closure is
called only for them) and only they have native views. As the comment on `build_list` says, "rows
sit at their absolute content position, which means a native scroll needs no repositioning when the
window changes".

Each row is wrapped in a `Column` whose key is the item's key:

```rust,ignore
// rust/src/engine/tree.rs, in build_fixed_rows
let mut row = RawElement::container(ElementKind::Column);
row.style.width = Length::Percent(1.0);
row.style.height = Length::Points(height);
row.key = Some(Cow::Owned((props.keys)(index)));
row.children.push((props.rows)(index));
```

Because the wrapper is keyed, its id (and the ids of everything inside it) does not depend on its
position in the window. When the user scrolls and the window moves from rows 7..10 to rows 8..11,
rows 8 and 9 keep their ids and their native views; row 7 is removed and row 10 is created. The
test `a_scrolled_list_moves_its_window_but_keeps_row_identifiers` checks this.

The window comes from the scroll offset, which the runtime passes in as a map from list id to
offset. A list not in the map is at offset zero.

#### Fixed row height

With `.item_height(h)` the arithmetic is direct:

```rust,ignore
// rust/src/engine/tree.rs
pub fn fixed_window(extent: ListExtent, viewport: f32, offset: f32, pin: Option<usize>) -> (usize, usize) {
    let height = extent.item_height.unwrap_or(0.0);
    if extent.count == 0 || height <= 0.0 {
        return (0, 0);
    }

    let offset = offset.clamp(0.0, (extent.count as f32 * height - viewport).max(0.0));
    let first = (offset / height).floor() as usize;
    let visible = ((viewport / height).ceil() as usize).max(1);
    let start = first.saturating_sub(extent.buffer).min(extent.count);
    let end = (first + visible + extent.buffer).min(extent.count);
    with_pin(start, end, pin, extent.count)
}
```

The window is the rows that intersect the viewport plus `buffer` rows on each side (default 3), so
a small scroll does not immediately need new rows. The spacers are `start * h` and
`(count - end) * h`. No per-row state is kept, and the key closure is called only for the rows in
the window.

**Which height.** The window is computed for the list's own height, taken from the last layout
(`BuildContext::window_height`). A list that has never been laid out is windowed for the whole
host viewport, which is never too little: on its first render a list that occupies half the
screen materializes about twice the rows it needs, and from the second render on only its own.
If a list turns out taller than the window it was built for (a header above it disappeared, the
device rotated), the runtime notices after layout and builds again in the same update
(chapter 7), so there is never a render with rows missing at the end.

#### Variable row height

Without `.item_height`, row heights are unknown until the rows have been laid out. The engine
keeps a `ListMeasurements` per list id, owned by the runtime and passed into every build:

```rust,ignore
// rust/src/engine/tree.rs
pub struct ListMeasurements {
    /// Measured row heights in logical points, keyed by the item key hash.
    pub heights: HashMap<u64, f32>,
    /// Height used for rows that have not been measured yet.
    pub estimate: f32,
    /// The layout width the measurements belong to.
    pub width: f32,
    /// Prefix sums of the row heights; `prefix[i]` is the offset of item `i`.
    pub prefix: Vec<f32>,
    /// The materialized range of the last build.
    pub window: (usize, usize),
    /// A hash of the item keys ...
    pub signature: u64,
    /// The application-supplied item revision, when there is one.
    pub revision: Option<u64>,
}
```

The central structure is `prefix`: an array of `count + 1` running totals, where `prefix[i]` is
the y position of row `i` and `prefix[count]` is the total height. A row that has been measured
contributes its real height; one that has not contributes `estimate` (56 points unless
`.estimated_height` says otherwise). With the prefix array, the window is two binary searches
(`variable_window`, using `partition_point`), and the spacers are `prefix[start]` and
`total - prefix[end]`.

`build_variable_rows` does this, in order:

1. **Take the cache** for this list id out of the map (or start empty).
2. **Invalidate on width change.** Heights depend on the width text wraps at. If the viewport
   width differs from `measurement.width` by more than half a point, all heights and the prefix are
   dropped.
3. **Decide whether the cached prefix can be reused.** This is where `revision` comes in (below).
4. **Rebuild the prefix if not reusable**: call the key closure for every item, look up each key's
   measured height or use the estimate, accumulate.
5. **Compute the window** with `variable_window` and record it in `measurement.window`.
6. **Build the spacer, the rows and the spacer**, like the fixed case but without a fixed height on
   the row wrapper.
7. **Record each built row** as `(node id, key hash, item index)` in `BuildInfo::rows`.
8. **Put the cache back.**

Step 7 is what closes the loop. After layout, the runtime reads each recorded row's frame height,
stores it under the key hash, and patches the prefix array (chapter 7). If any height changed, it
builds again; the window may now be different, because the rows turned out taller or shorter than
the estimate.

Heights are keyed by the hash of the item's *key*, not by index, so a measured height survives the
item moving within the list.

#### `revision`: telling the list its items did not change

Step 4 costs one key-closure call and one hash per item. For a list of ten thousand rows, doing
that on every render is the dominant cost. The engine needs to know whether the items are the same
as last time, and it has no way to see inside the closures. There are two modes.

**Without a revision** (the default) it finds out the hard way:

```rust,ignore
// rust/src/engine/tree.rs, in build_variable_rows
// No revision supplied: hash every key to detect a different list.
let all_keys: Vec<String> = (0..props.item_count).map(|index| (props.keys)(index)).collect();

let mut signature = 0xcbf2_9ce4_8422_2325_u64;
for key in &all_keys {
    signature ^= hash_string(key);
    signature = signature.wrapping_mul(0x0000_0100_0000_01b3);
}
signature ^= props.item_count as u64;
if measurement.signature != signature {
    measurement.heights.clear();
    measurement.prefix.clear();
    measurement.window = (0, 0);
    measurement.signature = signature;
    reset_scroll = true;
}
```

Every key is computed and hashed on every build. If the combined signature differs from the stored
one, the items changed: the cache is cleared and the list is marked for a scroll reset. This is
always correct and always O(n). The doc comment on `List::revision` says it "caps it at hundreds of
rows".

**With `.revision(n)`** the application promises: "while this number is the same, the items, their
order and their keys are the same." The build then compares one integer. If it matches and the
prefix is cached, nothing is recomputed, and the key closure is called only for the rows in the
window. The test `an_items_revision_reuses_the_prefix_and_only_keys_the_window` counts key calls
to prove it: 10,000 on the first build, then only the visible handful.

When the revision changes, the cache is cleared and `reset_scroll` is set, exactly as for a
changed signature.

The trade is manual bookkeeping for speed. If an application changes its items and forgets to
change the revision, the list shows stale heights and a window computed from the old prefix. The
`TODO.md` backlog lists "automatic item fingerprint" as the way out.

`reset_scroll` exists because the same node id can come to hold a different list. Ids are
positional, so replacing one screen's list with another at the same place in the tree keeps the id.
Without the reset the new list would open at the old list's scroll offset with the old list's row
heights. The runtime test `a_different_list_on_the_same_node_resets_the_scroll` covers it.

**Rough edges in this function.**

- The reset only exists on the variable-height path. `build_fixed_rows` has no signature and never
  looks at `revision`, so a different *fixed-height* list at the same node keeps the previous
  offset. The doc on `List::revision` ("it must also change when a different list takes over the
  same position in the tree") has no effect for fixed-height lists.
- `reset_scroll` makes Rust forget its recorded offset, and the runtime then emits
  `Op::ScrollTo` so the native scroll view returns to the top as well (chapter 7).
- `measurement.width` is documented as "the layout width the measurements belong to", but it is
  compared with the viewport width, not the list's own width.
- Two conditions in the function are always true or have no effect: in the no-revision branch the
  final `measurement.signature == signature` (the lines above have made them equal), and in
  the revision branch the outer `if !same_revision || !prefix_cached || width_changed`, whose body
  only does something when `!same_revision`.

#### Focus pinning

A text field inside a windowed list has a problem: if the user focuses it and scrolls it out of
the window, its row is removed, the native field is destroyed, and the keyboard closes.

The engine prevents that by keeping the focused row materialized. The mechanism spans two builds:

1. The runtime knows the id of the focused node (the platform reports focus events). It passes it
   to the build as `cx.focused`.
2. While building each row, `record_focus` checks whether the row's subtree contains that id
   (`node_contains`). If so it writes `list id -> item index` into `BuildInfo::focused_rows`.
3. The runtime passes that map into the *next* build as `cx.pins`.
4. `with_pin` extends the window to include the pinned index:

```rust,ignore
// rust/src/engine/tree.rs
fn with_pin(start: usize, end: usize, pin: Option<usize>, count: usize) -> (usize, usize) {
    match pin {
        Some(pin) if pin < count => (start.min(pin), end.max(pin + 1)),
        _ => (start, end),
    }
}
```

The window stays one contiguous range, so every row between the pinned row and the visible rows is
materialized too. The doc comment is honest about it: "during a normal scroll that distance is
small; a programmatic jump while a field is focused can materialize more." The pin is an item
index, found again on every build, and dropped when focus moves or the focused node disappears.

Recycled lists do not use this. Their rows are not in the tree; focus inside them is tracked by a
separate mechanism in the runtime (chapter 7).

## Colour resolution

```rust,ignore
// rust/src/engine/tree.rs
pub(crate) fn resolve_colors(node: &mut UiNode, scheme: crate::style::ColorScheme) {
    node.style.resolve_colors(scheme);
    if let NodeKind::Widget(widget) = &mut node.kind {
        widget.resolve_colors(scheme);
    }
    for child in &mut node.children {
        resolve_colors(child, scheme);
    }
}
```

A `Color` holds two ARGB values, one for light and one for dark (chapter 9). After the tree is
built, this walk replaces every colour with a "fixed" colour whose two values are both the one for
the current scheme. It visits the style (background, gradient, border) and, for widgets, the
properties (through `WidgetProps::resolve_colors`, chapter 3).

Doing this in the tree, before the diff, has a useful consequence: switching the device to dark
mode needs no special code path. The rebuilt tree differs from the previous one exactly in the
nodes whose resolved colour changed, the diff emits an `Update` for each, and the renderers never
see an unresolved colour. The runtime test
`a_new_color_scheme_updates_only_the_nodes_whose_colors_change` shows that a text with a fixed
colour is not touched.

## Tree walks

The file ends with four helpers used across the engine:

- `preorder(root)`: every node, parents before children, as a `Vec<&UiNode>`.
- `index(root)`: a `HashMap<NodeId, &UiNode>`.
- `find(root, id)`: one node by id. It builds the full pre-order list and scans it, so it is O(n)
  with an allocation per call. It is called once per event, which is fine; it would not be fine in
  a loop.
- `renderable_parents(root)`: described above.

## Where this lives

- `rust/src/engine/tree.rs`: everything in this chapter. The tests at the bottom are the best
  specification of id stability; read their names first.
- `Style::needs_view` in `rust/src/style.rs`: the list of properties that make a container real.
- `List` in `rust/src/widgets/list.rs`: the public options (`item_height`, `estimated_height`,
  `buffer`, `revision`, `recycled`) that become `ListProps`.
- `Runtime::build`, `record_heights` and `scroll` in `rust/src/engine/runtime.rs`: the callers
  that own the caches this file reads and writes.

Next: [5. Layout and measurement](05-layout-and-measurement.md).
