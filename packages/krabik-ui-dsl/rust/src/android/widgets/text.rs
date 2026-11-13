//! The `text` widget on Android.

use crate::android::handles::get as jni_handles;
use crate::android::{COMPLEX_UNIT_SP, GRAVITY_CENTER_HORIZONTAL, GRAVITY_END, GRAVITY_START};
use crate::tree::NodeKind;
use crate::ui::{FontWeight, TextAlign};
use crate::widgets::Widget;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) fn update_text(env: &mut Env<'_>, view: &JObject<'_>, kind: &NodeKind) -> JniResult<()> {
    let Some((content, size, color, weight)) = text_properties(kind) else {
        return Ok(());
    };

    // The off-screen view used for measurement disables font padding. Do the
    // same here so the rendered metrics match the measured ones, otherwise the
    // extra padding pushes descenders out of the frame.
    let ids = jni_handles(env)?;
    ids.set_include_font_padding
        .call_void(env, view, &[JValue::Bool(false)])?;

    let string = jcall!(env, env.new_string(content))?;
    ids.set_text
        .call_void(env, view, &[JValue::Object(string.as_ref())])?;
    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(size)],
    )?;
    ids.set_letter_spacing
        .call_void(env, view, &[JValue::Float(0.0)])?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(color as jint)])?;

    apply_typeface(env, view, matches!(weight, FontWeight::Bold))?;
    apply_text_limits(env, view, text_limits(kind))?;

    if let NodeKind::Widget(Widget::Text(props)) = kind {
        let gravity = match props.align {
            TextAlign::Start => GRAVITY_START,
            TextAlign::Center => GRAVITY_CENTER_HORIZONTAL,
            TextAlign::End => GRAVITY_END,
        };
        ids.set_gravity
            .call_void(env, view, &[JValue::Int(gravity)])?;
    }

    Ok(())
}

/// The line limit and truncation of a text-bearing node.
pub(crate) fn text_limits(kind: &NodeKind) -> (Option<u32>, bool) {
    match kind {
        NodeKind::Widget(Widget::Text(props)) => (props.max_lines, props.ellipsis),
        _ => (None, false),
    }
}

pub(crate) fn apply_text_limits(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    limits: (Option<u32>, bool),
) -> JniResult<()> {
    let (max_lines, ellipsis) = limits;
    let max_lines = max_lines.map_or(jint::MAX, |lines| {
        lines.max(1).min(jint::MAX as u32) as jint
    });
    let ids = jni_handles(env)?;
    ids.set_max_lines
        .call_void(env, view, &[JValue::Int(max_lines)])?;

    let truncate_at = if ellipsis {
        ids.truncate_at_end.get(env, &ids.truncate_at)?.l()?
    } else {
        JObject::null()
    };

    ids.set_ellipsize
        .call_void(env, view, &[JValue::Object(&truncate_at)])?;
    Ok(())
}

pub(crate) fn apply_typeface(env: &mut Env<'_>, view: &JObject<'_>, bold: bool) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let field = if bold {
        ids.typeface_default_bold
    } else {
        ids.typeface_default
    };
    let typeface = field.get(env, &ids.typeface)?.l()?;
    ids.set_typeface
        .call_void(env, view, &[JValue::Object(&typeface)])?;
    Ok(())
}

pub(crate) fn text_properties(kind: &NodeKind) -> Option<(&str, f32, u32, FontWeight)> {
    match kind {
        NodeKind::Widget(Widget::Text(props)) => {
            Some((&props.content, props.size, props.color, props.weight))
        }
        NodeKind::Widget(Widget::Button(props)) => {
            Some((&props.label, props.size, props.color, props.weight))
        }
        _ => None,
    }
}
