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
use jni::signature::{JavaType, Primitive};
use jni::strings::JNIStr;
use jni::{Env, jni_sig, jni_str};

use super::{
    BUTTON_CLASS, CHECKBOX_CLASS, DATE_PICKER_CLASS, FRAME_LAYOUT_CLASS, FRAME_PARAMS_CLASS,
    IMAGE_VIEW_CLASS, KRABIK_IMAGES_CLASS, KRABIK_RECYCLERS_CLASS, KRABIK_STYLE_CLASS,
    KRABIK_VIEWS_CLASS, LAYOUT_PARAMS_CLASS, LONG_CLASS, MEASURE_SPEC_CLASS, PROGRESS_BAR_CLASS,
    RADIO_CLASS, RADIO_GROUP_CLASS, RUST_ROOT_VIEW_CLASS, SEEK_BAR_CLASS, SWITCH_CLASS,
    TAB_BAR_CLASS, TEXT_INPUT_CLASS, TEXT_UTILS_CLASS, TEXT_VIEW_CLASS, TYPEFACE_CLASS, VIEW_CLASS,
    checked,
};

const VOID: JavaType = JavaType::Primitive(Primitive::Void);
const INT: JavaType = JavaType::Primitive(Primitive::Int);
const BOOLEAN: JavaType = JavaType::Primitive(Primitive::Boolean);
const OBJECT: JavaType = JavaType::Object;

/// A resolved instance-method handle.
#[derive(Clone, Copy)]
pub(super) struct Method {
    id: JMethodID,
    returns: JavaType,
}

impl Method {
    pub(super) fn call<'local>(
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

    pub(super) fn call_void(
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
pub(super) struct StaticMethod {
    id: JStaticMethodID,
    returns: JavaType,
}

impl StaticMethod {
    pub(super) fn call<'local>(
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

    pub(super) fn call_void(
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
pub(super) struct StaticField {
    id: JStaticFieldID,
    ty: JavaType,
}

impl StaticField {
    pub(super) fn get<'local>(
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
pub(super) struct Ctor {
    id: JMethodID,
}

impl Ctor {
    pub(super) fn construct<'local>(
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
            id: $env.get_method_id($class, jni_str!($name), jni_sig!($sig))?,
            returns: $returns,
        }
    };
}

macro_rules! static_method {
    ($env:expr, $class:expr, $name:literal, $sig:literal, $returns:expr) => {
        StaticMethod {
            id: $env.get_static_method_id($class, jni_str!($name), jni_sig!($sig))?,
            returns: $returns,
        }
    };
}

macro_rules! static_field {
    ($env:expr, $class:expr, $name:literal, $sig:literal, $ty:expr) => {
        StaticField {
            id: $env.get_static_field_id($class, jni_str!($name), jni_sig!($sig))?,
            ty: $ty,
        }
    };
}

macro_rules! ctor {
    ($env:expr, $class:expr, $sig:literal) => {
        Ctor {
            id: $env.get_method_id($class, jni_str!("<init>"), jni_sig!($sig))?,
        }
    };
}

fn class(env: &mut Env<'_>, name: &JNIStr) -> JniResult<Global<JClass<'static>>> {
    let class = env.find_class(name)?;
    env.new_global_ref(&class)
}

/// Every JNI handle the renderer uses.
pub(super) struct Jni {
    // Classes are kept alive for the cached IDs below. Most are also used to
    // construct instances or to make static calls.
    pub(super) view: Global<JClass<'static>>,
    /// Only keeps `ViewGroup` loaded for `addView`/`removeView`/`getChildAt`.
    pub(super) _view_group: Global<JClass<'static>>,
    pub(super) text_view: Global<JClass<'static>>,
    pub(super) button: Global<JClass<'static>>,
    pub(super) image_view: Global<JClass<'static>>,
    pub(super) frame_layout: Global<JClass<'static>>,
    pub(super) text_input: Global<JClass<'static>>,
    pub(super) switch: Global<JClass<'static>>,
    pub(super) checkbox: Global<JClass<'static>>,
    pub(super) radio_button: Global<JClass<'static>>,
    pub(super) radio_group: Global<JClass<'static>>,
    pub(super) tab_bar: Global<JClass<'static>>,
    pub(super) date_picker: Global<JClass<'static>>,
    pub(super) seek_bar: Global<JClass<'static>>,
    pub(super) progress_bar: Global<JClass<'static>>,
    pub(super) layout_params: Global<JClass<'static>>,
    pub(super) frame_params: Global<JClass<'static>>,
    pub(super) typeface: Global<JClass<'static>>,
    pub(super) truncate_at: Global<JClass<'static>>,
    pub(super) long: Global<JClass<'static>>,
    pub(super) measure_spec: Global<JClass<'static>>,
    pub(super) krabik_style: Global<JClass<'static>>,
    pub(super) krabik_views: Global<JClass<'static>>,
    pub(super) krabik_images: Global<JClass<'static>>,
    pub(super) krabik_recyclers: Global<JClass<'static>>,
    /// Only keeps `RustRootView` loaded for the cached `applyFrame` ID.
    pub(super) _rust_root_view: Global<JClass<'static>>,

    // `android.view.View`, shared by every widget.
    pub(super) set_padding: Method,
    pub(super) set_enabled: Method,
    pub(super) set_content_description: Method,
    pub(super) set_tag: Method,
    pub(super) set_layout_params: Method,
    pub(super) set_minimum_width: Method,
    pub(super) set_minimum_height: Method,
    pub(super) set_on_click_listener: Method,
    pub(super) request_focus: Method,
    pub(super) is_focused: Method,
    pub(super) measure: Method,
    pub(super) measured_width: Method,
    pub(super) measured_height: Method,

    // `android.view.ViewGroup`.
    pub(super) add_view: Method,
    pub(super) remove_view: Method,
    pub(super) get_child_at: Method,
    pub(super) remove_all_views: Method,

    // `android.widget.TextView`, shared by text, buttons and inputs.
    pub(super) set_text: Method,
    pub(super) set_text_size: Method,
    pub(super) set_text_color: Method,
    pub(super) set_letter_spacing: Method,
    pub(super) set_include_font_padding: Method,
    pub(super) set_gravity: Method,
    pub(super) set_max_lines: Method,
    pub(super) set_ellipsize: Method,
    pub(super) set_typeface: Method,
    pub(super) set_all_caps: Method,
    pub(super) set_hint: Method,

    // App subclasses.
    pub(super) set_text_silently: Method,
    pub(super) configure_input: Method,
    pub(super) set_checked_silently: Method,
    pub(super) checkbox_set_checked_silently: Method,
    pub(super) radio_set_checked_silently: Method,
    pub(super) configure_radio_group: Method,
    pub(super) configure_tab_bar: Method,
    pub(super) configure_date_picker: Method,
    pub(super) date_picker_set_date_range: Method,
    pub(super) date_picker_set_time_range: Method,
    pub(super) date_picker_set_style: Method,
    pub(super) configure_seek_bar: Method,
    pub(super) seek_set_value_silently: Method,

    // The Rust root view batches one node's frame into a single call.
    pub(super) apply_frame: Method,
    /// The same, for a whole batch of frames in one crossing.
    pub(super) apply_layouts: Method,
    pub(super) register_view: Method,
    pub(super) unregister_view: Method,
    pub(super) ensure_keyboard_shown: Method,

    // Statics and their fields.
    pub(super) make_measure_spec: StaticMethod,
    pub(super) long_value_of: StaticMethod,
    pub(super) create_scroll: StaticMethod,
    pub(super) load_image: StaticMethod,
    pub(super) create_recycler: StaticMethod,
    pub(super) update_recycler: StaticMethod,
    pub(super) create_progress: StaticMethod,
    pub(super) update_progress: StaticMethod,
    pub(super) update_spinner: StaticMethod,
    pub(super) apply_style: StaticMethod,
    pub(super) typeface_default: StaticField,
    pub(super) typeface_default_bold: StaticField,
    pub(super) truncate_at_end: StaticField,

    // Constructors.
    pub(super) text_view_ctor: Ctor,
    pub(super) button_ctor: Ctor,
    pub(super) image_view_ctor: Ctor,
    pub(super) view_ctor: Ctor,
    pub(super) frame_layout_ctor: Ctor,
    pub(super) text_input_ctor: Ctor,
    pub(super) switch_ctor: Ctor,
    pub(super) checkbox_ctor: Ctor,
    pub(super) radio_ctor: Ctor,
    pub(super) radio_group_ctor: Ctor,
    pub(super) tab_bar_ctor: Ctor,
    pub(super) date_picker_ctor: Ctor,
    pub(super) seek_bar_ctor: Ctor,
    pub(super) progress_bar_ctor: Ctor,
    pub(super) layout_params_ctor: Ctor,
    pub(super) frame_params_ctor: Ctor,
}

impl Jni {
    fn load(env: &mut Env<'_>) -> JniResult<Self> {
        let view = class(env, VIEW_CLASS)?;
        let view_group = class(env, jni_str!("android/view/ViewGroup"))?;
        let text_view = class(env, TEXT_VIEW_CLASS)?;
        let button = class(env, BUTTON_CLASS)?;
        let image_view = class(env, IMAGE_VIEW_CLASS)?;
        let frame_layout = class(env, FRAME_LAYOUT_CLASS)?;
        let text_input = class(env, TEXT_INPUT_CLASS)?;
        let switch = class(env, SWITCH_CLASS)?;
        let checkbox = class(env, CHECKBOX_CLASS)?;
        let radio_button = class(env, RADIO_CLASS)?;
        let radio_group = class(env, RADIO_GROUP_CLASS)?;
        let tab_bar = class(env, TAB_BAR_CLASS)?;
        let date_picker = class(env, DATE_PICKER_CLASS)?;
        let seek_bar = class(env, SEEK_BAR_CLASS)?;
        let progress_bar = class(env, PROGRESS_BAR_CLASS)?;
        let layout_params = class(env, LAYOUT_PARAMS_CLASS)?;
        let frame_params = class(env, FRAME_PARAMS_CLASS)?;
        let typeface = class(env, TYPEFACE_CLASS)?;
        let truncate_at = class(env, TEXT_UTILS_CLASS)?;
        let long = class(env, LONG_CLASS)?;
        let measure_spec = class(env, MEASURE_SPEC_CLASS)?;
        let krabik_style = class(env, KRABIK_STYLE_CLASS)?;
        let krabik_views = class(env, KRABIK_VIEWS_CLASS)?;
        let krabik_images = class(env, KRABIK_IMAGES_CLASS)?;
        let krabik_recyclers = class(env, KRABIK_RECYCLERS_CLASS)?;
        let rust_root_view = class(env, RUST_ROOT_VIEW_CLASS)?;

        Ok(Self {
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
            set_all_caps: method!(env, &text_view, "setAllCaps", "(Z)V", VOID),
            set_hint: method!(
                env,
                &text_view,
                "setHint",
                "(Ljava/lang/CharSequence;)V",
                VOID
            ),

            set_text_silently: method!(
                env,
                &text_input,
                "setTextSilently",
                "(Ljava/lang/String;)V",
                VOID
            ),
            configure_input: method!(env, &text_input, "configure", "(ZZII)V", VOID),
            set_checked_silently: method!(env, &switch, "setCheckedSilently", "(Z)V", VOID),
            checkbox_set_checked_silently: method!(
                env,
                &checkbox,
                "setCheckedSilently",
                "(Z)V",
                VOID
            ),
            radio_set_checked_silently: method!(
                env,
                &radio_button,
                "setCheckedSilently",
                "(Z)V",
                VOID
            ),
            configure_radio_group: method!(
                env,
                &radio_group,
                "configure",
                "([Ljava/lang/String;IFIFZ)V",
                VOID
            ),
            configure_tab_bar: method!(
                env,
                &tab_bar,
                "configure",
                "([Ljava/lang/String;IFIZ)V",
                VOID
            ),
            configure_date_picker: method!(env, &date_picker, "configure", "(IIIIIIZ)V", VOID),
            date_picker_set_date_range: method!(
                env,
                &date_picker,
                "setDateRange",
                "(IIIIII)V",
                VOID
            ),
            date_picker_set_time_range: method!(env, &date_picker, "setTimeRange", "(IIII)V", VOID),
            date_picker_set_style: method!(env, &date_picker, "setStyle", "(FIZ)V", VOID),
            configure_seek_bar: method!(env, &seek_bar, "configure", "(FFFI)V", VOID),
            seek_set_value_silently: method!(env, &seek_bar, "setValueSilently", "(F)V", VOID),

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
            ensure_keyboard_shown: method!(
                env,
                &rust_root_view,
                "ensureKeyboardShown",
                "(Landroid/view/View;)V",
                VOID
            ),

            make_measure_spec: static_method!(env, &measure_spec, "makeMeasureSpec", "(II)I", INT),
            long_value_of: static_method!(env, &long, "valueOf", "(J)Ljava/lang/Long;", OBJECT),
            create_scroll: static_method!(
                env,
                &krabik_views,
                "createScroll",
                "(Landroid/content/Context;ZLcom/example/krabik_ui_dsl/RustRootView;)Landroid/view/ViewGroup;",
                OBJECT
            ),
            load_image: static_method!(
                env,
                &krabik_images,
                "load",
                "(Landroid/widget/ImageView;Ljava/lang/String;ZZ)V",
                VOID
            ),
            create_recycler: static_method!(
                env,
                &krabik_recyclers,
                "create",
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;JII)Landroid/view/ViewGroup;",
                OBJECT
            ),
            update_recycler: static_method!(
                env,
                &krabik_recyclers,
                "update",
                "(Landroid/view/ViewGroup;IIJ)V",
                VOID
            ),
            create_progress: static_method!(
                env,
                &krabik_views,
                "createProgress",
                "(Landroid/content/Context;)Landroid/widget/ProgressBar;",
                OBJECT
            ),
            update_progress: static_method!(
                env,
                &krabik_views,
                "updateProgress",
                "(Landroid/widget/ProgressBar;FI)V",
                VOID
            ),
            update_spinner: static_method!(
                env,
                &krabik_views,
                "updateSpinner",
                "(Landroid/widget/ProgressBar;I)V",
                VOID
            ),
            apply_style: static_method!(
                env,
                &krabik_style,
                "apply",
                "(Landroid/view/View;IZIIZZFFFFFZFIZFZFII)V",
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
            button_ctor: ctor!(env, &button, "(Landroid/content/Context;)V"),
            image_view_ctor: ctor!(env, &image_view, "(Landroid/content/Context;)V"),
            view_ctor: ctor!(env, &view, "(Landroid/content/Context;)V"),
            frame_layout_ctor: ctor!(env, &frame_layout, "(Landroid/content/Context;)V"),
            text_input_ctor: ctor!(
                env,
                &text_input,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            switch_ctor: ctor!(
                env,
                &switch,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            checkbox_ctor: ctor!(
                env,
                &checkbox,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            radio_ctor: ctor!(
                env,
                &radio_button,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            radio_group_ctor: ctor!(
                env,
                &radio_group,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            tab_bar_ctor: ctor!(
                env,
                &tab_bar,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            date_picker_ctor: ctor!(
                env,
                &date_picker,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            seek_bar_ctor: ctor!(
                env,
                &seek_bar,
                "(Landroid/content/Context;Lcom/example/krabik_ui_dsl/RustRootView;J)V"
            ),
            progress_bar_ctor: ctor!(env, &progress_bar, "(Landroid/content/Context;)V"),
            layout_params_ctor: ctor!(env, &layout_params, "(II)V"),
            frame_params_ctor: ctor!(env, &frame_params, "(II)V"),

            view,
            _view_group: view_group,
            text_view,
            button,
            image_view,
            frame_layout,
            text_input,
            switch,
            checkbox,
            radio_button,
            radio_group,
            tab_bar,
            date_picker,
            seek_bar,
            progress_bar,
            layout_params,
            frame_params,
            typeface,
            truncate_at,
            long,
            measure_spec,
            krabik_style,
            krabik_views,
            krabik_images,
            krabik_recyclers,
            _rust_root_view: rust_root_view,
        })
    }
}

static JNI: OnceLock<Jni> = OnceLock::new();

/// Returns the cached handles, loading them on first use.
pub(super) fn get(env: &mut Env<'_>) -> JniResult<&'static Jni> {
    if let Some(jni) = JNI.get() {
        return Ok(jni);
    }

    let loaded = Jni::load(env)?;
    // Two threads can only race the very first load, and either value is
    // equivalent, so whichever lands first wins.
    let _ = JNI.set(loaded);
    Ok(JNI.get().expect("jni handles"))
}
