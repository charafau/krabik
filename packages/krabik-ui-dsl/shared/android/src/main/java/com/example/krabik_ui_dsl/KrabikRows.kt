package com.example.krabik_ui_dsl

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import android.view.ViewGroup
import android.view.animation.DecelerateInterpolator
import android.widget.ImageView

/**
 * Visual helpers for recycled rows.
 *
 * A rebind replaces the widgets inside a holder immediately. When the list opts
 * into a row transition, the previous content is captured into an overlay that
 * fades away underneath the new content. Nothing here delays holder reuse: the
 * overlay is a normal child of the holder and leaves with it.
 */
object KrabikRows {
    private const val OVERLAY_TAG = "krabik-row-overlay"

    /** Captures [host] into an overlay and returns it, or `null` when it has no size. */
    @JvmStatic
    fun snapshot(host: ViewGroup): ImageView? {
        val width = host.width
        val height = host.height
        if (width <= 0 || height <= 0) {
            return null
        }
        clear(host)
        val bitmap = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        host.draw(Canvas(bitmap))
        val overlay = ImageView(host.context).apply {
            setImageBitmap(bitmap)
            scaleType = ImageView.ScaleType.FIT_XY
            tag = OVERLAY_TAG
            isClickable = false
            isFocusable = false
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        host.addView(overlay, ViewGroup.LayoutParams(width, height))
        return overlay
    }

    /** Brings [overlay] to the front and fades it away. */
    @JvmStatic
    fun fadeOut(host: ViewGroup, overlay: View?, durationMs: Int) {
        if (overlay == null) {
            return
        }
        clear(host, keep = overlay)
        overlay.bringToFront()
        overlay.animate()
            .alpha(0f)
            .setDuration(durationMs.toLong())
            .setInterpolator(DecelerateInterpolator())
            .withEndAction { host.removeView(overlay) }
            .start()
    }

    private fun clear(host: ViewGroup, keep: View? = null) {
        for (index in host.childCount - 1 downTo 0) {
            val child = host.getChildAt(index)
            if (child !== keep && child.tag == OVERLAY_TAG) {
                host.removeViewAt(index)
            }
        }
    }
}
