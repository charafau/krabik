//! The `slider` widget on Android.

use crate::android::SEEK_BAR_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::{SliderProps, snap_slider_value};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_slider(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &SliderProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let max = props.max.max(props.min + 0.001);
    ids.slider.configure.call_void(
        env,
        view,
        &[
            JValue::Float(props.min),
            JValue::Float(max),
            JValue::Float(props.step.unwrap_or(0.0)),
            JValue::Int(props.color as jint),
        ],
    )?;
    let value = snap_slider_value(props.value, props.min, max, props.step);
    ids.slider
        .set_value_silently
        .call_void(env, view, &[JValue::Float(value)])
}

/// Creates the native view of a slider.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.slider;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
    pub(crate) set_value_silently: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let seek_bar = class(env, SEEK_BAR_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &seek_bar, widget_signature)?,
            configure: method!(env, &seek_bar, "configure", "(FFFI)V", VOID),
            set_value_silently: method!(env, &seek_bar, "setValueSilently", "(F)V", VOID),
            class: seek_bar,
        })
    }
}
