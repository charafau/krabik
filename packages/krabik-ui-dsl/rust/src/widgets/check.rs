//! The `check` widgets: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::message::MessageMap;
use crate::style::Color;
use crate::style::FontWeight;
use crate::widgets::IntrinsicSize;
use crate::widgets::TextProps;
use crate::widgets::TextStyleMut;
use crate::widgets::Widget;
use crate::widgets::WidgetProps;

/// Platform-independent properties of a checkbox or radio button.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckProps {
    /// The label shown next to the control.
    pub label: String,
    /// Whether the control is selected.
    pub checked: bool,
    /// Reported with the new state when the control is toggled.
    pub on_toggle: Handler<bool>,
    /// Font size of the label in logical points.
    pub size: f32,
    /// Color of the label.
    pub color: Color,
    /// Font weight of the label.
    pub weight: FontWeight,
}

impl CheckProps {
    /// Creates a control with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            on_toggle: Handler::none(),
            size: 16.0,
            color: Color::TEXT,
            weight: FontWeight::Normal,
        }
    }
}

/// A native checkbox with a text label.
pub struct Checkbox<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Checkbox<M> {
    /// Creates a checkbox with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = RawElement::leaf(ElementKind::Widget(Widget::Checkbox(CheckProps::new(
            label.clone(),
            checked,
        ))));
        element.style.accessibility_label = Some(label);
        Self(element, PhantomData)
    }

    /// Sets whether the checkbox is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::Checkbox(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Reports the new state when the control is toggled.
    pub fn on_toggle(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::Checkbox(props)) = &mut self.0.kind {
            props.on_toggle = Handler::new(message);
        }
        self
    }
}

impl<M> From<Checkbox<M>> for Element<M> {
    fn from(component: Checkbox<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// A native radio button with a text label.
///
/// Radio buttons are independent controls; mutual exclusion is application
/// state, usually a selected index rendered as one checked button.
pub struct RadioButton<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> RadioButton<M> {
    /// Creates a radio button with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = RawElement::leaf(ElementKind::Widget(Widget::RadioButton(
            CheckProps::new(label.clone(), checked),
        )));
        element.style.accessibility_label = Some(label);
        Self(element, PhantomData)
    }

    /// Sets whether the radio button is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::RadioButton(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Reports the new state when the control is toggled.
    pub fn on_toggle(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::RadioButton(props)) = &mut self.0.kind {
            props.on_toggle = Handler::new(message);
        }
        self
    }
}

impl<M> From<RadioButton<M>> for Element<M> {
    fn from(component: RadioButton<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Checkbox);
style_builder!(RadioButton);
text_builder!(Checkbox);
text_builder!(RadioButton);

/// The side of a checkbox or radio indicator in logical points.
pub(crate) const CHECK_INDICATOR: f32 = 20.0;
/// The space reserved between a check indicator and its label.
///
/// This covers the native button drawable and the control's internal text
/// inset, so the measured label always fits next to the indicator.
pub(crate) const CHECK_GAP: f32 = 28.0;
/// The minimum touch height of a checkbox or radio button.
pub(crate) const CHECK_HEIGHT: f32 = 48.0;

impl WidgetProps for CheckProps {
    fn map_messages(&mut self, wrap: &MessageMap) {
        self.on_toggle.map(wrap);
    }

    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        Some(TextStyleMut {
            size: &mut self.size,
            color: &mut self.color,
            weight: &mut self.weight,
        })
    }

    fn resolve_colors(&mut self, scheme: crate::style::ColorScheme) {
        self.color = self.color.resolve(scheme);
    }
}

impl IntrinsicSize for CheckProps {
    /// The indicator, the gap and the label, at least one touch target tall.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let inner_width = max_width.map(|width| (width - CHECK_INDICATOR - CHECK_GAP).max(0.0));
        let label = TextProps {
            content: self.label.clone(),
            size: self.size,
            color: self.color,
            weight: self.weight,
            ..TextProps::new("")
        };
        let text = label.intrinsic_size(inner_width, measurer);
        Size::new(
            text.width + CHECK_INDICATOR + CHECK_GAP,
            text.height.max(CHECK_HEIGHT),
        )
    }
}
