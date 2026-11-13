package dev.krabik.ui

import android.content.Context
import android.content.res.ColorStateList
import android.widget.SeekBar
import kotlin.math.roundToInt

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
    private var stepValue = 0f
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

    /** Applies the value range, quantization step and accent color from Rust. */
    fun configure(min: Float, max: Float, step: Float, color: Int) {
        minValue = min
        maxValue = max
        stepValue = step.coerceAtLeast(0f)
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
        val value = minValue + progress.toFloat() / STEPS * span
        if (stepValue <= 0f) {
            return value
        }
        val snapped = minValue + ((value - minValue) / stepValue).roundToInt() * stepValue
        return snapped.coerceIn(minValue, maxValue)
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
