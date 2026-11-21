//! The `tab_bar` widget on Android.

use crate::android::TAB_BAR_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::TabBarProps;
use jni::errors::Result as JniResult;
use jni::jni_sig;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue, jni_str};

pub(crate) fn update_tab_bar(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &TabBarProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let labels = env.new_object_array(
        props.tabs.len() as jint,
        jni_str!("java/lang/String"),
        JObject::null(),
    )?;
    for (index, tab) in props.tabs.iter().enumerate() {
        let label = jcall!(env, env.new_string(tab))?;
        let object: &JObject<'_> = label.as_ref();
        labels.set_element(env, index, object)?;
    }
    ids.tab_bar.configure.call_void(
        env,
        view,
        &[
            JValue::Object(&labels),
            JValue::Int(props.selected as jint),
            JValue::Float(props.size),
            JValue::Int(props.color.argb() as jint),
            // the control only knows regular and bold
            JValue::Bool(props.weight.is_bold()),
        ],
    )
}

/// Creates the native view of a tab bar.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.tab_bar;
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
        let tab_bar = class(env, TAB_BAR_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &tab_bar, widget_signature)?,
            configure: method!(
                env,
                &tab_bar,
                "configure",
                "([Ljava/lang/String;IFIZ)V",
                VOID
            ),
            class: tab_bar,
        })
    }
}
