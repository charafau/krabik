//! The `progress` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::ProgressProps;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_progress(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &ProgressProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.update_progress.call_void(
        env,
        &ids.krabik_views,
        &[
            JValue::Object(view),
            JValue::Float(props.value),
            JValue::Int(props.color as jint),
        ],
    )
}
