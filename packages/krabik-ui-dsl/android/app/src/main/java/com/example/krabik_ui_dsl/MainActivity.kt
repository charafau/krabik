package com.example.krabik_ui_dsl

/** The dev harness: renders the built-in gallery from the DSL renderer. */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl")
    }
}
