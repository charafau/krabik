package com.example.krabik_ui_dsl

/**
 * Installs the example's checklist app.
 *
 * [MainActivity] loads `krabik_ui_dsl_example` first, then calls [install]
 * before any [RustRootView] is created; without it the renderer falls back to
 * the built-in gallery.
 */
object ExampleBridge {
    /** Registers the example app factory with the DSL runtime. */
    external fun install()
}
