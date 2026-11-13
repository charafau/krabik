//! Android renderer driven directly through JNI.
//!
//! The Kotlin side is a thin `ViewGroup`: it forwards its size to Rust and
//! forwards clicks back. Every widget, every property update, and every frame
//! change is performed here on native `TextView`/`Button` instances.

use jni::errors::{Result as JniResult, ThrowRuntimeExAndDefault};
use jni::strings::JNIStr;
use jni::sys::jint;
use jni::{Env, EnvUnowned, jni_str};

// The Kotlin package of the framework's Android classes (`shared/android`).
// It is spelled here and nowhere else in Rust: once with slashes for class
// lookups, once mangled for the names of the native methods. Renaming the
// package means changing these lines and the Kotlin sources.

/// The JNI name of a framework class, such as `dev/krabik/ui/RustRootView`.
macro_rules! framework_class {
    ($name:literal) => {
        jni::jni_str!("dev/krabik/ui/", $name)
    };
}

/// The symbol the JVM looks up for a `native` method of `RustRootView`.
macro_rules! native_name {
    ($method:literal) => {
        concat!("Java_dev_krabik_ui_RustRootView_", $method)
    };
}

/// The descriptor of `RustRootView` inside a method signature.
pub(crate) const ROOT_VIEW_DESCRIPTOR: &str = "Ldev/krabik/ui/RustRootView;";

/// Calls a JNI method and turns a pending Java exception into a Rust error.
macro_rules! jcall {
    ($env:expr, $call:expr) => {{
        let value = $call;
        $crate::android::checked($env, value)
    }};
}

/// Builds a Java `String[]` from string slices.
macro_rules! string_array {
    ($env:expr, $values:expr) => {{
        let values: Vec<&str> = $values.collect();
        let array = $env.new_object_array(
            values.len() as jni::sys::jint,
            jni::jni_str!("java/lang/String"),
            jni::objects::JObject::null(),
        )?;
        for (index, value) in values.into_iter().enumerate() {
            let text = jcall!($env, $env.new_string(value))?;
            let text: &jni::objects::JObject<'_> = text.as_ref();
            array.set_element($env, index, text)?;
        }
        array
    }};
}

mod controller;
mod entry;
mod handles;
mod host;
mod measure;
mod rows;
mod style;
mod widgets;

pub(crate) const TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/TextView");
pub(crate) const BUTTON_CLASS: &JNIStr = jni_str!("android/widget/Button");
pub(crate) const FRAME_LAYOUT_CLASS: &JNIStr = jni_str!("android/widget/FrameLayout");
pub(crate) const IMAGE_VIEW_CLASS: &JNIStr = jni_str!("android/widget/ImageView");
pub(crate) const VIEW_CLASS: &JNIStr = jni_str!("android/view/View");
pub(crate) const TYPEFACE_CLASS: &JNIStr = jni_str!("android/graphics/Typeface");
pub(crate) const TEXT_UTILS_CLASS: &JNIStr = jni_str!("android/text/TextUtils$TruncateAt");
pub(crate) const LONG_CLASS: &JNIStr = jni_str!("java/lang/Long");
pub(crate) const MEASURE_SPEC_CLASS: &JNIStr = jni_str!("android/view/View$MeasureSpec");
pub(crate) const LAYOUT_PARAMS_CLASS: &JNIStr = jni_str!("android/view/ViewGroup$LayoutParams");
/// ScrollView casts its direct child's params to `MarginLayoutParams`, so every
/// sized view uses `FrameLayout.LayoutParams`, which is a safe superset.
pub(crate) const FRAME_PARAMS_CLASS: &JNIStr = jni_str!("android/widget/FrameLayout$LayoutParams");
pub(crate) const KRABIK_STYLE_CLASS: &JNIStr = framework_class!("KrabikStyle");
pub(crate) const RUST_ROOT_VIEW_CLASS: &JNIStr = framework_class!("RustRootView");
pub(crate) const TEXT_INPUT_CLASS: &JNIStr = framework_class!("KrabikTextInput");
pub(crate) const SWITCH_CLASS: &JNIStr = framework_class!("KrabikSwitch");
/// The framework switch, measured only for its intrinsic size.
pub(crate) const PLAIN_SWITCH_CLASS: &JNIStr = jni_str!("android/widget/Switch");
pub(crate) const CHECKBOX_CLASS: &JNIStr = framework_class!("KrabikCheckBox");
pub(crate) const RADIO_CLASS: &JNIStr = framework_class!("KrabikRadioButton");
pub(crate) const RADIO_GROUP_CLASS: &JNIStr = framework_class!("KrabikRadioGroup");
pub(crate) const TAB_BAR_CLASS: &JNIStr = framework_class!("KrabikTabBar");
pub(crate) const PAGER_CLASS: &JNIStr = framework_class!("KrabikPager");
pub(crate) const DATE_PICKER_CLASS: &JNIStr = framework_class!("KrabikDatePicker");
pub(crate) const SEEK_BAR_CLASS: &JNIStr = framework_class!("KrabikSeekBar");
pub(crate) const PROGRESS_BAR_CLASS: &JNIStr = jni_str!("android/widget/ProgressBar");
pub(crate) const KRABIK_IMAGES_CLASS: &JNIStr = framework_class!("KrabikImages");
pub(crate) const KRABIK_ROWS_CLASS: &JNIStr = framework_class!("KrabikRows");
pub(crate) const KRABIK_VIEWS_CLASS: &JNIStr = framework_class!("KrabikViews");
pub(crate) const KRABIK_RECYCLERS_CLASS: &JNIStr = framework_class!("KrabikRecyclers");

pub(crate) const MEASURE_UNSPECIFIED: jint = 0;
pub(crate) const MEASURE_AT_MOST: jint = -2_147_483_648;
pub(crate) const WRAP_CONTENT: jint = -2;

/// The unit expected by `TextView.setTextSize` for scaled pixels.
pub(crate) const COMPLEX_UNIT_SP: jint = 2;
/// `android.view.Gravity.CENTER`.
pub(crate) const GRAVITY_CENTER: jint = 17;
/// `android.view.Gravity.CENTER_HORIZONTAL`.
pub(crate) const GRAVITY_CENTER_HORIZONTAL: jint = 1;
/// `android.view.Gravity.START`.
pub(crate) const GRAVITY_START: jint = 0x0080_0003;
/// `android.view.Gravity.END`.
pub(crate) const GRAVITY_END: jint = 0x0080_0005;

pub(crate) fn checked<T>(env: &mut Env<'_>, result: JniResult<T>) -> JniResult<T> {
    let value = result?;
    env.exception_catch()?;
    Ok(value)
}

/// Executes a native operation and never lets a Rust panic reach the JVM.
pub(crate) fn run<'local, T, F>(unenv: &mut EnvUnowned<'local>, operation: &str, body: F) -> T
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
