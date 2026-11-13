//! The `check` widget on Android.

use crate::android::COMPLEX_UNIT_SP;
use crate::android::handles::get as jni_handles;
use crate::android::widgets::text::apply_typeface;
use crate::ui::{CheckProps, FontWeight};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_check(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &CheckProps,
    radio: bool,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.set_include_font_padding
        .call_void(env, view, &[JValue::Bool(false)])?;
    let label = jcall!(env, env.new_string(&props.label))?;
    ids.set_text
        .call_void(env, view, &[JValue::Object(label.as_ref())])?;
    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(props.size)],
    )?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(props.color as jint)])?;
    ids.set_max_lines.call_void(env, view, &[JValue::Int(1)])?;
    let ellipsis = ids.truncate_at_end.get(env, &ids.truncate_at)?;
    ids.set_ellipsize
        .call_void(env, view, &[JValue::Object(ellipsis.l()?.as_ref())])?;
    apply_typeface(env, view, matches!(props.weight, FontWeight::Bold))?;

    let setter = if radio {
        ids.radio_set_checked_silently
    } else {
        ids.checkbox_set_checked_silently
    };
    setter.call_void(env, view, &[JValue::Bool(props.checked)])
}
