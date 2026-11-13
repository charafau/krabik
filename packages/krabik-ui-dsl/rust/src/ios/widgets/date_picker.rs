//! The `date_picker` widget on iOS.

use crate::ios::events::dispatch_picker;
use crate::ios::{add_action, schedule};
use crate::tree::NodeId;
use crate::ui::{DatePickerMode, DatePickerProps, DatePickerValue, DateValue, TimeValue};
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_foundation::{NSCalendar, NSCalendarUnit, NSDate, NSDateComponents};
use objc2_ui_kit::{UIControlEvents, UIDatePicker, UIDatePickerMode, UIDatePickerStyle};

pub(crate) fn update_date_picker(picker: &UIDatePicker, props: &DatePickerProps) {
    let mode = match props.value {
        DatePickerValue::Date(_) => UIDatePickerMode::Date,
        DatePickerValue::Time(_) => UIDatePickerMode::Time,
        DatePickerValue::DateTime { .. } => UIDatePickerMode::DateAndTime,
    };
    picker.setDatePickerMode(mode);
    picker.setPreferredDatePickerStyle(match props.mode {
        DatePickerMode::Compact => UIDatePickerStyle::Compact,
        DatePickerMode::Inline => UIDatePickerStyle::Inline,
    });

    let current = picker_value_from_nsdate(&picker.date(), mode);
    if current != Some(props.value)
        && let Some(date) = nsdate_from_picker_value(props.value)
    {
        picker.setDate(&date);
    }

    // A date picker bounds whole days, so local noon keeps the calendar day
    // stable. A date-and-time picker bounds the exact instants: the start of
    // the first day and the end of the last day unless a time range narrows it.
    let (minimum, maximum) = match props.value {
        DatePickerValue::Date(_) => (
            props
                .date_range
                .and_then(|(min, _)| nsdate_from_date_value(min)),
            props
                .date_range
                .and_then(|(_, max)| nsdate_from_date_value(max)),
        ),
        DatePickerValue::DateTime { .. } => (
            props
                .date_range
                .map(|(min, _)| DatePickerValue::DateTime {
                    date: min,
                    time: props
                        .time_range
                        .map_or(TimeValue::new(0, 0), |(min, _)| min),
                })
                .and_then(nsdate_from_picker_value),
            props
                .date_range
                .map(|(_, max)| DatePickerValue::DateTime {
                    date: max,
                    time: props
                        .time_range
                        .map_or(TimeValue::new(23, 59), |(_, max)| max),
                })
                .and_then(nsdate_from_picker_value),
        ),
        DatePickerValue::Time(_) => (None, None),
    };
    picker.setMinimumDate(minimum.as_deref());
    picker.setMaximumDate(maximum.as_deref());
}

/// Builds a `Date` at local noon so DST transitions cannot shift the calendar day.
pub(crate) fn nsdate_from_date_value(value: DateValue) -> Option<Retained<NSDate>> {
    let components = NSDateComponents::new();
    components.setYear(value.year as isize);
    components.setMonth(value.month as isize);
    components.setDay(value.day as isize);
    components.setHour(12);
    NSCalendar::currentCalendar().dateFromComponents(&components)
}

pub(crate) fn nsdate_from_picker_value(value: DatePickerValue) -> Option<Retained<NSDate>> {
    let components = NSDateComponents::new();
    match value {
        DatePickerValue::Date(date) => {
            components.setYear(date.year as isize);
            components.setMonth(date.month as isize);
            components.setDay(date.day as isize);
            components.setHour(12);
        }
        DatePickerValue::Time(time) => {
            components.setYear(2000);
            components.setMonth(1);
            components.setDay(1);
            components.setHour(time.hour as isize);
            components.setMinute(time.minute as isize);
        }
        DatePickerValue::DateTime { date, time } => {
            components.setYear(date.year as isize);
            components.setMonth(date.month as isize);
            components.setDay(date.day as isize);
            components.setHour(time.hour as isize);
            components.setMinute(time.minute as isize);
        }
    }
    NSCalendar::currentCalendar().dateFromComponents(&components)
}

pub(crate) fn picker_value_from_nsdate(
    date: &NSDate,
    mode: UIDatePickerMode,
) -> Option<DatePickerValue> {
    let units = NSCalendarUnit::Year
        | NSCalendarUnit::Month
        | NSCalendarUnit::Day
        | NSCalendarUnit::Hour
        | NSCalendarUnit::Minute;
    let components = NSCalendar::currentCalendar().components_fromDate(units, date);
    let date = DateValue::new(
        components.year() as i32,
        components.month() as u8,
        components.day() as u8,
    );
    let time = TimeValue::new(components.hour() as u8, components.minute() as u8);
    if mode == UIDatePickerMode::Time {
        Some(DatePickerValue::Time(time))
    } else if mode == UIDatePickerMode::DateAndTime {
        Some(DatePickerValue::DateTime { date, time })
    } else {
        Some(DatePickerValue::Date(date))
    }
}

pub(crate) fn wire_date_picker(
    picker: &UIDatePicker,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        picker,
        UIControlEvents::ValueChanged,
        "krabik.picker",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UIDatePicker>().ok())
                .and_then(|picker| {
                    picker_value_from_nsdate(&picker.date(), picker.datePickerMode())
                });
            if let Some(value) = value {
                schedule(move || dispatch_picker(handle, host, id, value));
            }
        },
    );
}
