//! The `radio_group` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::ui::{FontWeight, RadioGroupProps};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue, jni_str};

pub(crate) fn update_radio_group(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &RadioGroupProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let labels = env.new_object_array(
        props.options.len() as jint,
        jni_str!("java/lang/String"),
        JObject::null(),
    )?;
    for (index, option) in props.options.iter().enumerate() {
        let label = jcall!(env, env.new_string(option))?;
        let object: &JObject<'_> = label.as_ref();
        labels.set_element(env, index, object)?;
    }
    ids.configure_radio_group.call_void(
        env,
        view,
        &[
            JValue::Object(&labels),
            JValue::Int(props.selected.map_or(-1, |index| index as jint)),
            JValue::Float(props.size),
            JValue::Int(props.color as jint),
            JValue::Float(props.spacing),
            JValue::Bool(matches!(props.weight, FontWeight::Bold)),
        ],
    )
}
