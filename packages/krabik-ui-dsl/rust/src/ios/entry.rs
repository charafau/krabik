//! C entry points called by the Swift host. They decode arguments and hand over to the session.

use crate::geometry::Size;
use crate::host::ChromeEvent;
use crate::host::HostKind;
use crate::ios::controller::{IosController, IosSession};
use crate::ios::host::IosState;
use crate::ios::{guard, infallible, schedule, with_session};
use crate::style::ColorScheme;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2_ui_kit::UIView;
use std::collections::HashMap;
use std::ffi::{CStr, c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub(crate) static STATES: OnceLock<Mutex<HashMap<u64, IosSession>>> = OnceLock::new();
pub(crate) static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);

pub(crate) fn states() -> &'static Mutex<HashMap<u64, IosSession>> {
    STATES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Creates the Rust-side state for a `RustRootView`.
///
/// # Safety
///
/// `root` must be a live `UIView`, `locale` must be null or a C string, and
/// the call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_create(
    root: *mut c_void,
    width: f64,
    height: f64,
    dark: bool,
    locale: *const c_char,
) -> u64 {
    let Some(mtm) = MainThreadMarker::new() else {
        return 0;
    };
    // SAFETY: the caller guarantees a live view for the duration of the call.
    let Some(root) = (unsafe { Retained::retain(root.cast::<UIView>()) }) else {
        return 0;
    };

    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    let mut main = IosState::new(handle, 0, HostKind::Main, &root, mtm);
    main.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
    let mut session = IosSession::new(crate::app::create_app(), main);
    session.set_color_scheme(scheme(dark));
    // SAFETY: the caller passes null or a C string that lives for the call.
    session.set_locale(&unsafe { language_tag(locale) });
    // Background work finishes on its own thread; the result is collected
    // on the main queue.
    session.set_waker(std::sync::Arc::new(move || schedule(move || drain(handle))));
    let mut cx = mtm;
    guard("create", || infallible(session.render_all(&mut cx)));
    states().lock().expect("ios states").insert(handle, session);
    handle
}

/// Reads a BCP 47 tag handed over as a C string; null reads as no tag.
///
/// # Safety
///
/// `locale` must be null or point to a C string that lives for the call.
unsafe fn language_tag(locale: *const c_char) -> String {
    if locale.is_null() {
        return String::new();
    }
    // SAFETY: not null, and a C string by the caller's guarantee.
    unsafe { CStr::from_ptr(locale) }
        .to_string_lossy()
        .into_owned()
}

/// Tells the session that the user's language changed while the app runs,
/// and renders what the application makes of it.
///
/// # Safety
///
/// `locale` must be null or a C string, and the call must happen on the main
/// thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_locale(handle: u64, locale: *const c_char) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    // SAFETY: the caller passes null or a C string that lives for the call.
    let locale = unsafe { language_tag(locale) };
    guard("locale", || {
        with_session(handle, |session| {
            session.set_locale(&locale);
            infallible(session.render_all(&mut mtm));
        });
    });
}

fn scheme(dark: bool) -> ColorScheme {
    if dark {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

/// Tells the session that the device switched between its light and dark
/// appearance, and repaints what depends on it.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_color_scheme(handle: u64, dark: bool) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("color scheme", || {
        with_session(handle, |session| {
            session.set_color_scheme(scheme(dark));
            infallible(session.render_all(&mut mtm));
        });
    });
}

/// Reports the factor the user's text size setting asks for (one for the
/// default size) and draws every text again in the new size.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_text_scale(handle: u64, scale: f64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("text scale", || {
        with_session(handle, |session| {
            session.set_text_scale(scale as f32);
            infallible(session.render_all(&mut mtm));
        });
    });
}

/// Rebuilds the tree for the new viewport and applies the resulting operations.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_render(handle: u64, host: u64, width: f64, height: f64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("render", || {
        with_session(handle, |session| {
            if let Some(state) = session.host_mut(host) {
                state.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
            }
            infallible(session.render_all(&mut mtm));
        });
    });
}

/// Applies the messages background work has delivered.
fn drain(handle: u64) {
    // SAFETY: only ever scheduled on the main queue.
    let mut mtm = unsafe { MainThreadMarker::new_unchecked() };
    guard("wake", || {
        with_session(handle, |session| infallible(session.drain(&mut mtm)));
    });
}

/// The app came to the foreground or left it. Subscriptions only run in the
/// foreground.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_active(handle: u64, active: bool) {
    guard("set active", || {
        with_session(handle, |session| session.set_active(active));
    });
}

/// Releases every widget and the Rust state of a scene.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_destroy(handle: u64) {
    let Some(mut session) = states().lock().expect("ios states").remove(&handle) else {
        return;
    };
    for host in session.hosts_mut() {
        for view in host.views.values() {
            view.view.removeFromSuperview();
        }
    }
}

/// Attaches a new native host view to a session.
///
/// # Safety
///
/// `root` must be a live `UIView` and the call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_attach_host(handle: u64, host: u64, root: *mut c_void) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    // SAFETY: the caller guarantees a live view for the duration of the call.
    let Some(root) = (unsafe { Retained::retain(root.cast::<UIView>()) }) else {
        return;
    };
    guard("attach host", || {
        with_session(handle, |session| {
            infallible(session.attach_host(host, |kind| {
                Ok(IosState::new(handle, host, kind, &root, mtm))
            }));
            infallible(session.render_all(&mut mtm));
        });
    });
}

/// Drops one host after its native view is destroyed.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_destroy_host(handle: u64, host: u64) {
    if MainThreadMarker::new().is_none() {
        return;
    }
    guard("destroy host", || {
        with_session(handle, |session| {
            session.remove_host(host);
        });
    });
}

/// Registers the native navigation/modal controller.
///
/// # Safety
///
/// `controller` must be a live Objective-C object and the call must happen on
/// the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_set_controller(handle: u64, controller: *mut c_void) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    // SAFETY: the caller guarantees a live Objective-C object.
    let Some(controller) = (unsafe { Retained::retain(controller.cast::<NSObject>()) }) else {
        return;
    };
    guard("controller", || {
        with_session(handle, |session| {
            session.set_controller(IosController(controller));
            infallible(session.render_all(&mut mtm));
        });
    });
}

/// Dispatches the native dismissal of the bottom sheet (swipe or scrim tap).
///
/// The platform has already closed the sheet, so the session forgets it
/// first: if the application still returns a sheet, it is presented again.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_modal_dismiss(handle: u64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("sheet dismiss", || {
        with_session(handle, |session| {
            infallible(session.sheet_dismissed(&mut mtm))
        });
    });
}

/// Dispatches a button press of the system alert.
///
/// The platform has already closed the alert, so the session forgets it
/// first: if the application still returns a dialog, it is shown again.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_dialog_action(handle: u64, button: i64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    // A negative index means the alert closed without a button.
    let button = usize::try_from(button).ok();
    guard("dialog action", || {
        with_session(handle, |session| {
            infallible(session.dialog_closed(&mut mtm, button));
        });
    });
}

/// Dispatches a selection in the host-level bottom navigation bar.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_bottom_navigation_selected(handle: u64, index: u64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("bottom navigation", || {
        with_session(handle, |session| {
            infallible(session.bottom_navigation_selected(&mut mtm, index as usize));
        });
    });
}

/// The height, in points, of the content a host rendered last.
///
/// Lets a content-sized native container, such as a sheet, wrap it.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_content_height(handle: u64, host: u64) -> f64 {
    if MainThreadMarker::new().is_none() {
        return 0.0;
    }
    with_session(handle, |session| {
        session
            .host(host)
            .map_or(0.0, |state| f64::from(state.runtime.content_height()))
    })
    .unwrap_or(0.0)
}

/// Dispatches a native toolbar action into the application.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_toolbar_action(handle: u64, index: u64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("toolbar action", || {
        with_session(handle, |session| {
            infallible(session.toolbar_action(&mut mtm, index as usize));
        });
    });
}

/// Dispatches the native back action to the application.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_back(handle: u64) -> bool {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return false;
    };
    let mut handled = false;
    guard("back", || {
        handled =
            with_session(handle, |session| infallible(session.back(&mut mtm))).unwrap_or(false);
    });
    handled
}

/// The user completed the swipe-back gesture; UIKit already removed the top
/// screen.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_native_pop(handle: u64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("native pop", || {
        with_session(handle, |session| infallible(session.native_pop(&mut mtm)));
    });
}

/// The user acted on the floating action button, the drawer or the snackbar.
///
/// `kind` and `index` are decoded by `ChromeEvent::from_code`.
///
/// # Safety
///
/// The call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_on_chrome_event(handle: u64, kind: i64, index: i64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(event) = ChromeEvent::from_code(kind, index) else {
        return;
    };
    guard("chrome event", || {
        with_session(handle, |session| {
            infallible(session.chrome_event(&mut mtm, event));
        });
    });
}
