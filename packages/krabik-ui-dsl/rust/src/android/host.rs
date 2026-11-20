//! One native root view: applying engine operations to Android views.

use crate::android::handles::get as jni_handles;
use crate::android::measure::AndroidMeasure;
use crate::android::rows::create_recycler;
use crate::android::style::{
    apply_accessibility, apply_content_size, apply_frame, apply_layouts, apply_padding,
    apply_style, set_tag,
};
use crate::android::widgets::{Site, configure_view, container, create_widget, pager, update_view};
use crate::frame_batch::FrameBatch;
use crate::geometry::{Rect, Size};
use crate::host::HostKind;
use crate::host::rows::RowBinder;
use crate::render::{Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{NodeId, NodeKind};
use crate::ui::Axis;
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::refs::Global;
use jni::sys::{jint, jlong};
use jni::{Env, JValue};
use std::collections::HashMap;

/// The native resources owned by one renderable node.
pub(crate) struct ViewHandle {
    /// The view that draws the node itself.
    pub(crate) view: Global<JObject<'static>>,
    /// The view that hosts the node's native children.
    ///
    /// For leaves this is the view itself; for containers it is the content
    /// host that child views are added to.
    pub(crate) children_host: Global<JObject<'static>>,
    /// The nearest renderable ancestor the view lives under.
    pub(crate) parent: Option<NodeId>,
    /// The last relative frame applied to the view.
    pub(crate) frame: Option<Rect>,
}

/// The state of one root view in a session.
pub(crate) struct AndroidState {
    pub(crate) runtime: Runtime,
    pub(crate) kind: HostKind,
    pub(crate) root: Global<JObject<'static>>,
    pub(crate) context: Global<JObject<'static>>,
    pub(crate) views: HashMap<NodeId, ViewHandle>,
    /// The rows bound to recycled-list holders, by (list, holder id), with
    /// their handlers.
    pub(crate) rows: RowBinder<(NodeId, i64), ViewHandle>,
    pub(crate) density: f32,
    pub(crate) viewport: Size,
}

impl AndroidState {
    pub(crate) fn new(
        env: &mut Env<'_>,
        context: &JObject<'_>,
        root: &JObject<'_>,
        density: f32,
        kind: HostKind,
    ) -> JniResult<Self> {
        let vm = env.get_java_vm()?;
        let context_ref = env.new_global_ref(context)?;
        let measure = AndroidMeasure::new(env, &vm, context, density)?;

        Ok(Self {
            runtime: Runtime::new(Box::new(measure)),
            kind,
            root: env.new_global_ref(root)?,
            context: context_ref,
            views: HashMap::new(),
            rows: RowBinder::default(),
            density,
            viewport: Size::ZERO,
        })
    }

    pub(crate) fn render(
        &mut self,
        env: &mut Env<'_>,
        element: &crate::ui::RawElement,
    ) -> JniResult<()> {
        let update = self.runtime.update(element, self.viewport);

        // The main views are taken out while the operations run so the same
        // application routine can also update a recycled row's private view
        // set without borrowing the state twice.
        let mut views = std::mem::take(&mut self.views);
        let mut frames = FrameBatch::new();
        let mut result = Ok(());
        for op in update.ops {
            if let Err(error) = self.apply_op(
                env,
                op,
                &mut views,
                self.root.as_obj(),
                true,
                Some(&mut frames),
            ) {
                result = Err(error);
                break;
            }
        }
        self.views = views;
        result?;

        if !frames.is_empty() {
            apply_layouts(env, self.root.as_obj(), &frames)?;
        }
        self.prune_row_actions();
        Ok(())
    }

    pub(crate) fn create_and_insert(
        &self,
        env: &mut Env<'_>,
        node: crate::render::RenderNode,
        register: bool,
        views: &mut HashMap<NodeId, ViewHandle>,
        root_host: &JObject<'_>,
    ) -> JniResult<()> {
        let view = self.create_view(env, &node, register)?;
        if view.is_null() {
            return Ok(());
        }

        {
            let parent_host = match node.parent.and_then(|id| views.get(&id)) {
                Some(handle) => handle.children_host.as_obj(),
                None => root_host,
            };
            jni_handles(env)?
                .add_view
                .call_void(env, parent_host, &[JValue::Object(&view)])?;
        }

        let children_host = if matches!(node.kind, NodeKind::Scroll(_)) {
            let ids = jni_handles(env)?;
            let host = ids
                .content_host
                .call(env, &ids.krabik_views, &[JValue::Object(&view)])?
                .l()?;
            jcall!(env, env.new_global_ref(&host))?
        } else {
            jcall!(env, env.new_global_ref(&view))?
        };
        views.insert(
            node.id,
            ViewHandle {
                view: jcall!(env, env.new_global_ref(&view))?,
                children_host,
                parent: node.parent,
                frame: None,
            },
        );
        Ok(())
    }

    /// Applies one renderer operation to a set of views.
    ///
    /// The main tree and a recycled row share this code; they only differ in
    /// the host that receives top-level views and in whether the views are
    /// registered for batches of frames.
    pub(crate) fn apply_op(
        &self,
        env: &mut Env<'_>,
        op: Op,
        views: &mut HashMap<NodeId, ViewHandle>,
        root_host: &JObject<'_>,
        register: bool,
        mut frames: Option<&mut FrameBatch>,
    ) -> JniResult<()> {
        match op {
            Op::Create(node) => {
                self.create_and_insert(env, node, register, views, root_host)?;
            }
            Op::Update(node) => {
                if let Some(handle) = views.get(&node.id) {
                    update_view(env, handle.view.as_obj(), &node, self.density)?;
                } else {
                    // A view can be missing after a screen or host swap. Rebuild
                    // it rather than leaving the node without a native widget.
                    self.create_and_insert(env, node, register, views, root_host)?;
                }
            }
            Op::Reparent { id, parent } => {
                let Some(handle) = views.get(&id) else {
                    return Ok(());
                };

                let old_parent = handle.parent;
                if old_parent != parent {
                    let view = handle.view.as_obj();
                    let old_host = match old_parent.and_then(|id| views.get(&id)) {
                        Some(handle) => handle.children_host.as_obj(),
                        None => root_host,
                    };
                    let new_host = match parent.and_then(|id| views.get(&id)) {
                        Some(handle) => handle.children_host.as_obj(),
                        None => root_host,
                    };

                    let ids = jni_handles(env)?;
                    ids.remove_view
                        .call_void(env, old_host, &[JValue::Object(view)])?;
                    ids.add_view
                        .call_void(env, new_host, &[JValue::Object(view)])?;
                }

                let frame = handle.frame;
                if let Some(frame) = frame {
                    // `applyFrame` is a method of the Rust root view; the host
                    // only decides which group the widget is attached to.
                    apply_frame(
                        env,
                        self.root.as_obj(),
                        handle.view.as_obj(),
                        frame,
                        self.density,
                    )?;
                }
                if let Some(handle) = views.get_mut(&id) {
                    handle.parent = parent;
                }
            }
            Op::Remove { id, exit } => {
                let (animated, view) = match views.get(&id) {
                    Some(handle) => {
                        let view = handle.view.as_obj();
                        let animated =
                            register && exit.as_ref().is_some_and(|exit| exit.duration_ms > 0);
                        (animated, Some(jcall!(env, env.new_local_ref(view))?))
                    }
                    None => (false, None),
                };
                if let Some(view) = view {
                    if animated {
                        let exit = exit.as_ref().expect("checked above");
                        let [kind, p1, p2, p3, p4] = exit.curve.packed();
                        jni_handles(env)?.start_exit.call_void(
                            env,
                            self.root.as_obj(),
                            &[
                                JValue::Long(id.0 as jlong),
                                JValue::Object(&view),
                                JValue::Int(exit.duration_ms.min(i32::MAX as u32) as jint),
                                JValue::Int(kind),
                                JValue::Int(p1),
                                JValue::Int(p2),
                                JValue::Int(p3),
                                JValue::Int(p4),
                            ],
                        )?;
                    } else {
                        let host = match views.get(&id).and_then(|handle| handle.parent) {
                            Some(parent) => match views.get(&parent) {
                                Some(parent) => parent.children_host.as_obj(),
                                None => root_host,
                            },
                            None => root_host,
                        };
                        jni_handles(env)?.remove_view.call_void(
                            env,
                            host,
                            &[JValue::Object(&view)],
                        )?;
                        if register {
                            jni_handles(env)?.unregister_view.call_void(
                                env,
                                self.root.as_obj(),
                                &[JValue::Long(id.0 as jlong)],
                            )?;
                        }
                    }
                }
                views.remove(&id);
            }
            Op::ScrollTo { id, offset } => {
                if let Some(handle) = views.get(&id) {
                    // Lists scroll vertically.
                    let pixels = (offset * self.density).round() as jint;
                    jni_handles(env)?.scroll_to.call_void(
                        env,
                        handle.view.as_obj(),
                        &[JValue::Int(0), JValue::Int(pixels)],
                    )?;
                }
            }
            Op::Layout {
                id,
                frame,
                content_size,
                animation,
            } => {
                if let Some(handle) = views.get(&id) {
                    if let Some(batch) = frames.as_mut() {
                        batch.push(id, frame, self.density, animation.as_ref());
                    } else {
                        apply_frame(
                            env,
                            self.root.as_obj(),
                            handle.view.as_obj(),
                            frame,
                            self.density,
                        )?;
                    }
                    if let Some(size) = content_size {
                        apply_content_size(env, handle.children_host.as_obj(), size, self.density)?;
                    }
                }
                if let Some(handle) = views.get_mut(&id) {
                    handle.frame = Some(frame);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn create_view<'local>(
        &self,
        env: &mut Env<'local>,
        node: &RenderNode,
        register: bool,
    ) -> JniResult<JObject<'local>> {
        let site = Site {
            context: self.context.as_obj(),
            root: self.root.as_obj(),
            node: node.id,
        };
        let view = match &node.kind {
            NodeKind::Widget(widget) => create_widget(env, &site, widget)?,
            NodeKind::Pager(_) => pager::create(env, &site)?,
            NodeKind::Container(_) => container::create(env, &site)?,
            NodeKind::Scroll(props) => create_scroll(
                env,
                site.context,
                site.root,
                props.axis == Axis::Horizontal,
                props.refresh.is_enabled(),
            )?,
            NodeKind::Recycler(props) => {
                create_recycler(env, site.context, site.root, node.id, props, self.density)?
            }
            // Decorated layout containers become real containers so they can
            // draw behind their children and clip them to rounded corners.
            NodeKind::Column | NodeKind::Row | NodeKind::Stack => {
                let ids = jni_handles(env)?;
                site.stock(env, &ids.frame_layout, &ids.frame_layout_ctor)?
            }
            NodeKind::Spacer => {
                let ids = jni_handles(env)?;
                site.stock(env, &ids.view, &ids.view_ctor)?
            }
            // The root is the host view itself.
            NodeKind::Root => return Ok(JObject::null()),
        };

        configure_view(env, &view, &node.kind, self.density)?;
        apply_padding(env, &view, node.style.padding, self.density)?;
        apply_style(env, &view, node, false)?;
        apply_accessibility(env, &view, node)?;
        set_tag(env, &view, node.id.0 as jlong)?;
        if register {
            jni_handles(env)?.register_view.call_void(
                env,
                self.root.as_obj(),
                &[JValue::Long(node.id.0 as jlong), JValue::Object(&view)],
            )?;
            if let Some(enter) = node
                .style
                .enter
                .as_ref()
                .filter(|enter| enter.duration_ms > 0)
            {
                let [kind, p1, p2, p3, p4] = enter.curve.packed();
                jni_handles(env)?.start_enter.call_void(
                    env,
                    self.root.as_obj(),
                    &[
                        JValue::Object(&view),
                        JValue::Float(node.style.opacity.unwrap_or(1.0)),
                        JValue::Int(enter.duration_ms.min(i32::MAX as u32) as jint),
                        JValue::Int(kind),
                        JValue::Int(p1),
                        JValue::Int(p2),
                        JValue::Int(p3),
                        JValue::Int(p4),
                    ],
                )?;
            }
        }

        Ok(view)
    }
}

pub(crate) fn create_scroll<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    root: &JObject<'_>,
    horizontal: bool,
    refreshable: bool,
) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    ids.create_scroll
        .call(
            env,
            &ids.krabik_views,
            &[
                JValue::Object(context),
                JValue::Bool(horizontal),
                JValue::Object(root),
                JValue::Bool(refreshable),
            ],
        )?
        .l()
}

/// Shows or hides the pull-to-refresh indicator of a refreshable container.
pub(crate) fn set_refreshing(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    refresh: &crate::ui::Refresh,
) -> JniResult<()> {
    if !refresh.is_enabled() {
        return Ok(());
    }
    let ids = jni_handles(env)?;
    ids.set_refreshing.call_void(
        env,
        &ids.krabik_views,
        &[JValue::Object(view), JValue::Bool(refresh.refreshing)],
    )
}
