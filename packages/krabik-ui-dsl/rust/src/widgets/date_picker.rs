//! The `date_picker` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder, text_builder};
use crate::engine::geometry::Size;
use crate::message::Handler;
use crate::style::FontWeight;
use crate::widgets::Widget;
use std::fmt;

/// A calendar date in the device's local calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DateValue {
    /// Full year, for example `2026`.
    pub year: i32,
    /// Month in the range `1..=12`.
    pub month: u8,
    /// Day in the range `1..=31`.
    pub day: u8,
}

impl DateValue {
    /// Creates a date, clamping the month and day to their valid ranges.
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month: if month < 1 {
                1
            } else if month > 12 {
                12
            } else {
                month
            },
            day: if day < 1 {
                1
            } else if day > 31 {
                31
            } else {
                day
            },
        }
    }
}

impl fmt::Display for DateValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// A time of day in the device's local timezone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeValue {
    /// Hour in the range `0..=23`.
    pub hour: u8,
    /// Minute in the range `0..=59`.
    pub minute: u8,
}

impl TimeValue {
    /// Creates a time, clamping the hour and minute to their valid ranges.
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self {
            hour: if hour > 23 { 23 } else { hour },
            minute: if minute > 59 { 59 } else { minute },
        }
    }
}

impl fmt::Display for TimeValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// The value of a [`DatePicker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerValue {
    /// A calendar date.
    Date(DateValue),
    /// A time of day.
    Time(TimeValue),
    /// A calendar date and a time of day.
    DateTime {
        /// The selected date.
        date: DateValue,
        /// The selected time.
        time: TimeValue,
    },
}

impl DatePickerValue {
    /// The date part, for a date or a date-and-time value.
    pub fn date(&self) -> Option<DateValue> {
        match *self {
            Self::Date(date) | Self::DateTime { date, .. } => Some(date),
            Self::Time(_) => None,
        }
    }

    /// The time part, for a time or a date-and-time value.
    pub fn time(&self) -> Option<TimeValue> {
        match *self {
            Self::Time(time) | Self::DateTime { time, .. } => Some(time),
            Self::Date(_) => None,
        }
    }

    /// Clamps this value to the optional date and time ranges.
    pub fn clamped(
        self,
        date_range: Option<(DateValue, DateValue)>,
        time_range: Option<(TimeValue, TimeValue)>,
    ) -> Self {
        // A range whose bounds are reversed is treated as unset.
        let date_range = date_range.filter(|(min, max)| max >= min);
        let time_range = time_range.filter(|(min, max)| max >= min);
        let clamp_date = |date: DateValue| match date_range {
            Some((min, max)) => date.max(min).min(max),
            None => date,
        };
        let clamp_time = |time: TimeValue| match time_range {
            Some((min, max)) => time.max(min).min(max),
            None => time,
        };
        match self {
            Self::Date(date) => Self::Date(clamp_date(date)),
            Self::Time(time) => Self::Time(clamp_time(time)),
            Self::DateTime { date, time } => {
                let date = clamp_date(date);
                let mut time = clamp_time(time);
                if let (Some((min, _)), Some((min_time, _))) = (date_range, time_range)
                    && date == min
                    && time < min_time
                {
                    time = min_time;
                }
                if let (Some((_, max)), Some((_, max_time))) = (date_range, time_range)
                    && date == max
                    && time > max_time
                {
                    time = max_time;
                }
                Self::DateTime { date, time }
            }
        }
    }
}

/// How a [`DatePicker`] presents its picker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerMode {
    /// A compact field that opens the platform's picker.
    #[default]
    Compact,
    /// An inline picker that is always visible.
    Inline,
}

/// Platform-independent properties of a date, time or date-and-time picker.
#[derive(Clone, Debug, PartialEq)]
pub struct DatePickerProps {
    /// The selected date, time or both.
    pub value: DatePickerValue,
    /// How the picker is presented.
    pub mode: DatePickerMode,
    /// The optional inclusive date range.
    pub date_range: Option<(DateValue, DateValue)>,
    /// The optional inclusive time range.
    pub time_range: Option<(TimeValue, TimeValue)>,
    /// Reported with the new value whenever it changes.
    pub on_change: Handler<DatePickerValue>,
    /// Font size of the compact field in logical points.
    pub size: f32,
    /// ARGB color of the compact field's text.
    pub color: u32,
    /// Font weight of the compact field's text.
    pub weight: FontWeight,
}

impl DatePickerProps {
    /// Creates a compact date picker with the given value.
    pub fn new(value: DatePickerValue) -> Self {
        Self {
            value,
            mode: DatePickerMode::Compact,
            date_range: None,
            time_range: None,
            on_change: Handler::none(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A native date, time or date-and-time picker.
///
/// The value kind is chosen by the constructor: [`DatePicker::new`] for a
/// date, [`DatePicker::time`] for a time, and [`DatePicker::date_time`] for
/// both. The reported event matches the kind.
pub struct DatePicker<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> DatePicker<M> {
    /// Creates a compact date picker with the given date.
    pub fn new(value: DateValue) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::DatePicker(
                DatePickerProps::new(DatePickerValue::Date(value)),
            ))),
            PhantomData,
        )
    }

    /// Creates a compact time picker with the given time.
    pub fn time(value: TimeValue) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::DatePicker(
                DatePickerProps::new(DatePickerValue::Time(value)),
            ))),
            PhantomData,
        )
    }

    /// Creates a compact date-and-time picker.
    pub fn date_time(date: DateValue, time: TimeValue) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::DatePicker(
                DatePickerProps::new(DatePickerValue::DateTime { date, time }),
            ))),
            PhantomData,
        )
    }

    /// Chooses between the compact field and the inline picker.
    pub fn mode(mut self, mode: DatePickerMode) -> Self {
        if let ElementKind::Widget(Widget::DatePicker(props)) = &mut self.0.kind {
            props.mode = mode;
        }
        self
    }

    /// Sets inclusive date bounds.
    pub fn range(mut self, min: DateValue, max: DateValue) -> Self {
        if let ElementKind::Widget(Widget::DatePicker(props)) = &mut self.0.kind {
            props.date_range = Some((min, max));
        }
        self
    }

    /// Sets inclusive time bounds.
    pub fn time_range(mut self, min: TimeValue, max: TimeValue) -> Self {
        if let ElementKind::Widget(Widget::DatePicker(props)) = &mut self.0.kind {
            props.time_range = Some((min, max));
        }
        self
    }

    /// Reports the new value whenever it changes.
    pub fn on_change(
        mut self,
        message: impl Fn(DatePickerValue) -> M + Send + Sync + 'static,
    ) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::DatePicker(props)) = &mut self.0.kind {
            props.on_change = Handler::new(message);
        }
        self
    }
}

impl<M> From<DatePicker<M>> for Element<M> {
    fn from(component: DatePicker<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(DatePicker);
text_builder!(DatePicker);

/// The width assumed for a compact date field without a constraint.
const DATE_PICKER_WIDTH: f32 = 200.0;
/// The width assumed for a compact time field.
const TIME_PICKER_WIDTH: f32 = 120.0;
/// The width assumed for a compact date-and-time field.
const DATETIME_PICKER_WIDTH: f32 = 240.0;
/// The height of a compact picker field.
const DATE_PICKER_HEIGHT: f32 = 48.0;

impl DatePickerProps {
    /// A compact field has a fixed size; an inline picker fits the available
    /// width up to its natural size.
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>) -> Size {
        let (compact_width, inline_width, inline_height) = match self.value {
            DatePickerValue::Date(_) => (DATE_PICKER_WIDTH, 320.0, 320.0),
            DatePickerValue::Time(_) => (TIME_PICKER_WIDTH, 240.0, 240.0),
            DatePickerValue::DateTime { .. } => (DATETIME_PICKER_WIDTH, 360.0, 480.0),
        };
        match self.mode {
            DatePickerMode::Compact => Size::new(compact_width, DATE_PICKER_HEIGHT),
            DatePickerMode::Inline => Size::new(
                max_width.unwrap_or(inline_width).min(inline_width),
                inline_height,
            ),
        }
    }
}
