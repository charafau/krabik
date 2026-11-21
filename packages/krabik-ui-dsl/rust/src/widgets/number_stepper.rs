//! The `number_stepper` widget: a number between a minus and a plus button.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Icon, TextAlign};
use crate::widgets::{IconButton, Row, Text};

/// A number between a minus and a plus button.
///
/// ```ignore
/// NumberStepper::new(self.quantity as f32)
///     .min(1)
///     .max(10)
///     .on_change(|value| Msg::Quantity(value as u32))
/// ```
///
/// The value is the application's state. A button that would leave the
/// range is dimmed and does nothing.
pub struct NumberStepper<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts<M>,
);

struct Parts<M> {
    value: f32,
    min: f32,
    max: f32,
    step: f32,
    decimals: usize,
    on_change: Option<Arc<dyn Fn(f32) -> M + Send + Sync>>,
}

impl<M> NumberStepper<M> {
    /// Creates a stepper that shows `value` and moves by one, from 0 to 100.
    pub fn new(value: f32) -> Self {
        let mut shell = RawElement::container(ElementKind::Row);
        shell.style.gap = 4.0;
        shell.style.align_items = Align::Center;
        Self(
            shell,
            PhantomData,
            Parts {
                value,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                decimals: 0,
                on_change: None,
            },
        )
    }

    /// Sets the smallest value.
    pub fn min(mut self, min: impl crate::style::IntoPoints) -> Self {
        self.2.min = min.into_points();
        self
    }

    /// Sets the largest value.
    pub fn max(mut self, max: impl crate::style::IntoPoints) -> Self {
        self.2.max = max.into_points();
        self
    }

    /// Sets how far one press moves the value. A step with a fraction also
    /// shows one more digit after the point, up to three.
    pub fn step(mut self, step: f32) -> Self {
        if step > 0.0 {
            self.2.step = step;
            self.2.decimals = (0..=3)
                .find(|digits| {
                    let scaled = step * 10f32.powi(*digits);
                    (scaled - scaled.round()).abs() < 1e-4
                })
                .unwrap_or(3) as usize;
        }
        self
    }

    /// Reports the value a press on minus or plus asks for.
    pub fn on_change(mut self, message: impl Fn(f32) -> M + Send + Sync + 'static) -> Self {
        self.2.on_change = Some(Arc::new(message));
        self
    }
}

impl<M: 'static> From<NumberStepper<M>> for Element<M> {
    fn from(component: NumberStepper<M>) -> Self {
        let NumberStepper(shell, _, parts) = component;
        let (min, max) = (parts.min, parts.max.max(parts.min));
        let button = |icon: Icon, label: &str, next: f32| {
            // Rounded to the shown digits, so 0.1 pressed three times is 0.3.
            let scale = 10f32.powi(parts.decimals as i32);
            let next = ((next * scale).round() / scale).clamp(min, max);
            let moves = (next - parts.value).abs() > f32::EPSILON;
            let handler = match &parts.on_change {
                Some(on_change) if moves => {
                    let on_change = Arc::clone(on_change);
                    Handler::new(move |()| on_change(next))
                }
                _ => Handler::none(),
            };
            IconButton::<M>::new(icon, label)
                .handler(handler)
                .opacity(if moves { 1.0 } else { 0.38 })
        };
        let row = Row::<M>::new()
            .child(button(Icon::remove(), "Decrease", parts.value - parts.step).key("minus"))
            .child(
                Text::new(format!("{:.*}", parts.decimals, parts.value))
                    .text_align(TextAlign::Center)
                    .min_width(32)
                    .key("value"),
            )
            .child(button(Icon::add(), "Increase", parts.value + parts.step).key("plus"));
        finish(shell, row)
    }
}

style_builder!(NumberStepper);
