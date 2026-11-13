//! The `date_picker` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::{DatePickerMode, DatePickerProps, DatePickerValue, FontWeight};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_date_picker(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &DatePickerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let (kind, year, month, day, hour, minute) = match props.value {
        DatePickerValue::Date(date) => (0, date.year, date.month as jint, date.day as jint, 0, 0),
        DatePickerValue::Time(time) => (1, 1970, 1, 1, time.hour as jint, time.minute as jint),
        DatePickerValue::DateTime { date, time } => (
            2,
            date.year,
            date.month as jint,
            date.day as jint,
            time.hour as jint,
            time.minute as jint,
        ),
    };
    ids.configure_date_picker.call_void(
        env,
        view,
        &[
            JValue::Int(kind),
            JValue::Int(year),
            JValue::Int(month),
            JValue::Int(day),
            JValue::Int(hour),
            JValue::Int(minute),
            JValue::Bool(props.mode == DatePickerMode::Inline),
        ],
    )?;
    let (min_date, max_date) = match props.date_range {
        Some((min, max)) => (
            [min.year, min.month as jint, min.day as jint],
            [max.year, max.month as jint, max.day as jint],
        ),
        None => ([0; 3], [0; 3]),
    };
    ids.date_picker_set_date_range.call_void(
        env,
        view,
        &[
            JValue::Int(min_date[0]),
            JValue::Int(min_date[1]),
            JValue::Int(min_date[2]),
            JValue::Int(max_date[0]),
            JValue::Int(max_date[1]),
            JValue::Int(max_date[2]),
        ],
    )?;
    let (min_time, max_time) = match props.time_range {
        Some((min, max)) => (
            [min.hour as jint, min.minute as jint],
            [max.hour as jint, max.minute as jint],
        ),
        None => ([-1; 2], [-1; 2]),
    };
    ids.date_picker_set_time_range.call_void(
        env,
        view,
        &[
            JValue::Int(min_time[0]),
            JValue::Int(min_time[1]),
            JValue::Int(max_time[0]),
            JValue::Int(max_time[1]),
        ],
    )?;
    ids.date_picker_set_style.call_void(
        env,
        view,
        &[
            JValue::Float(props.size),
            JValue::Int(props.color as jint),
            JValue::Bool(matches!(props.weight, FontWeight::Bold)),
        ],
    )
}
