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
    Axis, ButtonProps, Element, ElementKind, ImageProps, Length, ListProps, ScrollProps, Style,
    SwitchProps, TextFieldProps, TextProps,
};

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
    /// A run of text rendered by a native label.
    Text(TextProps),
    /// A native button.
    Button(ButtonProps),
    /// A native text input.
    TextField(TextFieldProps),
    /// A native on/off switch.
    Switch(SwitchProps),
    /// A native image.
    Image(ImageProps),
    /// A native scroll container.
    Scroll(ScrollProps),
}

impl NodeKind {
    /// Whether the node corresponds to an actual native widget.
    pub fn is_native(&self) -> bool {
        matches!(
            self,
            Self::Text(_)
                | Self::Button(_)
                | Self::TextField(_)
                | Self::Switch(_)
                | Self::Image(_)
                | Self::Scroll(_)
        )
    }

    /// Whether the node lays out children in a flex container.
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Self::Root | Self::Column | Self::Row | Self::Scroll(_)
        )
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
    /// The fixed height of one row, in logical points.
    pub item_height: f32,
    /// Extra rows materialized above and below the viewport.
    pub buffer: usize,
}

/// What a build learned about the tree, besides the tree itself.
#[derive(Debug, Default)]
pub struct BuildInfo {
    /// The extent of every list, keyed by its scroll node id.
    pub lists: HashMap<NodeId, ListExtent>,
}

/// Converts a declarative element into the internal tree.
///
/// Lists are materialized at offset zero; use [`build_with`] to account for
/// scroll positions.
pub fn build(element: &Element) -> UiNode {
    let offsets = HashMap::new();
    build_with(element, &offsets, Size::ZERO).0
}

/// Converts a declarative element into the internal tree with scroll state.
pub fn build_with(
    element: &Element,
    offsets: &HashMap<NodeId, f32>,
    viewport: Size,
) -> (UiNode, BuildInfo) {
    let mut info = BuildInfo::default();
    let mut root = UiNode::empty();
    let mut context = BuildContext {
        offsets,
        viewport,
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
    info: &'a mut BuildInfo,
}

/// The half-open range of items a list materializes for a viewport and offset.
///
/// The range includes `buffer` rows on each side so small scrolls do not
/// immediately cross a boundary.
pub fn list_window(extent: ListExtent, viewport: f32, offset: f32) -> (usize, usize) {
    if extent.count == 0 || extent.item_height <= 0.0 {
        return (0, 0);
    }

    let offset = offset.clamp(0.0, max_scroll_offset(extent, viewport));
    let first = (offset / extent.item_height).floor() as usize;
    let visible = ((viewport / extent.item_height).ceil() as usize).max(1);
    let start = first.saturating_sub(extent.buffer).min(extent.count);
    let end = (first + visible + extent.buffer).min(extent.count);
    (start, end)
}

/// The largest meaningful scroll offset of a list.
pub fn max_scroll_offset(extent: ListExtent, viewport: f32) -> f32 {
    (extent.count as f32 * extent.item_height - viewport).max(0.0)
}

fn root_style() -> Style {
    Style {
        width: Length::Percent(1.0),
        height: Length::Percent(1.0),
        ..Style::default()
    }
}

fn build_node(element: &Element, parent: NodeId, index: usize, cx: &mut BuildContext) -> UiNode {
    let id = node_id(parent, index, element.key(), element.kind.tag());

    if let ElementKind::List(props) = &element.kind {
        return build_list(element, id, props, cx);
    }

    let kind = match &element.kind {
        ElementKind::Column => NodeKind::Column,
        ElementKind::Row => NodeKind::Row,
        ElementKind::Spacer => NodeKind::Spacer,
        ElementKind::Text(props) => NodeKind::Text(props.clone()),
        ElementKind::Button(props) => NodeKind::Button(props.clone()),
        ElementKind::TextField(props) => NodeKind::TextField(props.clone()),
        ElementKind::Switch(props) => NodeKind::Switch(props.clone()),
        ElementKind::Image(props) => NodeKind::Image(props.clone()),
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

fn build_children(element: &Element, parent: NodeId, cx: &mut BuildContext) -> Vec<UiNode> {
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
fn build_list(element: &Element, id: NodeId, props: &ListProps, cx: &mut BuildContext) -> UiNode {
    let extent = ListExtent {
        count: props.item_count,
        item_height: props.item_height,
        buffer: props.buffer,
    };
    cx.info.lists.insert(id, extent);

    let offset = cx.offsets.get(&id).copied().unwrap_or(0.0);
    let (start, end) = list_window(extent, cx.viewport.height, offset);

    let column_id = node_id(id, 0, None, ElementKind::Column.tag());
    let mut children = Vec::with_capacity(end - start + 2);

    if start > 0 {
        children.push(build_spacer(
            column_id,
            children.len(),
            start as f32 * props.item_height,
            cx,
        ));
    }

    for index in start..end {
        let mut row = Element::container(ElementKind::Column);
        row.style.width = Length::Percent(1.0);
        row.style.height = Length::Points(props.item_height);
        row.key = Some(Cow::Owned((props.keys)(index)));
        row.children.push((props.rows)(index));

        let position = children.len();
        children.push(build_node(&row, column_id, position, cx));
    }

    if end < props.item_count {
        children.push(build_spacer(
            column_id,
            children.len(),
            (props.item_count - end) as f32 * props.item_height,
            cx,
        ));
    }

    let column = UiNode {
        id: column_id,
        kind: NodeKind::Column,
        style: Style {
            width: Length::Percent(1.0),
            ..Style::default()
        },
        children,
    };

    UiNode {
        id,
        kind: NodeKind::Scroll(ScrollProps {
            axis: Axis::Vertical,
        }),
        style: element.style.clone(),
        children: vec![column],
    }
}

fn build_spacer(parent: NodeId, index: usize, height: f32, cx: &mut BuildContext) -> UiNode {
    let mut spacer = Element::leaf(ElementKind::Spacer);
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

fn hash_string(value: &str) -> u64 {
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
    use std::sync::Arc;

    use super::*;
    use crate::ui::{Button, Column, Switch, Text};

    fn sample() -> Element {
        Column::new()
            .child(Text::new("Hello"))
            .child(Button::new("Click").action("click"))
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
        assert_eq!(info.lists[&scroll.id].item_height, 50.0);
        // The spacer represents the 94 rows that are not materialized.
        assert_eq!(column.children[6].style.height, Length::Points(94.0 * 50.0));
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
            NodeKind::Switch(_)
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
}
