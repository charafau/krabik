package com.example.krabik_ui_example

import android.os.Build
import android.os.Bundle
import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.appcompat.app.AppCompatActivity

/**
 * Thin Android entry point for the Rust UI PoC.
 *
 * All JNI plumbing lives in [AndroidBridge]; this activity only forwards lifecycle events.
 */
class MainActivity : AppCompatActivity() {

    private lateinit var bridge: AndroidBridge
    private var backCallback: OnBackInvokedCallback? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        bridge = AndroidBridge(this)
        bridge.createUi()

        // Apps targeting API 35+ no longer receive onBackPressed(). Register a platform back
        // callback so Rust still gets the first chance to close a demo screen.
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            val callback = OnBackInvokedCallback {
                if (!bridge.back()) {
                    finish()
                }
            }
            onBackInvokedDispatcher.registerOnBackInvokedCallback(
                OnBackInvokedDispatcher.PRIORITY_DEFAULT,
                callback,
            )
            backCallback = callback
        }
    }

    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        // Used on API levels below 33, where the platform back callback is unavailable.
        if (!bridge.back()) {
            super.onBackPressed()
        }
    }

    override fun onDestroy() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            backCallback?.let { onBackInvokedDispatcher.unregisterOnBackInvokedCallback(it) }
        }
        bridge.destroyUi()
        super.onDestroy()
    }
}
