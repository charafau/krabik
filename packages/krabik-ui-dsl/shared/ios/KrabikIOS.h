// The C interface between the Swift host and the Rust framework.
//
// Shared by every app; each app's bridging header includes this file and adds
// the declaration of its own install function.

#ifndef KRABIK_IOS_H
#define KRABIK_IOS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Creates the Rust renderer state for a root view. `dark` says whether the
/// device shows its dark appearance. Returns 0 on failure.
uint64_t krabik_ios_create(void *root, double width, double height, bool dark);

/// Reports a switch between the light and the dark appearance.
void krabik_ios_set_color_scheme(uint64_t handle, bool dark);

/// Rebuilds the tree for the given host viewport and applies the changes.
void krabik_ios_render(uint64_t handle, uint64_t host, double width, double height);

/// Attaches a child host view created by the native controller.
void krabik_ios_attach_host(uint64_t handle, uint64_t host, void *root);

/// Drops one host after its native view is destroyed.
void krabik_ios_destroy_host(uint64_t handle, uint64_t host);

/// Registers the native navigation/modal controller.
void krabik_ios_set_controller(uint64_t handle, void *controller);

/// Releases every widget and the Rust state.
void krabik_ios_destroy(uint64_t handle);

/// Reports that the user dismissed the sheet natively.
void krabik_ios_on_modal_dismiss(uint64_t handle);

/// Reports that the system alert closed by the button at `button`, or
/// without a button when `button` is negative.
void krabik_ios_on_dialog_action(uint64_t handle, int64_t button);

/// Reports a selection in the host-level tab bar.
void krabik_ios_on_bottom_navigation_selected(uint64_t handle, uint64_t index);

/// The height, in points, of the content a host rendered last.
double krabik_ios_content_height(uint64_t handle, uint64_t host);

/// Dispatches a native back action. Returns whether the application handled it.
bool krabik_ios_on_back(uint64_t handle);

/// Tells Rust whether the app is in the foreground; timers only run while it is.
void krabik_ios_set_active(uint64_t handle, bool active);

/// The user completed the swipe-back gesture: the top screen is already gone.
void krabik_ios_on_native_pop(uint64_t handle);

/// Reports a tap on the toolbar item at `index`.
void krabik_ios_on_toolbar_action(uint64_t handle, uint64_t index);

/// What `krabik_ios_on_chrome_event` can report. The numbers are defined by
/// `ChromeEvent` in Rust; a Rust test compares this list with them.
enum {
    /// The floating action button was pressed.
    KrabikChromeEventFab = 0,
    /// A drawer destination was picked; `index` says which.
    KrabikChromeEventDrawerSelected = 1,
    /// The user opened the drawer.
    KrabikChromeEventDrawerOpened = 2,
    /// The user closed the drawer.
    KrabikChromeEventDrawerClosed = 3,
    /// The snackbar's action was pressed.
    KrabikChromeEventSnackbarAction = 4,
    /// The snackbar timed out or was swiped away.
    KrabikChromeEventSnackbarDismissed = 5,
};

/// Reports what the user did to the floating action button, the drawer or
/// the snackbar; `kind` is one of the `KrabikChromeEvent` values above.
void krabik_ios_on_chrome_event(uint64_t handle, int64_t kind, int64_t index);

#ifdef __cplusplus
}
#endif

#endif
