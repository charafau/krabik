//! The `stepper` widget: the numbered steps of a task, one of them open.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Icon, Justify};
use crate::widgets::{Column, Container, Divider, Image, Row, Text};

/// The numbered steps of a task, top to bottom, with the content of the
/// current one shown.
///
/// ```ignore
/// Stepper::new()
///     .current(self.step)
///     .on_step(Msg::GoTo)
///     .step("Address", address_form(self))
///     .step("Payment", payment_form(self))
///     .step("Confirm", summary(self))
/// ```
///
/// Steps before the current one show a check mark. Which step is current is
/// the application's state; put the buttons that move on into each step's
/// content.
pub struct Stepper<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts<M>,
);

struct Parts<M> {
    steps: Vec<(String, RawElement)>,
    current: usize,
    on_step: Option<Arc<dyn Fn(usize) -> M + Send + Sync>>,
}

impl<M> Stepper<M> {
    /// Creates a stepper with no steps, at the first one.
    pub fn new() -> Self {
        Self(
            RawElement::container(ElementKind::Column),
            PhantomData,
            Parts {
                steps: Vec::new(),
                current: 0,
                on_step: None,
            },
        )
    }

    /// Appends a step: its title and what it shows while it is current.
    pub fn step(mut self, title: impl Into<String>, content: impl Into<Element<M>>) -> Self {
        self.2.steps.push((title.into(), content.into().into_raw()));
        self
    }

    /// Sets which step is open, counting from zero.
    pub fn current(mut self, index: usize) -> Self {
        self.2.current = index;
        self
    }

    /// Reports the index of a step whose title was tapped.
    pub fn on_step(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self {
        self.2.on_step = Some(Arc::new(message));
        self
    }
}

impl<M> Default for Stepper<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// The side of the circle a step's number stands in.
const MARK: f32 = 24.0;

impl<M: 'static> From<Stepper<M>> for Element<M> {
    fn from(component: Stepper<M>) -> Self {
        let Stepper(shell, _, parts) = component;
        let last = parts.steps.len().saturating_sub(1);
        let mut column = Column::<M>::new();
        for (index, (title, content)) in parts.steps.into_iter().enumerate() {
            let reached = index <= parts.current;
            let mark = Container::<M>::new()
                .width(MARK)
                .height(MARK)
                .corner_radius(MARK / 2.0)
                .background(if reached {
                    Color::ACCENT
                } else {
                    Color::OUTLINE
                })
                .align_items(Align::Center)
                .justify_content(Justify::Center);
            let white = Color::fixed(0xFFFF_FFFF);
            let mark = if index < parts.current {
                mark.child(Image::icon(Icon::check()).size(16).tint(white))
            } else {
                mark.child(
                    Text::new((index + 1).to_string())
                        .size(13)
                        .bold()
                        .color(white),
                )
            };
            let label = Text::new(title.clone());
            let label = if index == parts.current {
                label.bold()
            } else {
                label.color(Color::SECONDARY_TEXT)
            };
            let on_tap = match &parts.on_step {
                Some(on_step) => {
                    let on_step = Arc::clone(on_step);
                    Handler::new(move |()| on_step(index))
                }
                None => Handler::none(),
            };
            column = column.child(
                Container::new()
                    .handler(on_tap)
                    .padding_each(EdgeInsets::symmetric(0.0, 8.0))
                    .accessibility_label(title)
                    .key(format!("step-{index}"))
                    .child(
                        Row::new()
                            .spacing(12)
                            .align_items(Align::Center)
                            .child(mark)
                            .child(label),
                    ),
            );
            // Under the title: the line that joins this step to the next,
            // and beside it the content of the current step.
            let mut rail = Column::<M>::new().width(MARK).align_items(Align::Center);
            if index < last {
                rail = rail.child(Divider::new().vertical().grow(1));
            }
            let mut below = Row::<M>::new()
                .spacing(12)
                .min_height(if index < last { 16 } else { 0 })
                .key(format!("below-{index}"))
                .child(rail);
            if index == parts.current {
                below = below.child(
                    Column::new()
                        .grow(1)
                        .padding_each(EdgeInsets {
                            top: 4.0,
                            right: 0.0,
                            bottom: 12.0,
                            left: 0.0,
                        })
                        .animate_enter(150)
                        .key("content")
                        .child(Element::from_raw(content)),
                );
            }
            column = column.child(below);
        }
        finish(shell, column)
    }
}

style_builder!(Stepper);
