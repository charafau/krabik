//! The internal UI tree.
//!
//! The public [`crate::ui`] description is converted into this representation
//! before layout and rendering. Node identifiers are derived from the node's
//! position (or its key) so that rebuilding the same description yields the
//! same identifiers, which lets the renderer update native widgets in place.

use std::borrow::Cow;
use std::collections::HashMap;

use crate::geometry::Size;
use crate::ui::{
    Axis, ElementKind, Length, ListMode, ListProps, PagerProps, RawElement, ScrollProps, Style,
};
use crate::widgets::Widget;

/// The identifier of the synthetic root node.
pub const ROOT_ID: NodeId = NodeId(0);

/// A stable identifier of a node in the UI tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

/// The kind of a node in the UI tree.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeKind {
    /// The synthetic root that fills the viewport.
    Root,
    /// A vertical container.
    Column,
    /// A horizontal container.
    Row,
    /// Flexible empty space.
    Spacer,
    /// A native swipeable page container.
    Pager(PagerProps),
    /// A leaf widget drawn by one native view; see [`Widget`].
    Widget(Widget),
    /// A native scroll container.
    Scroll(ScrollProps),
    /// A native recycler that materializes rows on demand.
    Recycler(RecyclerProps),
}

impl NodeKind {
    /// Whether the node corresponds to an actual native widget.
    pub fn is_native(&self) -> bool {
        matches!(
            self,
            Self::Widget(_) | Self::Pager(_) | Self::Scroll(_) | Self::Recycler(_)
        )
    }
}

/// The configuration of a native recycler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecyclerProps {
    /// The number of items the recycler reports.
    pub item_count: usize,
    /// The fixed height of one row, in logical points.
    ///
    /// `None` lets the recycler size every row from the height its content
    /// needs, which the renderer reports back after laying the row out.
    pub item_height: Option<f32>,
    /// A counter that changes whenever the application description was rebuilt.
    ///
    /// RecyclerView owns the rows and would otherwise never notice that an app
    /// state change altered the content of visible rows. A new revision asks
    /// the adapter to rebind the attached cells; the renderer diffs each row,
    /// so unchanged widgets are not touched.
    pub revision: u64,
}

impl RecyclerProps {
    /// A recycler before the runtime stamps its revision.
    pub fn new(item_count: usize, item_height: Option<f32>) -> Self {
        Self {
            item_count,
            item_height,
            revision: 0,
        }
    }
}

/// A node of the internal UI tree.
#[derive(Clone, Debug, PartialEq)]
pub struct UiNode {
    /// The stable identifier of the node.
    pub id: NodeId,
    /// The kind of the node.
    pub kind: NodeKind,
    /// Layout and decoration properties.
    pub style: Style,
    /// The children of the node.
    pub children: Vec<UiNode>,
}

impl UiNode {
    /// Creates the root that wraps an application element.
    pub fn empty() -> Self {
        Self {
            id: ROOT_ID,
            kind: NodeKind::Root,
            style: root_style(),
            children: Vec::new(),
        }
    }

    /// Whether the node owns a native view.
    ///
    /// Native widgets always do. Layout nodes stay virtual unless they paint
    /// something, in which case they need a decoration view of their own.
    pub fn is_renderable(&self) -> bool {
        self.kind.is_native() || self.style.needs_view()
    }
}

/// The extent of one virtualized list, recorded while building it.
///
/// Scroll updates use it to decide whether the visible window changed without
/// rebuilding the tree.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ListExtent {
    /// The number of items in the list.
    pub count: usize,
    /// The fixed row height, when the list declares one.
    pub item_height: Option<f32>,
    /// Extra rows materialized above and below the viewport.
    pub buffer: usize,
}

/// Heights measured for a variable-height list.
#[derive(Clone, Debug)]
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
    /// A hash of the item keys, used to notice that a different list now owns
    /// this node id (for example after switching screens).
    pub signature: u64,
    /// The application-supplied item revision, when there is one.
    pub revision: Option<u64>,
}

impl Default for ListMeasurements {
    fn default() -> Self {
        Self {
            heights: HashMap::new(),
            estimate: 56.0,
            width: 0.0,
            prefix: Vec::new(),
            window: (0, 0),
            signature: 0,
            revision: None,
        }
    }
}

/// What a build learned about the tree, besides the tree itself.
#[derive(Debug, Default)]
pub struct BuildInfo {
    /// The extent of every list, keyed by its scroll node id.
    pub lists: HashMap<NodeId, ListExtent>,
    /// Materialized rows per list, with the key hash and item index needed to
    /// store and patch their measured height after layout.
    pub rows: HashMap<NodeId, Vec<(NodeId, u64, usize)>>,
    /// Lists whose scroll position must be forgotten because the items they
    /// show changed.
    pub reset_scroll: Vec<NodeId>,
    /// Item sources of recycled lists, kept so rows can be built on demand.
    pub(crate) recycled: HashMap<NodeId, ListProps>,
    /// The item index that holds the focused node, per list. The next build
    /// keeps that row materialized even if it would otherwise leave the window.
    pub focused_rows: HashMap<NodeId, usize>,
}

/// Converts a declarative element into the internal tree.
///
/// Lists are materialized at offset zero; use [`build_with`] to account for
/// scroll positions.
pub fn build(element: &RawElement) -> UiNode {
    let offsets = HashMap::new();
    build_with(element, &offsets, Size::ZERO).0
}

/// Converts one element into a subtree rooted at `root`.
///
/// Recycler rows use their own root id so their node identifiers cannot collide
/// with identifiers in the main tree, while a rebind of the same item keeps the
/// same ids.
pub fn build_at(element: &RawElement, root: NodeId) -> (UiNode, BuildInfo) {
    let offsets = HashMap::new();
    let mut measurements = HashMap::new();
    let mut info = BuildInfo::default();
    let mut tree_root = UiNode {
        id: root,
        kind: NodeKind::Root,
        // The recycler tells the row how wide it is; fill that width and let
        // the content decide the height.
        style: Style {
            width: Length::Percent(1.0),
            ..Style::default()
        },
        children: Vec::new(),
    };
    let mut context = BuildContext {
        offsets: &offsets,
        viewport: Size::ZERO,
        measurements: &mut measurements,
        pins: &HashMap::new(),
        focused: None,
        info: &mut info,
    };
    tree_root
        .children
        .push(build_node(element, root, 0, &mut context));
    (tree_root, info)
}

/// The identifier of one recycled row's subtree root.
///
/// Deriving it from the list and the item index keeps a row's widget ids stable
/// across rebinds while keeping them out of the main tree's identifier space.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn row_root(list: NodeId, index: usize) -> NodeId {
    let mixed = list.0.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (index as u64);
    NodeId(mixed | 1)
}

/// Stamps the current render revision into every recycled list.
///
/// RecyclerView's adapter needs a signal to rebind its attached cells when the
/// application description changed; the revision is part of the node's
/// properties, so a new value produces an update operation.
pub(crate) fn stamp_recycler_revision(node: &mut UiNode, revision: u64) {
    if let NodeKind::Recycler(props) = &mut node.kind {
        props.revision = revision;
    }
    for child in &mut node.children {
        stamp_recycler_revision(child, revision);
    }
}

/// Converts a declarative element into the internal tree with scroll state.
pub fn build_with(
    element: &RawElement,
    offsets: &HashMap<NodeId, f32>,
    viewport: Size,
) -> (UiNode, BuildInfo) {
    let mut measurements = HashMap::new();
    build_with_measurements(element, offsets, viewport, &mut measurements)
}

/// Converts a declarative element into the internal tree, reusing measured
/// row heights across rebuilds.
pub fn build_with_measurements(
    element: &RawElement,
    offsets: &HashMap<NodeId, f32>,
    viewport: Size,
    measurements: &mut HashMap<NodeId, ListMeasurements>,
) -> (UiNode, BuildInfo) {
    let empty = HashMap::new();
    build_with_focus(element, offsets, viewport, measurements, &empty, None)
}

/// Converts a declarative element into the internal tree, keeping the row that
/// holds `focused` materialized.
pub fn build_with_focus(
    element: &RawElement,
    offsets: &HashMap<NodeId, f32>,
    viewport: Size,
    measurements: &mut HashMap<NodeId, ListMeasurements>,
    pins: &HashMap<NodeId, usize>,
    focused: Option<NodeId>,
) -> (UiNode, BuildInfo) {
    let mut info = BuildInfo::default();
    let mut root = UiNode::empty();
    let mut context = BuildContext {
        offsets,
        viewport,
        measurements,
        pins,
        focused,
        info: &mut info,
    };
    root.children
        .push(build_node(element, ROOT_ID, 0, &mut context));
    (root, info)
}

/// State [`build_node`] needs beyond the element itself.
struct BuildContext<'a> {
    offsets: &'a HashMap<NodeId, f32>,
    viewport: Size,
    measurements: &'a mut HashMap<NodeId, ListMeasurements>,
    pins: &'a HashMap<NodeId, usize>,
    focused: Option<NodeId>,
    info: &'a mut BuildInfo,
}

/// The half-open range of items a fixed-height list materializes.
///
/// The range includes `buffer` rows on each side so small scrolls do not
/// immediately cross a boundary.
pub fn fixed_window(
    extent: ListExtent,
    viewport: f32,
    offset: f32,
    pin: Option<usize>,
) -> (usize, usize) {
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

/// Extends a window so it contains the pinned row.
///
/// Keeping the range contiguous also materializes any rows between the window
/// and the pinned row. During a normal scroll that distance is small; a
/// programmatic jump while a field is focused can materialize more.
fn with_pin(start: usize, end: usize, pin: Option<usize>, count: usize) -> (usize, usize) {
    match pin {
        Some(pin) if pin < count => (start.min(pin), end.max(pin + 1)),
        _ => (start, end),
    }
}

/// The half-open range of items a variable-height list materializes.
pub fn variable_window(
    prefix: &[f32],
    viewport: f32,
    offset: f32,
    buffer: usize,
    pin: Option<usize>,
) -> (usize, usize) {
    let count = prefix.len().saturating_sub(1);
    if count == 0 {
        return (0, 0);
    }
    let total = prefix[count];
    if total <= 0.0 {
        return (0, 0);
    }

    let offset = offset.clamp(0.0, (total - viewport).max(0.0));
    let limit = offset + viewport.max(1.0);
    // The item containing the offset is the one whose end is the first past
    // it; the visible range ends at the first item that starts at or after the
    // viewport's bottom edge.
    let first = prefix
        .partition_point(|end| *end <= offset)
        .saturating_sub(1)
        .min(count.saturating_sub(1));
    let visible_end = prefix.partition_point(|start| *start < limit).min(count);
    let start = first.saturating_sub(buffer);
    let end = visible_end.saturating_add(buffer).max(start + 1).min(count);
    with_pin(start, end, pin, count)
}

fn root_style() -> Style {
    Style {
        width: Length::Percent(1.0),
        height: Length::Percent(1.0),
        ..Style::default()
    }
}

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

    UiNode {
        id,
        kind,
        style: element.style.clone(),
        children,
    }
}

fn build_children(element: &RawElement, parent: NodeId, cx: &mut BuildContext) -> Vec<UiNode> {
    // Duplicate keys would hash two siblings to the same identifier, which
    // silently aliases their native views. Fail loudly in debug builds.
    #[cfg(debug_assertions)]
    {
        let mut keys = std::collections::HashSet::new();
        for child in &element.children {
            if let Some(key) = child.key() {
                debug_assert!(
                    keys.insert(key),
                    "duplicate element key {key:?} among the children of {parent:?}"
                );
            }
        }
    }

    element
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| build_node(child, parent, index, cx))
        .collect()
}

/// Expands a list into a scroll container holding only the visible rows.
///
/// The content is a column of `[top spacer, rows, bottom spacer]`, so the
/// platform scroll extent is the full list height while only the window exists
/// as native views. Rows sit at their absolute content position, which means a
/// native scroll needs no repositioning when the window changes.
fn build_list(
    element: &RawElement,
    id: NodeId,
    props: &ListProps,
    cx: &mut BuildContext,
) -> UiNode {
    if props.mode == ListMode::Recycled && cfg!(any(target_os = "android", target_os = "ios")) {
        cx.info.recycled.insert(id, props.clone());
        return UiNode {
            id,
            kind: NodeKind::Recycler(RecyclerProps::new(props.item_count, props.item_height)),
            style: element.style.clone(),
            children: Vec::new(),
        };
    }

    let extent = ListExtent {
        count: props.item_count,
        item_height: props.item_height,
        buffer: props.buffer,
    };
    cx.info.lists.insert(id, extent);

    let (column, rows) = match props.item_height {
        Some(height) => build_fixed_rows(id, props, height, cx),
        None => build_variable_rows(id, props, cx),
    };

    if !rows.is_empty() {
        cx.info.rows.insert(id, rows);
    }

    UiNode {
        id,
        kind: NodeKind::Scroll(ScrollProps {
            axis: Axis::Vertical,
        }),
        style: element.style.clone(),
        children: vec![column],
    }
}

/// Builds the spacer/rows/spacer content of a fixed-height list.
fn build_fixed_rows(
    id: NodeId,
    props: &ListProps,
    height: f32,
    cx: &mut BuildContext,
) -> (UiNode, Vec<(NodeId, u64, usize)>) {
    let offset = cx.offsets.get(&id).copied().unwrap_or(0.0);
    let (start, end) = fixed_window(
        ListExtent {
            count: props.item_count,
            item_height: Some(height),
            buffer: props.buffer,
        },
        cx.viewport.height,
        offset,
        cx.pins.get(&id).copied(),
    );

    let column_id = node_id(id, 0, None, ElementKind::Column.tag());
    let mut children = Vec::with_capacity(end - start + 2);

    if start > 0 {
        children.push(build_spacer(
            column_id,
            children.len(),
            start as f32 * height,
            cx,
        ));
    }

    for index in start..end {
        let mut row = RawElement::container(ElementKind::Column);
        row.style.width = Length::Percent(1.0);
        row.style.height = Length::Points(height);
        row.key = Some(Cow::Owned((props.keys)(index)));
        row.children.push((props.rows)(index));

        let position = children.len();
        let node = build_node(&row, column_id, position, cx);
        record_focus(cx, id, index, &node);
        children.push(node);
    }

    if end < props.item_count {
        children.push(build_spacer(
            column_id,
            children.len(),
            (props.item_count - end) as f32 * height,
            cx,
        ));
    }

    (content_column(column_id, children), Vec::new())
}

/// Builds the spacer/rows/spacer content of a variable-height list.
///
/// Unmeasured rows use the running estimate; measured rows are read back after
/// layout by the runtime, which rebuilds until the window stabilizes.
///
/// With an application revision the build reuses the key hashes, measured
/// heights and prefix sums when nothing changed, so only the visible rows cost
/// anything. Without one it hashes every key to notice a different list, which
/// is correct but O(n) per build.
fn build_variable_rows(
    id: NodeId,
    props: &ListProps,
    cx: &mut BuildContext,
) -> (UiNode, Vec<(NodeId, u64, usize)>) {
    let mut measurement = cx.measurements.remove(&id).unwrap_or_default();

    // The estimate is configuration, not state: keeping it stable means the
    // total content height only changes when a row is actually measured, so a
    // scroll clamp cannot feed back into another window change.
    let estimate_changed = (measurement.estimate - props.estimated_height).abs() > f32::EPSILON;
    measurement.estimate = props.estimated_height;

    let width_changed = (measurement.width - cx.viewport.width).abs() > 0.5;
    if width_changed {
        // Heights depend on the available width; measurements from a different
        // width are not comparable.
        measurement.heights.clear();
        measurement.prefix.clear();
        measurement.width = cx.viewport.width;
    }

    let prefix_cached = measurement.prefix.len() == props.item_count + 1;
    let mut keys: Option<Vec<String>> = None;
    let mut reset_scroll = false;

    let reusable = if let Some(revision) = props.revision {
        let same_revision = measurement.revision == Some(revision);
        if !same_revision || !prefix_cached || width_changed {
            // The node id survived, but the list it renders changed: cached
            // heights and the scroll position belong to the previous items.
            if !same_revision {
                measurement.heights.clear();
                measurement.prefix.clear();
                measurement.window = (0, 0);
                reset_scroll = true;
            }
            measurement.revision = Some(revision);
        }
        same_revision && prefix_cached && !width_changed && !estimate_changed
    } else {
        // No revision supplied: hash every key to detect a different list.
        let all_keys: Vec<String> = (0..props.item_count)
            .map(|index| (props.keys)(index))
            .collect();

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
        keys = Some(all_keys);
        measurement.signature == signature
            && measurement.prefix.len() == props.item_count + 1
            && !width_changed
            && !estimate_changed
    };

    if reset_scroll {
        cx.info.reset_scroll.push(id);
    }

    if !reusable {
        let all_keys = keys.get_or_insert_with(|| {
            (0..props.item_count)
                .map(|index| (props.keys)(index))
                .collect()
        });
        let mut prefix = Vec::with_capacity(props.item_count + 1);
        prefix.push(0.0);
        let mut total = 0.0;
        for key in all_keys.iter() {
            let hash = hash_string(key);
            total += measurement
                .heights
                .get(&hash)
                .copied()
                .unwrap_or(measurement.estimate);
            prefix.push(total);
        }
        measurement.prefix = prefix;
    }

    let prefix = std::mem::take(&mut measurement.prefix);
    let total = prefix.last().copied().unwrap_or(0.0);

    let offset = if cx.info.reset_scroll.contains(&id) {
        0.0
    } else {
        cx.offsets.get(&id).copied().unwrap_or(0.0)
    };
    let (start, end) = variable_window(
        &prefix,
        cx.viewport.height,
        offset,
        props.buffer,
        cx.pins.get(&id).copied(),
    );
    measurement.window = (start, end);

    let column_id = node_id(id, 0, None, ElementKind::Column.tag());
    let mut children = Vec::with_capacity(end - start + 2);
    let mut rows = Vec::with_capacity(end - start);

    if start > 0 {
        children.push(build_spacer(column_id, children.len(), prefix[start], cx));
    }

    for index in start..end {
        let key = match &keys {
            Some(keys) => keys[index].clone(),
            None => (props.keys)(index),
        };
        let mut row = RawElement::container(ElementKind::Column);
        row.style.width = Length::Percent(1.0);
        row.key = Some(Cow::Owned(key.clone()));
        row.children.push((props.rows)(index));

        let position = children.len();
        let node = build_node(&row, column_id, position, cx);
        record_focus(cx, id, index, &node);
        rows.push((node.id, hash_string(&key), index));
        children.push(node);
    }

    if end < props.item_count {
        children.push(build_spacer(
            column_id,
            children.len(),
            (total - prefix[end]).max(0.0),
            cx,
        ));
    }

    measurement.prefix = prefix;
    cx.measurements.insert(id, measurement);
    (content_column(column_id, children), rows)
}

/// Remembers which item holds the focused node, so the next build can pin it.
fn record_focus(cx: &mut BuildContext, list: NodeId, index: usize, row: &UiNode) {
    let Some(focused) = cx.focused else {
        return;
    };
    if node_contains(row, focused) {
        cx.info.focused_rows.insert(list, index);
    }
}

fn node_contains(root: &UiNode, target: NodeId) -> bool {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.id == target {
            return true;
        }
        stack.extend(&node.children);
    }
    false
}

fn content_column(id: NodeId, children: Vec<UiNode>) -> UiNode {
    UiNode {
        id,
        kind: NodeKind::Column,
        style: Style {
            width: Length::Percent(1.0),
            ..Style::default()
        },
        children,
    }
}

fn build_spacer(parent: NodeId, index: usize, height: f32, cx: &mut BuildContext) -> UiNode {
    let mut spacer = RawElement::leaf(ElementKind::Spacer);
    spacer.style.height = Length::Points(height.max(0.0));
    build_node(&spacer, parent, index, cx)
}

/// Derives a node identifier from its parent, position or key, and kind.
///
/// The identifier is stable while the node stays at the same position, which is
/// what keeps native widgets alive across rebuilds. Supplying a key keeps the
/// identifier stable when earlier siblings are inserted or removed; keys must
/// therefore be unique among siblings.
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

pub(crate) fn hash_string(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Visits the tree in pre-order, parents before children.
pub fn preorder(root: &UiNode) -> Vec<&UiNode> {
    let mut nodes = Vec::new();
    collect_preorder(root, &mut nodes);
    nodes
}

fn collect_preorder<'tree>(node: &'tree UiNode, nodes: &mut Vec<&'tree UiNode>) {
    nodes.push(node);
    for child in &node.children {
        collect_preorder(child, nodes);
    }
}

/// Indexes every node of the tree by its identifier.
pub fn index(root: &UiNode) -> HashMap<NodeId, &UiNode> {
    preorder(root)
        .into_iter()
        .map(|node| (node.id, node))
        .collect()
}

/// Finds a node by its identifier.
pub fn find(root: &UiNode, id: NodeId) -> Option<&UiNode> {
    preorder(root).into_iter().find(|node| node.id == id)
}

/// Maps every node to the nearest renderable ancestor that hosts its native view.
///
/// Layout-only nodes stay virtual, so their descendants are parented to the
/// closest node that actually owns a view. The synthetic root is not returned
/// as a parent: `None` means "a direct child of the platform root view".
pub fn renderable_parents(root: &UiNode) -> HashMap<NodeId, Option<NodeId>> {
    let mut parents = HashMap::new();
    collect_renderable_parents(root, None, &mut parents);
    parents
}

fn collect_renderable_parents(
    node: &UiNode,
    nearest: Option<NodeId>,
    parents: &mut HashMap<NodeId, Option<NodeId>>,
) {
    for child in &node.children {
        parents.insert(child.id, nearest);

        let next = if child.is_renderable() {
            Some(child.id)
        } else {
            nearest
        };
        collect_renderable_parents(child, next, parents);
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::Element;
    use std::sync::Arc;

    use super::*;
    use crate::ui::{Button, Column, Switch, TabBar, Text};

    fn sample() -> Element {
        Column::new()
            .child(Text::new("Hello"))
            .child(Button::new("Click").on_click(()))
            .into()
    }

    #[test]
    fn build_creates_a_root_and_children() {
        let tree = build(&sample());

        assert_eq!(tree.id, ROOT_ID);
        assert_eq!(tree.kind, NodeKind::Root);
        assert_eq!(tree.children.len(), 1);

        let column = &tree.children[0];
        assert_eq!(column.kind, NodeKind::Column);
        assert_eq!(column.children.len(), 2);
        assert!(column.children[0].kind.is_native());
        assert!(column.children[1].kind.is_native());
    }

    fn list_of(count: usize) -> Element {
        crate::ui::List::new(Arc::new(
            (0..count)
                .map(|index| format!("Row {index}"))
                .collect::<Vec<_>>(),
        ))
        .item_height(50.0)
        .buffer(2)
        .key(|item| item.clone())
        .row(|item| crate::ui::Text::new(item.clone()).into())
        .into()
    }

    #[test]
    fn a_list_only_materializes_the_window_around_the_viewport() {
        let offsets = HashMap::new();
        let (tree, info) = build_with(&list_of(100), &offsets, Size::new(300.0, 200.0));

        let scroll = &tree.children[0];
        let column = &scroll.children[0];
        // Four visible rows plus two buffered ones, plus the trailing spacer.
        assert_eq!(column.children.len(), 7);
        assert_eq!(info.lists[&scroll.id].count, 100);
        assert_eq!(info.lists[&scroll.id].item_height, Some(50.0));
        // The spacer represents the 94 rows that are not materialized.
        assert_eq!(column.children[6].style.height, Length::Points(94.0 * 50.0));
    }

    #[test]
    fn variable_window_follows_prefix_offsets() {
        let prefix = [0.0, 50.0, 100.0, 200.0, 250.0];

        assert_eq!(variable_window(&prefix, 100.0, 0.0, 0, None), (0, 2));
        assert_eq!(variable_window(&prefix, 100.0, 60.0, 0, None), (1, 3));
        assert_eq!(variable_window(&prefix, 100.0, 60.0, 1, None), (0, 4));
        assert_eq!(variable_window(&prefix, 100.0, 1_000.0, 0, None), (2, 4));
        assert_eq!(variable_window(&[], 100.0, 0.0, 0, None), (0, 0));
        // A pinned row outside the window extends it, keeping the range whole.
        assert_eq!(variable_window(&prefix, 100.0, 60.0, 0, Some(0)), (0, 3));
        assert_eq!(variable_window(&prefix, 100.0, 0.0, 0, Some(3)), (0, 4));
        assert_eq!(variable_window(&prefix, 100.0, 0.0, 0, Some(9)), (0, 2));
    }

    #[test]
    fn a_variable_list_starts_from_estimated_heights() {
        let element: Element = crate::ui::List::new(Arc::new(
            (0..100)
                .map(|index| format!("Row {index}"))
                .collect::<Vec<_>>(),
        ))
        .buffer(2)
        .key(|item| item.clone())
        .row(|item| crate::ui::Text::new(item.clone()).into())
        .into();

        let mut measurements = HashMap::new();
        let (tree, info) = build_with_measurements(
            &element,
            &HashMap::new(),
            Size::new(300.0, 200.0),
            &mut measurements,
        );

        let list_id = tree.children[0].id;
        assert_eq!(info.lists[&list_id].item_height, None);
        assert_eq!(measurements[&list_id].prefix.len(), 101);
        assert_eq!(measurements[&list_id].prefix[100], 56.0 * 100.0);
        let rows = &info.rows[&list_id];
        assert!(rows.len() >= 4, "window too small: {}", rows.len());
        // The column holds the rows plus the trailing spacer.
        assert_eq!(tree.children[0].children[0].children.len(), rows.len() + 1);
    }

    #[test]
    fn an_items_revision_reuses_the_prefix_and_only_keys_the_window() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let count = 10_000;
        let calls = Arc::new(AtomicUsize::new(0));
        let element = |revision: u64| -> Element {
            let calls = Arc::clone(&calls);
            crate::ui::List::new(Arc::new((0..count).collect::<Vec<usize>>()))
                .revision(revision)
                .buffer(2)
                .key(move |index| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    format!("Row {index}")
                })
                .row(|index| crate::ui::Text::new(format!("Row {index}")).into())
                .into()
        };

        let viewport = Size::new(300.0, 200.0);
        let mut measurements = HashMap::new();
        let (tree, _) =
            build_with_measurements(&element(1), &HashMap::new(), viewport, &mut measurements);
        let list_id = tree.children[0].id;
        assert_eq!(calls.load(Ordering::Relaxed), count);
        let prefix_ptr = measurements[&list_id].prefix.as_ptr();

        let (_, info) =
            build_with_measurements(&element(1), &HashMap::new(), viewport, &mut measurements);
        let visible = info.rows[&list_id].len();
        assert_eq!(
            calls.load(Ordering::Relaxed) - count,
            visible,
            "a stable revision must only key the materialized rows"
        );
        assert_eq!(
            measurements[&list_id].prefix.as_ptr(),
            prefix_ptr,
            "the cached prefix must be reused without reallocating"
        );

        let before = calls.load(Ordering::Relaxed);
        build_with_measurements(&element(2), &HashMap::new(), viewport, &mut measurements);
        assert_eq!(
            calls.load(Ordering::Relaxed) - before,
            count,
            "a new revision must re-key every item"
        );
    }

    #[test]
    fn a_new_width_rekeys_a_revisioned_list() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let count = 1_000;
        let calls = Arc::new(AtomicUsize::new(0));
        let element = {
            let calls = Arc::clone(&calls);
            crate::ui::List::new(Arc::new((0..count).collect::<Vec<usize>>()))
                .revision(1)
                .key(move |index| {
                    calls.fetch_add(1, Ordering::Relaxed);
                    format!("Row {index}")
                })
                .row(|index| crate::ui::Text::new(format!("Row {index}")).into())
                .into()
        };

        let mut measurements = HashMap::new();
        build_with_measurements(
            &element,
            &HashMap::new(),
            Size::new(300.0, 200.0),
            &mut measurements,
        );
        let before = calls.load(Ordering::Relaxed);
        build_with_measurements(
            &element,
            &HashMap::new(),
            Size::new(200.0, 200.0),
            &mut measurements,
        );
        assert_eq!(calls.load(Ordering::Relaxed) - before, count);
    }

    #[test]
    fn a_scrolled_list_moves_its_window_but_keeps_row_identifiers() {
        let element = list_of(100);
        let base = build_with(&element, &HashMap::new(), Size::new(300.0, 200.0)).0;
        let scroll_id = base.children[0].id;

        let mut offsets = HashMap::new();
        offsets.insert(scroll_id, 300.0);
        let (scrolled, _) = build_with(&element, &offsets, Size::new(300.0, 200.0));

        let base_rows = &base.children[0].children[0].children;
        let scrolled_rows = &scrolled.children[0].children[0].children;
        // Offset 300 starts at row 6; with a buffer of 2 the first row is 4.
        // Rows 4 and 5 overlap the unscrolled window at positions 4 and 5.
        // The scrolled column starts with a spacer for rows 0..4.
        assert_eq!(base_rows[4].id, scrolled_rows[1].id);
        assert_eq!(base_rows[5].id, scrolled_rows[2].id);
        assert_eq!(scrolled_rows[0].style.height, Length::Points(4.0 * 50.0));
        assert_eq!(
            scrolled_rows.last().unwrap().style.height,
            Length::Points((100.0 - 12.0) * 50.0)
        );
    }

    #[test]
    fn a_list_builds_rows_only_for_the_visible_window() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let built = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&built);
        let element: Element = crate::ui::List::new(Arc::new(
            (0..500)
                .map(|index| format!("Row {index}"))
                .collect::<Vec<_>>(),
        ))
        .item_height(50.0)
        .buffer(2)
        .key(|item| item.clone())
        .row(move |item| {
            counter.fetch_add(1, Ordering::Relaxed);
            crate::ui::Text::new(item.clone()).into()
        })
        .into();

        let _ = build_with(&element, &HashMap::new(), Size::new(300.0, 200.0));

        assert_eq!(built.load(Ordering::Relaxed), 6);
    }

    #[test]
    fn an_empty_list_has_no_rows() {
        let element: Element = crate::ui::List::new(Arc::new(Vec::<String>::new()))
            .item_height(50.0)
            .key(|item| item.clone())
            .row(|item| crate::ui::Text::new(item.clone()).into())
            .into();

        let (tree, info) = build_with(&element, &HashMap::new(), Size::new(300.0, 200.0));
        let column = &tree.children[0].children[0];

        assert!(column.children.is_empty());
        assert_eq!(info.lists[&tree.children[0].id].count, 0);
    }

    #[test]
    fn rebuilding_the_same_element_reuses_identifiers() {
        let first = build(&sample());
        let second = build(&sample());

        let first_ids: Vec<_> = preorder(&first).iter().map(|node| node.id).collect();
        let second_ids: Vec<_> = preorder(&second).iter().map(|node| node.id).collect();
        assert_eq!(first_ids, second_ids);
    }

    #[test]
    fn changing_the_content_keeps_the_identifier() {
        let first = build(&Column::new().child(Text::new("one")).into());
        let second = build(&Column::new().child(Text::new("two")).into());

        assert_eq!(
            first.children[0].children[0].id,
            second.children[0].children[0].id
        );
    }

    #[test]
    fn keys_stabilize_identifiers_when_siblings_change() {
        let with_key = |name: &str| -> Element {
            Column::new()
                .child(Text::new("a").key("a"))
                .child(Text::new(name).key("b"))
                .into()
        };

        let first = build(&with_key("one"));
        let second = build(&with_key("two"));

        let first_b = &first.children[0].children[1];
        let second_b = &second.children[0].children[1];
        assert_eq!(first_b.id, second_b.id);
    }

    #[test]
    fn dynamic_keys_hash_like_static_keys() {
        let borrowed = build(&Column::new().child(Text::new("a").key("item-7")).into());
        let owned = build(
            &Column::new()
                .child(Text::new("a").key(String::from("item-7")))
                .into(),
        );

        assert_eq!(
            borrowed.children[0].children[0].id,
            owned.children[0].children[0].id
        );
    }

    #[test]
    fn inserting_an_unkeyed_sibling_keeps_keyed_identifiers() {
        let first = build(
            &Column::new()
                .child(Text::new("a").key("a"))
                .child(Text::new("b").key("b"))
                .into(),
        );
        let second = build(
            &Column::new()
                .child(Text::new("new"))
                .child(Text::new("a").key("a"))
                .child(Text::new("b").key("b"))
                .into(),
        );

        assert_eq!(
            first.children[0].children[0].id,
            second.children[0].children[1].id
        );
        assert_eq!(
            first.children[0].children[1].id,
            second.children[0].children[2].id
        );
    }

    #[test]
    fn changing_a_key_changes_only_that_identifier() {
        let first = build(
            &Column::new()
                .child(Text::new("a").key("a"))
                .child(Text::new("b").key("b"))
                .into(),
        );
        let second = build(
            &Column::new()
                .child(Text::new("a").key("a-changed"))
                .child(Text::new("b").key("b"))
                .into(),
        );

        assert_ne!(
            first.children[0].children[0].id,
            second.children[0].children[0].id
        );
        assert_eq!(
            first.children[0].children[1].id,
            second.children[0].children[1].id
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "duplicate element key")]
    fn duplicate_sibling_keys_panic() {
        build(
            &Column::new()
                .child(Text::new("a").key("dup"))
                .child(Text::new("b").key("dup"))
                .into(),
        );
    }

    #[test]
    fn different_kinds_get_different_identifiers() {
        let text = build(&Column::new().child(Text::new("x")).into());
        let button = build(&Column::new().child(Button::new("x")).into());
        let switch = build(&Column::new().child(Switch::new(false)).into());

        assert_ne!(
            text.children[0].children[0].id,
            button.children[0].children[0].id
        );
        assert_ne!(
            text.children[0].children[0].id,
            switch.children[0].children[0].id
        );
    }

    #[test]
    fn a_switch_is_native() {
        let tree = build(&Column::new().child(Switch::new(true)).into());

        assert!(matches!(
            tree.children[0].children[0].kind,
            NodeKind::Widget(Widget::Switch(_))
        ));
        assert!(tree.children[0].children[0].kind.is_native());
    }

    #[test]
    fn a_tab_bar_is_native() {
        let tree = build(&Column::new().child(TabBar::new(["One", "Two"])).into());

        assert!(matches!(
            tree.children[0].children[0].kind,
            NodeKind::Widget(Widget::TabBar(_))
        ));
        assert!(tree.children[0].children[0].kind.is_native());
    }

    #[test]
    fn index_and_find_cover_every_node() {
        let tree = build(&sample());
        let index = index(&tree);

        assert_eq!(index.len(), preorder(&tree).len());
        assert_eq!(find(&tree, tree.children[0].id), Some(&tree.children[0]));
        assert!(find(&tree, NodeId(12345)).is_none());
    }

    #[test]
    fn row_roots_are_stable_and_unique_per_item() {
        let list = NodeId(7);
        assert_eq!(row_root(list, 3), row_root(list, 3));
        assert_ne!(row_root(list, 3), row_root(list, 4));
        assert_ne!(row_root(list, 3), row_root(NodeId(8), 3));
        // The root must never collide with the synthetic tree root.
        assert_ne!(row_root(list, 0), ROOT_ID);
    }

    #[test]
    fn stamping_reaches_nested_recyclers() {
        let mut tree = UiNode {
            id: ROOT_ID,
            kind: NodeKind::Root,
            style: Style::default(),
            children: vec![UiNode {
                id: NodeId(10),
                kind: NodeKind::Recycler(RecyclerProps::new(4, None)),
                style: Style::default(),
                children: vec![UiNode {
                    id: NodeId(11),
                    kind: NodeKind::Recycler(RecyclerProps::new(2, Some(10.0))),
                    style: Style::default(),
                    children: Vec::new(),
                }],
            }],
        };

        stamp_recycler_revision(&mut tree, 42);

        for child in &tree.children[0].children {
            match &child.kind {
                NodeKind::Recycler(props) => assert_eq!(props.revision, 42),
                _ => panic!("expected a recycler"),
            }
        }
        assert!(matches!(
            &tree.children[0].kind,
            NodeKind::Recycler(RecyclerProps {
                revision: 42,
                item_height: None,
                ..
            })
        ));
    }
}
