package dev.krabik.ui

import android.content.res.Configuration
import android.os.Bundle
import android.view.ViewGroup
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.Toolbar
import androidx.activity.OnBackPressedCallback
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.fragment.app.FragmentActivity

/**
 * Hosts the Rust session full screen, edge to edge.
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

    /**
     * Sends the system back action to Rust. It is only enabled while the
     * application says it handles back; otherwise the system's own behaviour
     * applies, including the predictive "back to home" animation.
     */
    private val backCallback = object : OnBackPressedCallback(false) {
        override fun handleOnBackPressed() {
            root.onBackPressed()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        // The Rust session does not survive the activity, so fragments saved
        // by a previous instance would point at a session that no longer
        // exists. Start clean instead of restoring them.
        super.onCreate(null)

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
        }
        drawEdgeToEdge(column, toolbar, content, bottomNavigation)

        setContentView(
            column,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )

        onBackPressedDispatcher.addCallback(this, backCallback)
        controller = KrabikHostController(
            this,
            root.session,
            toolbar,
            bottomNavigation,
            mainContainer,
            navContainer,
            root,
            onBackEnabled = { enabled -> backCallback.isEnabled = enabled },
        )
    }

    /**
     * Lets the app draw behind the system bars and keeps its content clear of
     * them.
     *
     * The toolbar extends under the status bar and the bottom navigation bar
     * under the gesture area, the way native Material screens do. When either
     * is hidden, the content takes that padding instead.
     */
    private fun drawEdgeToEdge(
        column: View,
        toolbar: View,
        content: View,
        bottomNavigation: View,
    ) {
        WindowCompat.setDecorFitsSystemWindows(window, false)
        // Dark icons on a light theme, light icons on a dark one.
        val night = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK ==
            Configuration.UI_MODE_NIGHT_YES
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = !night
            isAppearanceLightNavigationBars = !night
        }

        ViewCompat.setOnApplyWindowInsetsListener(column) { _, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout(),
            )
            val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom

            // Sides always; the bottom only for the keyboard, so a focused
            // field stays above it.
            column.setPadding(bars.left, 0, bars.right, keyboard)
            toolbar.setPadding(0, bars.top, 0, 0)
            bottomNavigation.setPadding(0, 0, 0, if (keyboard > 0) 0 else bars.bottom)
            content.setPadding(
                0,
                if (toolbar.visibility == View.VISIBLE) 0 else bars.top,
                0,
                if (bottomNavigation.visibility == View.VISIBLE || keyboard > 0) 0 else bars.bottom,
            )
            WindowInsetsCompat.CONSUMED
        }
    }

    override fun onDestroy() {
        root.destroy()
        super.onDestroy()
    }
}
