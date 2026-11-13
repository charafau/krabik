//! The `slider` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::engine::geometry::Size;
use crate::style::CONTROL_ACCENT;
use crate::widgets::CONTROL_WIDTH;
use crate::widgets::Widget;
use std::borrow::Cow;

/// Platform-independent properties of a slider.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    /// The current value.
    pub value: f32,
    /// The lowest selectable value.
    pub min: f32,
    /// The highest selectable value.
    pub max: f32,
    /// The interval values snap to, or `None` for a continuous slider.
    pub step: Option<f32>,
    /// The action dispatched whenever the value changes.
    pub action: Cow<'static, str>,
    /// ARGB color of the active track and thumb.
    pub color: u32,
}

impl SliderProps {
    /// Creates a slider with the given value and the default range `0..=1`.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            min: 0.0,
            max: 1.0,
            step: None,
            action: Cow::Borrowed(""),
            color: CONTROL_ACCENT,
        }
    }
}

/// Snaps a slider value to its range and step.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn snap_slider_value(value: f32, min: f32, max: f32, step: Option<f32>) -> f32 {
    let low = min.min(max);
    let high = max.max(min);
    let clamped = value.clamp(low, high);
    match step {
        Some(step) if step > 0.0 => {
            let snapped = low + ((clamped - low) / step).round() * step;
            snapped.clamp(low, high)
        }
        _ => clamped,
    }
}

/// A native slider.
#[derive(Clone, Debug, PartialEq)]
pub struct Slider(pub(crate) Element);

impl Slider {
    /// Creates a slider with the given value and the default range `0..=1`.
    pub fn new(value: f32) -> Self {
        Self(Element::leaf(ElementKind::Widget(Widget::Slider(
            SliderProps::new(value),
        ))))
    }

    /// Sets the lowest selectable value.
    pub fn min(mut self, min: f32) -> Self {
        if let ElementKind::Widget(Widget::Slider(props)) = &mut self.0.kind {
            props.min = min;
        }
        self
    }

    /// Sets the highest selectable value.
    pub fn max(mut self, max: f32) -> Self {
        if let ElementKind::Widget(Widget::Slider(props)) = &mut self.0.kind {
            props.max = max;
        }
        self
    }

    /// Quantizes the slider to the given interval.
    ///
    /// A non-positive step restores continuous values.
    pub fn step(mut self, step: f32) -> Self {
        if let ElementKind::Widget(Widget::Slider(props)) = &mut self.0.kind {
            props.step = (step > 0.0).then_some(step);
        }
        self
    }

    /// Sets the action dispatched whenever the value changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::Slider(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Sets the ARGB color of the active track and thumb.
    pub fn color(mut self, color: u32) -> Self {
        if let ElementKind::Widget(Widget::Slider(props)) = &mut self.0.kind {
            props.color = color;
        }
        self
    }
}

impl From<Slider> for Element {
    fn from(component: Slider) -> Self {
        component.0
    }
}

style_builder!(Slider);

/// The intrinsic height of a slider.
const SLIDER_HEIGHT: f32 = 40.0;

impl SliderProps {
    /// A slider fills the width it is given and has a fixed touch height.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), SLIDER_HEIGHT)
    }
}
