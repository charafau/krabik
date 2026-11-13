package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/**
 * The standalone example: loads its own renderer library, which statically
 * includes the DSL, and installs the checklist app.
 */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl_example")
    }

    override fun installApp() {
        ExampleBridge.install()
    }
}
