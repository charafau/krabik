//! The UI runtime.
//!
//! The runtime owns the current tree and its frames. Every update rebuilds the
//! tree, reconciles it with the previous one, lays it out, and emits one batch
//! of operations for the platform renderer.

use std::collections::HashMap;

use crate::geometry::{Rect, Size};
use crate::layout::{self, LayoutResult};
use crate::measure::Measure;
use crate::render::{self, Op, Renderer};
use crate::tree::{self, NodeId, NodeKind, ROOT_ID, UiNode};
use crate::ui::Element;

/// The result of rebuilding the UI: the used size and the operations to apply.
#[derive(Clone, Debug, Default)]
pub struct Update {
    /// The size the root node occupies in the viewport.
    pub size: Size,
    /// The operations the renderer has to apply.
    pub ops: Vec<Op>,
}

impl Update {
    /// Whether the renderer has nothing to do.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Owns the UI tree and coordinates layout and reconciliation.
pub struct Runtime {
    tree: UiNode,
    layout: LayoutResult,
    measurer: Box<dyn Measure>,
}

impl Runtime {
    /// Creates a runtime with the given measurement implementation.
    pub fn new(measurer: Box<dyn Measure>) -> Self {
        Self {
            tree: UiNode::empty(),
            layout: LayoutResult::default(),
            measurer,
        }
    }

    /// Rebuilds the tree from `element` and returns the batch to apply.
    pub fn update(&mut self, element: &Element, viewport: Size) -> Update {
        let next = tree::build(element);

        let mut ops = render::diff(&self.tree, &next);
        let layout = layout::compute(&next, viewport, self.measurer.as_ref());
        ops.extend(render::diff_layout(
            &self.tree,
            &next,
            &self.layout,
            &layout,
        ));

        let size = layout
            .frames
            .get(&ROOT_ID)
            .map_or(viewport, |root| Size::new(root.width, root.height));

        self.tree = next;
        self.layout = layout;

        Update { size, ops }
    }

    /// Rebuilds the tree and applies the resulting batch in one call.
    pub fn render(
        &mut self,
        element: &Element,
        viewport: Size,
        renderer: &mut dyn Renderer,
    ) -> Size {
        let update = self.update(element, viewport);
        renderer.apply(&update.ops);
        update.size
    }

    /// Returns the action of the button with the given identifier, if any.
    pub fn click(&self, id: NodeId) -> Option<&'static str> {
        let node = tree::find(&self.tree, id)?;
        match &node.kind {
            NodeKind::Button(props) if !props.action.is_empty() => Some(props.action),
            _ => None,
        }
    }

    /// Returns the action of the text input with the given identifier, if any.
    pub fn text_field_action(&self, id: NodeId) -> Option<&'static str> {
        let node = tree::find(&self.tree, id)?;
        match &node.kind {
            NodeKind::TextField(props) if !props.action.is_empty() => Some(props.action),
            _ => None,
        }
    }

    /// The current tree.
    pub fn tree(&self) -> &UiNode {
        &self.tree
    }

    /// The frames of the last layout pass.
    pub fn frames(&self) -> &HashMap<NodeId, Rect> {
        &self.layout.frames
    }

    /// The scrollable content sizes of the last layout pass.
    pub fn content_sizes(&self) -> &HashMap<NodeId, Size> {
        &self.layout.content_sizes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Constraints;
    use crate::measure::ApproximateMeasure;
    use crate::render::RecordingRenderer;
    use crate::ui::{Button, Column, Text, TextField, TextProps};

    fn viewport() -> Size {
        Size::new(300.0, 600.0)
    }

    fn screen(content: &str) -> Element {
        Column::new()
            .child(Text::new(content))
            .child(Button::new("Click").action("click"))
            .into()
    }

    #[test]
    fn the_first_update_creates_every_native_node() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let update = runtime.update(&screen("Hello"), viewport());

        let creates = update
            .ops
            .iter()
            .filter(|op| matches!(op, Op::Create(_)))
            .count();
        assert_eq!(creates, 2);
        assert_eq!(
            update
                .ops
                .iter()
                .filter(|op| matches!(op, Op::Layout { .. }))
                .count(),
            2
        );
    }

    #[test]
    fn rebuilding_an_unchanged_screen_emits_nothing() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());
        let update = runtime.update(&screen("Hello"), viewport());

        assert!(update.is_empty(), "{:?}", update.ops);
    }

    #[test]
    fn changing_text_emits_a_single_update() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());
        let update = runtime.update(&screen("Hello, Rust!"), viewport());

        assert_eq!(update.ops.len(), 1, "{:?}", update.ops);
        assert!(matches!(&update.ops[0], Op::Update(_)));
    }

    #[test]
    fn clicks_resolve_to_the_button_action() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());

        let button = runtime.tree().children[0]
            .children
            .iter()
            .find(|node| matches!(node.kind, NodeKind::Button(_)))
            .unwrap();

        assert_eq!(runtime.click(button.id), Some("click"));
        assert_eq!(runtime.click(ROOT_ID), None);
    }

    #[test]
    fn text_fields_resolve_to_their_action() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let element: Element = Column::new()
            .child(TextField::new("").placeholder("Name").action("name"))
            .into();
        runtime.update(&element, viewport());

        let field = runtime.tree().children[0].children[0].id;
        assert_eq!(runtime.text_field_action(field), Some("name"));
        assert_eq!(runtime.click(field), None);
    }

    #[test]
    fn render_applies_one_batch() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let mut renderer = RecordingRenderer::default();

        runtime.render(&screen("Hello"), viewport(), &mut renderer);
        runtime.render(&screen("Hello"), viewport(), &mut renderer);

        assert_eq!(renderer.batches.len(), 2);
        assert!(!renderer.batches[0].is_empty());
        assert!(renderer.batches[1].is_empty());
    }

    #[test]
    fn a_narrower_viewport_moves_existing_widgets() {
        struct TwoLines;

        impl Measure for TwoLines {
            fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
                let width = constraints.max_width.unwrap_or(200.0);
                let lines = if text.content.len() > 10 { 2.0 } else { 1.0 };
                Size::new(width, 20.0 * lines)
            }
        }

        let element: Element = Column::new()
            .width_percent(1.0)
            .child(Text::new("a long text"))
            .into();

        let mut runtime = Runtime::new(Box::new(TwoLines));
        runtime.update(&element, Size::new(300.0, 600.0));
        let update = runtime.update(&element, Size::new(100.0, 600.0));

        assert!(
            update.ops.iter().any(|op| matches!(op, Op::Layout { .. })),
            "{:?}",
            update.ops
        );
        assert!(
            !update
                .ops
                .iter()
                .any(|op| matches!(op, Op::Create(_) | Op::Update(_))),
            "{:?}",
            update.ops
        );
    }
}
