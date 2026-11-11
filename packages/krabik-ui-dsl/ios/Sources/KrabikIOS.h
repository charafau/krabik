#ifndef KRABIK_IOS_H
#define KRABIK_IOS_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Creates the Rust renderer state for a root view. Returns 0 on failure.
uint64_t krabik_ios_create(void *root, double width, double height);

/// Rebuilds the tree for the given viewport and applies the resulting changes.
void krabik_ios_render(uint64_t handle, double width, double height);

/// Releases the widgets and the Rust state.
void krabik_ios_destroy(uint64_t handle);

void krabik_ios_on_click(uint64_t handle, uint64_t node_id);
void krabik_ios_on_text_changed(uint64_t handle, uint64_t node_id, const char *text);
void krabik_ios_on_submitted(uint64_t handle, uint64_t node_id);
void krabik_ios_on_focus_changed(uint64_t handle, uint64_t node_id, bool focused);
void krabik_ios_on_checked_changed(uint64_t handle, uint64_t node_id, bool checked);
void krabik_ios_on_slider_changed(uint64_t handle, uint64_t node_id, double value);

#ifdef __cplusplus
}
#endif

#endif
