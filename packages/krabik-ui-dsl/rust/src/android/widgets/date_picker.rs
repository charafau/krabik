//! The `date_picker` widget on Android.

use crate::android::DATE_PICKER_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::{DatePickerMode, DatePickerProps, DatePickerValue, FontWeight};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

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
    ids.date_picker.configure.call_void(
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
    ids.date_picker.set_date_range.call_void(
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
    ids.date_picker.set_time_range.call_void(
        env,
        view,
        &[
            JValue::Int(min_time[0]),
            JValue::Int(min_time[1]),
            JValue::Int(max_time[0]),
            JValue::Int(max_time[1]),
        ],
    )?;
    ids.date_picker.set_style.call_void(
        env,
        view,
        &[
            JValue::Float(props.size),
            JValue::Int(props.color as jint),
            JValue::Bool(matches!(props.weight, FontWeight::Bold)),
        ],
    )
}

/// Creates the native view of a date picker.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.date_picker;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
    pub(crate) set_date_range: Method,
    pub(crate) set_time_range: Method,
    pub(crate) set_style: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let date_picker = class(env, DATE_PICKER_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &date_picker, widget_signature)?,
            configure: method!(env, &date_picker, "configure", "(IIIIIIZ)V", VOID),
            set_date_range: method!(env, &date_picker, "setDateRange", "(IIIIII)V", VOID),
            set_time_range: method!(env, &date_picker, "setTimeRange", "(IIII)V", VOID),
            set_style: method!(env, &date_picker, "setStyle", "(FIZ)V", VOID),
            class: date_picker,
        })
    }
}
