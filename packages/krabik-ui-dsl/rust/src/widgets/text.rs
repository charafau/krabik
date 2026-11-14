//! The `text` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::{Constraints, Size};
use crate::engine::measure::Measure;
use crate::style::{FontWeight, TextAlign};
use crate::widgets::IntrinsicSize;

/// Platform-independent properties of a text node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(name = Text, text, about = "A run of text.", new = "Creates text with the default size and color.")]
pub struct TextProps {
    /// The text to display.
    #[widget(arg)]
    pub content: String,
    /// Font size in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// ARGB text color.
    #[widget(default = 0xFF00_0000)]
    pub color: u32,
    /// Font weight.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
    /// Aligns the text horizontally inside its frame.
    #[widget(default = TextAlign::Start, set(name = text_align))]
    pub align: TextAlign,
    /// Limits the number of lines the text may occupy; unlimited when `None`.
    #[widget(set(with = at_least_one_line, from = u32))]
    pub max_lines: Option<u32>,
    /// Truncates the last visible line with an ellipsis when the text does not fit.
    #[widget(set(flag = true))]
    pub ellipsis: bool,
}

fn at_least_one_line(lines: u32) -> Option<u32> {
    Some(lines.max(1))
}

impl IntrinsicSize for TextProps {
    /// Text is as large as the platform measures it at the available width.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        measurer.measure(
            self,
            Constraints {
                max_width,
                max_height: None,
            },
        )
    }
}
