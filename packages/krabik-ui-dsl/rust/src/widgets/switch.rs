//! The `switch` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::widgets::Widget;

/// Platform-independent properties of a switch node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchProps {
    /// Whether the switch is on.
    pub checked: bool,
    /// Reported with the new state when the switch is toggled.
    pub on_toggle: Handler<bool>,
}

impl SwitchProps {
    /// Creates a switch in the given state with no action.
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            on_toggle: Handler::none(),
        }
    }
}

/// A native on/off switch.
pub struct Switch<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Switch<M> {
    /// Creates a switch in the given state.
    pub fn new(checked: bool) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Switch(SwitchProps::new(
                checked,
            )))),
            PhantomData,
        )
    }

    /// Sets whether the switch is on.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::Switch(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Reports the new state when the switch is toggled.
    pub fn on_toggle(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::Switch(props)) = &mut self.0.kind {
            props.on_toggle = Handler::new(message);
        }
        self
    }
}

impl<M> From<Switch<M>> for Element<M> {
    fn from(component: Switch<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Switch);

impl SwitchProps {
    /// A switch keeps the native control's size; an explicit dimension in the
    /// style still overrides it.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let size = measurer.switch_size();
        Size::new(
            max_width.map_or(size.width, |width| width.min(size.width)),
            size.height,
        )
    }
}
