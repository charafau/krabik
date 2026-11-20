package dev.krabik.ui

import android.content.Context
import android.content.res.ColorStateList
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.HorizontalScrollView
import android.widget.ProgressBar
import android.widget.ScrollView
import androidx.swiperefreshlayout.widget.SwipeRefreshLayout
import kotlin.math.roundToInt

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
    fun createScroll(
        context: Context,
        horizontal: Boolean,
        root: RustRootView,
        refreshable: Boolean,
    ): ViewGroup {
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
        return if (refreshable && !horizontal) KrabikRefreshLayout(context, root, scroll) else scroll
    }

    /**
     * The view Rust adds a scroll container's children to: the content host
     * inside the scroll view, which may itself be inside a refresh layout.
     */
    @JvmStatic
    fun contentHost(view: ViewGroup): View = (scrolling(view) as ViewGroup).getChildAt(0)

    /** The scroll view or recycler itself, without a refresh layout around it. */
    @JvmStatic
    fun scrolling(view: View): View = (view as? KrabikRefreshLayout)?.content ?: view

    /** Shows or hides the refresh indicator of a refreshable container. */
    @JvmStatic
    fun setRefreshing(view: View, refreshing: Boolean) {
        val layout = view as? KrabikRefreshLayout ?: return
        if (layout.isRefreshing != refreshing) {
            layout.isRefreshing = refreshing
        }
    }

    /**
     * Makes a container accept taps, or stops it. Rust set the root view as
     * its click listener when it created the view; a tappable container also
     * shows the theme's press highlight over its content.
     */
    @JvmStatic
    fun setTappable(view: View, tappable: Boolean) {
        if (view.isClickable == tappable && (view.foreground != null) == tappable) return
        view.isClickable = tappable
        view.isFocusable = tappable
        view.foreground = if (tappable) pressHighlight(view.context) else null
    }

    private fun pressHighlight(context: Context): android.graphics.drawable.Drawable? {
        val value = android.util.TypedValue()
        val found = context.theme.resolveAttribute(
            android.R.attr.selectableItemBackground,
            value,
            true,
        )
        return if (found && value.resourceId != 0) context.getDrawable(value.resourceId) else null
    }

    /** Creates the horizontal progress bar used for `ProgressIndicator` nodes. */
    @JvmStatic
    fun createProgress(context: Context): ProgressBar =
        ProgressBar(context, null, android.R.attr.progressBarStyleHorizontal)

    /** Applies a `0..1` fraction and the fill color to a progress bar. */
    @JvmStatic
    fun updateProgress(view: ProgressBar, value: Float, color: Int) {
        view.max = PROGRESS_STEPS
        view.progress = (value.coerceIn(0f, 1f) * PROGRESS_STEPS).roundToInt()
        view.progressTintList = ColorStateList.valueOf(color)
    }

    /** Configures a circular indeterminate spinner and its color. */
    @JvmStatic
    fun updateSpinner(view: ProgressBar, color: Int) {
        if (!view.isIndeterminate) {
            view.isIndeterminate = true
        }
        view.indeterminateTintList = ColorStateList.valueOf(color)
    }

    private const val PROGRESS_STEPS = 1000
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

/**
 * The platform's pull-to-refresh around a vertical scroll view or a recycler.
 *
 * To Rust this is the node's view: it gets the node's frame and tag. What
 * belongs to the scrolling view inside is passed on.
 */
class KrabikRefreshLayout(
    context: Context,
    root: RustRootView,
    val content: ViewGroup,
) : SwipeRefreshLayout(context) {
    init {
        addView(
            content,
            ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        setOnRefreshListener {
            val id = tag as? Long
            if (id != null) root.onRefresh(id) else isRefreshing = false
        }
    }

    /** The scroll view reports its offset under the node's id. */
    override fun setTag(tag: Any?) {
        super.setTag(tag)
        content.tag = tag
    }

    /** Rust resets a list's offset on the node's view. */
    override fun scrollTo(x: Int, y: Int) {
        content.scrollTo(x, y)
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
