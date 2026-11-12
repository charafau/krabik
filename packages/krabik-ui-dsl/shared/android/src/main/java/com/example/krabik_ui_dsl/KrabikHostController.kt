package com.example.krabik_ui_dsl

import android.graphics.Color
import android.view.View
import android.widget.FrameLayout
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

    init {
        if (navContainer.id == View.NO_ID) {
            navContainer.id = View.generateViewId()
        }
        main.setController(this)
        navContainer.setBackgroundColor(Color.WHITE)
        navContainer.isClickable = true
        toolbar.visibility = View.GONE
        toolbar.setNavigationIcon(android.R.drawable.ic_media_previous)
        toolbar.setNavigationContentDescription("Back")
        toolbar.setNavigationOnClickListener {
            main.onBackPressed()
        }
    }

    /** Pushes a screen fragment for a Rust host. */
    fun push(hostId: Long, title: String) {
        val transaction = activity.supportFragmentManager.beginTransaction()
        transaction.replace(navContainer.id, KrabikScreenFragment.newInstance(session, hostId))
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

    /** Switches between the main app root and the navigation container. */
    fun setRootVisible(visible: Boolean) {
        // Keep the main root laid out behind the navigation container. Hiding
        // and re-showing it leaves its native view with stale measured bounds.
        mainContainer.visibility = View.VISIBLE
        navContainer.visibility = if (visible) View.GONE else View.VISIBLE
    }

    private fun updateChrome() {
        if (titles.isEmpty()) {
            toolbar.visibility = View.GONE
        } else {
            toolbar.visibility = View.VISIBLE
            toolbar.title = titles.last()
        }
    }

    private companion object {
        const val MODAL_TAG = "krabik-modal"
    }
}
