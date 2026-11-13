//! The `radio_group` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::{FontWeight, IntoPoints};
use crate::widgets::Widget;
use crate::widgets::{CHECK_GAP, CHECK_HEIGHT, CHECK_INDICATOR, TextProps};

/// Platform-independent properties of a group of mutually exclusive radio buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioGroupProps {
    /// The labels of the options, in display order.
    pub options: Vec<String>,
    /// The index of the selected option, or `None` when nothing is selected.
    pub selected: Option<usize>,
    /// Reported with the index of the option the user picked.
    pub on_select: Handler<usize>,
    /// Font size of the labels in logical points.
    pub size: f32,
    /// ARGB color of the labels and indicators.
    pub color: u32,
    /// Font weight of the labels.
    pub weight: FontWeight,
    /// Space between two options in logical points.
    pub spacing: f32,
}

impl RadioGroupProps {
    /// Creates a radio group with the given option labels.
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            selected: None,
            on_select: Handler::none(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
            spacing: 8.0,
        }
    }
}

/// A native group of mutually exclusive radio buttons.
///
/// Unlike individual [`RadioButton`](crate::ui::RadioButton)s, the group owns the selection: tapping an
/// option reports the chosen index through [`RadioGroup::on_select`].
pub struct RadioGroup<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> RadioGroup<M> {
    /// Creates a radio group with the given option labels.
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::RadioGroup(
                RadioGroupProps::new(options),
            ))),
            PhantomData,
        )
    }

    /// Sets the index of the selected option.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Widget(Widget::RadioGroup(props)) = &mut self.0.kind {
            props.selected = Some(index);
        }
        self
    }

    /// Reports the index of the option the user picked.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::RadioGroup(props)) = &mut self.0.kind {
            props.on_select = Handler::new(message);
        }
        self
    }

    /// Sets the space between two options in logical points.
    pub fn spacing(mut self, spacing: impl IntoPoints) -> Self {
        if let ElementKind::Widget(Widget::RadioGroup(props)) = &mut self.0.kind {
            props.spacing = spacing.into_points();
        }
        self
    }
}

impl<M> From<RadioGroup<M>> for Element<M> {
    fn from(component: RadioGroup<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(RadioGroup);
text_builder!(RadioGroup);

impl RadioGroupProps {
    /// The options stacked vertically, as wide as the widest one.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        if self.options.is_empty() {
            return Size::ZERO;
        }
        let inner_width = max_width.map(|width| (width - CHECK_INDICATOR - CHECK_GAP).max(0.0));
        let mut widest = 0.0f32;
        let mut height = 0.0f32;
        for option in &self.options {
            let label = TextProps {
                content: option.clone(),
                size: self.size,
                color: self.color,
                weight: self.weight,
                ..TextProps::new("")
            };
            let text = label.intrinsic_size(inner_width, measurer);
            widest = widest.max(text.width);
            height += text.height.max(CHECK_HEIGHT);
        }
        height += self.spacing * (self.options.len() - 1) as f32;
        Size::new(widest + CHECK_INDICATOR + CHECK_GAP, height)
    }
}
