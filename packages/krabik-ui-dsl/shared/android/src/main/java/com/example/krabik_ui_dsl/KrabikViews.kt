package com.example.krabik_ui_dsl

import android.content.Context
import android.view.View
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
    fun createScroll(context: Context, horizontal: Boolean, root: RustRootView): ViewGroup {
        val scroll = if (horizontal) {
            KrabikHorizontalScrollView(context, root)
        } else {
            KrabikScrollView(context, root)
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

/**
 * Reports a scroll offset to Rust at most once per frame.
 *
 * A virtualized list only needs a new render when the visible row range
 * changes; coalescing here keeps a fling from queueing a callback per pixel.
 */
private class ScrollReporter(
    private val root: RustRootView,
    private val view: View,
) {
    private var scheduled = false
    private var latest = 0

    fun report(offset: Int) {
        latest = offset
        if (scheduled) return
        scheduled = true
        view.postOnAnimation {
            scheduled = false
            val id = view.tag as? Long ?: return@postOnAnimation
            root.onScrolled(id, latest)
        }
    }
}

private class KrabikScrollView(
    context: Context,
    root: RustRootView,
) : ScrollView(context) {
    private val reporter = ScrollReporter(root, this)

    override fun onScrollChanged(l: Int, t: Int, oldl: Int, oldt: Int) {
        super.onScrollChanged(l, t, oldl, oldt)
        reporter.report(t)
    }
}

private class KrabikHorizontalScrollView(
    context: Context,
    root: RustRootView,
) : HorizontalScrollView(context) {
    private val reporter = ScrollReporter(root, this)

    override fun onScrollChanged(l: Int, t: Int, oldl: Int, oldt: Int) {
        super.onScrollChanged(l, t, oldl, oldt)
        reporter.report(l)
    }
}
