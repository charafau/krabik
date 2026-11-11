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

use crate::app::{App, Event};
use crate::frame_batch::FrameBatch;
use crate::geometry::{Constraints, Rect, Size};
use crate::measure::Measure;
use crate::render::{Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{NodeId, NodeKind};
use crate::ui::{
    Axis, EdgeInsets, FontWeight, GradientDirection, ImageProps, ImageScale, ImageSource,
    ImeAction, KeyboardType, SwitchProps, TextAlign, TextFieldProps, TextProps,
};

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
const KRABIK_IMAGES_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikImages");
const KRABIK_VIEWS_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_dsl/KrabikViews");

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
    text_view: Global<JObject<'static>>,
    density: f32,
    cache: Mutex<HashMap<MeasureKey, Size>>,
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
            text_view: jcall!(env, env.new_global_ref(&view))?,
            density,
            cache: Mutex::new(HashMap::new()),
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

/// The state of one `RustRootView`.
struct AndroidState {
    runtime: Runtime,
    app: Box<dyn App>,
    root: Global<JObject<'static>>,
    context: Global<JObject<'static>>,
    views: HashMap<NodeId, ViewHandle>,
    density: f32,
    viewport: Size,
}

impl AndroidState {
    fn new(
        env: &mut Env<'_>,
        context: &JObject<'_>,
        root: &JObject<'_>,
        density: f32,
    ) -> JniResult<Self> {
        let vm = env.get_java_vm()?;
        let context_ref = env.new_global_ref(context)?;
        let measure = AndroidMeasure::new(env, &vm, context, density)?;

        Ok(Self {
            runtime: Runtime::new(Box::new(measure)),
            app: crate::app::create_app(),
            root: env.new_global_ref(root)?,
            context: context_ref,
            views: HashMap::new(),
            density,
            viewport: Size::ZERO,
        })
    }

    fn render(&mut self, env: &mut Env<'_>) -> JniResult<()> {
        let element = self.app.view();
        let update = self.runtime.update(&element, self.viewport);
        let mut frames = FrameBatch::new();
        for op in update.ops {
            if let Op::Layout {
                id,
                frame,
                content_size,
            } = op
            {
                if let Some(handle) = self.views.get(&id) {
                    frames.push(id, frame, self.density);
                    if let Some(size) = content_size {
                        apply_content_size(env, handle.children_host.as_obj(), size, self.density)?;
                    }
                }
                if let Some(handle) = self.views.get_mut(&id) {
                    handle.frame = Some(frame);
                }
                continue;
            }
            self.apply(env, op)?;
        }

        if !frames.is_empty() {
            apply_layouts(env, self.root.as_obj(), &frames)?;
        }
        Ok(())
    }

    fn apply(&mut self, env: &mut Env<'_>, op: Op) -> JniResult<()> {
        match op {
            Op::Create(node) => {
                let view = self.create_view(env, &node)?;
                if view.is_null() {
                    return Ok(());
                }

                let parent_host = self.parent_host(node.parent);
                {
                    let ids = jni_handles(env)?;
                    ids.add_view
                        .call_void(env, parent_host, &[JValue::Object(&view)])?;
                }

                let children_host = if matches!(node.kind, NodeKind::Scroll(_)) {
                    // A scroll view wraps a single host that holds the content.
                    let host = jni_handles(env)?
                        .get_child_at
                        .call(env, &view, &[JValue::Int(0)])?
                        .l()?;
                    jcall!(env, env.new_global_ref(&host))?
                } else {
                    jcall!(env, env.new_global_ref(&view))?
                };
                self.views.insert(
                    node.id,
                    ViewHandle {
                        view: jcall!(env, env.new_global_ref(&view))?,
                        children_host,
                        parent: node.parent,
                        frame: None,
                    },
                );
            }
            Op::Update(node) => {
                if let Some(handle) = self.views.get(&node.id) {
                    update_view(env, handle.view.as_obj(), &node, self.density)?;
                }
            }
            Op::Reparent { id, parent } => {
                if !self.views.contains_key(&id) {
                    return Ok(());
                }

                let old_parent = self.views[&id].parent;
                if old_parent != parent {
                    let old_host = self.parent_host(old_parent);
                    let new_host = self.parent_host(parent);
                    let view = self.views[&id].view.as_obj();

                    {
                        let ids = jni_handles(env)?;
                        ids.remove_view
                            .call_void(env, old_host, &[JValue::Object(view)])?;
                        ids.add_view
                            .call_void(env, new_host, &[JValue::Object(view)])?;
                    }
                }

                let frame = self.views[&id].frame;
                if let Some(frame) = frame {
                    apply_frame(
                        env,
                        self.root.as_obj(),
                        self.views[&id].view.as_obj(),
                        frame,
                        self.density,
                    )?;
                }
                if let Some(handle) = self.views.get_mut(&id) {
                    handle.parent = parent;
                }
            }
            Op::Remove { id } => {
                if let Some(handle) = self.views.get(&id) {
                    let host = self.parent_host(handle.parent);
                    jni_handles(env)?.remove_view.call_void(
                        env,
                        host,
                        &[JValue::Object(handle.view.as_obj())],
                    )?;
                    jni_handles(env)?.unregister_view.call_void(
                        env,
                        self.root.as_obj(),
                        &[JValue::Long(id.0 as jlong)],
                    )?;
                }
                self.views.remove(&id);
            }
            // `render` collects every layout operation into one batch, so this
            // arm is only reachable if an operation is applied out of band.
            Op::Layout { .. } => {}
        }
        Ok(())
    }

    /// The view that hosts the children of the given native parent.
    fn parent_host(&self, parent: Option<NodeId>) -> &JObject<'static> {
        match parent.and_then(|id| self.views.get(&id)) {
            Some(handle) => handle.children_host.as_obj(),
            None => self.root.as_obj(),
        }
    }

    fn create_view<'local>(
        &self,
        env: &mut Env<'local>,
        node: &RenderNode,
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
                NodeKind::Text(_) => ids
                    .text_view_ctor
                    .construct(env, &ids.text_view, &context)?,
                NodeKind::Button(_) => ids.button_ctor.construct(env, &ids.button, &context)?,
                NodeKind::TextField(_) => {
                    ids.text_input_ctor
                        .construct(env, &ids.text_input, &child_context)?
                }
                NodeKind::Switch(_) => {
                    ids.switch_ctor
                        .construct(env, &ids.switch, &child_context)?
                }
                NodeKind::Image(_) => {
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
                _ => return Ok(JObject::null()),
            }
        };

        if let NodeKind::TextField(props) = &node.kind {
            update_text_field(env, &view, props)?;
        } else if let NodeKind::Switch(props) = &node.kind {
            update_switch(env, &view, props)?;
        } else if let NodeKind::Image(props) = &node.kind {
            update_image(env, &view, props)?;
        } else {
            update_text(env, &view, &node.kind)?;
        }
        if matches!(node.kind, NodeKind::Button(_)) {
            configure_button(env, &view)?;
            jni_handles(env)?.set_on_click_listener.call_void(
                env,
                &view,
                &[JValue::Object(self.root.as_obj())],
            )?;
        }
        apply_padding(env, &view, node.style.padding, self.density)?;
        apply_style(env, &view, node)?;
        apply_accessibility(env, &view, node)?;
        set_tag(env, &view, node.id.0 as jlong)?;
        jni_handles(env)?.register_view.call_void(
            env,
            self.root.as_obj(),
            &[JValue::Long(node.id.0 as jlong), JValue::Object(&view)],
        )?;

        Ok(view)
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

fn update_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    node: &RenderNode,
    density: f32,
) -> JniResult<()> {
    if let NodeKind::TextField(props) = &node.kind {
        update_text_field(env, view, props)?;
    } else if let NodeKind::Switch(props) = &node.kind {
        update_switch(env, view, props)?;
    } else if let NodeKind::Image(props) = &node.kind {
        update_image(env, view, props)?;
    } else {
        update_text(env, view, &node.kind)?;
    }
    apply_padding(env, view, node.style.padding, density)?;
    apply_style(env, view, node)?;
    apply_accessibility(env, view, node)
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

    if let NodeKind::Text(props) = kind {
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
        NodeKind::Text(props) => (props.max_lines, props.ellipsis),
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
        NodeKind::Text(props) => Some((&props.content, props.size, props.color, props.weight)),
        NodeKind::Button(props) => Some((&props.label, props.size, props.color, props.weight)),
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

fn apply_style(env: &mut Env<'_>, view: &JObject<'_>, node: &RenderNode) -> JniResult<()> {
    let ids = jni_handles(env)?;
    let background = node.style.background;
    let gradient = node.style.gradient;
    let radii = node.style.corner_radii();
    let border = node.style.border;
    let ripple = matches!(node.kind, NodeKind::Button(_));

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
            JValue::Float(node.style.elevation),
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
    let ids = env.new_long_array(frames.len())?;
    ids.set_region(env, 0, frames.ids())?;
    let values = env.new_int_array(frames.frames().len())?;
    values.set_region(env, 0, frames.frames())?;

    jni_handles(env)?.apply_layouts.call_void(
        env,
        root,
        &[JValue::Object(&ids), JValue::Object(&values)],
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

static STATES: OnceLock<Mutex<HashMap<jlong, AndroidState>>> = OnceLock::new();
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn states() -> &'static Mutex<HashMap<jlong, AndroidState>> {
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
fn lock_states_for_event() -> Option<MutexGuard<'static, HashMap<jlong, AndroidState>>> {
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
        let state = AndroidState::new(env, &context, &this, density)?;
        let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
        states()
            .lock()
            .expect("android states")
            .insert(handle, state);
        Ok(handle)
    })
}

/// Rebuilds the tree for the new viewport and applies the resulting operations.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeRender<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    width_px: jint,
    height_px: jint,
) {
    run(&mut unenv, "nativeRender", |env| {
        let mut states = states().lock().expect("android states");
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        state.viewport = Size::new(
            width_px.max(0) as f32 / state.density,
            height_px.max(0) as f32 / state.density,
        );
        state.render(env)
    });
}

/// Dispatches a click on a native widget back into the Rust tree.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnClick<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnClick", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        if let Some(action) = state.runtime.click(NodeId(node_id as u64))
            && state.app.handle(Event::Click { action: &action })
        {
            state.render(env)?;
        }
        Ok(())
    });
}

/// Dispatches a text edit on a native input back into the Rust tree.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnTextChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
    text: JString<'caller>,
) {
    run(&mut unenv, "nativeOnTextChanged", |env| {
        let value = text.try_to_string(env)?;
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        if let Some(action) = state.runtime.text_field_action(NodeId(node_id as u64))
            && state.app.handle(Event::TextChanged {
                action: &action,
                value: &value,
            })
        {
            state.render(env)?;
        }
        Ok(())
    });
}

/// Dispatches a submit on a native input back into the Rust tree.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnSubmitted<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnSubmitted", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        if let Some(action) = state
            .runtime
            .text_field_submit_action(NodeId(node_id as u64))
            && state.app.handle(Event::Submitted { action: &action })
        {
            state.render(env)?;
        }
        Ok(())
    });
}

/// Dispatches a focus change on a native input back into the Rust tree.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnFocusChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
    focused: jboolean,
) {
    run(&mut unenv, "nativeOnFocusChanged", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        let focus_changed = state.runtime.set_focus(NodeId(node_id as u64), focused);
        let handled = state
            .runtime
            .text_field_focus_action(NodeId(node_id as u64))
            .is_some_and(|action| {
                state.app.handle(Event::FocusChanged {
                    action: &action,
                    focused,
                })
            });
        if focus_changed || handled {
            state.render(env)?;
        }
        Ok(())
    });
}

/// Dispatches a switch toggle back into the Rust tree.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnCheckedChanged<
    'caller,
>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
    checked: jboolean,
) {
    run(&mut unenv, "nativeOnCheckedChanged", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        if let Some(action) = state.runtime.switch_action(NodeId(node_id as u64))
            && state.app.handle(Event::Toggled {
                action: &action,
                checked,
            })
        {
            state.render(env)?;
        }
        Ok(())
    });
}

/// Dispatches a scroll offset of a native scroll container back to the runtime.
///
/// The offset arrives in pixels; the runtime works in logical points. Only a
/// list whose visible window changed rebuilds, so pixel-level scrolling costs
/// a lookup and a range check.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_RustRootView_nativeOnScroll<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    node_id: jlong,
    offset_px: jfloat,
) {
    run(&mut unenv, "nativeOnScroll", |env| {
        let Some(mut states) = lock_states_for_event() else {
            return Ok(());
        };
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        let offset = offset_px.max(0.0) / state.density;
        if state.runtime.scroll(NodeId(node_id as u64), offset) {
            state.render(env)?;
        }
        Ok(())
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
        if let Some(state) = states().lock().expect("android states").remove(&handle) {
            let _ = jcall!(
                env,
                env.call_method(
                    state.root.as_obj(),
                    jni_str!("removeAllViews"),
                    jni_sig!("()V"),
                    &[],
                )
            )?;
        }
        Ok(())
    });
}
