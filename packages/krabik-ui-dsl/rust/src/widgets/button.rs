//! The `button` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::style::{Font, FontWeight};
use crate::widgets::IntrinsicSize;
use crate::widgets::TextProps;

/// Platform-independent properties of a button node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(name = Button, text, about = "A native button.", new = "Creates a button with the given label.")]
pub struct ButtonProps {
    /// The label shown on the button.
    #[widget(arg)]
    pub label: String,
    /// Sends `message` when the button is pressed.
    pub on_click: Handler<()>,
    /// Font size in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// Text color.
    #[widget(default = Color::TEXT)]
    pub color: Color,
    /// Font weight.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
    /// The font of the label; the system's unless a file is named.
    #[widget(set)]
    pub font: Font,
}

/// The horizontal padding a button adds around its label.
const BUTTON_PADDING_X: f32 = 16.0;
/// The vertical padding a button adds around its label.
const BUTTON_PADDING_Y: f32 = 12.0;
/// The smallest height a button label is given.
const BUTTON_MIN_HEIGHT: f32 = 20.0;

impl IntrinsicSize for ButtonProps {
    /// The label plus the button's own padding.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let inner_width = max_width.map(|width| (width - BUTTON_PADDING_X * 2.0).max(0.0));
        let label = TextProps {
            content: self.label.clone(),
            size: self.size,
            color: self.color,
            weight: self.weight,
            font: self.font.clone(),
            ..TextProps::new("")
        };
        let text = label.intrinsic_size(inner_width, measurer);
        Size::new(
            text.width + BUTTON_PADDING_X * 2.0,
            text.height.max(BUTTON_MIN_HEIGHT) + BUTTON_PADDING_Y * 2.0,
        )
    }
}

impl<M> Button<M> {
    /// Creates a button that stands out: white text on the accent color,
    /// with rounded ends. For the one main action of a screen.
    ///
    /// It is a [`Button::new`] with a look, so every style method still
    /// applies and replaces what it sets.
    pub fn filled(label: impl Into<String>) -> Self {
        Self::new(label)
            .color(Color::fixed(0xFFFF_FFFF))
            .weight(FontWeight::Medium)
            .background(Color::ACCENT)
            .corner_radius(22)
    }

    /// Creates a button with an outline: accent text in a rounded border.
    /// For an action next to a filled one.
    pub fn outlined(label: impl Into<String>) -> Self {
        Self::new(label)
            .color(Color::ACCENT)
            .weight(FontWeight::Medium)
            .border(1, Color::OUTLINE)
            .corner_radius(22)
    }
}
