//! Taffy integration.
//!
//! The internal tree is translated into a Taffy tree, laid out against the
//! viewport, and the resulting frames are read back as absolute rectangles in
//! logical points.

use std::collections::HashMap;

use taffy::geometry::{Rect as TaffyRect, Size as TaffySize};
use taffy::prelude::{
    AlignItems, AvailableSpace, Dimension, Display, FlexDirection, JustifyContent,
    LengthPercentage, LengthPercentageAuto, NodeId as TaffyId, Style as TaffyStyle, TaffyTree,
};
use taffy::tree::{LayoutInput, LayoutOutput};

use crate::geometry::{Constraints, Rect, Size};
use crate::measure::Measure;
use crate::tree::{self, NodeId, NodeKind, UiNode};
use crate::ui::{Align, Justify, Length, TextProps};

/// The horizontal padding a button adds around its label.
const BUTTON_PADDING_X: f32 = 16.0;
/// The vertical padding a button adds around its label.
const BUTTON_PADDING_Y: f32 = 12.0;
/// The smallest height a button is allowed to have.
const BUTTON_MIN_HEIGHT: f32 = 20.0;

/// Calculates the absolute frame of every node in the tree.
pub fn compute(root: &UiNode, viewport: Size, measurer: &dyn Measure) -> HashMap<NodeId, Rect> {
    let mut taffy = TaffyTree::new();
    // Layout stays fractional in logical points; the platform renderer rounds
    // when it converts to pixels.
    taffy.disable_rounding();

    let mut taffy_ids = HashMap::new();
    let taffy_root = build_taffy(&mut taffy, root, &mut taffy_ids);

    let nodes = tree::index(root);
    let by_taffy: HashMap<TaffyId, NodeId> = taffy_ids
        .iter()
        .map(|(node, taffy_id)| (*taffy_id, *node))
        .collect();

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
                        TaffySize {
                            width: size.width,
                            height: size.height,
                        }
                    },
                )
            },
        )
        .expect("taffy layout");

    let mut frames = HashMap::with_capacity(taffy_ids.len());
    collect_frames(&taffy, root, &taffy_ids, 0.0, 0.0, &mut frames);
    frames
}

fn build_taffy(
    taffy: &mut TaffyTree,
    node: &UiNode,
    ids: &mut HashMap<NodeId, TaffyId>,
) -> TaffyId {
    let children: Vec<TaffyId> = node
        .children
        .iter()
        .map(|child| build_taffy(taffy, child, ids))
        .collect();

    let taffy_id = taffy
        .new_with_children(taffy_style(node), &children)
        .expect("taffy node");

    ids.insert(node.id, taffy_id);
    taffy_id
}

fn taffy_style(node: &UiNode) -> TaffyStyle {
    let direction = match node.kind {
        NodeKind::Row => FlexDirection::Row,
        _ => FlexDirection::Column,
    };

    TaffyStyle {
        display: Display::Flex,
        flex_direction: direction,
        size: TaffySize {
            width: dimension(node.style.width),
            height: dimension(node.style.height),
        },
        padding: TaffyRect {
            left: length_percentage(node.style.padding.left),
            right: length_percentage(node.style.padding.right),
            top: length_percentage(node.style.padding.top),
            bottom: length_percentage(node.style.padding.bottom),
        },
        margin: TaffyRect {
            left: margin(node.style.margin.left),
            right: margin(node.style.margin.right),
            top: margin(node.style.margin.top),
            bottom: margin(node.style.margin.bottom),
        },
        gap: TaffySize {
            width: length_percentage(node.style.gap),
            height: length_percentage(node.style.gap),
        },
        flex_grow: node.style.grow,
        align_items: align_items(node.style.align_items),
        justify_content: justify_content(node.style.justify_content),
        ..TaffyStyle::default()
    }
}

fn dimension(length: Length) -> Dimension {
    match length {
        Length::Auto => Dimension::auto(),
        Length::Points(points) => Dimension::length(points),
        Length::Percent(percent) => Dimension::percent(percent),
    }
}

fn length_percentage(value: f32) -> LengthPercentage {
    LengthPercentage::length(value)
}

fn margin(value: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::length(value)
}

fn align_items(align: Align) -> AlignItems {
    match align {
        Align::Start => AlignItems::FLEX_START,
        Align::Center => AlignItems::CENTER,
        Align::End => AlignItems::FLEX_END,
        Align::Stretch => AlignItems::STRETCH,
    }
}

fn justify_content(justify: Justify) -> JustifyContent {
    match justify {
        Justify::Start => JustifyContent::FLEX_START,
        Justify::Center => JustifyContent::CENTER,
        Justify::End => JustifyContent::FLEX_END,
        Justify::SpaceBetween => JustifyContent::SPACE_BETWEEN,
    }
}

/// Returns the size of a node's content.
///
/// Taffy calls this both while probing the intrinsic size and during the final
/// layout. The available space has already been reduced by the node's padding,
/// so it describes the content box exactly: text is measured at the width it
/// will actually occupy, no matter whether the parent stretched it or not.
fn content_size(
    node: &UiNode,
    available: TaffySize<AvailableSpace>,
    measurer: &dyn Measure,
) -> Size {
    let max_width = match available.width {
        AvailableSpace::Definite(value) => Some(value),
        AvailableSpace::MinContent | AvailableSpace::MaxContent => None,
    };

    match &node.kind {
        NodeKind::Text(props) => measurer.measure(
            props,
            Constraints {
                max_width,
                max_height: None,
            },
        ),
        NodeKind::Button(props) => {
            // The label sits inside the button chrome, which the layout adds
            // back below.
            let inner_width = max_width.map(|width| (width - BUTTON_PADDING_X * 2.0).max(0.0));
            let label = TextProps {
                content: props.label.clone(),
                size: props.size,
                color: props.color,
                weight: props.weight,
            };
            let text = measurer.measure(
                &label,
                Constraints {
                    max_width: inner_width,
                    max_height: None,
                },
            );

            Size::new(
                text.width + BUTTON_PADDING_X * 2.0,
                text.height.max(BUTTON_MIN_HEIGHT) + BUTTON_PADDING_Y * 2.0,
            )
        }
        _ => Size::ZERO,
    }
}

fn collect_frames(
    taffy: &TaffyTree,
    node: &UiNode,
    ids: &HashMap<NodeId, TaffyId>,
    parent_x: f32,
    parent_y: f32,
    frames: &mut HashMap<NodeId, Rect>,
) {
    let layout = taffy.layout(ids[&node.id]).expect("taffy layout");

    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    frames.insert(
        node.id,
        Rect::new(x, y, layout.size.width, layout.size.height),
    );

    for child in &node.children {
        collect_frames(taffy, child, ids, x, y, frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::ROOT_ID;
    use crate::ui::{Button, Column, EdgeInsets, Element, Row, Spacer, Text};

    /// Measures every text as the same fixed box.
    struct FixedMeasure;

    impl Measure for FixedMeasure {
        fn measure(&self, _text: &TextProps, _constraints: Constraints) -> Size {
            Size::new(100.0, 20.0)
        }
    }

    fn layout(element: Element, viewport: Size) -> HashMap<NodeId, Rect> {
        let tree = tree::build(&element);
        let frames = compute(&tree, viewport, &FixedMeasure);

        // sanity check: the collection is what the tests assert on
        assert_eq!(frames.len(), tree::preorder(&tree).len());
        assert!(frames.contains_key(&ROOT_ID));
        frames
    }

    #[test]
    fn column_padding_and_gap_are_applied() {
        let element: Element = Column::new()
            .padding(10)
            .spacing(5)
            .width(200)
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);
        let column = &frames[&tree.children[0].id];

        // the column is 200 wide and as tall as its content
        assert_eq!(column.width, 200.0);
        assert_eq!(column.height, 65.0);
    }

    #[test]
    fn column_children_are_stacked_with_padding_and_gap() {
        let element: Element = Column::new()
            .padding(10)
            .spacing(5)
            .width(200)
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);

        let first = &frames[&tree.children[0].children[0].id];
        let second = &frames[&tree.children[0].children[1].id];

        assert_eq!(*first, Rect::new(10.0, 10.0, 180.0, 20.0));
        assert_eq!(*second, Rect::new(10.0, 35.0, 180.0, 20.0));
    }

    #[test]
    fn row_spacer_claims_the_remaining_space() {
        let element: Element = Row::new()
            .width(300)
            .child(Text::new("left"))
            .child(Spacer::new())
            .child(Text::new("right"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);

        let row = &tree.children[0];
        assert_eq!(frames[&row.id].width, 300.0);
        assert_eq!(frames[&row.children[0].id].x, 0.0);
        assert_eq!(frames[&row.children[1].id].x, 100.0);
        assert_eq!(frames[&row.children[1].id].width, 100.0);
        assert_eq!(frames[&row.children[2].id].x, 200.0);
    }

    #[test]
    fn percent_size_is_relative_to_the_parent() {
        let element: Element = Column::new()
            .width(200)
            .child(Text::new("half").width_percent(0.5))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);

        assert_eq!(frames[&tree.children[0].children[0].id].width, 100.0);
    }

    #[test]
    fn button_gets_an_intrinsic_size_around_its_label() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(Button::new("Ok"))
            .into();
        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);

        let button = &tree.children[0].children[0];
        // label 100x20 plus 16 horizontal and 12 vertical padding
        assert_eq!(frames[&button.id].width, 132.0);
        assert_eq!(frames[&button.id].height, 44.0);
    }

    #[test]
    fn leaf_padding_is_added_to_its_intrinsic_size() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(Text::new("chip").padding_each(EdgeInsets::symmetric(10, 4)))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);
        let chip = &frames[&tree.children[0].children[0].id];

        // label 100x20 plus 10dp on the horizontal and 4dp on the vertical axis
        assert_eq!(*chip, Rect::new(0.0, 0.0, 120.0, 28.0));
    }

    #[test]
    fn stretched_text_is_measured_inside_its_padding() {
        /// Records the width the text was measured at.
        struct WidthMeasure(std::sync::Mutex<Option<f32>>);

        impl Measure for WidthMeasure {
            fn measure(&self, _text: &TextProps, constraints: Constraints) -> Size {
                *self.0.lock().unwrap() = constraints.max_width;
                Size::new(constraints.max_width.unwrap_or(0.0), 40.0)
            }
        }

        let element: Element = Column::new()
            .padding(10)
            .width(200)
            .child(Text::new("long text").padding_each(EdgeInsets::symmetric(10, 4)))
            .into();

        let tree = tree::build(&element);
        let measurer = WidthMeasure(std::sync::Mutex::new(None));
        let frames = compute(&tree, Size::new(300.0, 600.0), &measurer);
        let text = &frames[&tree.children[0].children[0].id];

        // 200dp column minus 10dp padding on each side, then the text's own 10dp
        assert_eq!(*measurer.0.lock().unwrap(), Some(160.0));
        assert_eq!(*text, Rect::new(10.0, 10.0, 180.0, 48.0));
    }

    #[test]
    fn the_root_fills_the_viewport() {
        let frames = layout(Column::new().into(), Size::new(320.0, 480.0));
        assert_eq!(frames[&ROOT_ID], Rect::new(0.0, 0.0, 320.0, 480.0));
    }

    /// Reports the height of two lines whenever the width is constrained.
    struct WrappingMeasure;

    impl Measure for WrappingMeasure {
        fn measure(&self, _text: &TextProps, constraints: Constraints) -> Size {
            match constraints.max_width {
                Some(width) if width < 300.0 => Size::new(width, 40.0),
                _ => Size::new(300.0, 20.0),
            }
        }
    }

    #[test]
    fn text_wraps_to_the_parent_content_width() {
        let element: Element = Column::new()
            .padding(10)
            .width(200)
            .child(Text::new("long text"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &WrappingMeasure);
        let text = &frames[&tree.children[0].children[0].id];

        // the column's 200dp minus its 10dp padding on each side
        assert_eq!(text.width, 180.0);
        assert_eq!(text.height, 40.0);
    }

    #[test]
    fn overflowing_column_reports_its_content_height() {
        let element: Element = Column::new()
            .width(100)
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(100.0, 10.0), &FixedMeasure);

        assert_eq!(frames[&tree.children[0].id].height, 40.0);
    }
}
