//! Cached JNI class, method and field handles for the Android renderer.
//!
//! A frame touches every node property, and every property used to resolve its
//! class and method by name (`FindClass` + `GetMethodID`) before the call. The
//! renderer now resolves each handle once and reuses it.
//!
//! # Handle lifetime
//!
//! A method or field ID is only valid while the class that declared it is
//! loaded. [`Jni`] therefore holds a global reference to every class it took a
//! handle from, which keeps the class (and its loader) alive for the process
//! lifetime. The IDs are valid as long as the value in [`JNI`] exists, which is
//! static.
//!
//! IDs resolved from a superclass are also valid for subclass instances, so the
//! `View` and `TextView` handles are shared by every widget. Methods declared by
//! the app's own Kotlin subclasses (`setTextSilently`,
//! `setCheckedSilently`, `configure`) are resolved from those subclasses.
//!
//! # Why the handles are loaded from `nativeCreate`
//!
//! `FindClass` resolves names through the class loader of the calling native
//! method. Loading happens in `nativeCreate`, which the app calls on the main
//! thread, so the app's own classes resolve; a later lazy load on a native
//! thread would only see the system loader.

use std::sync::OnceLock;

use jni::errors::Result as JniResult;
use jni::ids::{JMethodID, JStaticFieldID, JStaticMethodID};
use jni::objects::{JClass, JObject, JValue, JValueOwned};
use jni::refs::Global;
use jni::signature::{JavaType, Primitive, RuntimeMethodSignature};
use jni::strings::JNIStr;
use jni::{Env, jni_sig, jni_str};

use super::{
    FRAME_LAYOUT_CLASS, FRAME_PARAMS_CLASS, KRABIK_STYLE_CLASS, KRABIK_VIEWS_CLASS,
    LAYOUT_PARAMS_CLASS, LONG_CLASS, MEASURE_SPEC_CLASS, ROOT_VIEW_DESCRIPTOR,
    RUST_ROOT_VIEW_CLASS, TEXT_UTILS_CLASS, TEXT_VIEW_CLASS, TYPEFACE_CLASS, VIEW_CLASS, checked,
};

pub(crate) const VOID: JavaType = JavaType::Primitive(Primitive::Void);
pub(crate) const INT: JavaType = JavaType::Primitive(Primitive::Int);
pub(crate) const LONG: JavaType = JavaType::Primitive(Primitive::Long);
pub(crate) const BOOLEAN: JavaType = JavaType::Primitive(Primitive::Boolean);
pub(crate) const OBJECT: JavaType = JavaType::Object;

/// A resolved instance-method handle.
#[derive(Clone, Copy)]
pub(crate) struct Method {
    pub(crate) id: JMethodID,
    pub(crate) returns: JavaType,
}

impl Method {
    pub(crate) fn call<'local>(
        self,
        env: &mut Env<'local>,
        object: &JObject<'_>,
        args: &[JValue],
    ) -> JniResult<JValueOwned<'local>> {
        let args: Vec<_> = args.iter().map(JValue::as_jni).collect();
        // SAFETY: the ID was resolved from this exact name and signature while
        // the declaring class was held by `Jni`, so it is valid for any object
        // of that class or a subclass, and the arguments match the signature.
        let result = unsafe { env.call_method_unchecked(object, self.id, self.returns, &args) };
        checked(env, result)
    }

    pub(crate) fn call_void(
        self,
        env: &mut Env<'_>,
        object: &JObject<'_>,
        args: &[JValue],
    ) -> JniResult<()> {
        self.call(env, object, args).map(|_| ())
    }
}

/// A resolved static-method handle.
#[derive(Clone, Copy)]
pub(crate) struct StaticMethod {
    pub(crate) id: JStaticMethodID,
    pub(crate) returns: JavaType,
}

impl StaticMethod {
    pub(crate) fn call<'local>(
        self,
        env: &mut Env<'local>,
        class: &JClass<'_>,
        args: &[JValue],
    ) -> JniResult<JValueOwned<'local>> {
        let args: Vec<_> = args.iter().map(JValue::as_jni).collect();
        // SAFETY: see `Method::call`; static IDs are bound to the class held by
        // `Jni` and the arguments match the resolved signature.
        let result =
            unsafe { env.call_static_method_unchecked(class, self.id, self.returns, &args) };
        checked(env, result)
    }

    pub(crate) fn call_void(
        self,
        env: &mut Env<'_>,
        class: &JClass<'_>,
        args: &[JValue],
    ) -> JniResult<()> {
        self.call(env, class, args).map(|_| ())
    }
}

/// A resolved static-field handle.
#[derive(Clone, Copy)]
pub(crate) struct StaticField {
    pub(crate) id: JStaticFieldID,
    pub(crate) ty: JavaType,
}

impl StaticField {
    pub(crate) fn get<'local>(
        self,
        env: &mut Env<'local>,
        class: &JClass<'_>,
    ) -> JniResult<JValueOwned<'local>> {
        // SAFETY: the field ID is bound to the class held by `Jni` and was
        // resolved with the same type recorded in `ty`.
        let result = unsafe { env.get_static_field_unchecked(class, self.id, self.ty) };
        checked(env, result)
    }
}

/// A resolved constructor handle.
#[derive(Clone, Copy)]
pub(crate) struct Ctor {
    pub(crate) id: JMethodID,
}

impl Ctor {
    pub(crate) fn construct<'local>(
        self,
        env: &mut Env<'local>,
        class: &JClass<'_>,
        args: &[JValue],
    ) -> JniResult<JObject<'local>> {
        let args: Vec<_> = args.iter().map(JValue::as_jni).collect();
        // SAFETY: the constructor ID is bound to the class held by `Jni` and
        // the arguments match the signature it was resolved from.
        let result = unsafe { env.new_object_unchecked(class, self.id, &args) };
        checked(env, result)
    }
}

macro_rules! method {
    ($env:expr, $class:expr, $name:literal, $sig:literal, $returns:expr) => {
        Method {
            id: $crate::android::handles::named(
                $env.get_method_id($class, jni_str!($name), jni_sig!($sig)),
                concat!("method ", $name, " ", $sig),
            )?,
            returns: $returns,
        }
    };
}

macro_rules! static_method {
    ($env:expr, $class:expr, $name:literal, $sig:literal, $returns:expr) => {
        StaticMethod {
            id: $crate::android::handles::named(
                $env.get_static_method_id($class, jni_str!($name), jni_sig!($sig)),
                concat!("static method ", $name, " ", $sig),
            )?,
            returns: $returns,
        }
    };
}

macro_rules! static_field {
    ($env:expr, $class:expr, $name:literal, $sig:literal, $ty:expr) => {
        StaticField {
            id: $crate::android::handles::named(
                $env.get_static_field_id($class, jni_str!($name), jni_sig!($sig)),
                concat!("static field ", $name, " ", $sig),
            )?,
            ty: $ty,
        }
    };
}

macro_rules! ctor {
    ($env:expr, $class:expr, $sig:literal) => {
        Ctor {
            id: $crate::android::handles::named(
                $env.get_method_id($class, jni_str!("<init>"), jni_sig!($sig)),
                concat!("constructor ", $sig),
            )?,
        }
    };
}

/// A method signature that mentions `RustRootView`, written with `{root}` in
/// its place so the framework's package is not repeated here.
pub(crate) fn root_signature(template: &str) -> JniResult<RuntimeMethodSignature> {
    RuntimeMethodSignature::from_str(template.replace("{root}", ROOT_VIEW_DESCRIPTOR))
}

/// The constructor every framework widget has: `(Context, RustRootView, long)`.
pub(crate) fn widget_ctor(
    env: &mut Env<'_>,
    class: &Global<JClass<'static>>,
    signature: &RuntimeMethodSignature,
) -> JniResult<Ctor> {
    Ok(Ctor {
        id: named(
            env.get_method_id(class, jni_str!("<init>"), signature.method_signature()),
            "a widget constructor (Context, RustRootView, long)",
        )?,
    })
}

pub(crate) fn class(env: &mut Env<'_>, name: &JNIStr) -> JniResult<Global<JClass<'static>>> {
    let class = named(env.find_class(name), &format!("class {name:?}"))?;
    env.new_global_ref(&class)
}

/// Reports which lookup failed. The JNI error itself only says that one
/// did, and one wrong name here means nothing is ever rendered.
pub(crate) fn named<T>(result: JniResult<T>, what: &str) -> JniResult<T> {
    if result.is_err() {
        crate::log::error(&format!(
            "JNI lookup failed for {what}. The Kotlin and Rust sides disagree about a name \
             or signature; nothing can be rendered until they match."
        ));
    }
    result
}

/// Every JNI handle the renderer uses.
pub(crate) struct Jni {
    // Classes are kept alive for the cached IDs below. Most are also used to
    // construct instances or to make static calls.
    pub(crate) view: Global<JClass<'static>>,
    /// Only keeps `ViewGroup` loaded for `addView`/`removeView`/`getChildAt`.
    pub(crate) _view_group: Global<JClass<'static>>,
    pub(crate) text_view: Global<JClass<'static>>,
    pub(crate) frame_layout: Global<JClass<'static>>,
    pub(crate) layout_params: Global<JClass<'static>>,
    pub(crate) frame_params: Global<JClass<'static>>,
    pub(crate) typeface: Global<JClass<'static>>,
    pub(crate) truncate_at: Global<JClass<'static>>,
    pub(crate) long: Global<JClass<'static>>,
    pub(crate) measure_spec: Global<JClass<'static>>,
    pub(crate) krabik_style: Global<JClass<'static>>,
    pub(crate) krabik_views: Global<JClass<'static>>,
    /// Only keeps `RustRootView` loaded for the cached `applyFrame` ID.
    pub(crate) rust_root_view: Global<JClass<'static>>,
    /// `RustRootView.postWake`: asks the main thread to drain the mailbox.
    pub(crate) post_wake: StaticMethod,

    // `android.view.View`, shared by every widget.
    pub(crate) set_padding: Method,
    pub(crate) set_enabled: Method,
    pub(crate) set_content_description: Method,
    pub(crate) set_tag: Method,
    pub(crate) set_layout_params: Method,
    pub(crate) set_minimum_width: Method,
    pub(crate) set_minimum_height: Method,
    pub(crate) set_on_click_listener: Method,
    pub(crate) request_focus: Method,
    pub(crate) is_focused: Method,
    /// `View.scrollTo`, for a scroll container whose offset was reset.
    pub(crate) scroll_to: Method,
    pub(crate) measure: Method,
    pub(crate) measured_width: Method,
    pub(crate) measured_height: Method,

    // `android.view.ViewGroup`.
    pub(crate) add_view: Method,
    pub(crate) remove_view: Method,
    pub(crate) get_child_at: Method,
    pub(crate) remove_all_views: Method,

    // `android.widget.TextView`, shared by text, buttons and inputs.
    pub(crate) set_text: Method,
    pub(crate) set_text_size: Method,
    pub(crate) set_text_color: Method,
    pub(crate) set_letter_spacing: Method,
    pub(crate) set_include_font_padding: Method,
    pub(crate) set_gravity: Method,
    pub(crate) set_max_lines: Method,
    pub(crate) set_ellipsize: Method,
    pub(crate) set_typeface: Method,
    pub(crate) set_hint: Method,

    // The Rust root view batches one node's frame into a single call.
    pub(crate) apply_frame: Method,
    /// The same, for a whole batch of frames in one crossing.
    pub(crate) apply_layouts: Method,
    pub(crate) register_view: Method,
    pub(crate) unregister_view: Method,
    pub(crate) start_enter: Method,
    pub(crate) start_exit: Method,
    pub(crate) ensure_keyboard_shown: Method,

    // Statics and their fields.
    pub(crate) make_measure_spec: StaticMethod,
    pub(crate) long_value_of: StaticMethod,
    pub(crate) create_scroll: StaticMethod,
    pub(crate) apply_style: StaticMethod,
    pub(crate) typeface_default: StaticField,
    pub(crate) typeface_default_bold: StaticField,
    pub(crate) truncate_at_end: StaticField,

    // Constructors.
    pub(crate) text_view_ctor: Ctor,
    pub(crate) view_ctor: Ctor,
    pub(crate) frame_layout_ctor: Ctor,
    pub(crate) layout_params_ctor: Ctor,
    pub(crate) frame_params_ctor: Ctor,

    // Handles owned by one widget live in that widget's module.
    pub(crate) controller: crate::android::controller::Handles,
    pub(crate) text_field: crate::android::widgets::text_field::Handles,
    pub(crate) switch: crate::android::widgets::switch::Handles,
    pub(crate) check: crate::android::widgets::check::Handles,
    pub(crate) radio_group: crate::android::widgets::radio_group::Handles,
    pub(crate) tab_bar: crate::android::widgets::tab_bar::Handles,
    pub(crate) pager: crate::android::widgets::pager::Handles,
    pub(crate) date_picker: crate::android::widgets::date_picker::Handles,
    pub(crate) slider: crate::android::widgets::slider::Handles,
    pub(crate) progress: crate::android::widgets::progress::Handles,
    pub(crate) container: crate::android::widgets::container::Handles,
    pub(crate) image: crate::android::widgets::image::Handles,
    pub(crate) button: crate::android::widgets::button::Handles,
    pub(crate) rows: crate::android::rows::Handles,
}

impl Jni {
    fn load(env: &mut Env<'_>) -> JniResult<Self> {
        let widget_signature = root_signature("(Landroid/content/Context;{root}J)V")?;
        let view = class(env, VIEW_CLASS)?;
        let view_group = class(env, jni_str!("android/view/ViewGroup"))?;
        let text_view = class(env, TEXT_VIEW_CLASS)?;
        let frame_layout = class(env, FRAME_LAYOUT_CLASS)?;
        let layout_params = class(env, LAYOUT_PARAMS_CLASS)?;
        let frame_params = class(env, FRAME_PARAMS_CLASS)?;
        let typeface = class(env, TYPEFACE_CLASS)?;
        let truncate_at = class(env, TEXT_UTILS_CLASS)?;
        let long = class(env, LONG_CLASS)?;
        let measure_spec = class(env, MEASURE_SPEC_CLASS)?;
        let krabik_style = class(env, KRABIK_STYLE_CLASS)?;
        let krabik_views = class(env, KRABIK_VIEWS_CLASS)?;
        let rust_root_view = class(env, RUST_ROOT_VIEW_CLASS)?;

        Ok(Self {
            controller: crate::android::controller::Handles::load(env)?,
            text_field: crate::android::widgets::text_field::Handles::load(env, &widget_signature)?,
            switch: crate::android::widgets::switch::Handles::load(env, &widget_signature)?,
            check: crate::android::widgets::check::Handles::load(env, &widget_signature)?,
            radio_group: crate::android::widgets::radio_group::Handles::load(
                env,
                &widget_signature,
            )?,
            tab_bar: crate::android::widgets::tab_bar::Handles::load(env, &widget_signature)?,
            pager: crate::android::widgets::pager::Handles::load(env, &widget_signature)?,
            date_picker: crate::android::widgets::date_picker::Handles::load(
                env,
                &widget_signature,
            )?,
            slider: crate::android::widgets::slider::Handles::load(env, &widget_signature)?,
            progress: crate::android::widgets::progress::Handles::load(env, &widget_signature)?,
            container: crate::android::widgets::container::Handles::load(env, &widget_signature)?,
            image: crate::android::widgets::image::Handles::load(env, &widget_signature)?,
            button: crate::android::widgets::button::Handles::load(env, &widget_signature)?,
            rows: crate::android::rows::Handles::load(env, &widget_signature)?,
            set_padding: method!(env, &view, "setPadding", "(IIII)V", VOID),

            set_enabled: method!(env, &view, "setEnabled", "(Z)V", VOID),

            set_content_description: method!(
                env,
                &view,
                "setContentDescription",
                "(Ljava/lang/CharSequence;)V",
                VOID
            ),

            set_tag: method!(env, &view, "setTag", "(Ljava/lang/Object;)V", VOID),

            set_layout_params: method!(
                env,
                &view,
                "setLayoutParams",
                "(Landroid/view/ViewGroup$LayoutParams;)V",
                VOID
            ),

            set_minimum_width: method!(env, &view, "setMinimumWidth", "(I)V", VOID),

            set_minimum_height: method!(env, &view, "setMinimumHeight", "(I)V", VOID),

            set_on_click_listener: method!(
                env,
                &view,
                "setOnClickListener",
                "(Landroid/view/View$OnClickListener;)V",
                VOID
            ),

            request_focus: method!(env, &view, "requestFocus", "()Z", BOOLEAN),

            is_focused: method!(env, &view, "isFocused", "()Z", BOOLEAN),
            scroll_to: method!(env, &view, "scrollTo", "(II)V", VOID),

            measure: method!(env, &view, "measure", "(II)V", VOID),

            measured_width: method!(env, &view, "getMeasuredWidth", "()I", INT),

            measured_height: method!(env, &view, "getMeasuredHeight", "()I", INT),

            add_view: method!(env, &view_group, "addView", "(Landroid/view/View;)V", VOID),

            remove_view: method!(
                env,
                &view_group,
                "removeView",
                "(Landroid/view/View;)V",
                VOID
            ),

            get_child_at: method!(
                env,
                &view_group,
                "getChildAt",
                "(I)Landroid/view/View;",
                OBJECT
            ),

            remove_all_views: method!(env, &view_group, "removeAllViews", "()V", VOID),

            set_text: method!(
                env,
                &text_view,
                "setText",
                "(Ljava/lang/CharSequence;)V",
                VOID
            ),

            set_text_size: method!(env, &text_view, "setTextSize", "(IF)V", VOID),

            set_text_color: method!(env, &text_view, "setTextColor", "(I)V", VOID),

            set_letter_spacing: method!(env, &text_view, "setLetterSpacing", "(F)V", VOID),

            set_include_font_padding: method!(
                env,
                &text_view,
                "setIncludeFontPadding",
                "(Z)V",
                VOID
            ),

            set_gravity: method!(env, &text_view, "setGravity", "(I)V", VOID),

            set_max_lines: method!(env, &text_view, "setMaxLines", "(I)V", VOID),

            set_ellipsize: method!(
                env,
                &text_view,
                "setEllipsize",
                "(Landroid/text/TextUtils$TruncateAt;)V",
                VOID
            ),

            set_typeface: method!(
                env,
                &text_view,
                "setTypeface",
                "(Landroid/graphics/Typeface;)V",
                VOID
            ),

            set_hint: method!(
                env,
                &text_view,
                "setHint",
                "(Ljava/lang/CharSequence;)V",
                VOID
            ),

            apply_frame: method!(
                env,
                &rust_root_view,
                "applyFrame",
                "(Landroid/view/View;IIII)V",
                VOID
            ),

            apply_layouts: method!(env, &rust_root_view, "applyLayouts", "([J[I[I)V", VOID),

            register_view: method!(
                env,
                &rust_root_view,
                "registerView",
                "(JLandroid/view/View;)V",
                VOID
            ),

            unregister_view: method!(env, &rust_root_view, "unregisterView", "(J)V", VOID),

            start_enter: method!(
                env,
                &rust_root_view,
                "startEnter",
                "(Landroid/view/View;FIIIIII)V",
                VOID
            ),

            start_exit: method!(
                env,
                &rust_root_view,
                "startExit",
                "(JLandroid/view/View;IIIIII)V",
                VOID
            ),

            ensure_keyboard_shown: method!(
                env,
                &rust_root_view,
                "ensureKeyboardShown",
                "(Landroid/view/View;)V",
                VOID
            ),

            make_measure_spec: static_method!(env, &measure_spec, "makeMeasureSpec", "(II)I", INT),

            long_value_of: static_method!(env, &long, "valueOf", "(J)Ljava/lang/Long;", OBJECT),

            create_scroll: StaticMethod {
                id: env.get_static_method_id(
                    &krabik_views,
                    jni_str!("createScroll"),
                    root_signature("(Landroid/content/Context;Z{root})Landroid/view/ViewGroup;")?
                        .method_signature(),
                )?,
                returns: OBJECT,
            },

            apply_style: static_method!(
                env,
                &krabik_style,
                "apply",
                "(Landroid/view/View;IZIIZZFFFFFIZFZFIIIIII)V",
                VOID
            ),

            typeface_default: static_field!(
                env,
                &typeface,
                "DEFAULT",
                "Landroid/graphics/Typeface;",
                OBJECT
            ),

            typeface_default_bold: static_field!(
                env,
                &typeface,
                "DEFAULT_BOLD",
                "Landroid/graphics/Typeface;",
                OBJECT
            ),

            truncate_at_end: static_field!(
                env,
                &truncate_at,
                "END",
                "Landroid/text/TextUtils$TruncateAt;",
                OBJECT
            ),

            text_view_ctor: ctor!(env, &text_view, "(Landroid/content/Context;)V"),

            view_ctor: ctor!(env, &view, "(Landroid/content/Context;)V"),

            frame_layout_ctor: ctor!(env, &frame_layout, "(Landroid/content/Context;)V"),

            layout_params_ctor: ctor!(env, &layout_params, "(II)V"),

            frame_params_ctor: ctor!(env, &frame_params, "(II)V"),

            view,

            _view_group: view_group,

            text_view,

            frame_layout,

            layout_params,

            frame_params,

            typeface,

            truncate_at,

            long,

            measure_spec,

            krabik_style,

            krabik_views,

            post_wake: static_method!(env, &rust_root_view, "postWake", "(J)V", VOID),
            rust_root_view,
        })
    }
}

static JNI: OnceLock<Jni> = OnceLock::new();

/// Returns the cached handles, loading them on first use.
pub(crate) fn get(env: &mut Env<'_>) -> JniResult<&'static Jni> {
    if let Some(jni) = JNI.get() {
        return Ok(jni);
    }

    let loaded = Jni::load(env)?;
    // Two threads can only race the very first load, and either value is
    // equivalent, so whichever lands first wins.
    let _ = JNI.set(loaded);
    Ok(JNI.get().expect("jni handles"))
}

pub(crate) use {ctor, method, static_method};
