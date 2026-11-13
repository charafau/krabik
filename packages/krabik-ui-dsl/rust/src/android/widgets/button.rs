//! The `button` widget on Android.

use crate::android::GRAVITY_CENTER;
use crate::android::handles::get as jni_handles;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::{Env, JValue};

pub(crate) fn configure_button(env: &mut Env<'_>, view: &JObject<'_>) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.set_all_caps
        .call_void(env, view, &[JValue::Bool(false)])?;
    ids.set_minimum_height
        .call_void(env, view, &[JValue::Int(0)])?;
    ids.set_minimum_width
        .call_void(env, view, &[JValue::Int(0)])?;
    // The layout already reserves the button's chrome padding around the
    // label, so the label is centred instead of relying on native insets.
    ids.set_gravity
        .call_void(env, view, &[JValue::Int(GRAVITY_CENTER)])?;
    ids.set_padding.call_void(
        env,
        view,
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
        ],
    )?;
    Ok(())
}
