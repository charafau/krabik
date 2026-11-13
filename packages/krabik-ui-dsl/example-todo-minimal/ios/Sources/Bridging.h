// Exposes the framework's C interface and this app's install function to Swift.

#include "../../../shared/ios/KrabikIOS.h"

/// Registers the minimal todo app with the framework. Call once at startup, before any
/// root view is created.
void krabik_ui_dsl_todo_minimal_install(void);
