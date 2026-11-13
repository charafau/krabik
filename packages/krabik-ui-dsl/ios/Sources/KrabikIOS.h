#ifndef KRABIK_IOS_H
#define KRABIK_IOS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

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

/// Asks the application to close the presented modal.
void krabik_ios_on_modal_dismiss(uint64_t handle);

/// Dispatches a native back action. Returns whether the application handled it.
bool krabik_ios_on_back(uint64_t handle);
void krabik_ios_on_toolbar_action(uint64_t handle, const char *action);

void krabik_ios_on_click(uint64_t handle, uint64_t host, uint64_t node_id);
void krabik_ios_on_text_changed(uint64_t handle, uint64_t host, uint64_t node_id, const char *text);
void krabik_ios_on_submitted(uint64_t handle, uint64_t host, uint64_t node_id);
void krabik_ios_on_focus_changed(uint64_t handle, uint64_t host, uint64_t node_id, bool focused);
void krabik_ios_on_checked_changed(uint64_t handle, uint64_t host, uint64_t node_id, bool checked);
void krabik_ios_on_slider_changed(uint64_t handle, uint64_t host, uint64_t node_id, double value);
#ifdef __cplusplus
}
#endif

#endif
