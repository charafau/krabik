package dev.krabik.ui

import android.content.Context
import android.graphics.Typeface
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView

/**
 * The native widget used for `TabBar` nodes.
 *
 * The bar reports the tapped index back to Rust; the application owns the
 * selection and passes it down again on the next render. State applied from
 * Rust does not echo back as another event.
 */
class KrabikTabBar(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : LinearLayout(context) {

    private var silent = false
    private var labels: List<String> = emptyList()
    private var selectedIndex = 0
    private var size = 14f
    private var color = 0xFF00_0000.toInt()
    private var bold = false

    init {
        orientation = HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
    }

    /**
     * Applies the labels, selected index and styling from Rust.
     *
     * The children are only rebuilt when the labels change; selection and
     * color updates happen in place so focus is not lost on every render.
     */
    fun configure(
        labels: Array<String>,
        selected: Int,
        size: Float,
        color: Int,
        bold: Boolean,
    ) {
        val tabs = labels.toList()
        if (this.labels != tabs) {
            rebuild(tabs)
        }
        this.size = size
        this.color = color
        this.bold = bold
        this.selectedIndex = if (tabs.isEmpty()) 0 else selected.coerceIn(0, tabs.lastIndex)

        silent = true
        try {
            for (index in 0 until childCount) {
                updateTab(getChildAt(index) as TextView, index)
            }
        } finally {
            silent = false
        }
    }

    private fun rebuild(tabs: List<String>) {
        removeAllViews()
        labels = tabs
        for ((index, label) in tabs.withIndex()) {
            val tab = TextView(context).apply {
                text = label
                contentDescription = label
                gravity = Gravity.CENTER
                isClickable = true
                isFocusable = true
                setPadding(dp(12), 0, dp(12), 0)
                setOnClickListener {
                    if (!silent && index != selectedIndex) {
                        selectedIndex = index
                        applySelection()
                        root.onTabSelected(nodeId, index)
                    }
                }
            }
            addView(
                tab,
                LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f),
            )
        }
    }

    private fun applySelection() {
        silent = true
        try {
            for (index in 0 until childCount) {
                updateTab(getChildAt(index) as TextView, index)
            }
        } finally {
            silent = false
        }
    }

    private fun updateTab(tab: TextView, index: Int) {
        val selected = index == selectedIndex
        tab.setTextSize(TypedValue.COMPLEX_UNIT_SP, size)
        tab.setTextColor(if (selected) color else faded(color))
        tab.typeface = if (bold || selected) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        tab.isSelected = selected
    }

    private fun faded(argb: Int): Int = (argb and 0x00FFFFFF) or (0x99 shl 24)

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
