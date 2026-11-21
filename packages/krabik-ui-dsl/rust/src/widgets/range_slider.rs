//! The `range_slider` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::widgets::{CONTROL_WIDTH, IntrinsicSize, snap_slider_value};

/// Platform-independent properties of a range slider node.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = RangeSlider,
    about = "A slider with two thumbs, for choosing a range.\n\n```ignore\nRangeSlider::new(self.low, self.high)\n    .max(500)\n    .step(10)\n    .on_change(|(low, high)| Msg::Price(low, high))\n```\n\nThe two values are the application's state. A thumb cannot pass the other.",
    new = "Creates a range slider from 0 to 1 with its thumbs at `low` and `high`."
)]
pub struct RangeSliderProps {
    /// The value of the lower thumb.
    #[widget(arg)]
    pub low: f32,
    /// The value of the upper thumb.
    #[widget(arg)]
    pub high: f32,
    /// The smallest value.
    #[widget(default = 0.0, set(with = crate::style::IntoPoints::into_points, from = impl crate::style::IntoPoints))]
    pub min: f32,
    /// The largest value.
    #[widget(default = 1.0, set(with = crate::style::IntoPoints::into_points, from = impl crate::style::IntoPoints))]
    pub max: f32,
    /// The distance between the values a thumb can rest on, if any.
    #[widget(set(with = positive_step, from = f32))]
    pub step: Option<f32>,
    /// Reports both values, lower first, whenever a thumb moves.
    pub on_change: Handler<(f32, f32)>,
    /// The color of the track between the thumbs.
    #[widget(default = Color::ACCENT, set(into))]
    pub color: Color,
}

fn positive_step(step: f32) -> Option<f32> {
    (step > 0.0).then_some(step)
}

impl RangeSliderProps {
    /// The largest value, never at or below the smallest.
    pub(crate) fn upper(&self) -> f32 {
        self.max.max(self.min + 0.001)
    }

    /// Both values inside the range, on the step, lower first.
    pub(crate) fn snapped(&self, low: f32, high: f32) -> (f32, f32) {
        let snap = |value| snap_slider_value(value, self.min, self.upper(), self.step);
        let (low, high) = (snap(low), snap(high));
        (low.min(high), high.max(low))
    }

    /// The thumbs' places along the track, each from 0 to 1.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) fn fractions(&self) -> (f32, f32) {
        let (low, high) = self.snapped(self.low, self.high);
        let span = self.upper() - self.min;
        ((low - self.min) / span, (high - self.min) / span)
    }

    /// The values for two places along the track.
    pub(crate) fn values_at(&self, low: f32, high: f32) -> (f32, f32) {
        let span = self.upper() - self.min;
        self.snapped(self.min + low * span, self.min + high * span)
    }
}

/// The intrinsic height of a range slider: the smallest the platforms'
/// thumbs can be touched at.
const RANGE_SLIDER_HEIGHT: f32 = 48.0;

impl IntrinsicSize for RangeSliderProps {
    /// A range slider fills the width it is given and has a fixed height.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), RANGE_SLIDER_HEIGHT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_stay_in_the_range_on_the_step_and_in_order() {
        let props = RangeSliderProps {
            min: 0.0,
            max: 100.0,
            step: Some(10.0),
            ..RangeSliderProps::new(20.0, 80.0)
        };
        assert_eq!(props.fractions(), (0.2, 0.8));
        assert_eq!(props.values_at(0.26, 0.74), (30.0, 70.0));
        // Outside the track, and crossed over.
        assert_eq!(props.values_at(-0.5, 1.5), (0.0, 100.0));
        assert_eq!(props.values_at(0.9, 0.1), (10.0, 90.0));
        assert_eq!(props.snapped(90.0, 10.0), (10.0, 90.0));

        // A range that is empty still has a track.
        let flat = RangeSliderProps {
            min: 5.0,
            max: 5.0,
            ..RangeSliderProps::new(5.0, 5.0)
        };
        assert_eq!(flat.fractions(), (0.0, 0.0));
    }
}
