//! JNI entry points called by `RustRootView`. They decode arguments and hand over to the session.

use crate::android::controller::{Android, AndroidController};
use crate::android::handles::get as jni_handles;
use crate::android::host::AndroidState;
use crate::android::run;
use crate::geometry::Size;
use crate::host::{HostKind, NodeEvent, Session};
use crate::tree::NodeId;
use crate::ui::{DatePickerValue, DateValue, TimeValue};
use jni::errors::Result as JniResult;
use jni::objects::{JObject, JString};
use jni::refs::Global;
use jni::sys::{jboolean, jfloat, jint, jlong};
use jni::{Env, EnvUnowned, JValue, jni_sig, jni_str};
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// One Activity's session plus what Android needs to create more hosts.
pub(crate) struct AndroidSession {
    pub(crate) core: Session<Android>,
    pub(crate) context: Global<JObject<'static>>,
    pub(crate) density: f32,
}

pub(crate) static STATES: OnceLock<Mutex<HashMap<jlong, AndroidSession>>> = OnceLock::new();
pub(crate) static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

pub(crate) fn states() -> &'static Mutex<HashMap<jlong, AndroidSession>> {
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
pub(crate) fn lock_states_for_event() -> Option<MutexGuard<'static, HashMap<jlong, AndroidSession>>>
{
    states().try_lock().ok()
}

/// Creates the Rust-side state for a `RustRootView`.
#[unsafe(export_name = native_name!("nativeCreate"))]
pub extern "system" fn native_create<'caller>(
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
#[unsafe(export_name = native_name!("nativeContentHeight"))]
pub extern "system" fn native_content_height<'caller>(
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
#[unsafe(export_name = native_name!("nativeRender"))]
pub extern "system" fn native_render<'caller>(
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
pub(crate) fn on_event(
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
pub(crate) fn on_node_event(
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
#[unsafe(export_name = native_name!("nativeOnClick"))]
pub extern "system" fn native_on_click<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnTextChanged"))]
pub extern "system" fn native_on_text_changed<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnSubmitted"))]
pub extern "system" fn native_on_submitted<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnFocusChanged"))]
pub extern "system" fn native_on_focus_changed<'caller>(
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
#[unsafe(export_name = native_name!("nativeRestoreRowFocus"))]
pub extern "system" fn native_restore_row_focus<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnCheckedChanged"))]
pub extern "system" fn native_on_checked_changed<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnSliderChanged"))]
pub extern "system" fn native_on_slider_changed<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnPickerChanged"))]
pub extern "system" fn native_on_picker_changed<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnRadioGroupSelected"))]
pub extern "system" fn native_on_radio_group_selected<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnPageSelected"))]
pub extern "system" fn native_on_page_selected<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnTabSelected"))]
pub extern "system" fn native_on_tab_selected<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnScroll"))]
pub extern "system" fn native_on_scroll<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnToolbarAction"))]
pub extern "system" fn native_on_toolbar_action<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnDialogAction"))]
pub extern "system" fn native_on_dialog_action<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnBottomNavigationSelected"))]
pub extern "system" fn native_on_bottom_navigation_selected<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnModalDismiss"))]
pub extern "system" fn native_on_modal_dismiss<'caller>(
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
#[unsafe(export_name = native_name!("nativeOnBack"))]
pub extern "system" fn native_on_back<'caller>(
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
#[unsafe(export_name = native_name!("nativeBindRow"))]
pub extern "system" fn native_bind_row<'caller>(
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
#[unsafe(export_name = native_name!("nativeUnbindRow"))]
pub extern "system" fn native_unbind_row<'caller>(
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
#[unsafe(export_name = native_name!("nativeDestroy"))]
pub extern "system" fn native_destroy<'caller>(
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
#[unsafe(export_name = native_name!("nativeAttachHost"))]
pub extern "system" fn native_attach_host<'caller>(
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
#[unsafe(export_name = native_name!("nativeSetController"))]
pub extern "system" fn native_set_controller<'caller>(
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
#[unsafe(export_name = native_name!("nativeDestroyHost"))]
pub extern "system" fn native_destroy_host<'caller>(
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
