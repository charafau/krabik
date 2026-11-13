//! Android renderer driven directly through JNI.
//!
//! The Kotlin side is a thin `ViewGroup`: it forwards its size to Rust and
//! forwards clicks back. Every widget, every property update, and every frame
//! change is performed here on native `TextView`/`Button` instances.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard};

use jni::errors::{Result as JniResult, ThrowRuntimeExAndDefault};
use jni::objects::{JObject, JString};
use jni::refs::Global;
use jni::strings::JNIStr;
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::{Env, EnvUnowned, JValue, JavaVM, jni_sig, jni_str};

use crate::frame_batch::FrameBatch;
use crate::geometry::{Constraints, Rect, Size};
use crate::host::{Actions, Controller, Host, HostKind, NodeEvent, Platform, Session};
use crate::layout::LayoutResult;
use crate::measure::{DEFAULT_SWITCH_HEIGHT, DEFAULT_SWITCH_WIDTH, Measure};
use crate::navigation::{Screen, ScreenTransition};
use crate::presentation::Dialog;
use crate::render::{self, Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{self, NodeId, NodeKind, RecyclerProps, UiNode};
use crate::ui::{
    Axis, Border, CheckProps, DatePickerMode, DatePickerProps, DatePickerValue, DateValue,
    EdgeInsets, FontWeight, Gradient, GradientDirection, ImageProps, ImageScale, ImageSource,
    ImeAction, KeyboardType, PagerProps, ProgressProps, RadioGroupProps, SliderProps, SwitchProps,
    TabBarProps, TextAlign, TextFieldProps, TextProps, TimeValue, dim_color, snap_slider_value,
};
use crate::widgets::Widget;

mod handles;

use self::handles::get as jni_handles;

const TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/TextView");
const BUTTON_CLASS: &JNIStr = jni_str!("android/widget/Button");
const FRAME_LAYOUT_CLASS: &JNIStr = jni_str!("android/widget/FrameLayout");
const IMAGE_VIEW_CLASS: &JNIStr = jni_str!("android/widget/ImageView");
const VIEW_CLASS: &JNIStr = jni_str!("android/view/View");
const TYPEFACE_CLASS: &JNIStr = jni_str!("android/graphics/Typeface");
const TEXT_UTILS_CLASS: &JNIStr = jni_str!("android/text/TextUtils$TruncateAt");
const LONG_CLASS: &JNIStr = jni_str!("java/lang/Long");
const MEASURE_SPEC_CLASS: &JNIStr = jni_str!("android/view/View$MeasureSpec");
const LAYOUT_PARAMS_CLASS: &JNIStr = jni_str!("android/view/ViewGroup$LayoutParams");
/// ScrollView casts its direct child's params to `MarginLayoutParams`, so every
/// sized view uses `FrameLayout.LayoutParams`, which is a safe superset.
const FRAME_PARAMS_CLASS: &JNIStr = jni_str!("android/widget/FrameLayout$LayoutParams");
const KRABIK_STYLE_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikStyle");
const RUST_ROOT_VIEW_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/RustRootView");
const TEXT_INPUT_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikTextInput");
const SWITCH_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikSwitch");
/// The framework switch, measured only for its intrinsic size.
const PLAIN_SWITCH_CLASS: &JNIStr = jni_str!("android/widget/Switch");
const CHECKBOX_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikCheckBox");
const RADIO_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikRadioButton");
const RADIO_GROUP_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikRadioGroup");
const TAB_BAR_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikTabBar");
const PAGER_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikPager");
const DATE_PICKER_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikDatePicker");
const SEEK_BAR_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikSeekBar");
const PROGRESS_BAR_CLASS: &JNIStr = jni_str!("android/widget/ProgressBar");
const KRABIK_IMAGES_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikImages");
const KRABIK_ROWS_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikRows");
const KRABIK_VIEWS_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikViews");
const KRABIK_RECYCLERS_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikRecyclers");

const MEASURE_UNSPECIFIED: jint = 0;
const MEASURE_AT_MOST: jint = -2_147_483_648;
const WRAP_CONTENT: jint = -2;

/// The unit expected by `TextView.setTextSize` for scaled pixels.
const COMPLEX_UNIT_SP: jint = 2;
/// `android.view.Gravity.CENTER`.
const GRAVITY_CENTER: jint = 17;
/// `android.view.Gravity.CENTER_HORIZONTAL`.
const GRAVITY_CENTER_HORIZONTAL: jint = 1;
/// `android.view.Gravity.START`.
const GRAVITY_START: jint = 0x0080_0003;
/// `android.view.Gravity.END`.
const GRAVITY_END: jint = 0x0080_0005;

/// Calls a JNI method and turns a pending Java exception into a Rust error.
macro_rules! jcall {
    ($env:expr, $call:expr) => {{
        let value = $call;
        checked($env, value)
    }};
}

fn checked<T>(env: &mut Env<'_>, result: JniResult<T>) -> JniResult<T> {
    let value = result?;
    env.exception_catch()?;
    Ok(value)
}

/// Executes a native operation and never lets a Rust panic reach the JVM.
fn run<'local, T, F>(unenv: &mut EnvUnowned<'local>, operation: &str, body: F) -> T
where
    T: Default,
    F: FnOnce(&mut Env<'local>) -> JniResult<T>,
{
    unenv
        .with_env(|env| -> JniResult<T> {
            match body(env) {
                Ok(value) => Ok(value),
                Err(error) => {
                    if env.exception_check() {
                        env.exception_describe();
                        env.exception_clear();
                    }
                    eprintln!("{operation} failed: {error}");
                    Ok(T::default())
                }
            }
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

/// Measures text with an off-screen native `TextView`.
///
/// Results are cached because Taffy asks for the same intrinsic sizes several
/// times per layout pass.
struct AndroidMeasure {
    vm: JavaVM,
    context: Global<JObject<'static>>,
    text_view: Global<JObject<'static>>,
    density: f32,
    cache: Mutex<HashMap<MeasureKey, Size>>,
    switch_size: Mutex<Option<Size>>,
    image_sizes: Mutex<HashMap<String, Option<Size>>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct MeasureKey {
    content: String,
    size: u32,
    bold: bool,
    max_width: Option<u32>,
    max_lines: Option<u32>,
    ellipsis: bool,
}

impl AndroidMeasure {
    fn new(env: &mut Env<'_>, vm: &JavaVM, context: &JObject<'_>, density: f32) -> JniResult<Self> {
        let ids = jni_handles(env)?;
        let view = ids
            .text_view_ctor
            .construct(env, &ids.text_view, &[JValue::Object(context)])?;

        ids.set_include_font_padding
            .call_void(env, &view, &[JValue::Bool(false)])?;
        ids.set_padding.call_void(
            env,
            &view,
            &[
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
            ],
        )?;

        // `TextView.setText` consults the layout params while it decides
        // whether to request another layout pass, even for a detached view.
        let params = ids.layout_params_ctor.construct(
            env,
            &ids.layout_params,
            &[JValue::Int(WRAP_CONTENT), JValue::Int(WRAP_CONTENT)],
        )?;
        ids.set_layout_params
            .call_void(env, &view, &[JValue::Object(&params)])?;

        Ok(Self {
            vm: vm.clone(),
            context: jcall!(env, env.new_global_ref(context))?,
            text_view: jcall!(env, env.new_global_ref(&view))?,
            density,
            cache: Mutex::new(HashMap::new()),
            switch_size: Mutex::new(None),
            image_sizes: Mutex::new(HashMap::new()),
        })
    }

    fn measure_with(
        &self,
        env: &mut Env<'_>,
        text: &TextProps,
        constraints: Constraints,
    ) -> JniResult<Size> {
        let view = self.text_view.as_obj();

        let string = jcall!(env, env.new_string(&text.content))?;
        let ids = jni_handles(env)?;
        ids.set_text
            .call_void(env, view, &[JValue::Object(string.as_ref())])?;
        ids.set_text_size.call_void(
            env,
            view,
            &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(text.size)],
        )?;
        let bold = matches!(text.weight, FontWeight::Bold);
        apply_typeface(env, view, bold)?;
        apply_text_limits(env, view, (text.max_lines, text.ellipsis))?;

        let width = match constraints.max_width {
            Some(max_width) => {
                let px = (max_width * self.density).ceil().max(0.0) as jint;
                measure_spec(env, px, MEASURE_AT_MOST)?
            }
            None => measure_spec(env, 0, MEASURE_UNSPECIFIED)?,
        };
        let height = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;

        let ids = jni_handles(env)?;
        ids.measure
            .call_void(env, view, &[JValue::Int(width), JValue::Int(height)])?;

        let measured_width = ids.measured_width.call(env, view, &[])?.i()?;
        let measured_height = ids.measured_height.call(env, view, &[])?.i()?;

        Ok(Size::new(
            measured_width as f32 / self.density,
            measured_height as f32 / self.density,
        ))
    }

    /// Measures a framework switch with wrap-content specs.
    fn measure_switch(&self) -> JniResult<Size> {
        self.vm.attach_current_thread(|env| {
            let ids = jni_handles(env)?;
            let view = ids.plain_switch_ctor.construct(
                env,
                &ids.plain_switch,
                &[JValue::Object(self.context.as_obj())],
            )?;
            let width = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;
            let height = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;
            ids.measure
                .call_void(env, &view, &[JValue::Int(width), JValue::Int(height)])?;
            let width = ids.measured_width.call(env, &view, &[])?.i()?;
            let height = ids.measured_height.call(env, &view, &[])?.i()?;
            Ok(Size::new(
                width as f32 / self.density,
                height as f32 / self.density,
            ))
        })
    }

    /// Reads the header dimensions of a bundled asset.
    fn measure_image(&self, path: &str) -> JniResult<Option<Size>> {
        self.vm.attach_current_thread(|env| {
            let ids = jni_handles(env)?;
            let source = jcall!(env, env.new_string(path))?;
            let packed = ids
                .image_size
                .call(
                    env,
                    &ids.krabik_images,
                    &[
                        JValue::Object(self.context.as_obj()),
                        JValue::Object(source.as_ref()),
                    ],
                )?
                .j()?;
            if packed == 0 {
                return Ok(None);
            }
            let packed = packed as u64;
            let width = (packed >> 32) as u32 as f32 / self.density;
            let height = (packed as u32) as f32 / self.density;
            Ok(Some(Size::new(width, height)))
        })
    }
}

impl Measure for AndroidMeasure {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size {
        let key = MeasureKey {
            content: text.content.clone(),
            size: text.size.to_bits(),
            bold: matches!(text.weight, FontWeight::Bold),
            max_width: constraints.max_width.map(f32::to_bits),
            max_lines: text.max_lines,
            ellipsis: text.ellipsis,
        };

        if let Some(cached) = self
            .cache
            .lock()
            .expect("measurement cache")
            .get(&key)
            .copied()
        {
            return cached;
        }

        let measured = self
            .vm
            .attach_current_thread(|env| self.measure_with(env, text, constraints))
            .unwrap_or_else(|error| {
                eprintln!("text measurement failed: {error}");
                Size::ZERO
            });

        self.cache
            .lock()
            .expect("measurement cache")
            .insert(key, measured);
        measured
    }

    fn switch_size(&self) -> Size {
        if let Some(size) = *self.switch_size.lock().expect("switch size cache") {
            return size;
        }
        let size = self.measure_switch().unwrap_or_else(|error| {
            eprintln!("switch measurement failed: {error}");
            Size::new(DEFAULT_SWITCH_WIDTH, DEFAULT_SWITCH_HEIGHT)
        });
        *self.switch_size.lock().expect("switch size cache") = Some(size);
        size
    }

    fn image_size(&self, props: &ImageProps) -> Option<Size> {
        let ImageSource::Asset(path) = &props.source else {
            return None;
        };
        if let Some(cached) = self
            .image_sizes
            .lock()
            .expect("image size cache")
            .get(path)
            .copied()
        {
            return cached;
        }
        let size = self.measure_image(path).unwrap_or_else(|error| {
            eprintln!("image measurement failed: {error}");
            None
        });
        self.image_sizes
            .lock()
            .expect("image size cache")
            .insert(path.clone(), size);
        size
    }
}

fn measure_spec(env: &mut Env<'_>, size: jint, mode: jint) -> JniResult<jint> {
    let ids = jni_handles(env)?;
    ids.make_measure_spec
        .call(
            env,
            &ids.measure_spec,
            &[JValue::Int(size), JValue::Int(mode)],
        )?
        .i()
}

/// The native resources owned by one renderable node.
struct ViewHandle {
    /// The view that draws the node itself.
    view: Global<JObject<'static>>,
    /// The view that hosts the node's native children.
    ///
    /// For leaves this is the view itself; for containers it is the content
    /// host that child views are added to.
    children_host: Global<JObject<'static>>,
    /// The nearest renderable ancestor the view lives under.
    parent: Option<NodeId>,
    /// The last relative frame applied to the view.
    frame: Option<Rect>,
}

/// One recycled row currently bound to a holder.
///
/// The cached tree and widgets let a rebind diff against what the holder
/// already shows instead of tearing the row down and building it again.
struct RowState {
    /// The item index the holder shows.
    index: usize,
    /// The row subtree of the last bind, with its own root identifier.
    tree: UiNode,
    /// The layout the cached tree was built with.
    layout: LayoutResult,
    /// The row's widgets, keyed by their node ids.
    views: HashMap<NodeId, ViewHandle>,
}

/// The state of one root view in a session.
struct AndroidState {
    runtime: Runtime,
    kind: HostKind,
    root: Global<JObject<'static>>,
    context: Global<JObject<'static>>,
    views: HashMap<NodeId, ViewHandle>,
    /// Actions of the rows currently bound in recycled lists.
    row_actions: HashMap<NodeId, Actions>,
    /// The node ids a (list, row) binding created, for clearing on rebind.
    row_action_ids: HashMap<(NodeId, usize), Vec<NodeId>>,
    /// The rows currently bound to recycled-list holders, by (list, holder).
    row_states: HashMap<(NodeId, i64), RowState>,
    /// The item each row node belongs to, for routing focus back to its row.
    row_nodes: HashMap<NodeId, (NodeId, usize)>,
    density: f32,
    viewport: Size,
}

impl AndroidState {
    fn new(
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
            row_actions: HashMap::new(),
            row_action_ids: HashMap::new(),
            row_states: HashMap::new(),
            row_nodes: HashMap::new(),
            density,
            viewport: Size::ZERO,
        })
    }

    fn render(&mut self, env: &mut Env<'_>, element: &crate::ui::RawElement) -> JniResult<()> {
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

    fn create_and_insert(
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
            let host = jni_handles(env)?
                .get_child_at
                .call(env, &view, &[JValue::Int(0)])?
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
    fn apply_op(
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

    /// Forgets row actions of bindings whose index no longer exists.
    fn prune_row_actions(&mut self) {
        let stale: Vec<((NodeId, usize), Vec<NodeId>)> = self
            .row_action_ids
            .iter()
            .filter(|((list, index), _)| {
                self.runtime
                    .recycled_item_count(*list)
                    .is_none_or(|count| *index >= count)
            })
            .map(|(key, ids)| (*key, ids.clone()))
            .collect();

        for (key, ids) in stale {
            self.row_action_ids.remove(&key);
            for id in ids {
                self.row_actions.remove(&id);
            }
        }

        // A screen switch removes the recycler from the main tree; its cached
        // holders must not keep their widgets alive.
        let detached: Vec<(NodeId, i64)> = self
            .row_states
            .keys()
            .filter(|(list, _)| self.runtime.recycled_item_count(*list).is_none())
            .copied()
            .collect();
        for key in detached {
            if let Some(state) = self.row_states.remove(&key) {
                self.unregister_row_nodes(&state.tree);
            }
        }
    }

    /// Drops the actions and widgets recorded for one row binding.
    fn clear_row_actions(&mut self, list: NodeId, index: usize) {
        if let Some(ids) = self.row_action_ids.remove(&(list, index)) {
            for id in ids {
                self.row_actions.remove(&id);
            }
        }
    }

    fn create_view<'local>(
        &self,
        env: &mut Env<'local>,
        node: &RenderNode,
        register: bool,
    ) -> JniResult<JObject<'local>> {
        let view = if let NodeKind::Scroll(props) = &node.kind {
            create_scroll(
                env,
                self.context.as_obj(),
                self.root.as_obj(),
                props.axis == Axis::Horizontal,
            )?
        } else {
            let ids = jni_handles(env)?;
            let context = [JValue::Object(self.context.as_obj())];
            let child_context = [
                JValue::Object(self.context.as_obj()),
                JValue::Object(self.root.as_obj()),
                JValue::Long(node.id.0 as jlong),
            ];

            match &node.kind {
                NodeKind::Widget(Widget::Text(_)) => {
                    ids.text_view_ctor
                        .construct(env, &ids.text_view, &context)?
                }
                NodeKind::Widget(Widget::Button(_)) => {
                    ids.button_ctor.construct(env, &ids.button, &context)?
                }
                NodeKind::Widget(Widget::TextField(_)) => {
                    ids.text_input_ctor
                        .construct(env, &ids.text_input, &child_context)?
                }
                NodeKind::Widget(Widget::Switch(_)) => {
                    ids.switch_ctor
                        .construct(env, &ids.switch, &child_context)?
                }
                NodeKind::Widget(Widget::Checkbox(_)) => {
                    ids.checkbox_ctor
                        .construct(env, &ids.checkbox, &child_context)?
                }
                NodeKind::Widget(Widget::RadioButton(_)) => {
                    ids.radio_ctor
                        .construct(env, &ids.radio_button, &child_context)?
                }
                NodeKind::Widget(Widget::RadioGroup(_)) => {
                    ids.radio_group_ctor
                        .construct(env, &ids.radio_group, &child_context)?
                }
                NodeKind::Widget(Widget::TabBar(_)) => {
                    ids.tab_bar_ctor
                        .construct(env, &ids.tab_bar, &child_context)?
                }
                NodeKind::Pager(_) => ids.pager_ctor.construct(env, &ids.pager, &child_context)?,
                NodeKind::Widget(Widget::DatePicker(_)) => {
                    ids.date_picker_ctor
                        .construct(env, &ids.date_picker, &child_context)?
                }
                NodeKind::Widget(Widget::Slider(_)) => {
                    ids.seek_bar_ctor
                        .construct(env, &ids.seek_bar, &child_context)?
                }
                NodeKind::Widget(Widget::Progress(_)) => ids
                    .create_progress
                    .call(
                        env,
                        &ids.krabik_views,
                        &[JValue::Object(self.context.as_obj())],
                    )?
                    .l()?,
                NodeKind::Widget(Widget::Spinner(_)) => {
                    ids.progress_bar_ctor
                        .construct(env, &ids.progress_bar, &context)?
                }
                NodeKind::Widget(Widget::Image(_)) => {
                    ids.image_view_ctor
                        .construct(env, &ids.image_view, &context)?
                }
                // Decorated layout containers become real containers so they can
                // draw behind their children and clip them to rounded corners.
                NodeKind::Column | NodeKind::Row => {
                    ids.frame_layout_ctor
                        .construct(env, &ids.frame_layout, &context)?
                }
                NodeKind::Spacer => ids.view_ctor.construct(env, &ids.view, &context)?,
                NodeKind::Recycler(props) => create_recycler(
                    env,
                    self.context.as_obj(),
                    self.root.as_obj(),
                    node.id,
                    props,
                    self.density,
                )?,
                _ => return Ok(JObject::null()),
            }
        };

        configure_view(env, &view, &node.kind, self.density)?;
        if matches!(node.kind, NodeKind::Widget(Widget::Button(_))) {
            configure_button(env, &view)?;
            jni_handles(env)?.set_on_click_listener.call_void(
                env,
                &view,
                &[JValue::Object(self.root.as_obj())],
            )?;
        }
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

    /// Builds one recycled-list row into a holder host.
    ///
    /// Returns the row height in pixels, which a variable-height recycler uses
    /// to size the holder. A rebind of the same item diffs against the cached
    /// tree instead of recreating every widget.
    fn bind_row(
        &mut self,
        env: &mut Env<'_>,
        list: NodeId,
        index: usize,
        holder: i64,
        host: &JObject<'_>,
        width_px: jint,
    ) -> JniResult<jint> {
        let (transition_ms, element) = {
            let Some(source) = self.runtime.row_source(list) else {
                return Ok(0);
            };
            if index >= source.item_count {
                return Ok(0);
            }
            (source.row_transition_ms, (source.rows)(index))
        };

        let root = tree::row_root(list, index);
        let width = width_px.max(1) as f32 / self.density;
        let (tree, layout) =
            self.runtime
                .layout_subtree(&element, root, Size::new(width, 100_000.0));

        let previous = self.row_states.remove(&(list, holder));
        let had_previous = previous.is_some();
        let (old_tree, old_layout, mut views) = if let Some(mut state) = previous {
            self.clear_row_actions(list, state.index);
            if state.index != index
                && let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                // The holder is about to show another item; keep the logical
                // focus so it can be restored when the row comes back.
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            let views = std::mem::take(&mut state.views);
            (state.tree, state.layout, views)
        } else {
            // The holder was recycled without telling us, so its previous
            // widgets are unknown. Diffing against an empty tree creates the
            // whole row and its frames.
            jni_handles(env)?
                .remove_all_views
                .call_void(env, host, &[])?;
            (UiNode::empty(), LayoutResult::default(), HashMap::new())
        };

        let mut ops = render::diff(&old_tree, &tree);
        ops.extend(render::diff_layout(&old_tree, &tree, &old_layout, &layout));

        // The overlay is captured before the new content is inserted, so it
        // shows exactly what this holder displayed a moment ago.
        let overlay = if transition_ms > 0 && had_previous && !ops.is_empty() {
            let ids = jni_handles(env)?;
            let overlay = ids
                .snapshot_row
                .call(env, &ids.krabik_rows, &[JValue::Object(host)])?
                .l()?;
            if overlay.is_null() {
                None
            } else {
                Some(jcall!(env, env.new_global_ref(&overlay))?)
            }
        } else {
            None
        };

        for op in ops {
            self.apply_op(env, op, &mut views, host, false, None)?;
        }

        if let Some(overlay) = overlay {
            let ids = jni_handles(env)?;
            ids.fade_row.call_void(
                env,
                &ids.krabik_rows,
                &[
                    JValue::Object(host),
                    JValue::Object(overlay.as_obj()),
                    JValue::Int(transition_ms.min(i32::MAX as u32) as jint),
                ],
            )?;
        }

        self.record_row_actions(list, index, &tree);
        self.register_row_nodes(list, index, &tree);
        let height = layout
            .frames
            .get(&root)
            .map_or(0.0, |frame| frame.height)
            .max(0.0);
        self.row_states.insert(
            (list, holder),
            RowState {
                index,
                tree,
                layout,
                views,
            },
        );

        Ok((height * self.density).round().max(1.0) as jint)
    }

    /// Releases the widgets and actions of the row a holder currently shows.
    fn unbind_row(&mut self, list: NodeId, holder: i64) {
        if let Some(state) = self.row_states.remove(&(list, holder)) {
            if let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            self.clear_row_actions(list, state.index);
        }
    }

    /// Maps every node of a bound row back to its item.
    fn register_row_nodes(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.insert(node.id, (list, index));
        }
    }

    /// Forgets the nodes of a row that is no longer bound.
    fn unregister_row_nodes(&mut self, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.remove(&node.id);
        }
    }

    fn record_row_actions(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        let mut ids = Vec::new();
        for node in tree::preorder(tree) {
            if let Some(actions) = Actions::of(node) {
                self.row_actions.insert(node.id, actions);
                ids.push(node.id);
            }
        }
        self.row_action_ids.insert((list, index), ids);
    }
}

fn create_scroll<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    root: &JObject<'_>,
    horizontal: bool,
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
            ],
        )?
        .l()
}

fn create_recycler<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    root: &JObject<'_>,
    node_id: NodeId,
    props: &RecyclerProps,
    density: f32,
) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let height = row_height_px(props, density);
    ids.create_recycler
        .call(
            env,
            &ids.krabik_recyclers,
            &[
                JValue::Object(context),
                JValue::Object(root),
                JValue::Long(node_id.0 as jlong),
                JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
                JValue::Int(height),
            ],
        )?
        .l()
}

fn update_recycler(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &RecyclerProps,
    density: f32,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let height = row_height_px(props, density);
    ids.update_recycler.call_void(
        env,
        &ids.krabik_recyclers,
        &[
            JValue::Object(view),
            JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
            JValue::Int(height),
            JValue::Long(props.revision as jlong),
        ],
    )
}

/// The fixed row height in pixels, or `0` to let the recycler wrap content.
fn row_height_px(props: &RecyclerProps, density: f32) -> jint {
    props
        .item_height
        .map_or(0, |height| (height * density).round().max(1.0) as jint)
}

fn update_view(
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
fn configure_view(
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

fn update_image(env: &mut Env<'_>, view: &JObject<'_>, props: &ImageProps) -> JniResult<()> {
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

fn update_text_field(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &TextFieldProps,
) -> JniResult<()> {
    let value = jcall!(env, env.new_string(&props.value))?;
    let ids = jni_handles(env)?;
    ids.set_text_silently
        .call_void(env, view, &[JValue::Object(value.as_ref())])?;

    let hint = jcall!(env, env.new_string(&props.placeholder))?;
    ids.set_hint
        .call_void(env, view, &[JValue::Object(hint.as_ref())])?;

    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(props.size)],
    )?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(props.color as jint)])?;

    let keyboard = match props.keyboard {
        KeyboardType::Text => 0,
        KeyboardType::Email => 1,
        KeyboardType::Number => 2,
        KeyboardType::Phone => 3,
    };
    let ime_action = match props.ime_action {
        ImeAction::Default => 0,
        ImeAction::Done => 1,
        ImeAction::Next => 2,
        ImeAction::Search => 3,
        ImeAction::Send => 4,
    };
    ids.configure_input.call_void(
        env,
        view,
        &[
            JValue::Bool(props.secure),
            JValue::Bool(props.multiline),
            JValue::Int(keyboard),
            JValue::Int(ime_action),
        ],
    )?;
    // `configure` changes the input type and line mode, which can reset the
    // typeface, so the font is applied after the platform configuration.
    apply_typeface(env, view, matches!(props.weight, FontWeight::Bold))?;
    Ok(())
}

fn update_switch(env: &mut Env<'_>, view: &JObject<'_>, props: &SwitchProps) -> JniResult<()> {
    jni_handles(env)?
        .set_checked_silently
        .call_void(env, view, &[JValue::Bool(props.checked)])
}

fn update_check(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &CheckProps,
    radio: bool,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.set_include_font_padding
        .call_void(env, view, &[JValue::Bool(false)])?;
    let label = jcall!(env, env.new_string(&props.label))?;
    ids.set_text
        .call_void(env, view, &[JValue::Object(label.as_ref())])?;
    ids.set_text_size.call_void(
        env,
        view,
        &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(props.size)],
    )?;
    ids.set_text_color
        .call_void(env, view, &[JValue::Int(props.color as jint)])?;
    ids.set_max_lines.call_void(env, view, &[JValue::Int(1)])?;
    let ellipsis = ids.truncate_at_end.get(env, &ids.truncate_at)?;
    ids.set_ellipsize
        .call_void(env, view, &[JValue::Object(ellipsis.l()?.as_ref())])?;
    apply_typeface(env, view, matches!(props.weight, FontWeight::Bold))?;

    let setter = if radio {
        ids.radio_set_checked_silently
    } else {
        ids.checkbox_set_checked_silently
    };
    setter.call_void(env, view, &[JValue::Bool(props.checked)])
}

fn update_pager(env: &mut Env<'_>, view: &JObject<'_>, props: &PagerProps) -> JniResult<()> {
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

fn update_date_picker(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &DatePickerProps,
) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let (kind, year, month, day, hour, minute) = match props.value {
        DatePickerValue::Date(date) => (0, date.year, date.month as jint, date.day as jint, 0, 0),
        DatePickerValue::Time(time) => (1, 1970, 1, 1, time.hour as jint, time.minute as jint),
        DatePickerValue::DateTime { date, time } => (
            2,
            date.year,
            date.month as jint,
            date.day as jint,
            time.hour as jint,
            time.minute as jint,
        ),
    };
    ids.configure_date_picker.call_void(
        env,
        view,
        &[
            JValue::Int(kind),
            JValue::Int(year),
            JValue::Int(month),
            JValue::Int(day),
            JValue::Int(hour),
            JValue::Int(minute),
            JValue::Bool(props.mode == DatePickerMode::Inline),
        ],
    )?;
    let (min_date, max_date) = match props.date_range {
        Some((min, max)) => (
            [min.year, min.month as jint, min.day as jint],
            [max.year, max.month as jint, max.day as jint],
        ),
        None => ([0; 3], [0; 3]),
    };
    ids.date_picker_set_date_range.call_void(
        env,
        view,
        &[
            JValue::Int(min_date[0]),
            JValue::Int(min_date[1]),
            JValue::Int(min_date[2]),
            JValue::Int(max_date[0]),
            JValue::Int(max_date[1]),
            JValue::Int(max_date[2]),
        ],
    )?;
    let (min_time, max_time) = match props.time_range {
        Some((min, max)) => (
            [min.hour as jint, min.minute as jint],
            [max.hour as jint, max.minute as jint],
        ),
        None => ([-1; 2], [-1; 2]),
    };
    ids.date_picker_set_time_range.call_void(
        env,
        view,
        &[
            JValue::Int(min_time[0]),
            JValue::Int(min_time[1]),
            JValue::Int(max_time[0]),
            JValue::Int(max_time[1]),
        ],
    )?;
    ids.date_picker_set_style.call_void(
        env,
        view,
        &[
            JValue::Float(props.size),
            JValue::Int(props.color as jint),
            JValue::Bool(matches!(props.weight, FontWeight::Bold)),
        ],
    )
}

fn update_slider(env: &mut Env<'_>, view: &JObject<'_>, props: &SliderProps) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let max = props.max.max(props.min + 0.001);
    ids.configure_seek_bar.call_void(
        env,
        view,
        &[
            JValue::Float(props.min),
            JValue::Float(max),
            JValue::Float(props.step.unwrap_or(0.0)),
            JValue::Int(props.color as jint),
        ],
    )?;
    let value = snap_slider_value(props.value, props.min, max, props.step);
    ids.seek_set_value_silently
        .call_void(env, view, &[JValue::Float(value)])
}

fn update_radio_group(
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

fn update_tab_bar(env: &mut Env<'_>, view: &JObject<'_>, props: &TabBarProps) -> JniResult<()> {
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

fn update_progress(env: &mut Env<'_>, view: &JObject<'_>, props: &ProgressProps) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.update_progress.call_void(
        env,
        &ids.krabik_views,
        &[
            JValue::Object(view),
            JValue::Float(props.value),
            JValue::Int(props.color as jint),
        ],
    )
}

fn update_text(env: &mut Env<'_>, view: &JObject<'_>, kind: &NodeKind) -> JniResult<()> {
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
fn text_limits(kind: &NodeKind) -> (Option<u32>, bool) {
    match kind {
        NodeKind::Widget(Widget::Text(props)) => (props.max_lines, props.ellipsis),
        _ => (None, false),
    }
}

fn apply_text_limits(
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

fn apply_typeface(env: &mut Env<'_>, view: &JObject<'_>, bold: bool) -> JniResult<()> {
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

fn text_properties(kind: &NodeKind) -> Option<(&str, f32, u32, FontWeight)> {
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

fn configure_button(env: &mut Env<'_>, view: &JObject<'_>) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.set_all_caps
        .call_void(env, view, &[JValue::Bool(false)])?;
    ids.set_minimum_height
        .call_void(env, view, &[JValue::Int(0)])?;
    ids.set_minimum_width
        .call_void(env, view, &[JValue::Int(0)])?;
    // The layout already reserves the button's chrome padding around the
    // label, so the label is centred instead of relying on native insets.
    ids.set_gravity
        .call_void(env, view, &[JValue::Int(GRAVITY_CENTER)])?;
    ids.set_padding.call_void(
        env,
        view,
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(0),
        ],
    )?;
    Ok(())
}

fn apply_padding(
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

fn apply_style(
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
fn apply_accessibility(env: &mut Env<'_>, view: &JObject<'_>, node: &RenderNode) -> JniResult<()> {
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

fn set_tag(env: &mut Env<'_>, view: &JObject<'_>, id: jlong) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let tag = ids
        .long_value_of
        .call(env, &ids.long, &[JValue::Long(id)])?
        .l()?;

    ids.set_tag.call_void(env, view, &[JValue::Object(&tag)])?;
    Ok(())
}

/// Applies one node's frame with a single call into `RustRootView`.
fn apply_frame(
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
fn apply_layouts(env: &mut Env<'_>, root: &JObject<'_>, frames: &FrameBatch) -> JniResult<()> {
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
fn apply_size(
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
fn apply_content_size(
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

/// The Android pieces of a [`Session`].
struct Android;

impl Platform for Android {
    type Cx<'a> = Env<'a>;
    type Error = jni::errors::Error;
    type Host = AndroidState;
    type Controller = AndroidController;
}

impl Host<Android> for AndroidState {
    fn kind(&self) -> &HostKind {
        &self.kind
    }

    fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    fn row_actions(&self, id: NodeId) -> Option<&Actions> {
        self.row_actions.get(&id)
    }

    fn row_of(&self, id: NodeId) -> Option<(NodeId, usize)> {
        self.row_nodes.get(&id).copied()
    }

    fn render(&mut self, env: &mut Env<'_>, element: &crate::ui::RawElement) -> JniResult<()> {
        AndroidState::render(self, env, element)
    }
}

/// Builds a Java `String[]` from string slices.
macro_rules! string_array {
    ($env:expr, $values:expr) => {{
        let values: Vec<&str> = $values.collect();
        let array = $env.new_object_array(
            values.len() as jint,
            jni_str!("java/lang/String"),
            JObject::null(),
        )?;
        for (index, value) in values.into_iter().enumerate() {
            let text = jcall!($env, $env.new_string(value))?;
            let text: &JObject<'_> = text.as_ref();
            array.set_element($env, index, text)?;
        }
        array
    }};
}

/// The Kotlin `KrabikHostController`, which owns the screen stack, toolbar,
/// bottom navigation, alert and sheet.
struct AndroidController(Global<JObject<'static>>);

impl Controller<Android> for AndroidController {
    fn push(&self, env: &mut Env<'_>, host: u64, screen: &Screen) -> JniResult<()> {
        let title = env.new_string(&screen.title)?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("push"),
            jni_sig!("(JLjava/lang/String;I)V"),
            &[
                JValue::Long(host as jlong),
                JValue::Object(title.as_ref()),
                JValue::Int(screen.transition.code()),
            ],
        )?;
        Ok(())
    }

    fn pop(&self, env: &mut Env<'_>, count: usize, transition: ScreenTransition) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("pop"),
            jni_sig!("(II)V"),
            &[JValue::Int(count as jint), JValue::Int(transition.code())],
        )?;
        Ok(())
    }

    fn set_root_visible(&self, env: &mut Env<'_>, visible: bool) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("setRootVisible"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(visible)],
        )?;
        Ok(())
    }

    fn set_chrome(&self, env: &mut Env<'_>, screen: Option<&Screen>) -> JniResult<()> {
        let toolbar = screen.and_then(|screen| screen.toolbar.as_ref());
        let actions = toolbar.map_or(&[][..], |toolbar| toolbar.actions.as_slice());
        // A screen shows the toolbar unless it explicitly hides it.
        let hidden = screen.is_none() || toolbar.is_some_and(|toolbar| toolbar.hidden);

        let action_labels = string_array!(env, actions.iter().map(|a| a.label.as_str()));
        let action_icons = string_array!(
            env,
            actions.iter().map(|a| a.icon.android_name().unwrap_or(""))
        );
        let title = jcall!(
            env,
            env.new_string(screen.map_or("", |screen| screen.title.as_str()))
        )?;
        let subtitle = jcall!(
            env,
            env.new_string(toolbar.and_then(|t| t.subtitle.as_deref()).unwrap_or(""))
        )?;
        let back_title = jcall!(
            env,
            env.new_string(
                toolbar
                    .and_then(|t| t.back_title.as_deref())
                    .unwrap_or("Back")
            )
        )?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("setToolbar"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Z[Ljava/lang/String;[Ljava/lang/String;)V"),
            &[
                JValue::Object(title.as_ref()),
                JValue::Object(subtitle.as_ref()),
                JValue::Object(back_title.as_ref()),
                JValue::Bool(hidden),
                JValue::Object(&action_labels),
                JValue::Object(&action_icons),
            ],
        )?;

        // No items hides the bar.
        let bar = screen.and_then(|screen| screen.bottom_navigation.as_ref());
        let items = bar.map_or(&[][..], |bar| bar.items.as_slice());
        let labels = string_array!(env, items.iter().map(|item| item.label.as_str()));
        let icons = string_array!(
            env,
            items
                .iter()
                .map(|item| item.icon.android_name().unwrap_or(""))
        );
        env.call_method(
            self.0.as_obj(),
            jni_str!("setBottomNavigation"),
            jni_sig!("([Ljava/lang/String;[Ljava/lang/String;I)V"),
            &[
                JValue::Object(&labels),
                JValue::Object(&icons),
                JValue::Int(bar.map_or(0, |bar| bar.selected) as jint),
            ],
        )?;
        Ok(())
    }

    fn show_dialog(&self, env: &mut Env<'_>, dialog: &Dialog) -> JniResult<()> {
        let title = jcall!(env, env.new_string(&dialog.title))?;
        let message = jcall!(env, env.new_string(dialog.message.as_deref().unwrap_or("")))?;
        let labels = string_array!(env, dialog.buttons.iter().map(|b| b.label.as_str()));
        let role_codes: Vec<jint> = dialog.buttons.iter().map(|b| b.role.code()).collect();
        let roles = env.new_int_array(role_codes.len())?;
        roles.set_region(env, 0, &role_codes)?;
        env.call_method(
            self.0.as_obj(),
            jni_str!("showDialog"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;[IZ)V"),
            &[
                JValue::Object(title.as_ref()),
                JValue::Object(message.as_ref()),
                JValue::Object(&labels),
                JValue::Object(&roles),
                JValue::Bool(dialog.is_cancelable()),
            ],
        )?;
        Ok(())
    }

    fn dismiss_dialog(&self, env: &mut Env<'_>) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("dismissDialog"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    }

    fn present_sheet(&self, env: &mut Env<'_>, host: u64) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("presentSheet"),
            jni_sig!("(J)V"),
            &[JValue::Long(host as jlong)],
        )?;
        Ok(())
    }

    fn dismiss_sheet(&self, env: &mut Env<'_>) -> JniResult<()> {
        env.call_method(
            self.0.as_obj(),
            jni_str!("dismissSheet"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    }
}

/// One Activity's session plus what Android needs to create more hosts.
struct AndroidSession {
    core: Session<Android>,
    context: Global<JObject<'static>>,
    density: f32,
}

static STATES: OnceLock<Mutex<HashMap<jlong, AndroidSession>>> = OnceLock::new();
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn states() -> &'static Mutex<HashMap<jlong, AndroidSession>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Locks the state map for event dispatch, or gives up when a render holds it.
///
/// Event entry points and `nativeRender` all run on the Android main thread,
/// so a real user event can never race a render: while the render lock is
/// held, the main thread is inside `nativeRender` itself. Lock contention in
/// an event entry point therefore means the render synchronously triggered a
/// widget listener (for example `setTransformationMethod` calls `setText`,
/// which fires the input's text watcher). That callback re-enters the entry
/// point on the same thread, and `std` mutexes are not re-entrant, so
/// blocking on the lock would deadlock the UI. Such programmatic echoes are
/// dropped.
fn lock_states_for_event() -> Option<MutexGuard<'static, HashMap<jlong, AndroidSession>>> {
    states().try_lock().ok()
}

/// Creates the Rust-side state for a `RustRootView`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeCreate<'caller>(
    mut unenv: EnvUnowned<'caller>,
    this: JObject<'caller>,
    context: JObject<'caller>,
    density: jfloat,
) -> jlong {
    run(&mut unenv, "nativeCreate", |env| {
        // Resolving the handles here is what makes the app's own Kotlin
        // classes visible: `FindClass` uses the class loader of this entry
        // point, which is the app's.
        jni_handles(env)?;
        let main = AndroidState::new(env, &context, &this, density, HostKind::Main)?;
        let session = AndroidSession {
            core: Session::new(crate::app::create_app(), main),
            context: env.new_global_ref(&context)?,
            density,
        };
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        states()
            .lock()
            .expect("android states")
            .insert(handle, session);
        Ok(handle)
    })
}

/// The height, in pixels, of the content a host rendered last.
///
/// Lets a content-sized native container, such as a bottom sheet, wrap it.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeContentHeight<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
) -> jint {
    run(&mut unenv, "nativeContentHeight", |_env| {
        let states = states().lock().expect("android states");
        let height = states
            .get(&handle)
            .and_then(|session| session.core.host(host as u64))
            .map_or(0.0, |state| state.runtime.content_height() * state.density);
        Ok(height.ceil() as jint)
    })
}

/// Rebuilds the tree for the new viewport and applies the resulting operations.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeRender<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    width_px: jint,
    height_px: jint,
) {
    run(&mut unenv, "nativeRender", |env| {
        let mut states = states().lock().expect("android states");
        let Some(session) = states.get_mut(&handle) else {
            return Ok(());
        };
        let density = session.density;
        let Some(state) = session.core.host_mut(host as u64) else {
            return Ok(());
        };
        state.viewport = Size::new(
            width_px.max(0) as f32 / density,
            height_px.max(0) as f32 / density,
        );
        session.core.render_all(env)
    });
}

/// Runs `body` on a session for a native event.
///
/// The event is dropped when a render holds the lock; see
/// [`lock_states_for_event`].
fn on_event(
    handle: jlong,
    body: impl FnOnce(&mut AndroidSession) -> JniResult<()>,
) -> JniResult<()> {
    let Some(mut states) = lock_states_for_event() else {
        return Ok(());
    };
    match states.get_mut(&handle) {
        Some(session) => body(session),
        None => Ok(()),
    }
}

/// Hands an event on one widget to the session.
fn on_node_event(
    env: &mut Env<'_>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    event: NodeEvent,
) -> JniResult<()> {
    on_event(handle, |session| {
        session
            .core
            .node_event(env, host as u64, NodeId(node_id as u64), event)
    })
}

/// A button was pressed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnClick<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnClick", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Click)
    });
}

/// The text of an input changed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnTextChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    text: JString<'caller>,
) {
    run(&mut unenv, "nativeOnTextChanged", |env| {
        let value = text.try_to_string(env)?;
        on_node_event(env, handle, host, node_id, NodeEvent::Text(value))
    });
}

/// An input was submitted from the keyboard.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnSubmitted<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnSubmitted", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Submit)
    });
}

/// An input gained or lost focus. `attached` is false when the field lost
/// focus because its recycled row left the screen.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnFocusChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    focused: jboolean,
    attached: jboolean,
) {
    run(&mut unenv, "nativeOnFocusChanged", |env| {
        on_node_event(
            env,
            handle,
            host,
            node_id,
            NodeEvent::Focus { focused, attached },
        )
    });
}

/// Restores focus to the field a recycled row kept logically focused.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeRestoreRowFocus<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    holder_id: jlong,
) {
    run(&mut unenv, "nativeRestoreRowFocus", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(session) = states.get_mut(&handle) else {
            return Ok(());
        };
        let Some(state) = session.core.host_mut(host as u64) else {
            return Ok(());
        };
        let list = NodeId(node_id as u64);

        let focused = {
            let Some(row) = state.row_states.get(&(list, holder_id)) else {
                return Ok(());
            };
            let Some(focus) = state.runtime.row_focus() else {
                return Ok(());
            };
            if focus.list != list || focus.index != row.index {
                return Ok(());
            }
            let Some(handle) = row.views.get(&focus.field) else {
                return Ok(());
            };

            let ids = jni_handles(env)?;
            let already_focused = ids.is_focused.call(env, handle.view.as_obj(), &[])?.z()?;
            let focused = already_focused
                || ids
                    .request_focus
                    .call(env, handle.view.as_obj(), &[])?
                    .z()?;
            if focused && !already_focused {
                ids.ensure_keyboard_shown.call_void(
                    env,
                    state.root.as_obj(),
                    &[JValue::Object(handle.view.as_obj())],
                )?;
            }
            (focused, focus)
        };

        if focused.0 {
            state
                .runtime
                .set_row_focus(list, focused.1.index, focused.1.field, true);
        }
        Ok(())
    });
}

/// A switch, checkbox or radio button was toggled.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnCheckedChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    checked: jboolean,
) {
    run(&mut unenv, "nativeOnCheckedChanged", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Toggle(checked))
    });
}

/// A slider moved.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnSliderChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    value: jfloat,
) {
    run(&mut unenv, "nativeOnSliderChanged", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Slider(value))
    });
}

/// A picker chose a date (`kind` 0), a time (1) or both (2).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnPickerChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    kind: jint,
    year: jint,
    month: jint,
    day: jint,
    hour: jint,
    minute: jint,
) {
    run(&mut unenv, "nativeOnPickerChanged", |env| {
        let date = DateValue::new(year, month.clamp(0, 255) as u8, day.clamp(0, 255) as u8);
        let time = TimeValue::new(hour.clamp(0, 255) as u8, minute.clamp(0, 255) as u8);
        let value = match kind {
            0 => DatePickerValue::Date(date),
            1 => DatePickerValue::Time(time),
            2 => DatePickerValue::DateTime { date, time },
            _ => return Ok(()),
        };
        on_node_event(env, handle, host, node_id, NodeEvent::Picker(value))
    });
}

/// A radio group selected an option.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnRadioGroupSelected<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    index: jint,
) {
    run(&mut unenv, "nativeOnRadioGroupSelected", |env| {
        on_node_event(
            env,
            handle,
            host,
            node_id,
            NodeEvent::Select(index.max(0) as usize),
        )
    });
}

/// A pager settled on a page.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnPageSelected<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    index: jint,
) {
    run(&mut unenv, "nativeOnPageSelected", |env| {
        on_node_event(
            env,
            handle,
            host,
            node_id,
            NodeEvent::Select(index.max(0) as usize),
        )
    });
}

/// A tab bar selected a tab.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnTabSelected<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    index: jint,
) {
    run(&mut unenv, "nativeOnTabSelected", |env| {
        on_node_event(
            env,
            handle,
            host,
            node_id,
            NodeEvent::Select(index.max(0) as usize),
        )
    });
}

/// A scroll container moved to `offset_px`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnScroll<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    offset_px: jfloat,
) {
    run(&mut unenv, "nativeOnScroll", |env| {
        on_event(handle, |session| {
            let offset = offset_px.max(0.0) / session.density;
            session.core.node_event(
                env,
                host as u64,
                NodeId(node_id as u64),
                NodeEvent::Scroll(offset),
            )
        })
    });
}

/// The toolbar item at `index` was tapped.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnToolbarAction<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    index: jint,
) {
    run(&mut unenv, "nativeOnToolbarAction", |env| {
        on_event(handle, |session| {
            session.core.toolbar_action(env, index.max(0) as usize)
        })
    });
}

/// The system alert closed: by the button at index `button`, or by the
/// platform itself when `button` is negative.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnDialogAction<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    button: jint,
) {
    run(&mut unenv, "nativeOnDialogAction", |env| {
        // A negative index means the platform dismissed the alert itself.
        let button = usize::try_from(button).ok();
        on_event(handle, |session| session.core.dialog_closed(env, button))
    });
}

/// The user picked a destination in the bottom navigation bar.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnBottomNavigationSelected<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    index: jint,
) {
    run(&mut unenv, "nativeOnBottomNavigationSelected", |env| {
        on_event(handle, |session| {
            session
                .core
                .bottom_navigation_selected(env, index.max(0) as usize)
        })
    });
}

/// The user dismissed the bottom sheet natively (swipe, back, scrim).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnModalDismiss<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
) {
    run(&mut unenv, "nativeOnModalDismiss", |env| {
        on_event(handle, |session| session.core.sheet_dismissed(env))
    });
}

/// Android's system back button. Returns whether the application consumed
/// it; when it did not, the Activity falls back to its normal behaviour.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnBack<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
) -> jboolean {
    run(&mut unenv, "nativeOnBack", |env| {
        let mut handled = false;
        on_event(handle, |session| {
            handled = session.core.back(env)?;
            Ok(())
        })?;
        Ok(handled as jboolean)
    })
}

/// Builds one recycled-list row into the holder view the adapter provides.
///
/// Returns the row height in pixels, so a variable-height list can size the
/// holder to its content.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeBindRow<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    index: jint,
    holder_id: jlong,
    holder_host: JObject<'caller>,
    width_px: jint,
) -> jint {
    run(&mut unenv, "nativeBindRow", |env| {
        let mut states = states().lock().expect("android states");
        let Some(state) = states
            .get_mut(&handle)
            .and_then(|session| session.core.host_mut(host as u64))
        else {
            return Ok(0);
        };
        state.bind_row(
            env,
            NodeId(node_id as u64),
            index.max(0) as usize,
            holder_id,
            &holder_host,
            width_px,
        )
    })
}

/// Releases the row a recycled-list holder currently shows.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeUnbindRow<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    holder_id: jlong,
) {
    run(&mut unenv, "nativeUnbindRow", |_env| {
        // RecyclerView can recycle a holder while a render holds the state
        // lock. Dropping the cleanup is safe: the next bind of that holder
        // rebuilds the row from scratch and overwrites the stale entry.
        on_event(handle, |session| {
            if let Some(state) = session.core.host_mut(host as u64) {
                state.unbind_row(NodeId(node_id as u64), holder_id);
            }
            Ok(())
        })
    });
}

/// Releases the widgets and the Rust state of a `RustRootView`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeDestroy<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
) {
    run(&mut unenv, "nativeDestroy", |env| {
        if let Some(mut session) = states().lock().expect("android states").remove(&handle) {
            for host in session.core.hosts_mut() {
                let _ = jcall!(
                    env,
                    env.call_method(
                        host.root.as_obj(),
                        jni_str!("removeAllViews"),
                        jni_sig!("()V"),
                        &[],
                    )
                )?;
            }
        }
        Ok(())
    });
}

/// Attaches a host view the controller created (a screen or the sheet).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeAttachHost<'caller>(
    mut unenv: EnvUnowned<'caller>,
    this: JObject<'caller>,
    handle: jlong,
    host: jlong,
) {
    run(&mut unenv, "nativeAttachHost", |env| {
        let mut states = states().lock().expect("android states");
        let Some(session) = states.get_mut(&handle) else {
            return Ok(());
        };
        let (context, density) = (session.context.as_obj(), session.density);
        session.core.attach_host(host as u64, |kind| {
            AndroidState::new(env, context, &this, density, kind)
        })?;
        session.core.render_all(env)
    });
}

/// Registers the native controller that owns navigation and modals.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeSetController<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    controller: JObject<'caller>,
) {
    run(&mut unenv, "nativeSetController", |env| {
        let mut states = states().lock().expect("android states");
        let Some(session) = states.get_mut(&handle) else {
            return Ok(());
        };
        let controller = AndroidController(env.new_global_ref(&controller)?);
        session.core.set_controller(controller);
        session.core.render_all(env)
    });
}

/// Drops one host after its native view is destroyed.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeDestroyHost<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
) {
    run(&mut unenv, "nativeDestroyHost", |_env| {
        on_event(handle, |session| {
            session.core.remove_host(host as u64);
            Ok(())
        })
    });
}
