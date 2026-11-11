package com.example.krabik_ui_dsl

import android.content.Context
import android.content.res.ColorStateList
import android.widget.SeekBar

/**
 * The native widget used for `Slider` nodes.
 *
 * The SeekBar works in integer steps between 0 and [STEPS]; the widget maps
 * that range to the `min..max` values of the DSL slider and reports logical
 * values back to Rust. Programmatic updates do not echo back as events.
 */
class KrabikSeekBar(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : SeekBar(context) {

    private var minValue = 0f
    private var maxValue = 1f
    private var silent = false

    init {
        max = STEPS
        setOnSeekBarChangeListener(
            object : OnSeekBarChangeListener {
                override fun onProgressChanged(seekBar: SeekBar, progress: Int, fromUser: Boolean) {
                    if (fromUser && !silent) {
                        root.onSliderChanged(nodeId, valueFor(progress))
                    }
                }

                override fun onStartTrackingTouch(seekBar: SeekBar) = Unit

                override fun onStopTrackingTouch(seekBar: SeekBar) = Unit
            },
        )
    }

    /** Applies the value range and accent color from Rust. */
    fun configure(min: Float, max: Float, color: Int) {
        minValue = min
        maxValue = max
        val tint = ColorStateList.valueOf(color)
        progressTintList = tint
        thumbTintList = tint
    }

    /** Applies a value from Rust without reporting it back. */
    fun setValueSilently(value: Float) {
        val target = progressFor(value)
        if (target == progress) {
            return
        }

        silent = true
        try {
            progress = target
        } finally {
            silent = false
        }
    }

    private fun span(): Float = (maxValue - minValue).coerceAtLeast(0f)

    private fun valueFor(progress: Int): Float {
        val span = span()
        if (span <= 0f) {
            return minValue
        }
        return minValue + progress.toFloat() / STEPS * span
    }

    private fun progressFor(value: Float): Int {
        val span = span()
        if (span <= 0f) {
            return 0
        }
        val clamped = value.coerceIn(minValue, maxValue)
        return ((clamped - minValue) / span * STEPS).toInt().coerceIn(0, STEPS)
    }

    private companion object {
        const val STEPS = 1000
    }
}
