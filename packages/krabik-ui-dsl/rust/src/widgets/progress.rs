//! The `progress` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Wrap;
use crate::style::Color;
use crate::widgets::CONTROL_WIDTH;
use crate::widgets::IntrinsicSize;
use crate::widgets::Widget;
use crate::widgets::WidgetProps;

/// Platform-independent properties of a progress indicator.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressProps {
    /// The progress in the range `0.0..=1.0`.
    pub value: f32,
    /// Color of the filled part.
    pub color: Color,
}

impl ProgressProps {
    /// Creates a progress indicator with the given fraction.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            color: Color::ACCENT,
        }
    }
}

/// Platform-independent properties of an indeterminate progress spinner.
#[derive(Clone, Debug, PartialEq)]
pub struct SpinnerProps {
    /// Color of the spinner.
    pub color: Color,
}

impl SpinnerProps {
    /// Creates a spinner with the default accent color.
    pub fn new() -> Self {
        Self {
            color: Color::ACCENT,
        }
    }
}

impl Default for SpinnerProps {
    fn default() -> Self {
        Self::new()
    }
}

/// A native determinate progress indicator.
pub struct ProgressIndicator<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> ProgressIndicator<M> {
    /// Creates a determinate progress indicator with a fraction in `0.0..=1.0`.
    pub fn new(value: f32) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Progress(ProgressProps::new(
                value.clamp(0.0, 1.0),
            )))),
            PhantomData,
        )
    }

    /// Creates an indeterminate progress spinner.
    pub fn indeterminate() -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Spinner(SpinnerProps::new()))),
            PhantomData,
        )
    }

    /// Sets the color of the filled part or spinner.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        let color = color.into();
        match &mut self.0.kind {
            ElementKind::Widget(Widget::Progress(props)) => props.color = color,
            ElementKind::Widget(Widget::Spinner(props)) => props.color = color,
            _ => {}
        }
        self
    }
}

impl<M> From<ProgressIndicator<M>> for Element<M> {
    fn from(component: ProgressIndicator<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(ProgressIndicator);

/// The intrinsic height of a progress bar.
const PROGRESS_HEIGHT: f32 = 6.0;
/// The side of an indeterminate spinner in logical points.
const SPINNER_SIZE: f32 = 24.0;

impl WidgetProps for ProgressProps {
    fn map_messages(&mut self, _wrap: &Wrap) {}

    fn resolve_colors(&mut self, scheme: crate::style::ColorScheme) {
        self.color = self.color.resolve(scheme);
    }
}

impl IntrinsicSize for ProgressProps {
    /// A progress bar fills the width it is given.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(max_width.unwrap_or(CONTROL_WIDTH), PROGRESS_HEIGHT)
    }
}

impl WidgetProps for SpinnerProps {
    fn map_messages(&mut self, _wrap: &Wrap) {}

    fn resolve_colors(&mut self, scheme: crate::style::ColorScheme) {
        self.color = self.color.resolve(scheme);
    }
}

impl IntrinsicSize for SpinnerProps {
    /// A spinner is a fixed square.
    fn intrinsic_size(&self, _max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(SPINNER_SIZE, SPINNER_SIZE)
    }
}
