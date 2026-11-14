//! The `button` widget on Android.

use crate::android::handles::{Method, VOID, class, ctor, get as jni_handles, method};
use crate::android::widgets::{Ctor, Site};
use crate::android::{BUTTON_CLASS, GRAVITY_CENTER, TEXT_VIEW_CLASS};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

/// Creates the native view of a button.
///
/// A stock button has no node of its own to report to, so the root view is
/// its click listener and finds the node through the view's tag.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let view = site.stock(env, &ids.button.class, &ids.button.ctor)?;
    remove_native_chrome(env, &view)?;
    ids.set_on_click_listener
        .call_void(env, &view, &[JValue::Object(site.root)])?;
    Ok(view)
}

/// Removes the insets and caps a stock button adds around its label.
fn remove_native_chrome(env: &mut Env<'_>, view: &JObject<'_>) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.button
        .set_all_caps
        .call_void(env, view, &[JValue::Bool(false)])?;
    ids.set_minimum_height
        .call_void(env, view, &[JValue::Int(0)])?;
    ids.set_minimum_width
        .call_void(env, view, &[JValue::Int(0)])?;
    // The layout already reserves the button's chrome padding around the
    // label, so the label is centred instead of relying on native insets.
    ids.set_gravity
        .call_void(env, view, &[JValue::Int(GRAVITY_CENTER)])?;
    ids.set_padding.call_void(
        env,
        view,
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
        ],
    )?;
    Ok(())
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) set_all_caps: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let button = class(env, BUTTON_CLASS)?;
        let text_view = class(env, TEXT_VIEW_CLASS)?;
        Ok(Self {
            ctor: ctor!(env, &button, "(Landroid/content/Context;)V"),
            set_all_caps: method!(env, &text_view, "setAllCaps", "(Z)V", VOID),
            class: button,
        })
    }
}
