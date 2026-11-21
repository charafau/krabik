package dev.krabik.ui

import android.graphics.Color
import android.os.Build
import android.text.SpannableString
import android.text.style.ForegroundColorSpan
import android.view.View
import android.widget.PopupMenu
import java.util.WeakHashMap

/**
 * The popup menus of containers.
 *
 * Rust describes a menu when it creates or updates a container; nothing is
 * shown until the container is tapped or held. The root view asks here first
 * for both, and reports the picked line as an ordinary selection.
 */
object KrabikMenus {
    private class Menu(
        val labels: Array<String>,
        val icons: Array<String>,
        val destructive: BooleanArray,
        val selected: Int,
        val onLongPress: Boolean,
    )

    private val menus = WeakHashMap<View, Menu>()

    /** The color of a line that removes or destroys something. */
    private val DESTRUCTIVE = Color.rgb(0xD3, 0x2F, 0x2F)

    /** Gives a container its menu. `selected` is -1 for none. */
    @JvmStatic
    fun set(
        view: View,
        labels: Array<String>,
        icons: Array<String>,
        destructive: BooleanArray,
        selected: Int,
        onLongPress: Boolean,
    ) {
        menus[view] = Menu(labels, icons, destructive, selected, onLongPress)
    }

    /** Takes a container's menu away. */
    @JvmStatic
    fun clear(view: View) {
        menus.remove(view)
    }

    /**
     * Shows the menu of `view` if it has one for this gesture, anchored to
     * the view. Returns whether it did, in which case the gesture is not
     * reported to the application.
     */
    fun show(view: View, root: RustRootView, longPress: Boolean): Boolean {
        val menu = menus[view] ?: return false
        if (menu.onLongPress != longPress) return false
        val nodeId = view.tag as? Long ?: return false

        val popup = PopupMenu(view.context, view)
        menu.labels.forEachIndexed { index, label ->
            val title: CharSequence = if (menu.destructive[index]) {
                SpannableString(label).apply {
                    setSpan(ForegroundColorSpan(DESTRUCTIVE), 0, label.length, 0)
                }
            } else {
                label
            }
            val item = popup.menu.add(0, index, index, title)
            if (menu.selected >= 0) {
                item.isCheckable = true
                item.isChecked = index == menu.selected
            }
            val icon = menu.icons[index]
            if (icon.isNotEmpty()) {
                val context = view.context
                val id = context.resources.getIdentifier(icon, "drawable", context.packageName)
                if (id != 0) item.setIcon(id)
            }
        }
        if (menu.selected >= 0) {
            popup.menu.setGroupCheckable(0, true, true)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            popup.setForceShowIcon(menu.icons.any { it.isNotEmpty() })
        }
        popup.setOnMenuItemClickListener { item ->
            root.onRadioGroupSelected(nodeId, item.itemId)
            true
        }
        popup.show()
        return true
    }
}
