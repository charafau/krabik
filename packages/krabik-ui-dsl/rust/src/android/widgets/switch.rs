//! The `switch` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::SwitchProps;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::{Env, JValue};

pub(crate) fn update_switch(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &SwitchProps,
) -> JniResult<()> {
    jni_handles(env)?
        .set_checked_silently
        .call_void(env, view, &[JValue::Bool(props.checked)])
}
