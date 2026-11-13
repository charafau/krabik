//! The `text_field` widget on Android.

use crate::android::COMPLEX_UNIT_SP;
use crate::android::handles::get as jni_handles;
use crate::android::widgets::text::apply_typeface;
use crate::ui::{FontWeight, ImeAction, KeyboardType, TextFieldProps};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_text_field(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &TextFieldProps,
) -> JniResult<()> {
    let value = jcall!(env, env.new_string(&props.value))?;
    let ids = jni_handles(env)?;
    ids.set_text_silently
        .call_void(env, view, &[JValue::Object(value.as_ref())])?;

    let hint = jcall!(env, env.new_string(&props.placeholder))?;
    ids.set_hint
        .call_void(env, view, &[JValue::Object(hint.as_ref())])?;

    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(props.size)],
    )?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(props.color as jint)])?;

    let keyboard = match props.keyboard {
        KeyboardType::Text => 0,
        KeyboardType::Email => 1,
        KeyboardType::Number => 2,
        KeyboardType::Phone => 3,
    };
    let ime_action = match props.ime_action {
        ImeAction::Default => 0,
        ImeAction::Done => 1,
        ImeAction::Next => 2,
        ImeAction::Search => 3,
        ImeAction::Send => 4,
    };
    ids.configure_input.call_void(
        env,
        view,
        &[
            JValue::Bool(props.secure),
            JValue::Bool(props.multiline),
            JValue::Int(keyboard),
            JValue::Int(ime_action),
        ],
    )?;
    // `configure` changes the input type and line mode, which can reset the
    // typeface, so the font is applied after the platform configuration.
    apply_typeface(env, view, matches!(props.weight, FontWeight::Bold))?;
    Ok(())
}
