//! The `switch` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::widgets::Widget;
use std::borrow::Cow;

/// Platform-independent properties of a switch node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchProps {
    /// Whether the switch is on.
    pub checked: bool,
    /// The action dispatched whenever the switch is toggled.
    pub action: Cow<'static, str>,
}

impl SwitchProps {
    /// Creates a switch in the given state with no action.
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            action: Cow::Borrowed(""),
        }
    }
}

/// A native on/off switch.
#[derive(Clone, Debug, PartialEq)]
pub struct Switch(pub(crate) Element);

impl Switch {
    /// Creates a switch in the given state.
    pub fn new(checked: bool) -> Self {
        Self(Element::leaf(ElementKind::Widget(Widget::Switch(
            SwitchProps::new(checked),
        ))))
    }

    /// Sets whether the switch is on.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Widget(Widget::Switch(props)) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the switch is toggled.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Widget(Widget::Switch(props)) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<Switch> for Element {
    fn from(component: Switch) -> Self {
        component.0
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
