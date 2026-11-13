package dev.krabik.ui

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Typeface
import android.util.TypedValue
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import kotlin.math.roundToInt

/**
 * The native widget used for `RadioGroup` nodes.
 *
 * The group owns mutual exclusion: tapping an option checks it and unchecks
 * its siblings, then reports the selected index back to Rust. State applied
 * from Rust does not echo back as another event.
 */
class KrabikRadioGroup(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : RadioGroup(context) {

    private var silent = false
    private var labels: List<String> = emptyList()

    init {
        orientation = LinearLayout.VERTICAL
        setOnCheckedChangeListener { _, checkedId ->
            if (!silent && checkedId != NO_SELECTION) {
                root.onRadioGroupSelected(nodeId, checkedId - ID_BASE)
            }
        }
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
        spacing: Float,
        bold: Boolean,
    ) {
        val options = labels.toList()
        if (this.labels != options) {
            rebuild(options)
        }

        silent = true
        try {
            val typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
            val tint = ColorStateList.valueOf(color)
            val gap = (spacing * resources.displayMetrics.density).roundToInt()
            for (index in 0 until childCount) {
                val button = getChildAt(index) as RadioButton
                button.setTextSize(TypedValue.COMPLEX_UNIT_SP, size)
                button.setTextColor(color)
                button.typeface = typeface
                button.buttonTintList = tint
                (button.layoutParams as LayoutParams).bottomMargin =
                    if (index == childCount - 1) 0 else gap
                button.requestLayout()
            }

            val target = if (selected in options.indices) selected + ID_BASE else NO_SELECTION
            if (checkedRadioButtonId != target) {
                if (target == NO_SELECTION) {
                    clearCheck()
                } else {
                    check(target)
                }
            }
        } finally {
            silent = false
        }
    }

    private fun rebuild(options: List<String>) {
        silent = true
        try {
            removeAllViews()
            for ((index, label) in options.withIndex()) {
                val button = RadioButton(context)
                button.id = ID_BASE + index
                button.text = label
                button.isClickable = true
                button.setPadding(0, 0, 0, 0)
                addView(
                    button,
                    LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT),
                )
            }
            labels = options
        } finally {
            silent = false
        }
    }

    private companion object {
        const val ID_BASE = 1
        const val NO_SELECTION = -1
    }
}
