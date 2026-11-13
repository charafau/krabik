package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/**
 * Installs the todo showcase app.
 *
 * [MainActivity] loads `krabik_ui_dsl_todo` first, then calls [install] before
 * any [RustRootView] is created: the framework has no application until one
 * is registered.
 */
object TodoBridge {
    /** Registers the todo app factory with the DSL runtime. */
    external fun install()
}
