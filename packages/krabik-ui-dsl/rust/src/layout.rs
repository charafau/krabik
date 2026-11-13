//! Taffy integration.
//!
//! The internal tree is translated into a Taffy tree, laid out against the
//! viewport, and the resulting frames are read back as absolute rectangles in
//! logical points.

use std::collections::HashMap;

use taffy::Overflow;
use taffy::geometry::{Point as TaffyPoint, Rect as TaffyRect, Size as TaffySize};
use taffy::prelude::{
    AlignItems, AvailableSpace, Dimension, Display, FlexDirection, JustifyContent,
    LengthPercentage, LengthPercentageAuto, NodeId as TaffyId, Style as TaffyStyle, TaffyTree,
};
use taffy::tree::{Layout, LayoutInput, LayoutOutput};

use crate::geometry::{Constraints, Rect, Size};
use crate::measure::Measure;
use crate::tree::{self, NodeId, NodeKind, UiNode};
use crate::ui::{Align, Axis, Justify, Length, TextProps};

/// The result of one layout pass.
#[derive(Clone, Debug, Default)]
pub struct LayoutResult {
    /// The absolute frame of every node in the tree.
    pub frames: HashMap<NodeId, Rect>,
    /// The scrollable content size of scroll containers, keyed by node.
    pub content_sizes: HashMap<NodeId, Size>,
}

/// The horizontal padding a button adds around its label.
const BUTTON_PADDING_X: f32 = 16.0;
/// The vertical padding a button adds around its label.
const BUTTON_PADDING_Y: f32 = 12.0;
/// The smallest height a button is allowed to have.
const BUTTON_MIN_HEIGHT: f32 = 20.0;
/// The default intrinsic width of a switch in logical points.
const SWITCH_WIDTH: f32 = 52.0;
/// The default intrinsic height of a switch in logical points.
const SWITCH_HEIGHT: f32 = 32.0;
/// The side of a checkbox or radio indicator in logical points.
const CHECK_INDICATOR: f32 = 20.0;
/// The space reserved between a check indicator and its label.
///
/// This covers the native button drawable and the control's internal text
/// inset, so the measured label always fits next to the indicator.
const CHECK_GAP: f32 = 28.0;
/// The minimum touch height of a checkbox or radio button.
const CHECK_HEIGHT: f32 = 48.0;
/// The height of a native tab bar.
const TAB_BAR_HEIGHT: f32 = 48.0;
/// The intrinsic height of a slider.
const SLIDER_HEIGHT: f32 = 40.0;
/// The intrinsic height of a progress indicator.
const PROGRESS_HEIGHT: f32 = 6.0;
/// The default side of an indeterminate spinner in logical points.
const SPINNER_SIZE: f32 = 24.0;
/// The width assumed for a compact date field without a constraint.
const DATE_PICKER_WIDTH: f32 = 200.0;
/// The width assumed for a compact time field.
const TIME_PICKER_WIDTH: f32 = 120.0;
/// The width assumed for a compact date-and-time field.
const DATETIME_PICKER_WIDTH: f32 = 240.0;
/// The height of a compact picker field.
const DATE_PICKER_HEIGHT: f32 = 48.0;
/// The width assumed for a slider or progress indicator without a constraint.
const CONTROL_WIDTH: f32 = 200.0;

/// Calculates the absolute frame of every node in the tree.
pub fn compute(root: &UiNode, viewport: Size, measurer: &dyn Measure) -> LayoutResult {
    let mut taffy = TaffyTree::new();
    // Layout stays fractional in logical points; the platform renderer rounds
    // when it converts to pixels.
    taffy.disable_rounding();

    let mut taffy_ids = HashMap::new();
    let taffy_root = build_taffy(&mut taffy, root, &mut taffy_ids, false, false);

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
    let mut content_sizes = HashMap::new();
    collect_frames(
        &taffy,
        root,
        &taffy_ids,
        0.0,
        0.0,
        &mut frames,
        &mut content_sizes,
    );

    LayoutResult {
        frames,
        content_sizes,
    }
}

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

fn taffy_style(node: &UiNode, scroll_child: bool, pager_child: bool) -> TaffyStyle {
    let direction = match node.kind {
        NodeKind::Row => FlexDirection::Row,
        NodeKind::Pager(_) => FlexDirection::Row,
        NodeKind::Scroll(props) if props.axis == Axis::Horizontal => FlexDirection::Row,
        _ => FlexDirection::Column,
    };

    let overflow = match node.kind {
        NodeKind::Pager(_) => TaffyPoint {
            x: Overflow::Hidden,
            y: Overflow::Hidden,
        },
        NodeKind::Scroll(props) => TaffyPoint {
            x: if props.axis == Axis::Horizontal {
                Overflow::Scroll
            } else {
                Overflow::Hidden
            },
            y: if props.axis == Axis::Vertical {
                Overflow::Scroll
            } else {
                Overflow::Hidden
            },
        },
        _ => TaffyPoint::default(),
    };

    TaffyStyle {
        display: Display::Flex,
        flex_direction: direction,
        size: if pager_child {
            // Every pager page fills the pager viewport; the row overflows
            // horizontally so the native pager can scroll between pages.
            TaffySize {
                width: Dimension::percent(1.0),
                height: Dimension::percent(1.0),
            }
        } else {
            TaffySize {
                width: dimension(node.style.width),
                height: dimension(node.style.height),
            }
        },
        // Taffy's automatic minimum size uses the content's min-content size.
        // For text that is the full unwrapped line, which would stop a label
        // from shrinking next to a sibling (for example in a list row). A text
        // node may shrink to the space its parent leaves and wrap instead.
        min_size: TaffySize {
            width: if pager_child || matches!(node.kind, NodeKind::Text(_)) {
                LengthPercentageAuto::length(0.0)
            } else {
                LengthPercentageAuto::auto()
            },
            height: LengthPercentageAuto::auto(),
        },
        aspect_ratio: node.style.aspect_ratio,
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
        // Scroll containers must let their content keep its natural size in
        // the scroll axis instead of shrinking it into the viewport.
        flex_shrink: if scroll_child || pager_child {
            0.0
        } else {
            1.0
        },
        align_items: align_items(node.style.align_items),
        justify_content: justify_content(node.style.justify_content),
        overflow,
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
                ..TextProps::new("")
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
        NodeKind::TextField(props) => {
            // An empty input is measured from its hint, so the field keeps a
            // stable height and width before the user types.
            let content = if props.value.is_empty() {
                &props.placeholder
            } else {
                &props.value
            };
            let mut text = TextProps {
                content: content.clone(),
                size: props.size,
                color: props.color,
                weight: props.weight,
                ..TextProps::new("")
            };
            if !props.multiline {
                // A single-line input never grows taller when the text is
                // longer than the available width.
                text.max_lines = Some(1);
                text.ellipsis = true;
            }
            measurer.measure(
                &text,
                Constraints {
                    max_width,
                    max_height: None,
                },
            )
        }
        // Switches have a platform-defined intrinsic size. The default matches
        // the Material track and is overridable with an explicit dimension.
        NodeKind::Switch(_) => Size::new(
            max_width.map_or(SWITCH_WIDTH, |width| width.min(SWITCH_WIDTH)),
            SWITCH_HEIGHT,
        ),
        NodeKind::Checkbox(props) | NodeKind::RadioButton(props) => {
            let inner_width = max_width.map(|width| (width - CHECK_INDICATOR - CHECK_GAP).max(0.0));
            let label = TextProps {
                content: props.label.clone(),
                size: props.size,
                color: props.color,
                weight: props.weight,
                ..TextProps::new("")
            };
            let text = measurer.measure(
                &label,
                Constraints {
                    max_width: inner_width,
                    max_height: None,
                },
            );
            Size::new(
                text.width + CHECK_INDICATOR + CHECK_GAP,
                text.height.max(CHECK_HEIGHT),
            )
        }
        NodeKind::RadioGroup(props) => {
            if props.options.is_empty() {
                return Size::ZERO;
            }
            let inner_width = max_width.map(|width| (width - CHECK_INDICATOR - CHECK_GAP).max(0.0));
            let mut widest = 0.0f32;
            let mut height = 0.0f32;
            for option in &props.options {
                let label = TextProps {
                    content: option.clone(),
                    size: props.size,
                    color: props.color,
                    weight: props.weight,
                    ..TextProps::new("")
                };
                let text = measurer.measure(
                    &label,
                    Constraints {
                        max_width: inner_width,
                        max_height: None,
                    },
                );
                widest = widest.max(text.width);
                height += text.height.max(CHECK_HEIGHT);
            }
            height += props.spacing * (props.options.len() - 1) as f32;
            Size::new(widest + CHECK_INDICATOR + CHECK_GAP, height)
        }
        // A tab bar fills the available width and has a fixed touch height.
        NodeKind::TabBar(_) => Size::new(max_width.unwrap_or(CONTROL_WIDTH), TAB_BAR_HEIGHT),
        // A bottom navigation bar fills the width below its content.
        NodeKind::BottomNavigation(_) => Size::new(max_width.unwrap_or(CONTROL_WIDTH), 56.0),
        NodeKind::DatePicker(props) => {
            let (compact_width, inline_width, inline_height) = match props.value {
                crate::ui::DatePickerValue::Date(_) => (DATE_PICKER_WIDTH, 320.0, 320.0),
                crate::ui::DatePickerValue::Time(_) => (TIME_PICKER_WIDTH, 240.0, 240.0),
                crate::ui::DatePickerValue::DateTime { .. } => {
                    (DATETIME_PICKER_WIDTH, 360.0, 480.0)
                }
            };
            match props.mode {
                // A field keeps its compact size unless the style sets one.
                crate::ui::DatePickerMode::Compact => Size::new(compact_width, DATE_PICKER_HEIGHT),
                // An inline picker fits the available width up to its natural size.
                crate::ui::DatePickerMode::Inline => Size::new(
                    max_width.unwrap_or(inline_width).min(inline_width),
                    inline_height,
                ),
            }
        }
        // A slider fills the width it is given and has a fixed touch height.
        NodeKind::Slider(_) => Size::new(max_width.unwrap_or(CONTROL_WIDTH), SLIDER_HEIGHT),
        NodeKind::Progress(_) => Size::new(max_width.unwrap_or(CONTROL_WIDTH), PROGRESS_HEIGHT),
        NodeKind::Spinner(_) => Size::new(SPINNER_SIZE, SPINNER_SIZE),
        // Images are sized explicitly by the application in this milestone.
        NodeKind::Image(_) => Size::ZERO,
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
    content_sizes: &mut HashMap<NodeId, Size>,
) {
    let layout = taffy.layout(ids[&node.id]).expect("taffy layout");

    let x = parent_x + layout.location.x;
    let y = parent_y + layout.location.y;
    frames.insert(
        node.id,
        Rect::new(x, y, layout.size.width, layout.size.height),
    );

    if let Some(content_size) = scroll_content_size(node, layout) {
        content_sizes.insert(node.id, content_size);
    }

    for child in &node.children {
        collect_frames(taffy, child, ids, x, y, frames, content_sizes);
    }
}

/// Returns the size of the scrollable content of a scroll container.
///
/// Taffy measures the overflow from the node's scroll origin, which is the
/// padding-box corner. The native content host lives at the content-box corner,
/// so the padding is subtracted before the two sizes are compared.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::measure::ApproximateMeasure;
    use crate::tree::ROOT_ID;
    use crate::ui::{
        Button, Checkbox, Column, DatePicker, DatePickerMode, DateValue, EdgeInsets, Element,
        Image, ImageSource, Pager, ProgressIndicator, RadioButton, RadioGroup, Row, ScrollView,
        Slider, Spacer, Switch, TabBar, Text, TextField, TimeValue,
    };

    /// Measures every text as the same fixed box.
    struct FixedMeasure;

    impl Measure for FixedMeasure {
        fn measure(&self, _text: &TextProps, _constraints: Constraints) -> Size {
            Size::new(100.0, 20.0)
        }
    }

    fn layout(element: Element, viewport: Size) -> HashMap<NodeId, Rect> {
        let tree = tree::build(&element);
        let frames = compute(&tree, viewport, &FixedMeasure).frames;

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
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;
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
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

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
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

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
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

        assert_eq!(frames[&tree.children[0].children[0].id].width, 100.0);
    }

    #[test]
    fn button_gets_an_intrinsic_size_around_its_label() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(Button::new("Ok"))
            .into();
        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

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
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;
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
        let frames = compute(&tree, Size::new(300.0, 600.0), &measurer).frames;
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
        let frames = compute(&tree, Size::new(300.0, 600.0), &WrappingMeasure).frames;
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
        let frames = compute(&tree, Size::new(100.0, 10.0), &FixedMeasure).frames;

        assert_eq!(frames[&tree.children[0].id].height, 40.0);
    }

    #[test]
    fn vertical_scroll_reports_its_content_height() {
        let mut scroll = ScrollView::new().width(200).height(100);
        for label in ["1", "2", "3", "4", "5", "6"] {
            scroll = scroll.child(Text::new(label));
        }
        let element: Element = scroll.into();

        let tree = tree::build(&element);
        let result = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);
        let scroll = &tree.children[0];

        assert_eq!(result.frames[&scroll.id], Rect::new(0.0, 0.0, 200.0, 100.0));
        // six 20pt rows overflow the 100pt viewport
        assert_eq!(result.content_sizes[&scroll.id], Size::new(200.0, 120.0));
        assert_eq!(result.frames[&scroll.children[5].id].y, 100.0);
    }

    #[test]
    fn horizontal_scroll_reports_its_content_width() {
        let mut scroll = ScrollView::horizontal().width(100).height(50);
        for label in ["1", "2", "3", "4", "5"] {
            scroll = scroll.child(Text::new(label));
        }
        let element: Element = scroll.into();

        let tree = tree::build(&element);
        let result = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);
        let scroll = &tree.children[0];

        assert_eq!(result.frames[&scroll.id], Rect::new(0.0, 0.0, 100.0, 50.0));
        // five 100pt columns overflow the 100pt viewport
        assert_eq!(result.content_sizes[&scroll.id], Size::new(500.0, 50.0));
        assert_eq!(result.frames[&scroll.children[4].id].x, 400.0);
    }

    #[test]
    fn scroll_content_accounts_for_padding() {
        let mut scroll = ScrollView::new()
            .width(200)
            .height(100)
            .padding_each(EdgeInsets::all(10));
        for label in ["1", "2", "3", "4", "5", "6"] {
            scroll = scroll.child(Text::new(label));
        }
        let element: Element = scroll.into();

        let tree = tree::build(&element);
        let result = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure);
        let scroll = &tree.children[0];

        // content starts at the padding origin, so six rows need 120pt
        assert_eq!(result.content_sizes[&scroll.id], Size::new(180.0, 120.0));
    }

    /// Measures text as ten points per character.
    struct LengthMeasure;

    impl Measure for LengthMeasure {
        fn measure(&self, text: &TextProps, _constraints: Constraints) -> Size {
            Size::new(text.content.chars().count() as f32 * 10.0, 20.0)
        }
    }

    #[test]
    fn an_empty_text_field_is_measured_from_its_hint() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(TextField::new("").placeholder("hint"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &LengthMeasure).frames;

        assert_eq!(frames[&tree.children[0].children[0].id].width, 40.0);
    }

    #[test]
    fn a_filled_text_field_is_measured_from_its_value() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(TextField::new("abc").placeholder("hint"))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &LengthMeasure).frames;

        assert_eq!(frames[&tree.children[0].children[0].id].width, 30.0);
    }

    #[test]
    fn images_use_their_explicit_size() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(
                Image::new(ImageSource::asset("krabik.webp"))
                    .width(64)
                    .height(48),
            )
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

        assert_eq!(
            frames[&tree.children[0].children[0].id],
            Rect::new(0.0, 0.0, 64.0, 48.0)
        );
    }

    #[test]
    fn aspect_ratio_derives_the_missing_dimension() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(
                Image::new(ImageSource::asset("krabik.webp"))
                    .width(200)
                    .aspect_ratio(2.0),
            )
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

        assert_eq!(
            frames[&tree.children[0].children[0].id],
            Rect::new(0.0, 0.0, 200.0, 100.0)
        );
    }

    #[test]
    fn a_single_line_text_field_never_wraps() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(TextField::new("aaaaaaaaaa").width(44))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure).frames;

        // Ten 8.8pt glyphs need two lines at 44pt, but a single-line field
        // stays one line high.
        assert_eq!(frames[&tree.children[0].children[0].id].height, 20.0);
    }

    #[test]
    fn a_multiline_text_field_wraps() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(TextField::new("aaaaaaaaaa").multiline(true).width(44))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure).frames;

        assert_eq!(frames[&tree.children[0].children[0].id].height, 40.0);
    }

    #[test]
    fn a_switch_has_an_intrinsic_size() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(Switch::new(false))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

        assert_eq!(
            frames[&tree.children[0].children[0].id],
            Rect::new(0.0, 0.0, 52.0, 32.0)
        );
    }

    #[test]
    fn an_explicit_size_overrides_the_switch_intrinsic_size() {
        let element: Element = Column::new()
            .align_items(Align::Start)
            .child(Switch::new(false).width(80).height(40))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &FixedMeasure).frames;

        assert_eq!(
            frames[&tree.children[0].children[0].id],
            Rect::new(0.0, 0.0, 80.0, 40.0)
        );
    }

    #[test]
    fn new_controls_have_intrinsic_sizes() {
        let element: Element = Column::new()
            .width(300)
            .align_items(Align::Start)
            .child(Checkbox::new("Wi-Fi", false))
            .child(RadioButton::new("Large", true))
            .child(Slider::new(0.5).width(300))
            .child(ProgressIndicator::new(0.5).width(300))
            .child(ProgressIndicator::indeterminate())
            .child(RadioGroup::new(["Small", "Medium", "Large"]).spacing(8))
            .child(DatePicker::new(DateValue::new(2026, 11, 12)))
            .child(DatePicker::new(DateValue::new(2026, 11, 12)).mode(DatePickerMode::Inline))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure).frames;
        let column = &tree.children[0];

        let checkbox = frames[&column.children[0].id];
        let radio = frames[&column.children[1].id];
        let slider = frames[&column.children[2].id];
        let progress = frames[&column.children[3].id];
        let spinner = frames[&column.children[4].id];
        let group = frames[&column.children[5].id];
        let compact_date = frames[&column.children[6].id];
        let inline_date = frames[&column.children[7].id];

        assert!(checkbox.width > 28.0 && checkbox.height >= 48.0);
        assert!(radio.width > 28.0 && radio.height >= 48.0);
        assert_eq!(slider.width, 300.0);
        assert_eq!(slider.height, 40.0);
        assert_eq!(progress.width, 300.0);
        assert_eq!(progress.height, 6.0);
        assert_eq!(spinner.width, 24.0);
        assert_eq!(spinner.height, 24.0);
        assert!(group.width > 28.0);
        // Three 48-point rows plus two 8-point gaps.
        assert_eq!(group.height, 160.0);
        assert_eq!(compact_date.width, 200.0);
        assert_eq!(compact_date.height, 48.0);
        assert_eq!(inline_date.width, 300.0);
        assert_eq!(inline_date.height, 320.0);
    }

    #[test]
    fn pager_pages_fill_the_viewport_side_by_side() {
        let element: Element = Pager::new()
            .height(400)
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure).frames;
        let pager = &tree.children[0];

        assert_eq!(frames[&pager.id], Rect::new(0.0, 0.0, 300.0, 400.0));
        assert_eq!(
            frames[&pager.children[0].id],
            Rect::new(0.0, 0.0, 300.0, 400.0)
        );
        assert_eq!(
            frames[&pager.children[1].id],
            Rect::new(300.0, 0.0, 300.0, 400.0)
        );
    }

    #[test]
    fn pickers_and_tab_bars_have_intrinsic_sizes() {
        let element: Element = Column::new()
            .width(300)
            .align_items(Align::Start)
            .child(DatePicker::time(TimeValue::new(10, 30)))
            .child(DatePicker::date_time(
                DateValue::new(2026, 11, 12),
                TimeValue::new(10, 30),
            ))
            .child(TabBar::new(["One", "Two", "Three"]))
            .into();

        let tree = tree::build(&element);
        let frames = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure).frames;
        let column = &tree.children[0];

        assert_eq!(frames[&column.children[0].id].width, 120.0);
        assert_eq!(frames[&column.children[0].id].height, 48.0);
        assert_eq!(frames[&column.children[1].id].width, 240.0);
        assert_eq!(frames[&column.children[1].id].height, 48.0);
        assert_eq!(frames[&column.children[2].id].width, 300.0);
        assert_eq!(frames[&column.children[2].id].height, 48.0);
    }

    #[test]
    fn a_long_label_shrinks_beside_a_button() {
        let element: Element = Row::new()
            .width(300)
            .child(Text::new("wrapping text ".repeat(20)).grow(1.0))
            .child(Button::new("Tap"))
            .into();

        let tree = tree::build(&element);
        let layout = compute(&tree, Size::new(300.0, 600.0), &ApproximateMeasure);
        let row = &tree.children[0];
        let text = layout.frames[&row.children[0].id];
        let button = layout.frames[&row.children[1].id];

        assert!(text.width < 300.0, "text kept its natural width: {text:?}");
        assert!(text.height > 20.0, "text did not wrap: {text:?}");
        assert!(
            button.x + button.width <= 300.5,
            "button was pushed outside the row: {button:?}"
        );
    }
}
