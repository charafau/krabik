//! The `image` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::{ImageProps, ImageScale, ImageSource};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::{Env, JValue};

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

    ids.load_image.call_void(
        env,
        &ids.krabik_images,
        &[
            JValue::Object(view),
            JValue::Object(source.as_ref()),
            JValue::Bool(is_asset),
            JValue::Bool(matches!(props.scale, ImageScale::Fill)),
        ],
    )
}
