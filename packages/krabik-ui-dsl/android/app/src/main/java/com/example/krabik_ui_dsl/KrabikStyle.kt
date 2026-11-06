package com.example.krabik_ui_dsl

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.graphics.drawable.RippleDrawable
import android.view.View
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
        cornerRadius: Float,
        borderWidth: Float,
        borderColor: Int,
        hasBorder: Boolean,
        opacity: Float,
        ripple: Boolean,
    ) {
        val density = view.resources.displayMetrics.density
        val radiusPx = cornerRadius * density
        val borderPx = borderWidth * density

        if (!hasBackground && !hasBorder && radiusPx <= 0f) {
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
                setColor(if (hasBackground) backgroundColor else Color.TRANSPARENT)
                this.cornerRadius = radiusPx
                if (hasBorder && borderPx > 0f) {
                    setStroke(borderPx.toInt().coerceAtLeast(1), borderColor)
                }
            }

            view.background = if (ripple) {
                val mask = GradientDrawable().apply {
                    this.shape = GradientDrawable.RECTANGLE
                    setColor(Color.WHITE)
                    this.cornerRadius = radiusPx
                }
                RippleDrawable(ColorStateList.valueOf(RIPPLE_COLOR), shape, mask)
            } else {
                shape
            }
        }

        view.clipToOutline = radiusPx > 0f
        view.alpha = opacity.coerceIn(0f, 1f)
    }

    private const val RIPPLE_COLOR: Int = 0x1F000000
}
