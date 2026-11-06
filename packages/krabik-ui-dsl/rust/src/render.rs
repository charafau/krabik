//! Reconciliation between two UI trees.
//!
//! A rebuild produces a minimal batch of operations: nodes that appeared are
//! created, nodes that changed are updated, nodes that disappeared are removed,
//! and native nodes whose frame moved are moved. Layout-only nodes never reach
//! the renderer.

use std::collections::HashMap;

use crate::geometry::{Rect, Size};
use crate::layout::LayoutResult;
use crate::tree::{self, NodeId, NodeKind, UiNode};
use crate::ui::Style;

/// The data the renderer needs to create or update a native widget.
#[derive(Clone, Debug, PartialEq)]
pub struct RenderNode {
    /// The stable identifier of the node.
    pub id: NodeId,
    /// The nearest renderable ancestor whose native view hosts this node.
    ///
    /// `None` means the node is a direct child of the platform root view.
    pub parent: Option<NodeId>,
    /// The kind and properties of the node.
    pub kind: NodeKind,
    /// Layout and decoration properties.
    pub style: Style,
}

impl RenderNode {
    fn new(node: &UiNode, parent: Option<NodeId>) -> Self {
        Self {
            id: node.id,
            parent,
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
    /// Move an existing native widget under another parent view.
    Reparent {
        /// The node to move.
        id: NodeId,
        /// The new nearest renderable ancestor, or the platform root.
        parent: Option<NodeId>,
    },
    /// Remove the native widget of a node.
    Remove {
        /// The node to remove.
        id: NodeId,
    },
    /// Move the native widget of a node to its calculated frame.
    Layout {
        /// The node to move.
        id: NodeId,
        /// The frame relative to the node's native parent, in logical points.
        frame: Rect,
        /// The scrollable content size, for scroll containers only.
        content_size: Option<Size>,
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
/// emitted immediately before the creation of its replacement. A node that
/// gains or loses a decoration view is created or removed without touching its
/// children, which are reparented instead. All removals come last, deepest
/// first, so a container is detached after the views it hosts.
pub fn diff(old: &UiNode, new: &UiNode) -> Vec<Op> {
    let previous = tree::index(old);
    let old_parents = tree::renderable_parents(old);
    let new_parents = tree::renderable_parents(new);

    let mut ops = Vec::new();
    let mut replaced = Vec::new();

    for node in tree::preorder(new) {
        if !node.is_renderable() {
            continue;
        }

        let parent = new_parents.get(&node.id).copied().flatten();
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
                ops.push(Op::Remove { id: node.id });
                ops.push(Op::Create(RenderNode::new(node, parent)));
                replaced.push(node.id);
            }
            Some(previous) => {
                let old_parent = old_parents.get(&node.id).copied().flatten();
                if old_parent != parent {
                    ops.push(Op::Reparent {
                        id: node.id,
                        parent,
                    });
                }
                if previous.kind != node.kind || previous.style != node.style {
                    ops.push(Op::Update(RenderNode::new(node, parent)));
                }
            }
        }
    }

    let next = tree::index(new);
    let mut removals: Vec<NodeId> = tree::preorder(old)
        .into_iter()
        .filter(|node| node.is_renderable())
        .filter(|node| next.get(&node.id).is_none_or(|next| !next.is_renderable()))
        .map(|node| node.id)
        .collect();

    // Pre-order lists parents before children; detach children first.
    removals.reverse();
    for id in removals {
        if !replaced.contains(&id) {
            ops.push(Op::Remove { id });
        }
    }

    ops
}

/// Compares the previous and the next layout and returns the movement operations.
///
/// Frames are expressed relative to the nearest renderable ancestor, which is
/// where the native view actually lives. The platform root is the origin of
/// top-level views.
pub fn diff_layout(
    old: &UiNode,
    new: &UiNode,
    old_layout: &LayoutResult,
    new_layout: &LayoutResult,
) -> Vec<Op> {
    let old_frames = relative_frames(old, &old_layout.frames);
    let new_frames = relative_frames(new, &new_layout.frames);

    tree::preorder(new)
        .into_iter()
        .filter(|node| node.is_renderable())
        .filter_map(|node| {
            let frame = *new_frames.get(&node.id)?;
            let content_size = new_layout.content_sizes.get(&node.id).copied();
            let moved = old_frames.get(&node.id) != Some(&frame);
            let content_changed = old_layout.content_sizes.get(&node.id).copied() != content_size;

            (moved || content_changed).then_some(Op::Layout {
                id: node.id,
                frame,
                content_size,
            })
        })
        .collect()
}

/// Expresses every renderable node's absolute frame relative to its native parent.
fn relative_frames(root: &UiNode, frames: &HashMap<NodeId, Rect>) -> HashMap<NodeId, Rect> {
    let mut relative = HashMap::new();
    collect_relative_frames(root, frames, (0.0, 0.0), &mut relative);
    relative
}

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
            Rect::new(
                frame.x - origin.0,
                frame.y - origin.1,
                frame.width,
                frame.height,
            ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::LayoutResult;
    use crate::ui::{Button, Column, Element, Row, ScrollView, Text};

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
    fn decorating_a_container_creates_it_and_reparents_its_children() {
        let old = tree(Column::new().child(Text::new("a")).into());
        let new = tree(
            Column::new()
                .background(0xFF22_2222)
                .child(Text::new("a"))
                .into(),
        );
        let column = new.children[0].id;
        let text = new.children[0].children[0].id;

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 2, "{ops:?}");
        assert!(
            matches!(&ops[0], Op::Create(node) if node.id == column && node.parent.is_none()),
            "{ops:?}"
        );
        assert!(
            matches!(&ops[1], Op::Reparent { id, parent: Some(parent) } if *id == text && *parent == column),
            "{ops:?}"
        );
    }

    #[test]
    fn removing_a_decoration_reparents_children_before_dropping_it() {
        let old = tree(
            Column::new()
                .background(0xFF22_2222)
                .child(Text::new("a"))
                .into(),
        );
        let new = tree(Column::new().child(Text::new("a")).into());
        let column = old.children[0].id;
        let text = old.children[0].children[0].id;

        let ops = diff(&old, &new);
        assert!(
            matches!(&ops[0], Op::Reparent { id, parent: None } if *id == text),
            "{ops:?}"
        );
        assert!(
            matches!(&ops[1], Op::Remove { id } if *id == column),
            "{ops:?}"
        );
    }

    #[test]
    fn scroll_containers_are_native_and_host_their_children() {
        let old = UiNode::empty();
        let new = tree(
            ScrollView::new()
                .child(Text::new("a"))
                .child(Text::new("b"))
                .into(),
        );
        let scroll = new.children[0].id;
        let text = new.children[0].children[0].id;

        let ops = diff(&old, &new);
        assert!(
            matches!(&ops[0], Op::Create(node) if node.id == scroll),
            "{ops:?}"
        );
        assert!(
            matches!(&ops[1], Op::Create(node) if node.id == text && node.parent == Some(scroll)),
            "{ops:?}"
        );
    }

    #[test]
    fn layout_ops_carry_the_scroll_content_size() {
        let tree = tree(ScrollView::new().child(Text::new("a")).into());
        let scroll = tree.children[0].id;
        let old_layout = LayoutResult {
            frames: HashMap::from([(scroll, Rect::new(0.0, 0.0, 100.0, 100.0))]),
            content_sizes: HashMap::from([(scroll, Size::new(100.0, 100.0))]),
        };
        let new_layout = LayoutResult {
            frames: HashMap::from([(scroll, Rect::new(0.0, 0.0, 100.0, 100.0))]),
            content_sizes: HashMap::from([(scroll, Size::new(100.0, 240.0))]),
        };

        let ops = diff_layout(&tree, &tree, &old_layout, &new_layout);
        assert_eq!(ops.len(), 1, "{ops:?}");
        assert!(matches!(
            &ops[0],
            Op::Layout {
                content_size: Some(size),
                ..
            } if *size == Size::new(100.0, 240.0)
        ));
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

        let old_layout = LayoutResult {
            frames: HashMap::from([
                (native, Rect::new(0.0, 0.0, 10.0, 10.0)),
                (tree.id, Rect::new(0.0, 0.0, 100.0, 100.0)),
            ]),
            content_sizes: HashMap::new(),
        };
        let new_layout = LayoutResult {
            frames: HashMap::from([
                (native, Rect::new(0.0, 0.0, 10.0, 10.0)),
                (tree.id, Rect::new(0.0, 0.0, 100.0, 200.0)),
            ]),
            content_sizes: HashMap::new(),
        };

        assert!(diff_layout(&tree, &tree, &old_layout, &new_layout).is_empty());

        let moved = LayoutResult {
            frames: HashMap::from([(native, Rect::new(5.0, 0.0, 10.0, 10.0))]),
            content_sizes: HashMap::new(),
        };
        let ops = diff_layout(&tree, &tree, &old_layout, &moved);
        assert_eq!(ops.len(), 1);
        assert_eq!(
            ops[0],
            Op::Layout {
                id: native,
                frame: Rect::new(5.0, 0.0, 10.0, 10.0),
                content_size: None,
            }
        );
    }
}
