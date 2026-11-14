//! The `text_field` widget on Android.

use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::text::apply_typeface;
use crate::android::widgets::{Ctor, Site};
use crate::android::{COMPLEX_UNIT_SP, TEXT_INPUT_CLASS};
use crate::ui::{FontWeight, ImeAction, KeyboardType, TextFieldProps};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_text_field(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &TextFieldProps,
) -> JniResult<()> {
    let value = jcall!(env, env.new_string(&props.value))?;
    let ids = jni_handles(env)?;
    ids.text_field
        .set_text_silently
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
    ids.text_field.configure.call_void(
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

/// Creates the native view of a text field.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.text_field;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) set_text_silently: Method,
    pub(crate) configure: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let text_input = class(env, TEXT_INPUT_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &text_input, widget_signature)?,
            set_text_silently: method!(
                env,
                &text_input,
                "setTextSilently",
                "(Ljava/lang/String;)V",
                VOID
            ),
            configure: method!(env, &text_input, "configure", "(ZZII)V", VOID),
            class: text_input,
        })
    }
}
