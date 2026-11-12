package com.example.krabik_ui_dsl

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import android.view.animation.AccelerateInterpolator
import android.view.animation.DecelerateInterpolator
import android.view.animation.Interpolator
import android.view.animation.LinearInterpolator
import java.util.WeakHashMap

/**
 * Applies the platform-independent style properties to native views.
 *
 * Keeping drawable construction here keeps the Rust renderer to a single JNI
 * call per styled node and preserves the native ripple of buttons.
 */
object KrabikStyle {
    private val originalBackgrounds = WeakHashMap<View, Drawable?>()

    @JvmStatic
    fun apply(
        view: View,
        backgroundColor: Int,
        hasBackground: Boolean,
        gradientStart: Int,
        gradientEnd: Int,
        hasGradient: Boolean,
        gradientHorizontal: Boolean,
        cornerRadius: Float,
        topLeftRadius: Float,
        topRightRadius: Float,
        bottomRightRadius: Float,
        bottomLeftRadius: Float,
        hasCornerRadii: Boolean,
        borderWidth: Float,
        borderColor: Int,
        hasBorder: Boolean,
        opacity: Float,
        ripple: Boolean,
        elevation: Float,
        animationDuration: Int,
        animationCurve: Int,
    ) {
        val density = view.resources.displayMetrics.density
        val borderPx = borderWidth * density
        val radii = radiiInPixels(
            density,
            if (hasCornerRadii) {
                floatArrayOf(
                    topLeftRadius,
                    topRightRadius,
                    bottomRightRadius,
                    bottomLeftRadius,
                )
            } else {
                floatArrayOf(cornerRadius, cornerRadius, cornerRadius, cornerRadius)
            },
        )
        val hasShape = hasBackground || hasGradient || hasBorder ||
            radii.any { it > 0f } || elevation > 0f

        if (!hasShape) {
            // Restore what the platform gave the view, if we ever replaced it.
            if (originalBackgrounds.containsKey(view)) {
                view.background = originalBackgrounds.remove(view)
            }
        } else {
            if (!originalBackgrounds.containsKey(view)) {
                originalBackgrounds[view] = view.background
            }

            val shape = GradientDrawable().apply {
                this.shape = GradientDrawable.RECTANGLE
                if (hasGradient) {
                    colors = intArrayOf(gradientStart, gradientEnd)
                    orientation = if (gradientHorizontal) {
                        GradientDrawable.Orientation.LEFT_RIGHT
                    } else {
                        GradientDrawable.Orientation.TOP_BOTTOM
                    }
                } else {
                    setColor(if (hasBackground) backgroundColor else Color.TRANSPARENT)
                }
                setCornerRadii(radii)
                if (hasBorder && borderPx > 0f) {
                    setStroke(borderPx.toInt().coerceAtLeast(1), borderColor)
                }
            }

            view.background = if (ripple) {
                val mask = GradientDrawable().apply {
                    this.shape = GradientDrawable.RECTANGLE
                    setColor(Color.WHITE)
                    setCornerRadii(radii)
                }
                RippleDrawable(ColorStateList.valueOf(RIPPLE_COLOR), shape, mask)
            } else {
                shape
            }
        }

        view.clipToOutline = radii.any { it > 0f }
        view.elevation = (elevation * density).coerceAtLeast(0f)

        val targetAlpha = opacity.coerceIn(0f, 1f)
        if (animationDuration > 0) {
            view.animate()
                .setDuration(animationDuration.toLong())
                .setInterpolator(interpolator(animationCurve))
                .alpha(targetAlpha)
        } else {
            view.animate().cancel()
            view.alpha = targetAlpha
        }
    }

    /** The native interpolator for a shared animation curve code. */
    @JvmStatic
    fun interpolator(curve: Int): Interpolator = when (curve) {
        1 -> LinearInterpolator()
        2 -> AccelerateInterpolator()
        3 -> DecelerateInterpolator()
        else -> AccelerateDecelerateInterpolator()
    }

    /**
     * Converts four radii into the eight-value form expected by
     * `GradientDrawable.setCornerRadii`, scaling to pixels.
     */
    private fun radiiInPixels(density: Float, radii: FloatArray): FloatArray {
        val px = FloatArray(8)
        for (corner in 0 until 4) {
            px[corner * 2] = radii[corner] * density
            px[corner * 2 + 1] = radii[corner] * density
        }
        return px
    }

    private const val RIPPLE_COLOR: Int = 0x1F000000
}
