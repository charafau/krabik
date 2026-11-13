//! The `text_field` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::style::FontWeight;
use crate::widgets::TextProps;
use crate::widgets::Widget;
use std::borrow::Cow;

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
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldProps {
    /// The current text of the input.
    pub value: String,
    /// Hint shown while the input is empty.
    pub placeholder: String,
    /// The action dispatched whenever the text changes.
    pub action: Cow<'static, str>,
    /// The action dispatched when the user submits the input.
    pub submit_action: Cow<'static, str>,
    /// The action dispatched when the input gains or loses focus.
    pub focus_action: Cow<'static, str>,
    /// Whether the input accepts multiple lines of text.
    pub multiline: bool,
    /// Whether the text is obscured while typing.
    pub secure: bool,
    /// The keyboard the input requests.
    pub keyboard: KeyboardType,
    /// The action key shown on the keyboard.
    pub ime_action: ImeAction,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
}

impl TextFieldProps {
    /// Creates an empty input with the default size and color.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            action: Cow::Borrowed(""),
            submit_action: Cow::Borrowed(""),
            focus_action: Cow::Borrowed(""),
            multiline: false,
            secure: false,
            keyboard: KeyboardType::Text,
            ime_action: ImeAction::Default,
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native text input.
#[derive(Clone, Debug, PartialEq)]
pub struct TextField(pub(crate) Element);

impl TextField {
    /// Creates an input with the given initial value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(Element::leaf(ElementKind::Widget(Widget::TextField(
            TextFieldProps::new(value),
        ))))
    }

    /// Sets the hint shown while the input is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.placeholder = text.into();
        }
        self
    }

    /// Sets the action dispatched whenever the text changes.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime, such as `format!("field:{index}")`.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Sets the action dispatched when the user submits the input.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn submit_action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.submit_action = action.into();
        }
        self
    }

    /// Sets the action dispatched when the input gains or loses focus.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn focus_action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.focus_action = action.into();
        }
        self
    }

    /// Lets the input accept multiple lines of text.
    pub fn multiline(mut self, multiline: bool) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.multiline = multiline;
        }
        self
    }

    /// Obscures the text while typing.
    pub fn secure(mut self, secure: bool) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.secure = secure;
        }
        self
    }

    /// Requests a specific keyboard layout.
    pub fn keyboard(mut self, keyboard: KeyboardType) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.keyboard = keyboard;
        }
        self
    }

    /// Sets the action key shown on the keyboard.
    pub fn ime_action(mut self, action: ImeAction) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.ime_action = action;
        }
        self
    }

    /// The current text of the input.
    pub fn value(&self) -> Option<&str> {
        match &self.0.kind {
            ElementKind::Widget(Widget::TextField(props)) => Some(&props.value),
            _ => None,
        }
    }
}

impl From<TextField> for Element {
    fn from(component: TextField) -> Self {
        component.0
    }
}

style_builder!(TextField);
text_builder!(TextField);

impl TextFieldProps {
    /// The size of the typed text. An empty input is measured from its hint,
    /// so the field keeps a stable size before the user types.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
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
