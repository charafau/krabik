//! Android renderer driven directly through JNI.
//!
//! The Kotlin side is a thin `ViewGroup`: it forwards its size to Rust and
//! forwards clicks back. Every widget, every property update, and every frame
//! change is performed here on native `TextView`/`Button` instances.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicI64, Ordering};

use jni::errors::{Result as JniResult, ThrowRuntimeExAndDefault};
use jni::objects::JObject;
use jni::refs::Global;
use jni::strings::JNIStr;
use jni::sys::{jfloat, jint, jlong};
use jni::{Env, EnvUnowned, JValue, JavaVM, jni_sig, jni_str};

use crate::app::App;
use crate::geometry::{Constraints, Rect, Size};
use crate::measure::Measure;
use crate::render::{Op, RenderNode};
use crate::runtime::Runtime;
use crate::tree::{NodeId, NodeKind};
use crate::ui::{EdgeInsets, FontWeight, TextProps};

const TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/TextView");
const BUTTON_CLASS: &JNIStr = jni_str!("android/widget/Button");
const TYPEFACE_CLASS: &JNIStr = jni_str!("android/graphics/Typeface");
const LONG_CLASS: &JNIStr = jni_str!("java/lang/Long");
const MEASURE_SPEC_CLASS: &JNIStr = jni_str!("android/view/View$MeasureSpec");
const LAYOUT_PARAMS_CLASS: &JNIStr = jni_str!("android/view/ViewGroup$LayoutParams");

const MEASURE_UNSPECIFIED: jint = 0;
const MEASURE_AT_MOST: jint = -2_147_483_648;
const WRAP_CONTENT: jint = -2;

/// The unit expected by `TextView.setTextSize` for scaled pixels.
const COMPLEX_UNIT_SP: jint = 2;
/// `android.view.Gravity.CENTER`.
const GRAVITY_CENTER: jint = 17;

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
}

impl AndroidMeasure {
    fn new(env: &mut Env<'_>, vm: &JavaVM, context: &JObject<'_>, density: f32) -> JniResult<Self> {
        let class = jcall!(env, env.find_class(TEXT_VIEW_CLASS))?;
        let view = jcall!(
            env,
            env.new_object(
                &class,
                jni_sig!("(Landroid/content/Context;)V"),
                &[JValue::Object(context)],
            )
        )?;

        let _ = jcall!(
            env,
            env.call_method(
                &view,
                jni_str!("setIncludeFontPadding"),
                jni_sig!("(Z)V"),
                &[JValue::Bool(false)],
            )
        )?;
        let _ = jcall!(
            env,
            env.call_method(
                &view,
                jni_str!("setPadding"),
                jni_sig!("(IIII)V"),
                &[
                    JValue::Int(0),
                    JValue::Int(0),
                    JValue::Int(0),
                    JValue::Int(0),
                ],
            )
        )?;

        // `TextView.setText` consults the layout params while it decides
        // whether to request another layout pass, even for a detached view.
        let params_class = jcall!(env, env.find_class(LAYOUT_PARAMS_CLASS))?;
        let params = jcall!(
            env,
            env.new_object(
                &params_class,
                jni_sig!("(II)V"),
                &[JValue::Int(WRAP_CONTENT), JValue::Int(WRAP_CONTENT)],
            )
        )?;
        let _ = jcall!(
            env,
            env.call_method(
                &view,
                jni_str!("setLayoutParams"),
                jni_sig!("(Landroid/view/ViewGroup$LayoutParams;)V"),
                &[JValue::Object(&params)],
            )
        )?;

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
        let _ = jcall!(
            env,
            env.call_method(
                view,
                jni_str!("setText"),
                jni_sig!("(Ljava/lang/CharSequence;)V"),
                &[JValue::Object(string.as_ref())],
            )
        )?;
        let _ = jcall!(
            env,
            env.call_method(
                view,
                jni_str!("setTextSize"),
                jni_sig!("(IF)V"),
                &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(text.size)],
            )
        )?;
        let bold = matches!(text.weight, FontWeight::Bold);
        apply_typeface(env, view, bold)?;

        let width = match constraints.max_width {
            Some(max_width) => {
                let px = (max_width * self.density).ceil().max(0.0) as jint;
                measure_spec(env, px, MEASURE_AT_MOST)?
            }
            None => measure_spec(env, 0, MEASURE_UNSPECIFIED)?,
        };
        let height = measure_spec(env, 0, MEASURE_UNSPECIFIED)?;

        let _ = jcall!(
            env,
            env.call_method(
                view,
                jni_str!("measure"),
                jni_sig!("(II)V"),
                &[JValue::Int(width), JValue::Int(height)],
            )
        )?;

        let measured_width = jcall!(
            env,
            env.call_method(view, jni_str!("getMeasuredWidth"), jni_sig!("()I"), &[])
        )?
        .i()?;
        let measured_height = jcall!(
            env,
            env.call_method(view, jni_str!("getMeasuredHeight"), jni_sig!("()I"), &[])
        )?
        .i()?;

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
    let class = jcall!(env, env.find_class(MEASURE_SPEC_CLASS))?;
    jcall!(
        env,
        env.call_static_method(
            &class,
            jni_str!("makeMeasureSpec"),
            jni_sig!("(II)I"),
            &[JValue::Int(size), JValue::Int(mode)],
        )
    )?
    .i()
}

/// The state of one `RustRootView`.
struct AndroidState {
    runtime: Runtime,
    app: Box<dyn App>,
    root: Global<JObject<'static>>,
    context: Global<JObject<'static>>,
    views: HashMap<NodeId, Global<JObject<'static>>>,
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
        for op in update.ops {
            self.apply(env, op)?;
        }
        Ok(())
    }

    fn apply(&mut self, env: &mut Env<'_>, op: Op) -> JniResult<()> {
        match op {
            Op::Create(node) => {
                let view = self.create_view(env, &node)?;
                let _ = jcall!(
                    env,
                    env.call_method(
                        self.root.as_obj(),
                        jni_str!("addView"),
                        jni_sig!("(Landroid/view/View;)V"),
                        &[JValue::Object(&view)],
                    )
                )?;
                self.views
                    .insert(node.id, jcall!(env, env.new_global_ref(&view))?);
            }
            Op::Update(node) => {
                if let Some(view) = self.views.get(&node.id) {
                    update_view(env, view.as_obj(), &node, self.density)?;
                }
            }
            Op::Remove { id } => {
                if let Some(view) = self.views.remove(&id) {
                    let _ = jcall!(
                        env,
                        env.call_method(
                            self.root.as_obj(),
                            jni_str!("removeView"),
                            jni_sig!("(Landroid/view/View;)V"),
                            &[JValue::Object(view.as_obj())],
                        )
                    )?;
                }
            }
            Op::Layout { id, frame } => {
                if let Some(view) = self.views.get(&id) {
                    apply_layout(env, view.as_obj(), frame, self.density)?;
                }
            }
        }
        Ok(())
    }

    fn create_view<'local>(
        &self,
        env: &mut Env<'local>,
        node: &RenderNode,
    ) -> JniResult<JObject<'local>> {
        let class_name = match &node.kind {
            NodeKind::Text(_) => TEXT_VIEW_CLASS,
            NodeKind::Button(_) => BUTTON_CLASS,
            _ => return Ok(JObject::null()),
        };

        let class = jcall!(env, env.find_class(class_name))?;
        let view = jcall!(
            env,
            env.new_object(
                &class,
                jni_sig!("(Landroid/content/Context;)V"),
                &[JValue::Object(self.context.as_obj())],
            )
        )?;

        update_text(env, &view, &node.kind)?;
        if matches!(node.kind, NodeKind::Button(_)) {
            configure_button(env, &view)?;
            let _ = jcall!(
                env,
                env.call_method(
                    &view,
                    jni_str!("setOnClickListener"),
                    jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                    &[JValue::Object(self.root.as_obj())],
                )
            )?;
        }
        apply_padding(env, &view, node.style.padding, self.density)?;
        apply_background(env, &view, node)?;
        set_tag(env, &view, node.id.0 as jlong)?;

        Ok(view)
    }
}

fn update_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    node: &RenderNode,
    density: f32,
) -> JniResult<()> {
    update_text(env, view, &node.kind)?;
    apply_padding(env, view, node.style.padding, density)?;
    apply_background(env, view, node)
}

fn update_text(env: &mut Env<'_>, view: &JObject<'_>, kind: &NodeKind) -> JniResult<()> {
    let Some((content, size, color, weight)) = text_properties(kind) else {
        return Ok(());
    };

    // The off-screen view used for measurement disables font padding. Do the
    // same here so the rendered metrics match the measured ones, otherwise the
    // extra padding pushes descenders out of the frame.
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setIncludeFontPadding"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(false)],
        )
    )?;

    let string = jcall!(env, env.new_string(content))?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setText"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(string.as_ref())],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setTextSize"),
            jni_sig!("(IF)V"),
            &[JValue::Int(COMPLEX_UNIT_SP), JValue::Float(size)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setLetterSpacing"),
            jni_sig!("(F)V"),
            &[JValue::Float(0.0)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setTextColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(color as jint)],
        )
    )?;

    apply_typeface(env, view, matches!(weight, FontWeight::Bold))?;

    Ok(())
}

fn apply_typeface(env: &mut Env<'_>, view: &JObject<'_>, bold: bool) -> JniResult<()> {
    let class = jcall!(env, env.find_class(TYPEFACE_CLASS))?;
    let field = if bold {
        jni_str!("DEFAULT_BOLD")
    } else {
        jni_str!("DEFAULT")
    };
    let typeface = jcall!(
        env,
        env.get_static_field(&class, field, jni_sig!("Landroid/graphics/Typeface;"))
    )?
    .l()?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setTypeface"),
            jni_sig!("(Landroid/graphics/Typeface;)V"),
            &[JValue::Object(&typeface)],
        )
    )?;
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
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setAllCaps"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(false)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setMinimumHeight"),
            jni_sig!("(I)V"),
            &[JValue::Int(0)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setMinimumWidth"),
            jni_sig!("(I)V"),
            &[JValue::Int(0)],
        )
    )?;
    // The layout already reserves the button's chrome padding around the
    // label, so the label is centred instead of relying on native insets.
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setGravity"),
            jni_sig!("(I)V"),
            &[JValue::Int(GRAVITY_CENTER)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setPadding"),
            jni_sig!("(IIII)V"),
            &[
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
                JValue::Int(0),
            ],
        )
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
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setPadding"),
            jni_sig!("(IIII)V"),
            &[
                JValue::Int(px(padding.left)),
                JValue::Int(px(padding.top)),
                JValue::Int(px(padding.right)),
                JValue::Int(px(padding.bottom)),
            ],
        )
    )?;
    Ok(())
}

fn apply_background(env: &mut Env<'_>, view: &JObject<'_>, node: &RenderNode) -> JniResult<()> {
    if let Some(color) = node.style.background {
        let _ = jcall!(
            env,
            env.call_method(
                view,
                jni_str!("setBackgroundColor"),
                jni_sig!("(I)V"),
                &[JValue::Int(color as jint)],
            )
        )?;
    }
    Ok(())
}

fn set_tag(env: &mut Env<'_>, view: &JObject<'_>, id: jlong) -> JniResult<()> {
    let class = jcall!(env, env.find_class(LONG_CLASS))?;
    let tag = jcall!(
        env,
        env.call_static_method(
            &class,
            jni_str!("valueOf"),
            jni_sig!("(J)Ljava/lang/Long;"),
            &[JValue::Long(id)],
        )
    )?
    .l()?;

    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setTag"),
            jni_sig!("(Ljava/lang/Object;)V"),
            &[JValue::Object(&tag)],
        )
    )?;
    Ok(())
}

fn apply_layout(env: &mut Env<'_>, view: &JObject<'_>, frame: Rect, density: f32) -> JniResult<()> {
    let x = (frame.x * density).round() as jint;
    let y = (frame.y * density).round() as jint;
    let width = (frame.width * density).round().max(0.0) as jint;
    let height = (frame.height * density).round().max(0.0) as jint;

    let class = jcall!(env, env.find_class(LAYOUT_PARAMS_CLASS))?;
    let params = jcall!(
        env,
        env.new_object(
            &class,
            jni_sig!("(II)V"),
            &[JValue::Int(width), JValue::Int(height)],
        )
    )?;

    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setLayoutParams"),
            jni_sig!("(Landroid/view/ViewGroup$LayoutParams;)V"),
            &[JValue::Object(&params)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setX"),
            jni_sig!("(F)V"),
            &[JValue::Float(x as f32)],
        )
    )?;
    let _ = jcall!(
        env,
        env.call_method(
            view,
            jni_str!("setY"),
            jni_sig!("(F)V"),
            &[JValue::Float(y as f32)],
        )
    )?;
    Ok(())
}

static STATES: OnceLock<Mutex<HashMap<jlong, AndroidState>>> = OnceLock::new();
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn states() -> &'static Mutex<HashMap<jlong, AndroidState>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
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
        let mut states = states().lock().expect("android states");
        let Some(state) = states.get_mut(&handle) else {
            return Ok(());
        };

        if let Some(action) = state.runtime.click(NodeId(node_id as u64))
            && state.app.handle(action)
        {
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
