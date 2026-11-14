//! The `image` widget on Android.

use crate::android::handles::{
    LONG, StaticMethod, VOID, class, ctor, get as jni_handles, static_method,
};
use crate::android::widgets::{Ctor, Site};
use crate::android::{IMAGE_VIEW_CLASS, KRABIK_IMAGES_CLASS};
use crate::ui::{ImageProps, ImageScale, ImageSource};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_image(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &ImageProps,
) -> JniResult<()> {
    let (source, is_asset) = match &props.source {
        ImageSource::Asset(path) => (path.as_str(), true),
        ImageSource::Url(url) => (url.as_str(), false),
    };
    let source = jcall!(env, env.new_string(source))?;
    let ids = jni_handles(env)?;

    ids.image.load.call_void(
        env,
        &ids.image.images_class,
        &[
            JValue::Object(view),
            JValue::Object(source.as_ref()),
            JValue::Bool(is_asset),
            JValue::Bool(matches!(props.scale, ImageScale::Fill)),
        ],
    )
}

/// Creates the native view of an image.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.image;
    site.stock(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) images_class: Global<JClass<'static>>,
    pub(crate) load: StaticMethod,
    pub(crate) size: StaticMethod,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let image_view = class(env, IMAGE_VIEW_CLASS)?;
        let krabik_images = class(env, KRABIK_IMAGES_CLASS)?;
        Ok(Self {
            ctor: ctor!(env, &image_view, "(Landroid/content/Context;)V"),
            load: static_method!(
                env,
                &krabik_images,
                "load",
                "(Landroid/widget/ImageView;Ljava/lang/String;ZZ)V",
                VOID
            ),
            size: static_method!(
                env,
                &krabik_images,
                "size",
                "(Landroid/content/Context;Ljava/lang/String;)J",
                LONG
            ),
            class: image_view,
            images_class: krabik_images,
        })
    }
}
