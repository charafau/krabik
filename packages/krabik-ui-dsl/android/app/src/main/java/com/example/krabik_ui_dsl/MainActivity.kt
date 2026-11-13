package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/** The dev harness: renders the built-in gallery from the DSL renderer. */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl")
    }
}
