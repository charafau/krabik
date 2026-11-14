//! The `radio_group` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::style::{FontWeight, IntoPoints};
use crate::widgets::IntrinsicSize;
use crate::widgets::{CHECK_GAP, CHECK_HEIGHT, CHECK_INDICATOR, TextProps};

/// Platform-independent properties of a group of mutually exclusive radio buttons.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = RadioGroup,
    text,
    about = "A native group of mutually exclusive radio buttons.\n\nUnlike individual `RadioButton`s, the group owns the selection: tapping an option reports the chosen index through `on_select`.",
    new = "Creates a radio group with the given option labels."
)]
pub struct RadioGroupProps {
    /// The labels of the options, in display order.
    #[widget(arg)]
    pub options: Vec<String>,
    /// The index of the selected option; nothing is selected when `None`.
    #[widget(set(some))]
    pub selected: Option<usize>,
    /// Reports the index of the option the user picked.
    pub on_select: Handler<usize>,
    /// Font size of the labels in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// Color of the labels and indicators.
    #[widget(default = Color::TEXT)]
    pub color: Color,
    /// Font weight of the labels.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
    /// The space between two options in logical points.
    #[widget(default = 8.0, set(with = IntoPoints::into_points, from = impl IntoPoints))]
    pub spacing: f32,
}

impl IntrinsicSize for RadioGroupProps {
    /// The options stacked vertically, as wide as the widest one.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        if self.options.is_empty() {
            return Size::ZERO;
        }
        let inner_width = max_width.map(|width| (width - CHECK_INDICATOR - CHECK_GAP).max(0.0));
        let mut widest = 0.0f32;
        let mut height = 0.0f32;
        for option in &self.options {
            let label = TextProps {
                content: option.clone(),
                size: self.size,
                color: self.color,
                weight: self.weight,
                ..TextProps::new("")
            };
            let text = label.intrinsic_size(inner_width, measurer);
            widest = widest.max(text.width);
            height += text.height.max(CHECK_HEIGHT);
        }
        height += self.spacing * (self.options.len() - 1) as f32;
        Size::new(widest + CHECK_INDICATOR + CHECK_GAP, height)
    }
}
