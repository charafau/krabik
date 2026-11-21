//! The `range_slider` widget on Android.

use crate::android::RANGE_SLIDER_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::RangeSliderProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_range_slider(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &RangeSliderProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.range_slider
        .configure
        .call_void(env, view, &[JValue::Int(props.color.argb() as jint)])?;
    let (low, high) = props.fractions();
    ids.range_slider.set_places_silently.call_void(
        env,
        view,
        &[JValue::Float(low), JValue::Float(high)],
    )
}

/// Creates the native view of a range slider.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.range_slider;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
    pub(crate) set_places_silently: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let slider = class(env, RANGE_SLIDER_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &slider, widget_signature)?,
            configure: method!(env, &slider, "configure", "(I)V", VOID),
            set_places_silently: method!(env, &slider, "setPlacesSilently", "(FF)V", VOID),
            class: slider,
        })
    }
}
