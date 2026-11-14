package dev.krabik.ui

import android.app.Dialog
import android.content.DialogInterface
import android.graphics.Color
import android.view.Gravity
import android.view.MenuItem
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toolbar
import androidx.appcompat.app.AlertDialog
import androidx.core.view.GravityCompat
import androidx.drawerlayout.widget.DrawerLayout
import androidx.fragment.app.DialogFragment
import androidx.fragment.app.FragmentActivity
import com.google.android.material.color.MaterialColors
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.android.material.floatingactionbutton.ExtendedFloatingActionButton
import com.google.android.material.navigation.NavigationView
import com.google.android.material.snackbar.Snackbar

/**
 * Owns the native chrome of one session: the screen stack, the toolbar, the
 * bottom navigation bar, the floating action button, the navigation drawer,
 * the snackbar, the system alert and the bottom sheet.
 *
 * Rust is the source of truth. After an application state change it calls
 * [push] / [pop], [setToolbar], [setBottomNavigation], [setFab], [setDrawer],
 * [showSnackbar] / [dismissSnackbar], [showDialog] / [dismissDialog] and
 * [presentSheet] / [dismissSheet]. Nothing here changes
 * what is shown on its own; user input is reported to Rust, which decides.
 */
class KrabikHostController(
    private val activity: FragmentActivity,
    private val session: Long,
    private val toolbar: Toolbar,
    private val bottomNavigation: KrabikBottomNavigation,
    private val mainContainer: FrameLayout,
    private val navContainer: FrameLayout,
    private val main: RustRootView,
    /** The floating action button, above the bottom corner of the content. */
    private val fab: ExtendedFloatingActionButton,
    /** Wraps the whole screen so [drawer] can slide over it. */
    private val drawerLayout: DrawerLayout,
    private val drawer: NavigationView,
    /** Arms or disarms the activity's back callback; see [setBackEnabled]. */
    private val onBackEnabled: (Boolean) -> Unit,
) {
    /**
     * The screen stack, root first. Screens are plain fragments added to
     * [navContainer]; the fragment back stack is not used, so the system back
     * button can never pop a screen without Rust knowing.
     */
    private val screens = ArrayList<KrabikScreenFragment>()
    private var dialog: Dialog? = null
    private var snackbar: Snackbar? = null

    /** Whether the top screen has a drawer, and whether Rust wants it open. */
    private var drawerPresent = false
    private var drawerOpen = false
    private var backTitle = "Back"
    private var lastPopAnimated = false
    /** The length of a screen transition, shared with the animations in `res/anim`. */
    private val transitionMs =
        activity.resources.getInteger(R.integer.krabik_transition_ms).toLong()
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
        bottomNavigation.visibility = View.GONE
        // The navigation icon is the back arrow on a pushed screen and the
        // menu button on a root screen that has a drawer.
        toolbar.setNavigationOnClickListener {
            if (screens.size > 1) {
                main.onBackPressed()
            } else if (drawerPresent) {
                drawerLayout.openDrawer(GravityCompat.START)
            }
        }
        fab.visibility = View.GONE
        fab.setOnClickListener { main.onChromeEvent(EVENT_FAB) }
        drawerLayout.setDrawerLockMode(DrawerLayout.LOCK_MODE_LOCKED_CLOSED)
        drawerLayout.addDrawerListener(object : DrawerLayout.SimpleDrawerListener() {
            // Only a change the user made is reported; one Rust asked for is
            // already known there.
            override fun onDrawerOpened(drawerView: View) {
                if (!drawerOpen) {
                    drawerOpen = true
                    main.onChromeEvent(EVENT_DRAWER_OPENED)
                }
            }

            override fun onDrawerClosed(drawerView: View) {
                if (drawerOpen) {
                    drawerOpen = false
                    main.onChromeEvent(EVENT_DRAWER_CLOSED)
                }
            }
        })
        drawer.setNavigationItemSelectedListener { item ->
            // Rust decides what the choice does, including closing the drawer.
            main.onChromeEvent(EVENT_DRAWER_SELECTED, item.itemId)
            false
        }
        toolbar.setOnMenuItemClickListener { item ->
            main.onToolbarAction(item.itemId - ACTION_ID_BASE)
            true
        }
        setRootVisible(true)
        // Register last: this synchronously syncs the session, which may push
        // the first navigation screen and update the toolbar/containers.
        main.setController(this)
    }

    // region Screen stack

    /**
     * Pushes a screen for a Rust host.
     *
     * The new screen is added above the current one, which is hidden once the
     * transition ends. Its view stays alive, so a later [pop] reveals it
     * exactly as the user left it.
     */
    fun push(hostId: Long, transition: Int) {
        navContainer.removeCallbacks(hideNavigation)
        val fragment = KrabikScreenFragment.newInstance(session, hostId)
        val previous = screens.lastOrNull()
        val transaction = activity.supportFragmentManager.beginTransaction()
            .setReorderingAllowed(true)
        if (previous != null) {
            when (transition) {
                // The new screen slides in above while the old one drifts back.
                TRANSITION_SLIDE -> transaction.setCustomAnimations(
                    R.anim.krabik_slide_in_right,
                    R.anim.krabik_slide_out_left,
                )
                // Only the new screen fades; the old one stays solid beneath.
                TRANSITION_FADE -> transaction.setCustomAnimations(
                    R.anim.krabik_fade_in,
                    R.anim.krabik_hold,
                )
            }
        }
        transaction.add(navContainer.id, fragment)
        if (previous != null) {
            transaction.hide(previous)
        }
        transaction.commitAllowingStateLoss()
        screens.add(fragment)
    }

    /**
     * Pops [count] screens in one step, animating only the visible top screen
     * with the [transition] it was pushed with.
     */
    fun pop(count: Int, transition: Int) {
        if (count <= 0 || screens.isEmpty()) {
            return
        }
        val removed = ArrayList<KrabikScreenFragment>()
        repeat(minOf(count, screens.size)) {
            removed.add(screens.removeAt(screens.size - 1))
        }
        val top = removed.first()
        val target = screens.lastOrNull()
        val manager = activity.supportFragmentManager

        // Screens between the top and the target are hidden; drop them quietly.
        if (removed.size > 1) {
            val quiet = manager.beginTransaction().setReorderingAllowed(true)
            removed.drop(1).forEach { quiet.remove(it) }
            quiet.commitAllowingStateLoss()
        }

        val transaction = manager.beginTransaction().setReorderingAllowed(true)
        when (transition) {
            // The top screen slides away above while the target drifts in.
            TRANSITION_SLIDE -> transaction.setCustomAnimations(
                R.anim.krabik_slide_in_left,
                R.anim.krabik_slide_out_right,
            )
            // Only the top screen fades; the target is already solid beneath.
            TRANSITION_FADE -> transaction.setCustomAnimations(0, R.anim.krabik_fade_out)
        }
        transaction.remove(top)
        if (target != null) {
            transaction.show(target)
        }
        transaction.commitAllowingStateLoss()
        lastPopAnimated = transition == TRANSITION_SLIDE || transition == TRANSITION_FADE
    }

    /** Switches between the main app root and the navigation container. */
    fun setRootVisible(visible: Boolean) {
        navContainer.removeCallbacks(hideNavigation)
        if (visible) {
            mainContainer.visibility = View.VISIBLE
            if (lastPopAnimated && navContainer.visibility == View.VISIBLE) {
                // Keep the navigation container above the root until the last
                // screen has animated out, then reveal the root underneath.
                navContainer.postDelayed(hideNavigation, transitionMs + 40L)
            } else {
                navContainer.visibility = View.GONE
            }
        } else {
            // A navigation app renders its root as a screen too, so the unused
            // main host must leave the accessibility tree to avoid duplicate rows.
            mainContainer.visibility = View.GONE
            navContainer.visibility = View.VISIBLE
        }
        lastPopAnimated = false
    }

    /**
     * Whether the application handles the system back action right now. When
     * it does not, the system's own behaviour applies, including the
     * predictive "back to home" animation on the root screen.
     */
    fun setBackEnabled(enabled: Boolean) {
        onBackEnabled(enabled)
    }

    // endregion

    // region Toolbar and bottom navigation

    /**
     * Applies the top screen's toolbar. Rust calls this after every navigation
     * change, so the toolbar never keeps a previous screen's configuration.
     */
    fun setToolbar(
        title: String,
        subtitle: String,
        backTitle: String,
        hidden: Boolean,
        actionLabels: Array<String>,
        actionIcons: Array<String>,
    ) {
        toolbar.title = title
        toolbar.subtitle = subtitle.ifEmpty { null }
        this.backTitle = backTitle.ifEmpty { "Back" }
        updateNavigationIcon()
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
                        main.onToolbarAction(index)
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
        toolbar.visibility = if (hidden) View.GONE else View.VISIBLE
        // The system-bar padding depends on which bars are showing.
        toolbar.rootView.requestApplyInsets()
    }

    /** Applies the top screen's bottom navigation bar; no items hides it. */
    fun setBottomNavigation(labels: Array<String>, icons: Array<String>, selected: Int) {
        bottomNavigation.configure(labels.toList(), icons.toList(), selected)
        bottomNavigation.visibility = if (labels.isEmpty()) View.GONE else View.VISIBLE
        bottomNavigation.rootView.requestApplyInsets()
    }

    private fun updateNavigationIcon() {
        when {
            screens.size > 1 -> {
                toolbar.setNavigationIcon(R.drawable.krabik_ic_arrow_back)
                toolbar.navigationContentDescription = backTitle
            }
            drawerPresent -> {
                toolbar.setNavigationIcon(R.drawable.krabik_ic_menu)
                toolbar.navigationContentDescription = "Menu"
            }
            else -> toolbar.navigationIcon = null
        }
    }

    // endregion

    // region Floating action button, drawer and snackbar

    /** Applies the top screen's floating action button; hidden when not [visible]. */
    fun setFab(label: String, icon: String, visible: Boolean) {
        fab.text = label
        fab.contentDescription = label
        val resource = if (icon.isNotEmpty()) {
            activity.resources.getIdentifier(icon, "drawable", activity.packageName)
        } else {
            0
        }
        if (resource != 0) fab.setIconResource(resource) else fab.icon = null
        fab.visibility = if (visible) View.VISIBLE else View.GONE
    }

    /**
     * Applies the top screen's navigation drawer.
     *
     * Without one ([present] false) the drawer is locked shut. [selected] is
     * the highlighted item, or -1 for none.
     */
    fun setDrawer(
        labels: Array<String>,
        icons: Array<String>,
        selected: Int,
        present: Boolean,
        open: Boolean,
    ) {
        drawer.menu.clear()
        for (index in labels.indices) {
            val item = drawer.menu.add(0, index, index, labels[index])
            item.isCheckable = true
            item.isChecked = index == selected
            val icon = icons.getOrNull(index).orEmpty()
            val resource = if (icon.isNotEmpty()) {
                activity.resources.getIdentifier(icon, "drawable", activity.packageName)
            } else {
                0
            }
            if (resource != 0) item.setIcon(resource)
        }
        drawerPresent = present
        drawerOpen = present && open
        drawerLayout.setDrawerLockMode(
            if (present) DrawerLayout.LOCK_MODE_UNLOCKED else DrawerLayout.LOCK_MODE_LOCKED_CLOSED,
        )
        if (drawerOpen) {
            drawerLayout.openDrawer(GravityCompat.START)
        } else {
            drawerLayout.closeDrawer(GravityCompat.START)
        }
        updateNavigationIcon()
    }

    /** Closes the drawer for the system back action. Returns whether it was open. */
    fun closeDrawerForBack(): Boolean {
        if (!drawerLayout.isDrawerOpen(GravityCompat.START)) {
            return false
        }
        // The listener reports the close, so Rust hears about it as usual.
        drawerLayout.closeDrawer(GravityCompat.START)
        return true
    }

    /**
     * Shows a snackbar above the floating action button or the bottom bar.
     * An empty [action] means no action button.
     */
    fun showSnackbar(message: String, action: String) {
        dismissSnackbar()
        val bar = Snackbar.make(fab.parent as View, message, Snackbar.LENGTH_LONG)
        when {
            fab.visibility == View.VISIBLE -> bar.anchorView = fab
            bottomNavigation.visibility == View.VISIBLE -> bar.anchorView = bottomNavigation
        }
        if (action.isNotEmpty()) {
            bar.setAction(action) { main.onChromeEvent(EVENT_SNACKBAR_ACTION) }
        }
        bar.addCallback(object : Snackbar.Callback() {
            override fun onDismissed(transientBottomBar: Snackbar, event: Int) {
                if (snackbar !== transientBottomBar) {
                    // Replaced or taken down by Rust; nothing to report.
                    return
                }
                snackbar = null
                if (event == DISMISS_EVENT_TIMEOUT || event == DISMISS_EVENT_SWIPE) {
                    main.onChromeEvent(EVENT_SNACKBAR_DISMISSED)
                }
            }
        })
        snackbar = bar
        bar.show()
    }

    /** Removes the snackbar, if any, without reporting it. */
    fun dismissSnackbar() {
        val bar = snackbar ?: return
        snackbar = null
        bar.dismiss()
    }

    // endregion

    // region Dialog and bottom sheet

    /**
     * Shows the system alert. [roles] holds one code per button: 0 default,
     * 1 cancel, 2 destructive. Buttons report their index; a dismissal by back
     * or an outside tap, allowed only when [cancelable], reports -1.
     */
    fun showDialog(
        title: String,
        message: String,
        labels: Array<String>,
        roles: IntArray,
        cancelable: Boolean,
    ) {
        dismissDialog()
        val builder = MaterialAlertDialogBuilder(activity).setTitle(title)
        if (message.isNotEmpty()) {
            builder.setMessage(message)
        }

        // Android places buttons by slot, not by order: the cancel button is
        // the negative one, the first other choice the positive one.
        var destructiveSlot = 0
        var positiveUsed = false
        var neutralUsed = false
        for (index in labels.indices) {
            val listener = DialogInterface.OnClickListener { _, _ ->
                dialog = null
                main.onDialogAction(index)
            }
            val slot = when {
                roles[index] == ROLE_CANCEL -> DialogInterface.BUTTON_NEGATIVE
                !positiveUsed -> DialogInterface.BUTTON_POSITIVE
                !neutralUsed -> DialogInterface.BUTTON_NEUTRAL
                else -> continue
            }
            when (slot) {
                DialogInterface.BUTTON_NEGATIVE -> builder.setNegativeButton(labels[index], listener)
                DialogInterface.BUTTON_POSITIVE -> {
                    positiveUsed = true
                    builder.setPositiveButton(labels[index], listener)
                }
                else -> {
                    neutralUsed = true
                    builder.setNeutralButton(labels[index], listener)
                }
            }
            if (roles[index] == ROLE_DESTRUCTIVE) {
                destructiveSlot = slot
            }
        }

        builder.setCancelable(cancelable)
        builder.setOnCancelListener {
            dialog = null
            main.onDialogAction(DISMISSED)
        }
        val alert: AlertDialog = builder.show()
        if (destructiveSlot != 0) {
            alert.getButton(destructiveSlot)?.setTextColor(
                MaterialColors.getColor(
                    alert.context,
                    com.google.android.material.R.attr.colorError,
                    Color.RED,
                ),
            )
        }
        dialog = alert
    }

    /** Closes the system alert without reporting anything back. */
    fun dismissDialog() {
        dialog?.dismiss()
        dialog = null
    }

    /** Presents the native bottom sheet around a Rust host. */
    fun presentSheet(hostId: Long) {
        KrabikSheetFragment.newInstance(session, hostId)
            .show(activity.supportFragmentManager, SHEET_TAG)
    }

    /** Dismisses the bottom sheet, if any. */
    fun dismissSheet() {
        val fragment = activity.supportFragmentManager.findFragmentByTag(SHEET_TAG)
        if (fragment is DialogFragment) {
            fragment.dismissAllowingStateLoss()
        }
    }

    // endregion

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        // What `RustRootView.onChromeEvent` reports; Rust decodes the same
        // numbers in `ChromeEvent::from_code`.
        const val EVENT_FAB = 0
        const val EVENT_DRAWER_SELECTED = 1
        const val EVENT_DRAWER_OPENED = 2
        const val EVENT_DRAWER_CLOSED = 3
        const val EVENT_SNACKBAR_ACTION = 4
        const val EVENT_SNACKBAR_DISMISSED = 5
        const val SHEET_TAG = "krabik-sheet"
        const val TRANSITION_SLIDE = 0
        const val TRANSITION_FADE = 1
        const val ROLE_CANCEL = 1
        const val ROLE_DESTRUCTIVE = 2
        const val ACTION_ID_BASE = 0x1000
        const val DISMISSED = -1
    }
}
