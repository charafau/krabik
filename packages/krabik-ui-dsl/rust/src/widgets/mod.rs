//! The built-in widgets, one module each.

mod button;
mod check;
mod containers;
mod date_picker;
mod image;
mod list;
mod pager;
mod progress;
mod radio_group;
mod scroll;
mod slider;
mod switch;
mod tab_bar;
mod text;
mod text_field;

pub use button::*;
pub use check::*;
pub use containers::*;
pub use date_picker::*;
pub use image::*;
pub use list::*;
pub use pager::*;
pub use progress::*;
pub use radio_group::*;
pub use scroll::*;
pub use slider::*;
pub use switch::*;
pub use tab_bar::*;
pub use text::*;
pub use text_field::*;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;

/// A leaf widget: one native view with no children of its own.
///
/// This is the single list of widgets. Every place that needs to treat
/// widgets differently matches on it exhaustively, so adding a variant makes
/// the compiler point at each place that must handle it.
#[derive(Clone, Debug, PartialEq)]
pub enum Widget {
    /// A run of text rendered by a native label.
    Text(TextProps),
    /// A native button.
    Button(ButtonProps),
    /// A native text input.
    TextField(TextFieldProps),
    /// A native on/off switch.
    Switch(SwitchProps),
    /// A native checkbox with a label.
    Checkbox(CheckProps),
    /// A native radio button with a label.
    RadioButton(CheckProps),
    /// A native group of mutually exclusive radio buttons.
    RadioGroup(RadioGroupProps),
    /// A native tab bar.
    TabBar(TabBarProps),
    /// A native date, time or date-and-time picker.
    DatePicker(DatePickerProps),
    /// A native slider.
    Slider(SliderProps),
    /// A native determinate progress indicator.
    Progress(ProgressProps),
    /// A native indeterminate progress spinner.
    Spinner(SpinnerProps),
    /// A native image.
    Image(ImageProps),
}

impl Widget {
    /// A stable tag mixed into node identifiers, so a node that changes
    /// widget type gets a new identity. Values must never be reused.
    pub(crate) fn tag(&self) -> u64 {
        match self {
            Self::Text(_) => 4,
            Self::Button(_) => 5,
            Self::TextField(_) => 6,
            Self::Image(_) => 7,
            Self::Switch(_) => 10,
            Self::Checkbox(_) => 12,
            Self::RadioButton(_) => 13,
            Self::Slider(_) => 14,
            Self::Progress(_) => 15,
            Self::RadioGroup(_) => 16,
            Self::Spinner(_) => 17,
            Self::DatePicker(_) => 18,
            Self::TabBar(_) => 19,
        }
    }

    /// The size the widget wants for its content when the style does not set
    /// one. `max_width` is the width available to the content, if bounded.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        match self {
            Self::Text(props) => props.intrinsic_size(max_width, measurer),
            Self::Button(props) => props.intrinsic_size(max_width, measurer),
            Self::TextField(props) => props.intrinsic_size(max_width, measurer),
            Self::Switch(props) => props.intrinsic_size(max_width, measurer),
            Self::Checkbox(props) | Self::RadioButton(props) => {
                props.intrinsic_size(max_width, measurer)
            }
            Self::RadioGroup(props) => props.intrinsic_size(max_width, measurer),
            Self::TabBar(props) => props.intrinsic_size(max_width),
            Self::DatePicker(props) => props.intrinsic_size(max_width),
            Self::Slider(props) => props.intrinsic_size(max_width),
            Self::Progress(props) => props.intrinsic_size(max_width),
            Self::Spinner(props) => props.intrinsic_size(),
            Self::Image(props) => props.intrinsic_size(measurer),
        }
    }
}

/// The width assumed for a control that fills its container when the
/// container gives no bound.
pub(crate) const CONTROL_WIDTH: f32 = 200.0;
