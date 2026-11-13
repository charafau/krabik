//! The `text` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder, text_builder};
use crate::engine::geometry::{Constraints, Size};
use crate::engine::measure::Measure;
use crate::style::{FontWeight, TextAlign};
use crate::widgets::Widget;

/// Platform-independent properties of a text node.
#[derive(Clone, Debug, PartialEq)]
pub struct TextProps {
    /// The text to display.
    pub content: String,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
    /// Horizontal alignment inside the node's frame.
    pub align: TextAlign,
    /// Maximum number of lines to draw, or unlimited when `None`.
    pub max_lines: Option<u32>,
    /// Whether the last visible line is truncated with an ellipsis.
    pub ellipsis: bool,
}

impl TextProps {
    /// Creates text with the default size and color.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
            align: TextAlign::Start,
            max_lines: None,
            ellipsis: false,
        }
    }
}

/// A run of text.
#[derive(Clone, Debug, PartialEq)]
pub struct Text(pub(crate) Element);

impl Text {
    /// Creates a text node.
    pub fn new(content: impl Into<String>) -> Self {
        Self(Element::leaf(ElementKind::Widget(Widget::Text(
            TextProps::new(content),
        ))))
    }

    /// Aligns the text horizontally inside its frame.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        if let ElementKind::Widget(Widget::Text(props)) = &mut self.0.kind {
            props.align = align;
        }
        self
    }

    /// Limits the number of lines the text may occupy.
    pub fn max_lines(mut self, lines: u32) -> Self {
        if let ElementKind::Widget(Widget::Text(props)) = &mut self.0.kind {
            props.max_lines = Some(lines.max(1));
        }
        self
    }

    /// Truncates the text with an ellipsis when it does not fit.
    pub fn ellipsis(mut self) -> Self {
        if let ElementKind::Widget(Widget::Text(props)) = &mut self.0.kind {
            props.ellipsis = true;
        }
        self
    }
}

impl From<Text> for Element {
    fn from(component: Text) -> Self {
        component.0
    }
}

style_builder!(Text);
text_builder!(Text);

impl TextProps {
    /// Text is as large as the platform measures it at the available width.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        measurer.measure(
            self,
            Constraints {
                max_width,
                max_height: None,
            },
        )
    }
}
