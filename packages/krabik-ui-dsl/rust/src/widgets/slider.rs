//! The `slider` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::widgets::CONTROL_WIDTH;
use crate::widgets::IntrinsicSize;

/// Platform-independent properties of a slider.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = Slider,
    about = "A native slider.",
    new = "Creates a slider with the given value and the default range `0..=1`."
)]
pub struct SliderProps {
    /// The current value.
    #[widget(arg)]
    pub value: f32,
    /// The lowest selectable value.
    #[widget(default = 0.0, set)]
    pub min: f32,
    /// The highest selectable value.
    #[widget(default = 1.0, set)]
    pub max: f32,
    /// The interval values snap to. A non-positive step, or `None`, means a
    /// continuous slider.
    #[widget(set(with = positive_step, from = f32))]
    pub step: Option<f32>,
    /// Reports the new value whenever it changes.
    pub on_change: Handler<f32>,
    /// Color of the active track and thumb.
    #[widget(default = Color::ACCENT, set(into))]
    pub color: Color,
}

fn positive_step(step: f32) -> Option<f32> {
    (step > 0.0).then_some(step)
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

/// The intrinsic height of a slider.
const SLIDER_HEIGHT: f32 = 40.0;

impl IntrinsicSize for SliderProps {
    /// A slider fills the width it is given and has a fixed touch height.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), SLIDER_HEIGHT)
    }
}
