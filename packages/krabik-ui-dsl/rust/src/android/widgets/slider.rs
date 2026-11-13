//! The `slider` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::{SliderProps, snap_slider_value};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_slider(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &SliderProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let max = props.max.max(props.min + 0.001);
    ids.configure_seek_bar.call_void(
        env,
        view,
        &[
            JValue::Float(props.min),
            JValue::Float(max),
            JValue::Float(props.step.unwrap_or(0.0)),
            JValue::Int(props.color as jint),
        ],
    )?;
    let value = snap_slider_value(props.value, props.min, max, props.step);
    ids.seek_set_value_silently
        .call_void(env, view, &[JValue::Float(value)])
}
