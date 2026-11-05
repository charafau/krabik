//! The internal UI tree.
//!
//! The public [`crate::ui`] description is converted into this representation
//! before layout and rendering. Node identifiers are derived from the node's
//! position (or its key) so that rebuilding the same description yields the
//! same identifiers, which lets the renderer update native widgets in place.

use std::collections::HashMap;

use crate::ui::{ButtonProps, Element, ElementKind, Length, Style, TextProps};

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
}

impl NodeKind {
    /// Whether the node corresponds to an actual native widget.
    pub fn is_native(&self) -> bool {
        matches!(self, Self::Text(_) | Self::Button(_))
    }

    /// Whether the node lays out children in a flex container.
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Root | Self::Column | Self::Row)
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
}

/// Converts a declarative element into the internal tree.
pub fn build(element: &Element) -> UiNode {
    let mut root = UiNode::empty();
    root.children.push(build_node(element, ROOT_ID, 0));
    root
}

fn root_style() -> Style {
    Style {
        width: Length::Percent(1.0),
        height: Length::Percent(1.0),
        ..Style::default()
    }
}

fn build_node(element: &Element, parent: NodeId, index: usize) -> UiNode {
    let id = node_id(parent, index, element.key(), element.kind.tag());

    let kind = match &element.kind {
        ElementKind::Column => NodeKind::Column,
        ElementKind::Row => NodeKind::Row,
        ElementKind::Spacer => NodeKind::Spacer,
        ElementKind::Text(props) => NodeKind::Text(props.clone()),
        ElementKind::Button(props) => NodeKind::Button(props.clone()),
    };

    let children = element
        .children
        .iter()
        .enumerate()
        .map(|(index, child)| build_node(child, id, index))
        .collect();

    UiNode {
        id,
        kind,
        style: element.style.clone(),
        children,
    }
}

/// Derives a node identifier from its parent, position or key, and kind.
///
/// The identifier is stable while the node stays at the same position, which is
/// what keeps native widgets alive across rebuilds. Supplying a key keeps the
/// identifier stable when earlier siblings are inserted or removed.
fn node_id(parent: NodeId, index: usize, key: Option<&'static str>, tag: u64) -> NodeId {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{Button, Column, Text};

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
    fn different_kinds_get_different_identifiers() {
        let text = build(&Column::new().child(Text::new("x")).into());
        let button = build(&Column::new().child(Button::new("x")).into());

        assert_ne!(
            text.children[0].children[0].id,
            button.children[0].children[0].id
        );
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
