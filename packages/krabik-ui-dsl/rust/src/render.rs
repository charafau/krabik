//! Reconciliation between two UI trees.
//!
//! A rebuild produces a minimal batch of operations: nodes that appeared are
//! created, nodes that changed are updated, nodes that disappeared are removed,
//! and native nodes whose frame moved are moved. Layout-only nodes never reach
//! the renderer.

use std::collections::HashMap;

use crate::geometry::Rect;
use crate::tree::{self, NodeId, NodeKind, UiNode};
use crate::ui::Style;

/// The data the renderer needs to create or update a native widget.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderNode {
    /// The stable identifier of the node.
    pub id: NodeId,
    /// The kind and properties of the node.
    pub kind: NodeKind,
    /// Layout and decoration properties.
    pub style: Style,
}

impl RenderNode {
    fn new(node: &UiNode) -> Self {
        Self {
            id: node.id,
            kind: node.kind.clone(),
            style: node.style.clone(),
        }
    }
}

/// A single renderer operation.
#[derive(Clone, Debug, PartialEq)]
pub enum Op {
    /// Create the native widget of a new node.
    Create(RenderNode),
    /// Update the properties of an existing native widget.
    Update(RenderNode),
    /// Remove the native widget of a node.
    Remove {
        /// The node to remove.
        id: NodeId,
    },
    /// Move the native widget of a node to its calculated frame.
    Layout {
        /// The node to move.
        id: NodeId,
        /// The absolute frame in logical points.
        frame: Rect,
    },
}

/// Applies batches of operations to a platform.
pub trait Renderer {
    /// Applies one batch. All operations of a batch belong to the same frame.
    fn apply(&mut self, ops: &[Op]);
}

/// Records every batch it receives, for tests and debugging.
#[derive(Clone, Debug, Default)]
pub struct RecordingRenderer {
    /// The batches received so far, oldest first.
    pub batches: Vec<Vec<Op>>,
}

impl Renderer for RecordingRenderer {
    fn apply(&mut self, ops: &[Op]) {
        self.batches.push(ops.to_vec());
    }
}

/// Compares the previous and the next tree and returns the property operations.
///
/// A widget whose kind changed cannot be updated in place, so its removal is
/// emitted immediately before the creation of its replacement. All other
/// removals come last, after the creations and updates.
pub fn diff(old: &UiNode, new: &UiNode) -> Vec<Op> {
    let previous = tree::index(old);

    let mut ops = Vec::new();
    for node in tree::preorder(new) {
        if !node.kind.is_native() {
            continue;
        }

        match previous.get(&node.id) {
            None => ops.push(Op::Create(RenderNode::new(node))),
            // A native widget cannot change class, so the old one is removed
            // before the new one is created under the same identifier.
            Some(previous)
                if std::mem::discriminant(&previous.kind) != std::mem::discriminant(&node.kind) =>
            {
                ops.push(Op::Remove { id: node.id });
                ops.push(Op::Create(RenderNode::new(node)));
            }
            Some(previous) if previous.kind != node.kind || previous.style != node.style => {
                ops.push(Op::Update(RenderNode::new(node)));
            }
            Some(_) => {}
        }
    }

    let next = tree::index(new);
    for node in tree::preorder(old) {
        if node.kind.is_native() && !next.contains_key(&node.id) {
            ops.push(Op::Remove { id: node.id });
        }
    }

    ops
}

/// Compares the previous and the next frames and returns the movement operations.
pub fn diff_layout(
    new: &UiNode,
    old_frames: &HashMap<NodeId, Rect>,
    new_frames: &HashMap<NodeId, Rect>,
) -> Vec<Op> {
    tree::preorder(new)
        .into_iter()
        .filter(|node| node.kind.is_native())
        .filter_map(|node| {
            let frame = *new_frames.get(&node.id)?;
            (old_frames.get(&node.id) != Some(&frame)).then_some(Op::Layout { id: node.id, frame })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{Button, Column, Element, Row, Text};

    fn tree(element: Element) -> UiNode {
        crate::tree::build(&element)
    }

    fn text_tree(content: &str) -> UiNode {
        tree(Column::new().child(Text::new(content)).into())
    }

    #[test]
    fn first_build_creates_native_nodes() {
        let old = UiNode::empty();
        let new = tree(
            Column::new()
                .child(Text::new("a"))
                .child(Button::new("b").action("b"))
                .into(),
        );

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 2);
        assert!(matches!(&ops[0], Op::Create(node) if node.id == new.children[0].children[0].id));
        assert!(matches!(&ops[1], Op::Create(node) if node.id == new.children[0].children[1].id));
    }

    #[test]
    fn changing_text_updates_the_same_widget() {
        let old = text_tree("one");
        let new = text_tree("two");

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            Op::Update(node) => {
                assert_eq!(node.id, new.children[0].children[0].id);
                assert_eq!(node.id, old.children[0].children[0].id);
                assert!(matches!(&node.kind, NodeKind::Text(props) if props.content == "two"));
            }
            other => panic!("expected update, got {other:?}"),
        }
    }

    #[test]
    fn layout_only_changes_do_not_touch_the_renderer() {
        let old = tree(Column::new().child(Text::new("a")).into());
        let new = tree(Column::new().padding(8).child(Text::new("a")).into());

        assert!(diff(&old, &new).is_empty());
    }

    #[test]
    fn adding_a_sibling_creates_only_the_new_widget() {
        let old = tree(Column::new().child(Text::new("a")).into());
        let new = tree(
            Column::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .into(),
        );

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], Op::Create(node) if node.id == new.children[0].children[1].id));
    }

    #[test]
    fn removing_a_sibling_removes_only_its_widget() {
        let old = tree(
            Column::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .into(),
        );
        let new = tree(Column::new().child(Text::new("a")).into());

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], Op::Remove { id } if *id == old.children[0].children[1].id));
    }

    #[test]
    fn changing_a_kind_replaces_the_widget() {
        let old = tree(Column::new().child(Text::new("x")).into());
        let new = tree(Column::new().child(Button::new("x")).into());

        let ops = diff(&old, &new);
        assert!(
            ops.iter().any(|op| matches!(op, Op::Remove { .. })),
            "{ops:?}"
        );
        assert!(ops.iter().any(|op| matches!(op, Op::Create(_))), "{ops:?}");
        assert!(!ops.iter().any(|op| matches!(op, Op::Update(_))), "{ops:?}");
    }

    #[test]
    fn layout_ops_only_cover_moved_native_nodes() {
        let tree = tree(
            Row::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .into(),
        );
        let native = tree.children[0].children[0].id;

        let old_frames = HashMap::from([
            (native, Rect::new(0.0, 0.0, 10.0, 10.0)),
            (tree.id, Rect::new(0.0, 0.0, 100.0, 100.0)),
        ]);
        let new_frames = HashMap::from([
            (native, Rect::new(0.0, 0.0, 10.0, 10.0)),
            (tree.id, Rect::new(0.0, 0.0, 100.0, 200.0)),
        ]);

        assert!(diff_layout(&tree, &old_frames, &new_frames).is_empty());

        let moved = HashMap::from([(native, Rect::new(5.0, 0.0, 10.0, 10.0))]);
        let ops = diff_layout(&tree, &old_frames, &moved);
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            Op::Layout {
                id: native,
                frame: Rect::new(5.0, 0.0, 10.0, 10.0)
            }
        );
    }
}
