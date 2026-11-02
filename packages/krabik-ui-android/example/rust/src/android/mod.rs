//! Direct JNI bridge that creates and updates the Android view hierarchy from Rust.
//!
//! `AndroidBridge` only loads the native library, hands the activity and a click-listener adapter
//! to this module, and forwards button clicks and back presses back through the exported native
//! methods in [`ffi`].
//!
//! The app is a widget browser built entirely from Rust. The main screen is a scrollable list of
//! buttons, one per supported Android widget. Tapping a button replaces the content view with a
//! demo screen for that widget; the demo's Back button and the system back button both return to
//! the main list.
//!
//! Every widget is a real Android framework `View` created and configured through JNI. There are no
//! XML layouts, Compose screens, WebViews, or custom renderers.
//!
//! # Layout
//!
//! - [`constants`]: JNI class names, resource names, and Android constants.
//! - [`env`]: fallible JNI helpers and Logcat output.
//! - [`ui`]: layout primitives, widget components, and the declarative element tree.
//! - [`screens`]: the element trees for the main screen and the widget demos.
//! - [`views`]: creation and configuration of the native views.
//! - [`render`]: translates an element tree into a live view hierarchy.
//! - [`widget`]: the supported widgets and the actions a button can trigger.
//! - [`ffi`]: the `extern "system"` entry points called by Java.

mod adapter;
mod ffi;
mod screens;
mod widget;

use std::sync::{Mutex, MutexGuard, PoisonError};

use jni::objects::{JObject, JValue};
use jni::refs::Global;
use jni::strings::JNIStr;
use jni::{Env, jni_sig, jni_str};
use krabik_ui_android::constants::{ACTIVITY_CLASS, ON_CLICK_LISTENER_CLASS};
use krabik_ui_android::env::EnvExt;
use krabik_ui_android::error::{Error, Result};
use krabik_ui_android::material;
use krabik_ui_android::render::Renderer;
use krabik_ui_android::ui::Metrics;
use krabik_ui_android::views::ViewExt;

use widget::{Action, WidgetKind};

const SCROLL_RESTORER_CLASS: &JNIStr = jni_str!("com/example/krabik_ui_example/ScrollRestorer");

/// The currently displayed screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Main,
    Demo(WidgetKind),
}

/// State that has to survive between `nativeCreateUi`, navigation clicks, and back presses.
///
/// The activity and listener are held as global references so a later click can rebuild the
/// content view. Dropping the state releases those references in `nativeDestroyUi`, before the
/// activity is destroyed.
struct UiState {
    activity: Global<JObject<'static>>,
    listener: Global<JObject<'static>>,
    metrics: Metrics,
    screen: Screen,
    root: Option<Global<JObject<'static>>>,
    main_scroll_y: i32,
    button_taps: u32,
    button_status: Option<Global<JObject<'static>>>,
}

static UI_STATE: Mutex<Option<UiState>> = Mutex::new(None);

fn lock_state() -> MutexGuard<'static, Option<UiState>> {
    // A panic while the lock is held poisons it. The state is still structurally valid, so recover
    // rather than making every subsequent click fail.
    UI_STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Runs `body` against the live UI state.
fn with_state<T>(body: impl FnOnce(&mut UiState) -> Result<T>) -> Result<T> {
    let mut guard = lock_state();
    let state = guard
        .as_mut()
        .ok_or_else(|| Error::message("the Rust UI is not initialized"))?;
    body(state)
}

fn clear_state() {
    *lock_state() = None;
}

fn build_ui(env: &mut Env<'_>, activity: &JObject<'_>, listener: &JObject<'_>) -> Result<()> {
    // Sanity-check the two objects handed to us by the Java bridge before passing them to
    // framework APIs.
    require_instance(env, activity, ACTIVITY_CLASS, "an Activity")?;
    require_instance(
        env,
        listener,
        ON_CLICK_LISTENER_CLASS,
        "a View.OnClickListener",
    )?;

    let mut state = UiState {
        activity: env.global_ref(activity)?,
        listener: env.global_ref(listener)?,
        metrics: Metrics::from_context(env, activity)?,
        screen: Screen::Main,
        root: None,
        main_scroll_y: 0,
        button_taps: 0,
        button_status: None,
    };

    show_screen(env, &mut state, Screen::Main)?;

    // Replace any state left over from a previous activity instance.
    *lock_state() = Some(state);

    Ok(())
}

fn require_instance(
    env: &mut Env<'_>,
    object: &JObject<'_>,
    class_name: &JNIStr,
    expected: &str,
) -> Result<()> {
    if env.instance_of(object, class_name)? {
        Ok(())
    } else {
        Err(Error::message(format!(
            "nativeCreateUi expected {expected}"
        )))
    }
}

/// Renders `screen` and installs it as the activity's content view.
fn show_screen(env: &mut Env<'_>, state: &mut UiState, screen: Screen) -> Result<()> {
    // Remember where the widget list was scrolled before leaving it, so returning from a demo
    // lands on the same row instead of jumping back to the top.
    if state.screen == Screen::Main
        && let Some(root) = &state.root
    {
        state.main_scroll_y =
            env.call_int(root.as_obj(), jni_str!("getScrollY"), jni_sig!("()I"), &[])?;
    }

    let tree = screens::element(screen);

    let (root, status) = {
        let mut renderer = Renderer::new(
            env,
            state.activity.as_obj(),
            state.listener.as_obj(),
            state.metrics,
        )
        .with_item_adapter_factory(&adapter::TEXT_ADAPTER_FACTORY);
        let root = renderer.render(&tree)?;
        (root, renderer.take_status())
    };

    env.call_void(
        state.activity.as_obj(),
        jni_str!("setContentView"),
        jni_sig!("(Landroid/view/View;)V"),
        &[JValue::Object(&root)],
    )?;

    // `ScrollView` clamps `scrollTo` to zero until it has measured its child, so let the Java
    // helper apply the offset after the first layout pass.
    if screen == Screen::Main && state.main_scroll_y > 0 {
        env.call_static_void(
            SCROLL_RESTORER_CLASS,
            jni_str!("restore"),
            jni_sig!("(Landroid/view/View;I)V"),
            &[JValue::Object(&root), JValue::Int(state.main_scroll_y)],
        )?;
    }

    state.screen = screen;
    state.root = Some(env.global_ref(&root)?);
    state.button_status = status;

    Ok(())
}

fn navigate(env: &mut Env<'_>, target: Screen) -> Result<()> {
    with_state(|state| {
        if state.screen == target {
            return Ok(());
        }

        if target == Screen::Demo(WidgetKind::Button) {
            state.button_taps = 0;
        }

        show_screen(env, state, target)
    })
}

fn handle_click(env: &mut Env<'_>, view: &JObject<'_>) -> Result<()> {
    let Some(action) = krabik_ui_android::views::view_tag(env, view)?
        .as_deref()
        .and_then(Action::from_tag)
    else {
        return Ok(());
    };

    match action {
        Action::Tap => with_state(|state| {
            state.button_taps = state.button_taps.saturating_add(1);

            if let Some(status) = &state.button_status {
                // The click is delivered on Android's main thread, so mutating the view directly
                // is safe.
                env.set_text(
                    status.as_obj(),
                    &format!("Tapped {} times", state.button_taps),
                )?;
            }

            Ok(())
        }),
        Action::Back => navigate(env, Screen::Main),
        Action::Open(kind) => navigate(env, Screen::Demo(kind)),
        Action::ShowSnackbar => material::show_snackbar(env, view, "Saved to your library", false),
        Action::ShowAlertDialog => with_state(|state| {
            material::show_alert_dialog(
                env,
                state.activity.as_obj(),
                "Delete item?",
                "This action cannot be undone.",
            )
        }),
        Action::ShowDatePicker => with_state(|state| {
            material::show_date_picker(env, state.activity.as_obj(), "Pick a date")
        }),
        Action::ShowTimePicker => with_state(|state| {
            material::show_time_picker(env, state.activity.as_obj(), "Pick a time")
        }),
    }
}

fn handle_back(env: &mut Env<'_>) -> Result<bool> {
    with_state(|state| {
        if state.screen == Screen::Main {
            return Ok(false);
        }

        show_screen(env, state, Screen::Main)?;
        Ok(true)
    })
}
