//! The built-in widgets, one module each.

mod autocomplete;
mod avatar;
mod badge;
mod button;
mod card;
mod check;
mod chip;
mod containers;
mod date_picker;
mod divider;
mod expansion_tile;
mod grid;
mod icon_button;
mod image;
mod list;
mod list_tile;
mod menu;
mod native_view;
mod number_stepper;
mod pager;
mod progress;
mod radio_group;
mod range_slider;
mod scroll;
mod search_bar;
mod segmented_control;
mod slider;
mod stepper;
mod switch;
mod tab_bar;
mod text;
mod text_field;

pub use autocomplete::*;
pub use avatar::*;
pub use badge::*;
pub use button::*;
pub use card::*;
pub use check::*;
pub use chip::*;
pub use containers::*;
pub use date_picker::*;
pub use divider::*;
pub use expansion_tile::*;
pub use grid::*;
pub use icon_button::*;
pub use image::*;
pub use list::*;
pub use list_tile::*;
pub use menu::*;
pub use native_view::*;
pub use number_stepper::*;
pub use pager::*;
pub use progress::*;
pub use radio_group::*;
pub use range_slider::*;
pub use scroll::*;
pub use search_bar::*;
pub use segmented_control::*;
pub use slider::*;
pub use stepper::*;
pub use switch::*;
pub use tab_bar::*;
pub use text::*;
pub use text_field::*;

#[cfg(test)]
mod composed_tests;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::MessageMap;
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
// containers in `ElementKind` use 1, 2, 3, 8, 9, 11 and 20 to 24, and 26
// is a text that can be selected.
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
    /// A native slider with two thumbs.
    RangeSlider(RangeSliderProps) = 25,
    /// A view made by the application's own platform code.
    NativeView(NativeViewProps) = 27,
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
    pub(crate) fn map_messages(&mut self, wrap: &MessageMap) {
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

    /// Multiplies the size of the text the widget shows by `scale`, and
    /// what else is measured in the unit of that size.
    pub(crate) fn scale_text(&mut self, scale: f32) {
        if let Some(style) = self.text_style() {
            *style.size *= scale;
        }
        if let Widget::Text(props) = self {
            props.letter_spacing *= scale;
            if let Some(line_height) = &mut props.line_height {
                *line_height *= scale;
            }
            for span in &mut props.spans {
                if let Some(size) = &mut span.size {
                    *size *= scale;
                }
            }
        }
    }
}

/// What every widget's properties provide to the framework.
///
/// `Widget` forwards to this, so a new widget implements it once instead of
/// adding an arm to each method. `#[derive(Widget)]` writes the
/// implementation.
pub(crate) trait WidgetProps: IntrinsicSize {
    /// Re-wraps every message the widget can produce.
    fn map_messages(&mut self, wrap: &MessageMap);

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
