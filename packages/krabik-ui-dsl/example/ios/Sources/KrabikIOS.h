#ifndef KRABIK_IOS_H
#define KRABIK_IOS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Installs the example app as the global UI factory. Call once at startup,
/// before any root view is created.
void krabik_ui_dsl_example_install(void);

/// Creates the Rust renderer state for a root view. Returns 0 on failure.
uint64_t krabik_ios_create(void *root, double width, double height);

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

/// Reports a button press of the system alert.
void krabik_ios_on_dialog_action(uint64_t handle, const char *action);

/// Reports a selection in the host-level tab bar.
void krabik_ios_on_bottom_navigation_selected(uint64_t handle, uint64_t index);

/// The height, in points, of the content a host rendered last.
double krabik_ios_content_height(uint64_t handle, uint64_t host);

/// Dispatches a native back action. Returns whether the application handled it.
bool krabik_ios_on_back(uint64_t handle);

/// Dispatches a toolbar action.
void krabik_ios_on_toolbar_action(uint64_t handle, const char *action);

#ifdef __cplusplus
}
#endif

#endif
