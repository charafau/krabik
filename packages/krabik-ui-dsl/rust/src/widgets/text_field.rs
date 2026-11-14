//! The `text_field` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::element::ElementKind;
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::style::FontWeight;
use crate::widgets::IntrinsicSize;
use crate::widgets::TextProps;
use crate::widgets::Widget;

/// Which on-screen keyboard an input requests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardType {
    /// Free-form text.
    #[default]
    Text,
    /// An email address.
    Email,
    /// A number.
    Number,
    /// A phone number.
    Phone,
}

/// The action key shown on the keyboard of an input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImeAction {
    /// Let the platform choose the action.
    #[default]
    Default,
    /// Show a done key.
    Done,
    /// Show a next key.
    Next,
    /// Show a search key.
    Search,
    /// Show a send key.
    Send,
}

/// Platform-independent properties of a text input node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(name = TextField, text, about = "A native text input.", new = "Creates an input showing the given text.")]
pub struct TextFieldProps {
    /// The current text of the input.
    #[widget(arg)]
    pub value: String,
    /// The hint shown while the input is empty.
    #[widget(set)]
    pub placeholder: String,
    /// Reports the new text whenever it changes, for example
    /// `.on_change(Msg::NameChanged)`.
    pub on_change: Handler<String>,
    /// Sends `message` when the user submits the input from the keyboard.
    pub on_submit: Handler<()>,
    /// Reports `true` when the input gains focus and `false` when it loses it.
    pub on_focus: Handler<bool>,
    /// Whether the input accepts multiple lines of text.
    #[widget(set)]
    pub multiline: bool,
    /// Whether the text is obscured while typing.
    #[widget(set)]
    pub secure: bool,
    /// The keyboard layout the input requests.
    #[widget(set)]
    pub keyboard: KeyboardType,
    /// The action key shown on the keyboard.
    #[widget(set)]
    pub ime_action: ImeAction,
    /// Font size in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// Text color.
    #[widget(default = Color::TEXT)]
    pub color: Color,
    /// Font weight.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
}

impl<M> TextField<M> {
    /// The current text of the input.
    pub fn value(&self) -> Option<&str> {
        match &self.0.kind {
            ElementKind::Widget(Widget::TextField(props)) => Some(&props.value),
            _ => None,
        }
    }
}

impl IntrinsicSize for TextFieldProps {
    /// The size of the typed text. An empty input is measured from its hint,
    /// so the field keeps a stable size before the user types.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let content = if self.value.is_empty() {
            &self.placeholder
        } else {
            &self.value
        };
        let mut text = TextProps {
            content: content.clone(),
            size: self.size,
            color: self.color,
            weight: self.weight,
            ..TextProps::new("")
        };
        if !self.multiline {
            // A single-line input never grows taller when the text is longer
            // than the available width.
            text.max_lines = Some(1);
            text.ellipsis = true;
        }
        text.intrinsic_size(max_width, measurer)
    }
}
