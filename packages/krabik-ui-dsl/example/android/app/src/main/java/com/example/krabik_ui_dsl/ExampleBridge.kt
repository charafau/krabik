package com.example.krabik_ui_dsl

/**
 * Loads the example's Rust library and installs its checklist app.
 *
 * Call [install] once at startup, before any [RustRootView] is created;
 * without it the renderer falls back to the built-in gallery.
 */
object ExampleBridge {
    init {
        System.loadLibrary("krabik_ui_dsl_example")
    }

    /** Registers the example app factory with the DSL runtime. */
    external fun install()
}
