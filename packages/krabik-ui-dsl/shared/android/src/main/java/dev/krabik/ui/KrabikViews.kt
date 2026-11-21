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
        host.clipChildren = false
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
        clipToOwnBounds(scroll)
        return if (refreshable && !horizontal) KrabikRefreshLayout(context, root, scroll) else scroll
    }

    /**
     * Keeps what a scrolling view draws inside the view. Android leaves that
     * to the parent, and the parents here let their children overflow (see
     * [letChildrenOverflow]), so a view whose content is larger than itself
     * has to clip on its own.
     */
    @JvmStatic
    fun clipToOwnBounds(view: View) {
        view.addOnLayoutChangeListener { changed, left, top, right, bottom, _, _, _, _ ->
            changed.clipBounds = android.graphics.Rect(0, 0, right - left, bottom - top)
        }
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
     * Lets what is inside a container's children show outside those
     * children's own bounds, as UIKit does: a badge on the corner of an icon
     * is a child of the icon's stack and lies partly outside it. A view with
     * rounded corners still clips to its outline.
     */
    @JvmStatic
    fun letChildrenOverflow(view: View) {
        (view as? ViewGroup)?.clipChildren = false
    }

    /**
     * Sets or removes the text the platform shows while a view is pressed
     * and held. A view that never had a tooltip is left alone, so its own
     * long presses stay as they are.
     */
    @JvmStatic
    fun setTooltip(view: View, text: String?) {
        if (text == null && !tooltips.containsKey(view)) return
        if (text == null) tooltips.remove(view) else tooltips[view] = true
        androidx.appcompat.widget.TooltipCompat.setTooltipText(view, text)
    }

    private val tooltips = java.util.WeakHashMap<View, Boolean>()

    /**
     * Makes a container accept taps and long presses, or stops it. Rust set
     * the root view as its listener for both when it created the view; a
     * container that reacts to either also shows the theme's press highlight
     * over its content.
     */
    @JvmStatic
    fun setTappable(view: View, tappable: Boolean, longPressable: Boolean) {
        val reacts = tappable || longPressable
        // A tooltip is shown on a long press, by a listener of its own.
        val longClickable = longPressable || tooltips.containsKey(view)
        if (view.isClickable == tappable &&
            view.isLongClickable == longClickable &&
            (view.foreground != null) == reacts
        ) {
            return
        }
        view.isClickable = tappable
        view.isLongClickable = longClickable
        view.isFocusable = reacts
        view.foreground = if (reacts) pressHighlight(view.context) else null
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
