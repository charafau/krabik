//! The `tab_bar` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::{FontWeight, TabBarProps};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
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
    ids.configure_tab_bar.call_void(
        env,
        view,
        &[
            JValue::Object(&labels),
            JValue::Int(props.selected as jint),
            JValue::Float(props.size),
            JValue::Int(props.color as jint),
            JValue::Bool(matches!(props.weight, FontWeight::Bold)),
        ],
    )
}
