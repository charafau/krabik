package com.example.krabik_ui_dsl

import android.content.Context
import android.content.res.ColorStateList
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.android.material.bottomnavigation.LabelVisibilityMode

/**
 * The native widget used for `BottomNavigation` nodes.
 *
 * Items are rebuilt only when their labels or icon names change. Selection and
 * tint updates happen in place, with a silent guard so Rust state does not echo
 * back as another event.
 */
class KrabikBottomNavigation(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : BottomNavigationView(context) {

    private var silent = false
    private var labels: List<String> = emptyList()
    private var icons: List<String> = emptyList()

    init {
        labelVisibilityMode = LabelVisibilityMode.LABEL_VISIBILITY_LABELED
        setOnItemSelectedListener { item ->
            if (!silent) {
                root.onBottomNavigationSelected(nodeId, item.itemId - ID_BASE)
            }
            true
        }
    }

    /** Applies labels, icons, selection and tint from Rust. */
    fun configure(labels: Array<String>, icons: Array<String>, selected: Int, color: Int) {
        val newLabels = labels.toList()
        val newIcons = icons.toList()
        if (this.labels != newLabels || this.icons != newIcons) {
            rebuild(newLabels, newIcons)
        }
        this.labels = newLabels
        this.icons = newIcons

        val tint = ColorStateList(
            arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf()),
            intArrayOf(color, faded(color)),
        )
        itemIconTintList = tint
        itemTextColor = tint

        silent = true
        try {
            val target = if (newLabels.isEmpty()) {
                NO_SELECTION
            } else {
                ID_BASE + selected.coerceIn(0, newLabels.lastIndex)
            }
            if (selectedItemId != target) {
                selectedItemId = target
            }
        } finally {
            silent = false
        }
    }

    private fun rebuild(labels: List<String>, icons: List<String>) {
        menu.clear()
        silent = true
        try {
            for ((index, label) in labels.withIndex()) {
                val item = menu.add(0, ID_BASE + index, index, label)
                val icon = icons.getOrNull(index).orEmpty()
                if (icon.isNotEmpty()) {
                    val resource = resources.getIdentifier(icon, "drawable", context.packageName)
                    if (resource != 0) {
                        item.setIcon(resource)
                    }
                }
            }
        } finally {
            silent = false
        }
    }

    private fun faded(argb: Int): Int = (argb and 0x00FFFFFF) or (0x88 shl 24)

    private companion object {
        const val ID_BASE = 1
        const val NO_SELECTION = -1
    }
}
