//! `extern "system"` entry points called by the Java `AndroidBridge`.
//!
//! Every entry point funnels through [`run`], which guarantees that Rust errors and panics are
//! logged instead of unwinding into the JVM.

use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

use jni::objects::JObject;
use jni::sys::jboolean;
use jni::{Env, EnvUnowned, errors::ThrowRuntimeExAndDefault};

use krabik_ui_android::env::{discard_pending_exception, log_error};
use krabik_ui_android::error::Result;

use super::{build_ui, clear_state, handle_back, handle_click};

/// Runs a native operation, returning `T::default()` when it fails.
fn run<'local, T, F>(unowned_env: &mut EnvUnowned<'local>, operation: &str, body: F) -> T
where
    T: Default,
    F: FnOnce(&mut Env<'local>) -> Result<T>,
{
    unowned_env
        .with_env(|env| -> Result<T> {
            // A Rust panic must never unwind across the JNI boundary into the JVM.
            let outcome = catch_unwind(AssertUnwindSafe(|| body(env)));

            Ok(match outcome {
                Ok(Ok(value)) => value,
                Ok(Err(error)) => {
                    discard_pending_exception(env);
                    log_error(env, &format!("{operation} failed: {error}"));
                    T::default()
                }
                Err(payload) => {
                    discard_pending_exception(env);
                    log_error(
                        env,
                        &format!("{operation} panicked: {}", panic_message(&payload)),
                    );
                    T::default()
                }
            })
        })
        .resolve::<ThrowRuntimeExAndDefault>()
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic payload")
}

/// Entry point called from `AndroidBridge.createUi`.
///
/// The Java bridge passes the activity (the `Context` for the widgets) and a minimal listener
/// adapter that forwards clicks to `nativeOnClick`.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1example_AndroidBridge_nativeCreateUi<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    activity: JObject<'caller>,
    listener: JObject<'caller>,
) {
    run(&mut unowned_env, "nativeCreateUi", |env| {
        build_ui(env, &activity, &listener)
    });
}

/// Entry point invoked by the Java `View.OnClickListener` adapter.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1example_AndroidBridge_nativeOnClick<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    view: JObject<'caller>,
) {
    run(&mut unowned_env, "nativeOnClick", |env| {
        handle_click(env, &view)
    });
}

/// Entry point invoked by `MainActivity.onBackPressed`.
///
/// Returns `true` when Rust handled the back press (by returning to the main list) and `false`
/// when the activity should handle it normally.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1example_AndroidBridge_nativeBack<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) -> jboolean {
    run(&mut unowned_env, "nativeBack", handle_back)
}

/// Releases the global references held for the activity, listener, and status view when the
/// activity goes away.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1example_AndroidBridge_nativeDestroyUi<
    'caller,
>(
    mut unowned_env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    run(&mut unowned_env, "nativeDestroyUi", |_env| {
        clear_state();
        Ok(())
    });
}
