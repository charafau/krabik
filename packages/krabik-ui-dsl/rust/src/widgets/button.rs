//! The `button` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::FontWeight;
use crate::widgets::TextProps;
use crate::widgets::Widget;

/// Platform-independent properties of a button node.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    /// The label shown on the button.
    pub label: String,
    /// Reported when the button is pressed.
    pub on_click: Handler<()>,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
}

impl ButtonProps {
    /// Creates a button with the given label and no action.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: Handler::none(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native button.
pub struct Button<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Button<M> {
    /// Creates a button.
    pub fn new(label: impl Into<String>) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Button(ButtonProps::new(label)))),
            PhantomData,
        )
    }

    /// Sends `message` when the button is pressed.
    pub fn on_click(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Widget(Widget::Button(props)) = &mut self.0.kind {
            props.on_click = Handler::new(move |()| message.clone());
        }
        self
    }
}

impl<M> From<Button<M>> for Element<M> {
    fn from(component: Button<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Button);
text_builder!(Button);

/// The horizontal padding a button adds around its label.
const BUTTON_PADDING_X: f32 = 16.0;
/// The vertical padding a button adds around its label.
const BUTTON_PADDING_Y: f32 = 12.0;
/// The smallest height a button label is given.
const BUTTON_MIN_HEIGHT: f32 = 20.0;

impl ButtonProps {
    /// The label plus the button's own padding.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let inner_width = max_width.map(|width| (width - BUTTON_PADDING_X * 2.0).max(0.0));
        let label = TextProps {
            content: self.label.clone(),
            size: self.size,
            color: self.color,
            weight: self.weight,
            ..TextProps::new("")
        };
        let text = label.intrinsic_size(inner_width, measurer);
        Size::new(
            text.width + BUTTON_PADDING_X * 2.0,
            text.height.max(BUTTON_MIN_HEIGHT) + BUTTON_PADDING_Y * 2.0,
        )
    }
}
