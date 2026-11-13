package com.example.krabik_ui_dsl

import android.content.Context
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.android.material.bottomnavigation.LabelVisibilityMode

/**
 * The host-level Material bottom navigation bar.
 *
 * It keeps its natural Material height and theme colors. Items are rebuilt
 * only when their labels or icon names change; selection updates happen in
 * place, silently, so Rust state does not echo back as another event.
 */
class KrabikBottomNavigation(
    context: Context,
    private val onSelected: (Int) -> Unit,
) : BottomNavigationView(context) {

    private var silent = false
    private var labels: List<String> = emptyList()
    private var icons: List<String> = emptyList()

    init {
        labelVisibilityMode = LabelVisibilityMode.LABEL_VISIBILITY_LABELED
        setOnItemSelectedListener { item ->
            if (!silent) {
                onSelected(item.itemId - ID_BASE)
            }
            true
        }
    }

    /** Applies labels, icons and the selection from Rust. */
    fun configure(labels: List<String>, icons: List<String>, selected: Int) {
        silent = true
        try {
            if (this.labels != labels || this.icons != icons) {
                rebuild(labels, icons)
                this.labels = labels
                this.icons = icons
            }
            if (labels.isNotEmpty()) {
                val target = ID_BASE + selected.coerceIn(0, labels.lastIndex)
                if (selectedItemId != target) {
                    selectedItemId = target
                }
            }
        } finally {
            silent = false
        }
    }

    private fun rebuild(labels: List<String>, icons: List<String>) {
        menu.clear()
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
    }

    private companion object {
        const val ID_BASE = 1
    }
}
