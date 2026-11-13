//! The `check` widgets: properties and builder.

use crate::element::{Element, ElementKind, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::style::FontWeight;
use crate::widgets::TextProps;
use crate::widgets::Widget;
use std::borrow::Cow;

/// Platform-independent properties of a checkbox or radio button.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckProps {
    /// The label shown next to the control.
    pub label: String,
    /// Whether the control is selected.
    pub checked: bool,
    /// The action dispatched whenever the control is toggled.
    pub action: Cow<'static, str>,
    /// Font size of the label in logical points.
    pub size: f32,
    /// ARGB color of the label.
    pub color: u32,
    /// Font weight of the label.
    pub weight: FontWeight,
}

impl CheckProps {
    /// Creates a control with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            action: Cow::Borrowed(""),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native checkbox with a text label.
#[derive(Clone, Debug, PartialEq)]
pub struct Checkbox(pub(crate) Element);

impl Checkbox {
    /// Creates a checkbox with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = Element::leaf(ElementKind::Widget(Widget::Checkbox(CheckProps::new(
            label.clone(),
            checked,
        ))));
        element.style.accessibility_label = Some(label);
        Self(element)
    }

    /// Sets whether the checkbox is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::Checkbox(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the checkbox is toggled.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::Checkbox(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<Checkbox> for Element {
    fn from(component: Checkbox) -> Self {
        component.0
    }
}

/// A native radio button with a text label.
///
/// Radio buttons are independent controls; mutual exclusion is application
/// state, usually a selected index rendered as one checked button.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioButton(pub(crate) Element);

impl RadioButton {
    /// Creates a radio button with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = Element::leaf(ElementKind::Widget(Widget::RadioButton(CheckProps::new(
            label.clone(),
            checked,
        ))));
        element.style.accessibility_label = Some(label);
        Self(element)
    }

    /// Sets whether the radio button is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::RadioButton(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the radio button is selected.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::RadioButton(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<RadioButton> for Element {
    fn from(component: RadioButton) -> Self {
        component.0
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

impl CheckProps {
    /// The indicator, the gap and the label, at least one touch target tall.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
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
