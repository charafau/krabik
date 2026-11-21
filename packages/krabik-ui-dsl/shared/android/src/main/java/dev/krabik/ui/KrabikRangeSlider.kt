package dev.krabik.ui

import android.content.Context
import android.content.res.ColorStateList
import com.google.android.material.slider.LabelFormatter
import com.google.android.material.slider.RangeSlider

/**
 * The native widget used for `RangeSlider` nodes.
 *
 * The slider works between 0 and 1 without a step: Rust maps the two places
 * to the values of the DSL slider and snaps them, so a range or a step that
 * Material's slider would reject cannot reach it. Programmatic updates do
 * not echo back as events.
 */
class KrabikRangeSlider(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : RangeSlider(context) {

    private var silent = false

    init {
        valueFrom = 0f
        valueTo = 1f
        values = listOf(0f, 1f)
        labelBehavior = LabelFormatter.LABEL_GONE
        addOnChangeListener { slider, _, fromUser ->
            if (fromUser && !silent) {
                val places = slider.values
                root.onRangeChanged(nodeId, places[0], places[1])
            }
        }
    }

    /** Applies the accent color from Rust. */
    fun configure(color: Int) {
        val tint = ColorStateList.valueOf(color)
        trackActiveTintList = tint
        thumbTintList = tint
    }

    /** Applies the two places from Rust, each from 0 to 1, without reporting them back. */
    fun setPlacesSilently(low: Float, high: Float) {
        val first = low.coerceIn(0f, 1f)
        val second = high.coerceIn(first, 1f)
        val current = values
        if (current.size == 2 && current[0] == first && current[1] == second) {
            return
        }
        silent = true
        try {
            values = listOf(first, second)
        } finally {
            silent = false
        }
    }
}
