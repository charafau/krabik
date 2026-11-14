//! The `text_field` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::message::Wrap;
use crate::style::FontWeight;
use crate::widgets::TextProps;
use crate::widgets::TextStyleMut;
use crate::widgets::Widget;
use crate::widgets::WidgetProps;

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
    /// Reported with the new text whenever it changes.
    pub on_change: Handler<String>,
    /// Reported when the user submits the input from the keyboard.
    pub on_submit: Handler<()>,
    /// Reported with `true` when the input gains focus and `false` when it loses it.
    pub on_focus: Handler<bool>,
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
            on_change: Handler::none(),
            on_submit: Handler::none(),
            on_focus: Handler::none(),
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
pub struct TextField<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> TextField<M> {
    /// Creates an input with the given initial value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::TextField(TextFieldProps::new(
                value,
            )))),
            PhantomData,
        )
    }

    /// Sets the hint shown while the input is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.placeholder = text.into();
        }
        self
    }

    /// Reports the new text whenever it changes, for example
    /// `.on_change(Msg::NameChanged)`.
    pub fn on_change(mut self, message: impl Fn(String) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.on_change = Handler::new(message);
        }
        self
    }

    /// Sends `message` when the user submits the input from the keyboard.
    pub fn on_submit(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.on_submit = Handler::new(move |()| message.clone());
        }
        self
    }

    /// Reports `true` when the input gains focus and `false` when it loses it.
    pub fn on_focus(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::TextField(props)) = &mut self.0.kind {
            props.on_focus = Handler::new(message);
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

impl<M> From<TextField<M>> for Element<M> {
    fn from(component: TextField<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(TextField);
text_builder!(TextField);

impl WidgetProps for TextFieldProps {
    fn map_messages(&mut self, wrap: &Wrap) {
        self.on_change.map(wrap);
        self.on_submit.map(wrap);
        self.on_focus.map(wrap);
    }

    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        Some(TextStyleMut {
            size: &mut self.size,
            color: &mut self.color,
            weight: &mut self.weight,
        })
    }

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
