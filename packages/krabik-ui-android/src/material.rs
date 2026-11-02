//! Material Components helpers that are not part of the declarative view tree.
//!
//! Dialogs, snackbars, badges, tooltips, and dynamic colours are actions rather than views, so they
//! are exposed as functions. Every function takes borrowed JNI objects owned by the caller and
//! leaves long-lived references to the Android side.
//!
//! The consuming app must depend on `com.google.android.material:material` and use a Material
//! Components theme.

use jni::objects::{JObject, JValue};
use jni::strings::JNIStr;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use crate::constants::*;
use crate::env::EnvExt;
use crate::error::Result;
use crate::views::ViewExt;

/// Shows a `Snackbar` anchored to `parent`.
pub fn show_snackbar(
    env: &mut Env<'_>,
    parent: &JObject<'_>,
    text: &str,
    long: bool,
) -> Result<()> {
    let text = env.java_string(text)?;
    let duration = if long {
        MATERIAL_SNACKBAR_LENGTH_LONG
    } else {
        MATERIAL_SNACKBAR_LENGTH_SHORT
    };

    let snackbar = env.call_static_object(
        MATERIAL_SNACKBAR_CLASS,
        jni_str!("make"),
        jni_sig!(
            "(Landroid/view/View;Ljava/lang/CharSequence;I)Lcom/google/android/material/snackbar/Snackbar;"
        ),
        &[
            JValue::Object(parent),
            JValue::Object(text.as_ref()),
            JValue::Int(duration),
        ],
    )?;

    env.call_void(&snackbar, jni_str!("show"), jni_sig!("()V"), &[])
}

/// Shows a `MaterialAlertDialogBuilder` with a title, message, and OK button.
pub fn show_alert_dialog(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    title: &str,
    message: &str,
) -> Result<()> {
    let builder = env.construct(
        MATERIAL_ALERT_DIALOG_BUILDER_CLASS,
        jni_sig!("(Landroid/content/Context;)V"),
        &[JValue::Object(context)],
    )?;
    let title = env.java_string(title)?;
    let message = env.java_string(message)?;
    let ok = env.java_string("OK")?;

    let builder = env.call_object(
        &builder,
        jni_str!("setTitle"),
        jni_sig!(
            "(Ljava/lang/CharSequence;)Lcom/google/android/material/dialog/MaterialAlertDialogBuilder;"
        ),
        &[JValue::Object(title.as_ref())],
    )?;
    let builder = env.call_object(
        &builder,
        jni_str!("setMessage"),
        jni_sig!(
            "(Ljava/lang/CharSequence;)Lcom/google/android/material/dialog/MaterialAlertDialogBuilder;"
        ),
        &[JValue::Object(message.as_ref())],
    )?;
    let builder = env.call_object(
        &builder,
        jni_str!("setPositiveButton"),
        jni_sig!(
            "(Ljava/lang/CharSequence;Landroid/content/DialogInterface$OnClickListener;)Lcom/google/android/material/dialog/MaterialAlertDialogBuilder;"
        ),
        &[JValue::Object(ok.as_ref()), JValue::Object(&JObject::null())],
    )?;
    env.call_object(
        &builder,
        jni_str!("show"),
        jni_sig!("()Landroidx/appcompat/app/AlertDialog;"),
        &[],
    )?;

    Ok(())
}

/// Shows a Material date picker using `context`'s support `FragmentManager`.
///
/// `context` must be a `FragmentActivity` (for example an `AppCompatActivity`).
pub fn show_date_picker(env: &mut Env<'_>, context: &JObject<'_>, title: &str) -> Result<()> {
    let builder = env.call_static_object(
        MATERIAL_DATE_PICKER_BUILDER_CLASS,
        jni_str!("datePicker"),
        jni_sig!("()Lcom/google/android/material/datepicker/MaterialDatePicker$Builder;"),
        &[],
    )?;
    let title = env.java_string(title)?;
    let builder = env.call_object(
        &builder,
        jni_str!("setTitleText"),
        jni_sig!(
            "(Ljava/lang/CharSequence;)Lcom/google/android/material/datepicker/MaterialDatePicker$Builder;"
        ),
        &[JValue::Object(title.as_ref())],
    )?;
    let picker = env.call_object(
        &builder,
        jni_str!("build"),
        jni_sig!("()Lcom/google/android/material/datepicker/MaterialDatePicker;"),
        &[],
    )?;
    show_picker(env, context, &picker, "date-picker")
}

/// Shows a Material time picker using `context`'s support `FragmentManager`.
///
/// `context` must be a `FragmentActivity` (for example an `AppCompatActivity`).
pub fn show_time_picker(env: &mut Env<'_>, context: &JObject<'_>, title: &str) -> Result<()> {
    let builder = env.construct(MATERIAL_TIME_PICKER_BUILDER_CLASS, jni_sig!("()V"), &[])?;
    let title = env.java_string(title)?;
    let builder = env.call_object(
        &builder,
        jni_str!("setTitleText"),
        jni_sig!(
            "(Ljava/lang/CharSequence;)Lcom/google/android/material/timepicker/MaterialTimePicker$Builder;"
        ),
        &[JValue::Object(title.as_ref())],
    )?;
    let picker = env.call_object(
        &builder,
        jni_str!("build"),
        jni_sig!("()Lcom/google/android/material/timepicker/MaterialTimePicker;"),
        &[],
    )?;
    show_picker(env, context, &picker, "time-picker")
}

fn show_picker(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    picker: &JObject<'_>,
    tag: &str,
) -> Result<()> {
    let fragment_manager = env.call_object(
        context,
        jni_str!("getSupportFragmentManager"),
        jni_sig!("()Landroidx/fragment/app/FragmentManager;"),
        &[],
    )?;
    let tag = env.java_string(tag)?;
    env.call_void(
        picker,
        jni_str!("show"),
        jni_sig!("(Landroidx/fragment/app/FragmentManager;Ljava/lang/String;)V"),
        &[
            JValue::Object(&fragment_manager),
            JValue::Object(tag.as_ref()),
        ],
    )
}

/// Attaches a numbered `BadgeDrawable` to `anchor`.
pub fn attach_badge(
    env: &mut Env<'_>,
    context: &JObject<'_>,
    anchor: &JObject<'_>,
    number: jint,
) -> Result<()> {
    let badge = env.call_static_object(
        MATERIAL_BADGE_DRAWABLE_CLASS,
        jni_str!("create"),
        jni_sig!("(Landroid/content/Context;)Lcom/google/android/material/badge/BadgeDrawable;"),
        &[JValue::Object(context)],
    )?;
    env.call_void(
        &badge,
        jni_str!("setNumber"),
        jni_sig!("(I)V"),
        &[JValue::Int(number)],
    )?;
    env.call_void(
        &badge,
        jni_str!("setVisible"),
        jni_sig!("(Z)V"),
        &[JValue::Bool(true)],
    )?;
    env.call_static_void(
        MATERIAL_BADGE_UTILS_CLASS,
        jni_str!("attachBadgeDrawable"),
        jni_sig!("(Lcom/google/android/material/badge/BadgeDrawable;Landroid/view/View;)V"),
        &[JValue::Object(&badge), JValue::Object(anchor)],
    )
}

/// Applies Material You dynamic colours to `activity` when the platform supports them.
pub fn apply_dynamic_colors(env: &mut Env<'_>, activity: &JObject<'_>) -> Result<()> {
    env.call_static_void(
        MATERIAL_DYNAMIC_COLORS_CLASS,
        jni_str!("applyToActivityIfAvailable"),
        jni_sig!("(Landroid/app/Activity;)V"),
        &[JValue::Object(activity)],
    )
}

/// Sets a tooltip on `view` through `TooltipCompat`.
pub fn set_tooltip(env: &mut Env<'_>, view: &JObject<'_>, text: &str) -> Result<()> {
    let text = env.java_string(text)?;
    env.call_static_void(
        MATERIAL_TOOLTIP_COMPAT_CLASS,
        jni_str!("setTooltipText"),
        jni_sig!("(Landroid/view/View;Ljava/lang/CharSequence;)V"),
        &[JValue::Object(view), JValue::Object(text.as_ref())],
    )
}

/// Adds a titled item to a `Menu` for callers that build navigation menus themselves.
pub fn add_menu_item<'local>(
    env: &mut Env<'local>,
    menu: &JObject<'_>,
    item_id: jint,
    title: &str,
    icon: Option<&JNIStr>,
) -> Result<JObject<'local>> {
    let item = crate::views::add_menu_item(env, menu, item_id, title)?;
    if let Some(icon) = icon {
        let resource = env.static_int(DRAWABLE_CLASS, icon)?;
        env.set_menu_item_icon(&item, resource)?;
    }
    Ok(item)
}
