package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/**
 * The todo showcase: loads its own renderer library, which statically includes
 * the DSL, and installs the todo app.
 */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl_todo")
    }

    override fun installApp() {
        TodoBridge.install()
    }
}
