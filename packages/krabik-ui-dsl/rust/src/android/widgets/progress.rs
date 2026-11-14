//! The `progress` widget on Android.

use crate::android::handles::{
    OBJECT, StaticMethod, VOID, class, ctor, get as jni_handles, static_method,
};
use crate::android::widgets::{Ctor, Site};
use crate::android::{KRABIK_VIEWS_CLASS, PROGRESS_BAR_CLASS};
use crate::ui::ProgressProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_progress(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &ProgressProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.progress.update.call_void(
        env,
        &ids.krabik_views,
        &[
            JValue::Object(view),
            JValue::Float(props.value),
            JValue::Int(props.color.argb() as jint),
        ],
    )
}

/// Creates the native view of a determinate progress bar.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    ids.progress
        .create
        .call(env, &ids.krabik_views, &[JValue::Object(site.context)])?
        .l()
}

/// Creates the native view of an indeterminate spinner.
pub(crate) fn create_spinner<'local>(
    env: &mut Env<'local>,
    site: &Site<'_>,
) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.progress;
    site.stock(env, &ids.spinner_class, &ids.spinner_ctor)
}

/// Applies a spinner's colour.
pub(crate) fn update_spinner(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &crate::ui::SpinnerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.progress.update_spinner.call_void(
        env,
        &ids.krabik_views,
        &[
            JValue::Object(view),
            JValue::Int(props.color.argb() as jni::sys::jint),
        ],
    )
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) spinner_class: Global<JClass<'static>>,
    pub(crate) spinner_ctor: Ctor,
    pub(crate) create: StaticMethod,
    pub(crate) update: StaticMethod,
    pub(crate) update_spinner: StaticMethod,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let progress_bar = class(env, PROGRESS_BAR_CLASS)?;
        let krabik_views = class(env, KRABIK_VIEWS_CLASS)?;
        Ok(Self {
            spinner_ctor: ctor!(env, &progress_bar, "(Landroid/content/Context;)V"),
            create: static_method!(
                env,
                &krabik_views,
                "createProgress",
                "(Landroid/content/Context;)Landroid/widget/ProgressBar;",
                OBJECT
            ),
            update: static_method!(
                env,
                &krabik_views,
                "updateProgress",
                "(Landroid/widget/ProgressBar;FI)V",
                VOID
            ),
            update_spinner: static_method!(
                env,
                &krabik_views,
                "updateSpinner",
                "(Landroid/widget/ProgressBar;I)V",
                VOID
            ),
            spinner_class: progress_bar,
        })
    }
}
