//! The `switch` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::widgets::IntrinsicSize;

/// Platform-independent properties of a switch node.
#[derive(Clone, Debug, PartialEq, Eq, Widget)]
#[widget(name = Switch, about = "A native on/off switch.", new = "Creates a switch in the given state.")]
pub struct SwitchProps {
    /// Whether the switch is on.
    #[widget(arg, set)]
    pub checked: bool,
    /// Reports the new state when the switch is toggled.
    pub on_toggle: Handler<bool>,
}

impl IntrinsicSize for SwitchProps {
    /// A switch keeps the native control's size; an explicit dimension in the
    /// style still overrides it.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let size = measurer.switch_size();
        Size::new(
            max_width.map_or(size.width, |width| width.min(size.width)),
            size.height,
        )
    }
}
