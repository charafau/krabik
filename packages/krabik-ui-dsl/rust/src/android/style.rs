//! Padding, decoration, accessibility and frames shared by every view.

use crate::android::handles::get as jni_handles;
use crate::frame_batch::FrameBatch;
use crate::geometry::{Rect, Size};
use crate::render::RenderNode;
use crate::tree::NodeKind;
use crate::ui::{Border, EdgeInsets, Gradient, GradientDirection, dim_color};
use crate::widgets::Widget;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::{jint, jlong};
use jni::{Env, JValue};

pub(crate) fn apply_padding(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    padding: EdgeInsets,
    density: f32,
) -> JniResult<()> {
    let px = |points: f32| (points * density).round() as jint;
    jni_handles(env)?.set_padding.call_void(
        env,
        view,
        &[
            JValue::Int(px(padding.left)),
            JValue::Int(px(padding.top)),
            JValue::Int(px(padding.right)),
            JValue::Int(px(padding.bottom)),
        ],
    )?;
    Ok(())
}

pub(crate) fn apply_style(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    node: &RenderNode,
    animate: bool,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    // Native text and controls dim themselves; only the decoration the DSL
    // paints is dimmed here.
    let enabled = node.style.enabled.unwrap_or(true);
    let background = node.style.background.map(|value| dim_color(value, enabled));
    let gradient = node.style.gradient.map(|gradient| Gradient {
        start: dim_color(gradient.start, enabled),
        end: dim_color(gradient.end, enabled),
        ..gradient
    });
    let radii = node.style.corner_radii();
    let border = node.style.border.map(|border| Border {
        color: dim_color(border.color, enabled),
        ..border
    });
    let elevation = if enabled { node.style.elevation } else { 0.0 };
    let ripple = matches!(node.kind, NodeKind::Widget(Widget::Button(_)));
    let animation = animate
        .then_some(node.style.animation.as_ref())
        .flatten()
        .filter(|animation| animation.duration_ms > 0);
    let [curve_kind, p1, p2, p3, p4] = animation.map_or([0; 5], |a| a.curve.packed());

    ids.apply_style.call_void(
        env,
        &ids.krabik_style,
        &[
            JValue::Object(view),
            JValue::Int(background.unwrap_or(0) as jint),
            JValue::Bool(background.is_some()),
            JValue::Int(gradient.map_or(0, |gradient| gradient.start) as jint),
            JValue::Int(gradient.map_or(0, |gradient| gradient.end) as jint),
            JValue::Bool(gradient.is_some()),
            JValue::Bool(
                gradient
                    .is_some_and(|gradient| gradient.direction == GradientDirection::Horizontal),
            ),
            JValue::Float(node.style.corner_radius),
            JValue::Float(radii.top_left),
            JValue::Float(radii.top_right),
            JValue::Float(radii.bottom_right),
            JValue::Float(radii.bottom_left),
            JValue::Bool(node.style.corner_radii.is_some()),
            JValue::Float(border.map_or(0.0, |border| border.width)),
            JValue::Int(border.map_or(0, |border| border.color) as jint),
            JValue::Bool(border.is_some()),
            JValue::Float(node.style.opacity.unwrap_or(1.0)),
            JValue::Bool(ripple),
            JValue::Float(elevation),
            JValue::Int(animation.map_or(0, |animation| {
                animation.duration_ms.min(i32::MAX as u32) as jint
            })),
            JValue::Int(curve_kind),
            JValue::Int(p1),
            JValue::Int(p2),
            JValue::Int(p3),
            JValue::Int(p4),
        ],
    )?;
    Ok(())
}

/// Applies interaction state and accessibility metadata to a native view.
pub(crate) fn apply_accessibility(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    node: &RenderNode,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    // Always set the flag so removing `.enabled(false)` restores the view.
    ids.set_enabled.call_void(
        env,
        view,
        &[JValue::Bool(node.style.enabled.unwrap_or(true))],
    )?;

    match &node.style.accessibility_label {
        Some(label) => {
            let label = jcall!(env, env.new_string(label))?;
            ids.set_content_description
                .call_void(env, view, &[JValue::Object(label.as_ref())])?;
        }
        None => {
            ids.set_content_description.call_void(
                env,
                view,
                &[JValue::Object(&JObject::null())],
            )?;
        }
    }
    Ok(())
}

pub(crate) fn set_tag(env: &mut Env<'_>, view: &JObject<'_>, id: jlong) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let tag = ids
        .long_value_of
        .call(env, &ids.long, &[JValue::Long(id)])?
        .l()?;

    ids.set_tag.call_void(env, view, &[JValue::Object(&tag)])?;
    Ok(())
}

/// Applies one node's frame with a single call into `RustRootView`.
pub(crate) fn apply_frame(
    env: &mut Env<'_>,
    root: &JObject<'_>,
    view: &JObject<'_>,
    frame: Rect,
    density: f32,
) -> JniResult<()> {
    let pixels = frame.to_pixels(density);
    jni_handles(env)?.apply_frame.call_void(
        env,
        root,
        &[
            JValue::Object(view),
            JValue::Int(pixels.x),
            JValue::Int(pixels.y),
            JValue::Int(pixels.width),
            JValue::Int(pixels.height),
        ],
    )
}

/// Applies a batch of node frames with one call into `RustRootView`.
pub(crate) fn apply_layouts(
    env: &mut Env<'_>,
    root: &JObject<'_>,
    frames: &FrameBatch,
) -> JniResult<()> {
    let node_ids = env.new_long_array(frames.len())?;
    node_ids.set_region(env, 0, frames.ids())?;
    let values = env.new_int_array(frames.frames().len())?;
    values.set_region(env, 0, frames.frames())?;
    let animations = env.new_int_array(frames.animations().len())?;
    animations.set_region(env, 0, frames.animations())?;

    jni_handles(env)?.apply_layouts.call_void(
        env,
        root,
        &[
            JValue::Object(&node_ids),
            JValue::Object(&values),
            JValue::Object(&animations),
        ],
    )
}

/// Gives the view exact pixel dimensions without moving it.
pub(crate) fn apply_size(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    width: f32,
    height: f32,
    density: f32,
) -> JniResult<()> {
    let width = (width * density).round().max(0.0) as jint;
    let height = (height * density).round().max(0.0) as jint;

    let ids = jni_handles(env)?;
    let params = ids.frame_params_ctor.construct(
        env,
        &ids.frame_params,
        &[JValue::Int(width), JValue::Int(height)],
    )?;

    ids.set_layout_params
        .call_void(env, view, &[JValue::Object(&params)])?;
    Ok(())
}

/// Sizes the scrollable content host of a scroll container.
///
/// A platform scroll view measures its direct child with an unspecified
/// constraint and ignores the child's layout params, so the content extent is
/// communicated through the minimum size as well.
pub(crate) fn apply_content_size(
    env: &mut Env<'_>,
    host: &JObject<'_>,
    size: Size,
    density: f32,
) -> JniResult<()> {
    apply_size(env, host, size.width, size.height, density)?;

    let width = (size.width * density).round().max(0.0) as jint;
    let height = (size.height * density).round().max(0.0) as jint;
    let ids = jni_handles(env)?;
    ids.set_minimum_width
        .call_void(env, host, &[JValue::Int(width)])?;
    ids.set_minimum_height
        .call_void(env, host, &[JValue::Int(height)])?;
    Ok(())
}
