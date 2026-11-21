//! The `radio_group` widget on Android.

use crate::android::RADIO_GROUP_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::RadioGroupProps;
use jni::errors::Result as JniResult;
use jni::jni_sig;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue, jni_str};

pub(crate) fn update_radio_group(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &RadioGroupProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let labels = env.new_object_array(
        props.options.len() as jint,
        jni_str!("java/lang/String"),
        JObject::null(),
    )?;
    for (index, option) in props.options.iter().enumerate() {
        let label = jcall!(env, env.new_string(option))?;
        let object: &JObject<'_> = label.as_ref();
        labels.set_element(env, index, object)?;
    }
    ids.radio_group.configure.call_void(
        env,
        view,
        &[
            JValue::Object(&labels),
            JValue::Int(props.selected.map_or(-1, |index| index as jint)),
            JValue::Float(props.size),
            JValue::Int(props.color.argb() as jint),
            JValue::Float(props.spacing),
            // the control only knows regular and bold
            JValue::Bool(props.weight.is_bold()),
        ],
    )
}

/// Creates the native view of a radio group.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.radio_group;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let radio_group = class(env, RADIO_GROUP_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &radio_group, widget_signature)?,
            configure: method!(
                env,
                &radio_group,
                "configure",
                "([Ljava/lang/String;IFIFZ)V",
                VOID
            ),
            class: radio_group,
        })
    }
}
