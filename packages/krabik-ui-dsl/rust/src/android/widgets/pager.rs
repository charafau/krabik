//! The `pager` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::PagerProps;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_pager(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &PagerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.configure_pager.call_void(
        env,
        view,
        &[
            JValue::Int(props.pages as jint),
            JValue::Int(props.selected as jint),
        ],
    )
}
