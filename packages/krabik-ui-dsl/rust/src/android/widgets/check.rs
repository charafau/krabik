//! The `check` widget on Android.

use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::text::apply_typeface;
use crate::android::widgets::{Ctor, Site};
use crate::android::{CHECKBOX_CLASS, COMPLEX_UNIT_SP, RADIO_CLASS};
use crate::ui::{CheckProps, FontWeight};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

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
        .call_void(env, view, &[JValue::Int(props.color.argb() as jint)])?;
    ids.set_max_lines.call_void(env, view, &[JValue::Int(1)])?;
    let ellipsis = ids.truncate_at_end.get(env, &ids.truncate_at)?;
    ids.set_ellipsize
        .call_void(env, view, &[JValue::Object(ellipsis.l()?.as_ref())])?;
    apply_typeface(env, view, matches!(props.weight, FontWeight::Bold))?;

    let setter = if radio {
        ids.check.radio_set_checked
    } else {
        ids.check.checkbox_set_checked
    };
    setter.call_void(env, view, &[JValue::Bool(props.checked)])
}

/// Creates the native view of a checkbox, or of a radio button.
pub(crate) fn create<'local>(
    env: &mut Env<'local>,
    site: &Site<'_>,
    radio: bool,
) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.check;
    if radio {
        site.framework(env, &ids.radio_class, &ids.radio_ctor)
    } else {
        site.framework(env, &ids.checkbox_class, &ids.checkbox_ctor)
    }
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) checkbox_class: Global<JClass<'static>>,
    pub(crate) checkbox_ctor: Ctor,
    pub(crate) radio_class: Global<JClass<'static>>,
    pub(crate) radio_ctor: Ctor,
    pub(crate) checkbox_set_checked: Method,
    pub(crate) radio_set_checked: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let checkbox = class(env, CHECKBOX_CLASS)?;
        let radio_button = class(env, RADIO_CLASS)?;
        Ok(Self {
            checkbox_ctor: widget_ctor(env, &checkbox, widget_signature)?,
            radio_ctor: widget_ctor(env, &radio_button, widget_signature)?,
            checkbox_set_checked: method!(env, &checkbox, "setCheckedSilently", "(Z)V", VOID),
            radio_set_checked: method!(env, &radio_button, "setCheckedSilently", "(Z)V", VOID),
            checkbox_class: checkbox,
            radio_class: radio_button,
        })
    }
}
