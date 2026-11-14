//! C entry points called by the Swift host. They decode arguments and hand over to the session.

use crate::geometry::Size;
use crate::host::HostKind;
use crate::ios::controller::{IosController, IosSession};
use crate::ios::host::IosState;
use crate::ios::{guard, infallible, with_session};
use crate::style::ColorScheme;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2_ui_kit::UIView;
use std::collections::HashMap;
use std::ffi::c_void;
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
/// `root` must be a live `UIView` and the call must happen on the main thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_create(
    root: *mut c_void,
    width: f64,
    height: f64,
    dark: bool,
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
    let mut cx = mtm;
    guard("create", || infallible(session.render_all(&mut cx)));
    states().lock().expect("ios states").insert(handle, session);
    handle
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
