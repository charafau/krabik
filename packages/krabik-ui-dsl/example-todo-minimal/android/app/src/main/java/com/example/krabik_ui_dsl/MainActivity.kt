package com.example.krabik_ui_dsl

import dev.krabik.ui.*
/**
 * The minimal todo app: loads its own renderer library, which statically
 * includes the DSL, and installs the app.
 */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl_todo_minimal")
    }

    override fun installApp() {
        MinimalTodoBridge.install()
    }
}
