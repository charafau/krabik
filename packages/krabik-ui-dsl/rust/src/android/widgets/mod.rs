//! Per-widget configuration. A new view and an updated view both go through [`configure_view`].

use crate::android::handles::get as jni_handles;
use crate::android::rows::update_recycler;
use crate::android::style::{apply_accessibility, apply_padding, apply_style};
use crate::android::widgets::check::update_check;
use crate::android::widgets::date_picker::update_date_picker;
use crate::android::widgets::image::update_image;
use crate::android::widgets::pager::update_pager;
use crate::android::widgets::progress::update_progress;
use crate::android::widgets::radio_group::update_radio_group;
use crate::android::widgets::slider::update_slider;
use crate::android::widgets::switch::update_switch;
use crate::android::widgets::tab_bar::update_tab_bar;
use crate::android::widgets::text::update_text;
use crate::android::widgets::text_field::update_text_field;
use crate::render::RenderNode;
use crate::tree::NodeKind;
use crate::widgets::Widget;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, JValue};

pub(crate) mod button;
pub(crate) mod check;
pub(crate) mod date_picker;
pub(crate) mod image;
pub(crate) mod pager;
pub(crate) mod progress;
pub(crate) mod radio_group;
pub(crate) mod slider;
pub(crate) mod switch;
pub(crate) mod tab_bar;
pub(crate) mod text;
pub(crate) mod text_field;

pub(crate) fn update_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    node: &RenderNode,
    density: f32,
) -> JniResult<()> {
    configure_view(env, view, &node.kind, density)?;
    apply_padding(env, view, node.style.padding, density)?;
    apply_style(env, view, node, true)?;
    apply_accessibility(env, view, node)
}

/// Applies a node's own properties to its native view.
///
/// A new view and an updated view go through this same function, and the
/// match is exhaustive, so a widget cannot be configured on creation and then
/// ignore later changes.
pub(crate) fn configure_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    kind: &NodeKind,
    density: f32,
) -> JniResult<()> {
    match kind {
        NodeKind::Root
        | NodeKind::Column
        | NodeKind::Row
        | NodeKind::Spacer
        | NodeKind::Scroll(_) => Ok(()),
        NodeKind::Widget(Widget::Text(_)) | NodeKind::Widget(Widget::Button(_)) => {
            update_text(env, view, kind)
        }
        NodeKind::Widget(Widget::TextField(props)) => update_text_field(env, view, props),
        NodeKind::Widget(Widget::Switch(props)) => update_switch(env, view, props),
        NodeKind::Widget(Widget::Checkbox(props)) => update_check(env, view, props, false),
        NodeKind::Widget(Widget::RadioButton(props)) => update_check(env, view, props, true),
        NodeKind::Widget(Widget::RadioGroup(props)) => update_radio_group(env, view, props),
        NodeKind::Widget(Widget::TabBar(props)) => update_tab_bar(env, view, props),
        NodeKind::Pager(props) => update_pager(env, view, props),
        NodeKind::Widget(Widget::DatePicker(props)) => update_date_picker(env, view, props),
        NodeKind::Widget(Widget::Slider(props)) => update_slider(env, view, props),
        NodeKind::Widget(Widget::Progress(props)) => update_progress(env, view, props),
        NodeKind::Widget(Widget::Spinner(props)) => {
            let ids = jni_handles(env)?;
            ids.update_spinner.call_void(
                env,
                &ids.krabik_views,
                &[JValue::Object(view), JValue::Int(props.color as jint)],
            )
        }
        NodeKind::Widget(Widget::Image(props)) => update_image(env, view, props),
        NodeKind::Recycler(props) => update_recycler(env, view, props, density),
    }
}
