//! The `switch` widget on Android.

use crate::android::handles::{Method, VOID, class, ctor, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::android::{PLAIN_SWITCH_CLASS, SWITCH_CLASS};
use crate::ui::SwitchProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_switch(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &SwitchProps,
) -> JniResult<()> {
    jni_handles(env)?.switch.set_checked_silently.call_void(
        env,
        view,
        &[JValue::Bool(props.checked)],
    )
}

/// Creates the native view of a switch.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.switch;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) plain_class: Global<JClass<'static>>,
    pub(crate) plain_ctor: Ctor,
    pub(crate) set_checked_silently: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let switch = class(env, SWITCH_CLASS)?;
        let plain_switch = class(env, PLAIN_SWITCH_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &switch, widget_signature)?,
            plain_ctor: ctor!(env, &plain_switch, "(Landroid/content/Context;)V"),
            set_checked_silently: method!(env, &switch, "setCheckedSilently", "(Z)V", VOID),
            class: switch,
            plain_class: plain_switch,
        })
    }
}
