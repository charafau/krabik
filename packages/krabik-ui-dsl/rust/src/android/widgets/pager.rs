//! The `pager` widget on Android.

use crate::android::PAGER_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::PagerProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_pager(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &PagerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.pager.configure.call_void(
        env,
        view,
        &[
            JValue::Int(props.pages as jint),
            JValue::Int(props.selected as jint),
        ],
    )
}

/// Creates the native view of a pager.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.pager;
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
        let pager = class(env, PAGER_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &pager, widget_signature)?,
            configure: method!(env, &pager, "configure", "(II)V", VOID),
            class: pager,
        })
    }
}
