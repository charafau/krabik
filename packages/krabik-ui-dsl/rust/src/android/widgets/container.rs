//! The `container` widget on Android.

use crate::android::KRABIK_VIEWS_CLASS;
use crate::android::handles::{StaticMethod, VOID, class, get as jni_handles, static_method};
use crate::android::widgets::Site;
use crate::ui::ContainerProps;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::signature::RuntimeMethodSignature;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

/// Creates the native view of a container: a frame its children are placed
/// in.
///
/// Like a stock button it has no node of its own to report to, so the root
/// view is its click listener and finds the node through the view's tag.
/// Whether a tap is accepted at all is decided by [`update_container`].
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let view = site.stock(env, &ids.frame_layout, &ids.frame_layout_ctor)?;
    ids.set_on_click_listener
        .call_void(env, &view, &[JValue::Object(site.root)])?;
    Ok(view)
}

/// Makes the container react to taps while it has a handler.
pub(crate) fn update_container(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &ContainerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.container.set_tappable.call_void(
        env,
        &ids.krabik_views,
        &[JValue::Object(view), JValue::Bool(props.on_tap.is_set())],
    )
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) set_tappable: StaticMethod,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let krabik_views = class(env, KRABIK_VIEWS_CLASS)?;
        Ok(Self {
            set_tappable: static_method!(
                env,
                &krabik_views,
                "setTappable",
                "(Landroid/view/View;Z)V",
                VOID
            ),
        })
    }
}
