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
use crate::message::Wrap;
use crate::style::{Color, ColorScheme, FontWeight};

/// Declares [`Widget`] from one table: a variant, its properties type and its
/// tag per line.
macro_rules! widgets {
    ($($(#[$doc:meta])* $variant:ident($props:ty) = $tag:literal,)+) => {
        /// A leaf widget: one native view with no children of its own.
        ///
        /// This is the single list of widgets. Every place that needs to
        /// treat widgets differently matches on it exhaustively, so adding a
        /// variant makes the compiler point at each place that must handle
        /// it.
        #[derive(Clone, Debug, PartialEq)]
        pub enum Widget {
            $($(#[$doc])* $variant($props),)+
        }

        impl Widget {
            /// A stable tag mixed into node identifiers, so a node that
            /// changes widget type gets a new identity.
            pub(crate) fn tag(&self) -> u64 {
                match self {
                    $(Self::$variant(_) => $tag,)+
                }
            }

            /// The widget's properties, behind the interface every widget
            /// shares.
            fn props(&self) -> &dyn WidgetProps {
                match self {
                    $(Self::$variant(props) => props,)+
                }
            }

            fn props_mut(&mut self) -> &mut dyn WidgetProps {
                match self {
                    $(Self::$variant(props) => props,)+
                }
            }
        }

        /// Every tag in the table, for the test that keeps them distinct.
        #[cfg(test)]
        const WIDGET_TAGS: &[(&str, u64)] = &[$((stringify!($variant), $tag),)+];
    };
}

// Tags must never be reused: they are part of a node's identity. The
// containers in `ElementKind` use 1, 2, 3, 8, 9, 11 and 20 to 24.
widgets! {
    /// A run of text rendered by a native label.
    Text(TextProps) = 4,
    /// A native button.
    Button(ButtonProps) = 5,
    /// A native text input.
    TextField(TextFieldProps) = 6,
    /// A native image.
    Image(ImageProps) = 7,
    /// A native on/off switch.
    Switch(SwitchProps) = 10,
    /// A native checkbox with a label.
    Checkbox(CheckProps) = 12,
    /// A native radio button with a label.
    RadioButton(CheckProps) = 13,
    /// A native slider.
    Slider(SliderProps) = 14,
    /// A native determinate progress indicator.
    Progress(ProgressProps) = 15,
    /// A native group of mutually exclusive radio buttons.
    RadioGroup(RadioGroupProps) = 16,
    /// A native indeterminate progress spinner.
    Spinner(SpinnerProps) = 17,
    /// A native date, time or date-and-time picker.
    DatePicker(DatePickerProps) = 18,
    /// A native tab bar.
    TabBar(TabBarProps) = 19,
}

impl Widget {
    /// Re-wraps every message the widget can produce; see
    /// [`Element::map`](crate::element::Element::map).
    pub(crate) fn map_messages(&mut self, wrap: &Wrap) {
        self.props_mut().map_messages(wrap);
    }

    /// The size the widget wants for its content when the style does not set
    /// one. `max_width` is the width available to the content, if bounded.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        self.props().intrinsic_size(max_width, measurer)
    }

    /// Picks the value for `scheme` in every colour the widget has.
    pub(crate) fn resolve_colors(&mut self, scheme: ColorScheme) {
        self.props_mut().resolve_colors(scheme);
    }

    /// The text size, colour and weight, for a widget that shows text.
    pub(crate) fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        self.props_mut().text_style()
    }
}

/// What every widget's properties provide to the framework.
///
/// `Widget` forwards to this, so a new widget implements it once instead of
/// adding an arm to each method. `#[derive(Widget)]` writes the
/// implementation.
pub(crate) trait WidgetProps: IntrinsicSize {
    /// Re-wraps every message the widget can produce.
    fn map_messages(&mut self, wrap: &Wrap);

    /// The text style, for a widget that shows text.
    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        None
    }

    /// Picks the value for `scheme` in every colour the widget has.
    fn resolve_colors(&mut self, scheme: ColorScheme);
}

/// How much room a widget wants. Written by hand for every widget.
pub(crate) trait IntrinsicSize {
    /// The size the content wants when the style does not set one.
    /// `max_width` is the width available to the content, if bounded.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size;
}

/// The text properties the shared `size`, `color` and `bold` builders set.
pub(crate) struct TextStyleMut<'a> {
    pub size: &'a mut f32,
    pub color: &'a mut Color,
    pub weight: &'a mut FontWeight,
}

/// The width assumed for a control that fills its container when the
/// container gives no bound.
pub(crate) const CONTROL_WIDTH: f32 = 200.0;

#[cfg(test)]
mod tests {
    use super::WIDGET_TAGS;

    /// Tags used by the containers in `ElementKind::tag`.
    const CONTAINER_TAGS: [u64; 11] = [1, 2, 3, 8, 9, 11, 20, 21, 22, 23, 24];

    /// A tag is part of a node's identity. Two kinds sharing one would get
    /// the same id at the same position, and the diff would update one
    /// widget's native view with the other's properties.
    #[test]
    fn every_widget_tag_is_unique() {
        for (index, (name, tag)) in WIDGET_TAGS.iter().enumerate() {
            assert!(
                !CONTAINER_TAGS.contains(tag),
                "{name} uses tag {tag}, which belongs to a container"
            );
            for (other, other_tag) in &WIDGET_TAGS[index + 1..] {
                assert_ne!(tag, other_tag, "{name} and {other} share tag {tag}");
            }
        }
    }
}
