package com.example.krabik_ui_dsl

/**
 * Installs the minimal todo app.
 *
 * [MainActivity] loads `krabik_ui_dsl_todo_minimal` first, then calls [install]
 * before any [RustRootView] is created; without it the renderer falls back to
 * the built-in gallery.
 */
object MinimalTodoBridge {
    /** Registers the minimal todo app factory with the DSL runtime. */
    external fun install()
}
