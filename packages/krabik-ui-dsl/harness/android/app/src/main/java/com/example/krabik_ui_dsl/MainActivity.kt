package com.example.krabik_ui_dsl

import dev.krabik.ui.KrabikActivity

/** The dev harness: loads the gallery crate and installs the gallery app. */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl_harness")
    }

    override fun installApp() {
        GalleryBridge.install()
    }
}
