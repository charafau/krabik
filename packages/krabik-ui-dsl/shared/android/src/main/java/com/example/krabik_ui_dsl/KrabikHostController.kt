package com.example.krabik_ui_dsl

import android.view.Gravity
import android.view.MenuItem
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toolbar
import androidx.fragment.app.FragmentActivity

/**
 * Owns the native navigation stack and modal presentations for one session.
 *
 * Rust is the source of truth: it calls [push], [pop], [present] and [dismiss]
 * after an application state change. The toolbar and fragment transactions are
 * native; the back button asks Rust first.
 */
class KrabikHostController(
    private val activity: FragmentActivity,
    private val session: Long,
    private val toolbar: Toolbar,
    private val mainContainer: FrameLayout,
    private val navContainer: FrameLayout,
    private val main: RustRootView,
) {
    private val titles = ArrayList<String>()
    private var poppingToRoot = false
    private var toolbarActions: List<String> = emptyList()
    private val hideNavigation = Runnable {
        navContainer.visibility = View.GONE
    }

    init {
        if (navContainer.id == View.NO_ID || navContainer.id == 0) {
            navContainer.id = View.generateViewId()
        }
        // Screens paint their own opaque background; leaving the container
        // transparent lets a pop reveal the root underneath.
        navContainer.isClickable = true
        toolbar.visibility = View.GONE
        toolbar.setNavigationContentDescription("Back")
        toolbar.setNavigationOnClickListener {
            main.onBackPressed()
        }
        toolbar.setOnMenuItemClickListener { item ->
            val action = toolbarActions.getOrNull(item.itemId - ACTION_ID_BASE)
            if (action != null) {
                main.onToolbarAction(action)
                true
            } else {
                false
            }
        }
        setRootVisible(true)
        // Register last: this synchronously syncs the session, which may push
        // the first navigation screen and update the toolbar/containers.
        main.setController(this)
    }

    /** Pushes a screen fragment for a Rust host with the requested transition. */
    fun push(hostId: Long, title: String, transition: Int) {
        navContainer.removeCallbacks(hideNavigation)
        poppingToRoot = false
        val fragment = KrabikScreenFragment.newInstance(session, hostId)
        val transaction = activity.supportFragmentManager.beginTransaction()
        val (enter, exit, popEnter, popExit) = when (transition) {
            TRANSITION_FADE -> intArrayOf(
                R.anim.krabik_fade_in,
                R.anim.krabik_fade_out,
                R.anim.krabik_fade_in,
                R.anim.krabik_fade_out,
            )
            TRANSITION_SLIDE -> intArrayOf(
                R.anim.krabik_slide_in_right,
                R.anim.krabik_slide_out_left,
                R.anim.krabik_slide_in_left,
                R.anim.krabik_slide_out_right,
            )
            else -> intArrayOf(0, 0, 0, 0)
        }
        transaction.setCustomAnimations(enter, exit, popEnter, popExit)
        transaction.replace(navContainer.id, fragment)
        transaction.addToBackStack(title)
        transaction.commit()
        titles.add(title)
        updateChrome()
    }

    /** Pops the current screen fragment. */
    fun pop() {
        activity.supportFragmentManager.popBackStack()
        if (titles.isNotEmpty()) {
            titles.removeAt(titles.size - 1)
        }
        poppingToRoot = titles.isEmpty()
        updateChrome()
    }

    /** Presents a dialog or bottom sheet host. */
    fun present(hostId: Long, kind: Int) {
        val fragment = if (kind == 1) {
            KrabikSheetFragment.newInstance(session, hostId)
        } else {
            KrabikDialogFragment.newInstance(session, hostId)
        }
        fragment.show(activity.supportFragmentManager, MODAL_TAG)
    }

    /** Dismisses the current modal, if any. */
    fun dismiss() {
        val fragment = activity.supportFragmentManager.findFragmentByTag(MODAL_TAG)
        if (fragment is androidx.fragment.app.DialogFragment) {
            fragment.dismissAllowingStateLoss()
        }
    }

    /** Applies the top screen's toolbar metadata. */
    fun setToolbar(
        title: String,
        subtitle: String,
        backTitle: String,
        hidden: Boolean,
        actionIds: Array<String>,
        actionLabels: Array<String>,
        actionIcons: Array<String>,
    ) {
        toolbarActions = actionIds.toList()
        toolbar.title = title
        toolbar.subtitle = subtitle.ifEmpty { null }
        toolbar.setNavigationContentDescription(backTitle.ifEmpty { "Back" })
        toolbar.menu.clear()
        for (index in actionLabels.indices) {
            val item = toolbar.menu.add(0, ACTION_ID_BASE + index, index, actionLabels[index])
            item.setShowAsAction(
                MenuItem.SHOW_AS_ACTION_ALWAYS or MenuItem.SHOW_AS_ACTION_WITH_TEXT,
            )
            val icon = actionIcons.getOrNull(index).orEmpty()
            val resource = if (icon.isNotEmpty()) {
                activity.resources.getIdentifier(icon, "drawable", activity.packageName)
            } else {
                0
            }
            if (resource != 0) {
                // Framework menu items show either the icon or the text. Use a
                // custom action view so an icon never hides the required label.
                item.setIcon(resource)
                val row = LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = Gravity.CENTER_VERTICAL
                    setPadding(dp(8), 0, dp(8), 0)
                    contentDescription = actionLabels[index]
                    isClickable = true
                    setOnClickListener {
                        main.onToolbarAction(toolbarActions[index])
                    }
                }
                row.addView(
                    ImageView(activity).apply {
                        setImageResource(resource)
                        contentDescription = actionLabels[index]
                    },
                    LinearLayout.LayoutParams(dp(20), dp(20)),
                )
                row.addView(
                    TextView(activity).apply {
                        text = actionLabels[index]
                        setTextSize(android.util.TypedValue.COMPLEX_UNIT_SP, 14f)
                    },
                    LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT,
                    ).apply { marginStart = dp(4) },
                )
                item.actionView = row
                item.setShowAsAction(MenuItem.SHOW_AS_ACTION_ALWAYS)
            }
        }
        if (hidden || titles.isEmpty()) {
            toolbar.visibility = View.GONE
        } else {
            toolbar.visibility = View.VISIBLE
        }
    }

    /** Switches between the main app root and the navigation container. */
    fun setRootVisible(visible: Boolean) {
        navContainer.removeCallbacks(hideNavigation)
        if (visible) {
            // Keep the navigation container above the root while a pop
            // transition runs, then reveal the root underneath.
            mainContainer.visibility = View.VISIBLE
            navContainer.visibility = View.VISIBLE
            if (poppingToRoot) {
                poppingToRoot = false
                navContainer.postDelayed(hideNavigation, TRANSITION_MS + 40L)
            } else {
                navContainer.visibility = View.GONE
            }
        } else {
            // A navigation app renders its root as a screen too, so the unused
            // main host must leave the accessibility tree to avoid duplicate rows.
            mainContainer.visibility = View.GONE
            navContainer.visibility = View.VISIBLE
        }
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private fun updateChrome() {
        if (titles.isEmpty()) {
            toolbar.visibility = View.GONE
            return
        }
        toolbar.visibility = View.VISIBLE
        toolbar.title = titles.last()
        if (titles.size > 1) {
            toolbar.setNavigationIcon(android.R.drawable.ic_media_previous)
            toolbar.setNavigationContentDescription("Back")
        } else {
            toolbar.navigationIcon = null
        }
    }

    private companion object {
        const val MODAL_TAG = "krabik-modal"
        const val TRANSITION_MS = 280L
        const val TRANSITION_FADE = 1
        const val TRANSITION_SLIDE = 0
        const val ACTION_ID_BASE = 0x1000
    }
}
