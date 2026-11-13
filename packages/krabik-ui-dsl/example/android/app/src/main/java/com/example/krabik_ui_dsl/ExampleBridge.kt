package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/**
 * Installs the example's checklist app.
 *
 * [MainActivity] loads `krabik_ui_dsl_example` first, then calls [install]
 * before any [RustRootView] is created: the framework has no application
 * until one is registered.
 */
object ExampleBridge {
    /** Registers the example app factory with the DSL runtime. */
    external fun install()
}
