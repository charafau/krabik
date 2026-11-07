package com.example.krabik_ui_dsl

import android.content.Context
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ScrollView

/**
 * Creates the native containers that have no direct `View` counterpart.
 *
 * A scroll container wraps a single content host. The host keeps its natural
 * size in the scroll axis, while the outer view clips it, so the children
 * rendered by Rust can overflow and scroll. Rust reparents the children into
 * the host and sizes it after layout.
 */
object KrabikViews {
    @JvmStatic
    fun createScroll(context: Context, horizontal: Boolean): ViewGroup {
        val scroll = if (horizontal) {
            HorizontalScrollView(context)
        } else {
            ScrollView(context)
        }

        val host = FrameLayout(context)
        host.isClickable = false
        host.isFocusable = false
        scroll.addView(
            host,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )

        // Fade the edge where content continues, so overflow is discoverable.
        val fadingEdge = (24 * context.resources.displayMetrics.density).toInt()
        scroll.setFadingEdgeLength(fadingEdge)
        scroll.isVerticalFadingEdgeEnabled = !horizontal
        scroll.isHorizontalFadingEdgeEnabled = horizontal
        return scroll
    }
}
