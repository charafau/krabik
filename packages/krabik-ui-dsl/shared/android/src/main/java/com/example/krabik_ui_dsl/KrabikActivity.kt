package com.example.krabik_ui_dsl

import android.os.Build
import android.os.Bundle
import android.view.ViewGroup
import android.view.WindowInsets
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.Toolbar
import androidx.activity.OnBackPressedCallback
import androidx.fragment.app.FragmentActivity

/**
 * Hosts the Rust session full screen and keeps it inside the system bars.
 *
 * The main root, navigation screens and modal hosts share one session handle.
 * A [KrabikHostController] owns the native toolbar, bottom navigation bar,
 * screen stack, alert and bottom sheet; Rust remains the source of truth for
 * what is shown.
 *
 * Subclasses load their native renderer and, if they provide one, register
 * their app factory. That ordering matters: the renderer must be loaded before
 * the root view is constructed, and the factory installed before it renders.
 */
abstract class KrabikActivity : FragmentActivity() {

    private lateinit var root: RustRootView
    private lateinit var controller: KrabikHostController

    /** Loads the native renderer, e.g. with `System.loadLibrary`. */
    protected open fun loadNativeLibrary() {}

    /** Registers the app factory, when the shell ships its own app. */
    protected open fun installApp() {}

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        loadNativeLibrary()
        installApp()
        root = RustRootView(this)

        val toolbar = Toolbar(this)
        val bottomNavigation = KrabikBottomNavigation(this) { index ->
            root.onBottomNavigationSelected(index)
        }
        val mainContainer = FrameLayout(this).apply {
            addView(
                root,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        val navContainer = FrameLayout(this)
        // The root and the navigation container overlap, so revealing the root
        // during a pop does not resize the outgoing screen. The navigation
        // container is added last and therefore draws above the root.
        val content = FrameLayout(this).apply {
            addView(
                mainContainer,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
            addView(
                navContainer,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.MATCH_PARENT,
                    FrameLayout.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        val column = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(
                toolbar,
                LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                ),
            )
            addView(
                content,
                LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    0,
                    1f,
                ),
            )
            // The bar keeps its natural Material height below the content.
            addView(
                bottomNavigation,
                LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
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

        setContentView(
            column,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )

        controller = KrabikHostController(
            this,
            root.session,
            toolbar,
            bottomNavigation,
            mainContainer,
            navContainer,
            root,
        )

        // Rust owns the screen stack, so it decides what back does. When it
        // declines (the root screen), the system default runs and leaves the
        // app; the callback is re-armed for when the user returns.
        onBackPressedDispatcher.addCallback(
            this,
            object : OnBackPressedCallback(true) {
                override fun handleOnBackPressed() {
                    if (!root.onBackPressed()) {
                        isEnabled = false
                        onBackPressedDispatcher.onBackPressed()
                        isEnabled = true
                    }
                }
            },
        )
    }

    override fun onDestroy() {
        root.destroy()
        super.onDestroy()
    }
}
