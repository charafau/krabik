package com.example.krabik_ui_dsl

/**
 * Installs the todo showcase app.
 *
 * [MainActivity] loads `krabik_ui_dsl_todo` first, then calls [install] before
 * any [RustRootView] is created; without it the renderer falls back to the
 * built-in gallery.
 */
object TodoBridge {
    /** Registers the todo app factory with the DSL runtime. */
    external fun install()
}
