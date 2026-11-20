//! Per-widget configuration. A new view and an updated view both go through [`configure_view`].

use crate::android::handles::Ctor;
use crate::android::host::set_refreshing;
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
use crate::tree::{NodeId, NodeKind};
use crate::widgets::Widget;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::sys::jlong;
use jni::{Env, JValue};

pub(crate) mod button;
pub(crate) mod check;
pub(crate) mod container;
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
        NodeKind::Root | NodeKind::Column | NodeKind::Row | NodeKind::Spacer | NodeKind::Stack => {
            Ok(())
        }
        NodeKind::Scroll(props) => set_refreshing(env, view, &props.refresh),
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
        NodeKind::Container(props) => container::update_container(env, view, props),
        NodeKind::Widget(Widget::DatePicker(props)) => update_date_picker(env, view, props),
        NodeKind::Widget(Widget::Slider(props)) => update_slider(env, view, props),
        NodeKind::Widget(Widget::Progress(props)) => update_progress(env, view, props),
        NodeKind::Widget(Widget::Spinner(props)) => progress::update_spinner(env, view, props),
        NodeKind::Widget(Widget::Image(props)) => update_image(env, view, props),
        NodeKind::Recycler(props) => {
            update_recycler(env, view, props, density)?;
            set_refreshing(env, view, &props.refresh)
        }
    }
}

/// Where a new native view is being created.
pub(crate) struct Site<'a> {
    /// The Android context views are constructed with.
    pub(crate) context: &'a JObject<'a>,
    /// The root view, which framework widgets report their events to.
    pub(crate) root: &'a JObject<'a>,
    /// The node the view is created for.
    pub(crate) node: NodeId,
}

impl Site<'_> {
    /// Constructs a stock Android view: `View(Context)`.
    pub(crate) fn stock<'local>(
        &self,
        env: &mut Env<'local>,
        class: &Global<JClass<'static>>,
        ctor: &Ctor,
    ) -> JniResult<JObject<'local>> {
        ctor.construct(env, class, &[JValue::Object(self.context)])
    }

    /// Constructs a framework widget: `Widget(Context, RustRootView, nodeId)`.
    ///
    /// The widget keeps the root view and its node id, and reports its own
    /// events through them.
    pub(crate) fn framework<'local>(
        &self,
        env: &mut Env<'local>,
        class: &Global<JClass<'static>>,
        ctor: &Ctor,
    ) -> JniResult<JObject<'local>> {
        ctor.construct(
            env,
            class,
            &[
                JValue::Object(self.context),
                JValue::Object(self.root),
                JValue::Long(self.node.0 as jlong),
            ],
        )
    }
}

/// Creates the native view of a widget. The view is configured afterwards by
/// [`configure_view`], the same call an update makes.
pub(crate) fn create_widget<'local>(
    env: &mut Env<'local>,
    site: &Site<'_>,
    widget: &Widget,
) -> JniResult<JObject<'local>> {
    match widget {
        Widget::Text(_) => text::create(env, site),
        Widget::Button(_) => button::create(env, site),
        Widget::TextField(_) => text_field::create(env, site),
        Widget::Switch(_) => switch::create(env, site),
        Widget::Checkbox(_) => check::create(env, site, false),
        Widget::RadioButton(_) => check::create(env, site, true),
        Widget::RadioGroup(_) => radio_group::create(env, site),
        Widget::TabBar(_) => tab_bar::create(env, site),
        Widget::DatePicker(_) => date_picker::create(env, site),
        Widget::Slider(_) => slider::create(env, site),
        Widget::Progress(_) => progress::create(env, site),
        Widget::Spinner(_) => progress::create_spinner(env, site),
        Widget::Image(_) => image::create(env, site),
    }
}
