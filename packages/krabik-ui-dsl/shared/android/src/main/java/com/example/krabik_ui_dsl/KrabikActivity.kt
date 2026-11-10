package com.example.krabik_ui_dsl

import android.app.Activity
import android.os.Build
import android.os.Bundle
import android.view.WindowInsets
import android.widget.FrameLayout

/**
 * Hosts a [RustRootView] full screen and keeps it inside the system bars.
 *
 * Subclasses load their native renderer and, if they provide one, register
 * their app factory. That ordering matters: the renderer must be loaded before
 * the root view is constructed, and the factory installed before it renders.
 */
abstract class KrabikActivity : Activity() {

    private lateinit var root: RustRootView

    /** Loads the native renderer, e.g. with `System.loadLibrary`. */
    protected open fun loadNativeLibrary() {}

    /** Registers the app factory, when the shell ships its own app. */
    protected open fun installApp() {}

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        loadNativeLibrary()
        installApp()
        root = RustRootView(this)

        // Android 15 draws app content edge to edge. Keep the Rust UI inside
        // the system bars by insetting the container instead of the root view.
        val container = FrameLayout(this).apply {
            addView(
                root,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
            setOnApplyWindowInsetsListener { view, insets ->
                val left: Int
                val top: Int
                val right: Int
                val bottom: Int

                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    val bars = insets.getInsets(
                        WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout(),
                    )
                    val ime = insets.getInsets(WindowInsets.Type.ime())
                    left = bars.left
                    top = bars.top
                    right = bars.right
                    // Keep the focused field above the keyboard when it opens.
                    bottom = maxOf(bars.bottom, ime.bottom)
                } else {
                    @Suppress("DEPRECATION")
                    left = insets.systemWindowInsetLeft
                    @Suppress("DEPRECATION")
                    top = insets.systemWindowInsetTop
                    @Suppress("DEPRECATION")
                    right = insets.systemWindowInsetRight
                    @Suppress("DEPRECATION")
                    bottom = insets.systemWindowInsetBottom
                }

                view.setPadding(left, top, right, bottom)
                insets
            }
        }

        setContentView(container)
    }

    override fun onDestroy() {
        root.destroy()
        super.onDestroy()
    }
}
