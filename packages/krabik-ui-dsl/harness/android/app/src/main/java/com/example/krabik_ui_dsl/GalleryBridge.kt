package com.example.krabik_ui_dsl

/**
 * Installs the gallery app.
 *
 * [MainActivity] loads `krabik_ui_dsl_harness` first, then calls [install]
 * before any root view is created.
 */
object GalleryBridge {
    /** Registers the gallery with the framework. */
    external fun install()
}
