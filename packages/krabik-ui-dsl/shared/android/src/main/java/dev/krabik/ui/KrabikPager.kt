package dev.krabik.ui

import android.content.Context
import android.view.MotionEvent
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.OverScroller
import kotlin.math.abs
import kotlin.math.roundToInt

/**
 * The native widget used for `Pager` nodes.
 *
 * Pages are ordinary Rust-rendered children laid out side by side by the
 * renderer. This view scrolls horizontally, snaps to page boundaries and
 * reports a settled page back to Rust. Programmatic page changes from Rust
 * never echo back as events.
 */
class KrabikPager(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : FrameLayout(context) {

    private val scroller = OverScroller(context)
    private val touchSlop = ViewConfiguration.get(context).scaledTouchSlop
    private val minimumVelocity = ViewConfiguration.get(context).scaledMinimumFlingVelocity
    private val maximumVelocity = ViewConfiguration.get(context).scaledMaximumFlingVelocity

    private var pages = 0
    private var selected = 0
    private var configured = false
    private var lastReported = 0
    private var dragging = false
    private var notifyOnSettle = false
    private var downX = 0f
    private var downY = 0f
    private var lastX = 0f
    private var velocity: android.view.VelocityTracker? = null

    init {
        isFocusable = true
        clipChildren = true
        clipToPadding = true
    }

    /** Applies the page count and selected index from Rust. */
    fun configure(pages: Int, selected: Int) {
        val first = !configured
        val count = pages.coerceAtLeast(0)
        val target = if (count == 0) 0 else selected.coerceIn(0, count - 1)
        val changed = this.pages != count || this.selected != target
        this.pages = count
        this.selected = target
        configured = true
        if (changed) {
            lastReported = target
            scrollToPage(target, animated = !first)
        }
    }

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        super.onSizeChanged(width, height, oldWidth, oldHeight)
        scroller.abortAnimation()
        if (width > 0) {
            scrollTo((selected * width).coerceIn(0, maxScroll()), 0)
        }
    }

    override fun onInterceptTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.x
                downY = event.y
                lastX = event.x
                dragging = false
                notifyOnSettle = false
                scroller.abortAnimation()
                beginVelocity()
                velocity?.addMovement(event)
                return false
            }
            MotionEvent.ACTION_MOVE -> {
                velocity?.addMovement(event)
                val dx = abs(event.x - downX)
                val dy = abs(event.y - downY)
                if (!dragging && dx > touchSlop && dx > dy) {
                    dragging = true
                    lastX = event.x
                    parent?.requestDisallowInterceptTouchEvent(true)
                    return true
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> endVelocity()
        }
        return false
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (pages <= 1 || width <= 0) {
            return super.onTouchEvent(event)
        }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.x
                downY = event.y
                lastX = event.x
                dragging = false
                notifyOnSettle = false
                scroller.abortAnimation()
                beginVelocity()
                velocity?.addMovement(event)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                velocity?.addMovement(event)
                val dx = abs(event.x - downX)
                val dy = abs(event.y - downY)
                if (!dragging && dx > touchSlop && dx > dy) {
                    dragging = true
                    lastX = event.x
                    parent?.requestDisallowInterceptTouchEvent(true)
                }
                if (dragging) {
                    val delta = lastX - event.x
                    lastX = event.x
                    val target = (scrollX + delta).roundToInt().coerceIn(0, maxScroll())
                    scrollTo(target, 0)
                    return true
                }
            }
            MotionEvent.ACTION_UP -> {
                velocity?.addMovement(event)
                if (dragging) {
                    val tracker = velocity
                    tracker?.computeCurrentVelocity(1000, maximumVelocity.toFloat())
                    val velocityX = tracker?.xVelocity ?: 0f
                    notifyOnSettle = true
                    if (abs(velocityX) > minimumVelocity) {
                        scroller.fling(
                            scrollX,
                            0,
                            -velocityX.roundToInt(),
                            0,
                            0,
                            maxScroll(),
                            0,
                            0,
                        )
                    } else {
                        snapToNearest()
                    }
                    postInvalidateOnAnimation()
                }
                endVelocity()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (dragging) {
                    notifyOnSettle = true
                    snapToNearest()
                    postInvalidateOnAnimation()
                }
                endVelocity()
                return true
            }
        }
        return super.onTouchEvent(event)
    }

    override fun computeScroll() {
        if (scroller.computeScrollOffset()) {
            scrollTo(scroller.currX, 0)
            postInvalidateOnAnimation()
        } else if (notifyOnSettle) {
            finishGesture()
        }
    }

    private fun snapToNearest() {
        if (width <= 0 || pages <= 0) return
        val page = (scrollX.toFloat() / width).roundToInt().coerceIn(0, pages - 1)
        val target = page * width
        scroller.startScroll(scrollX, 0, target - scrollX, 0, SNAP_MS)
    }

    private fun finishGesture() {
        notifyOnSettle = false
        dragging = false
        if (width <= 0 || pages <= 0) return
        val page = (scrollX.toFloat() / width).roundToInt().coerceIn(0, pages - 1)
        if (page != lastReported) {
            lastReported = page
            selected = page
            root.onPageSelected(nodeId, page)
        }
    }

    private fun scrollToPage(index: Int, animated: Boolean) {
        if (width <= 0) {
            return
        }
        val target = (index.coerceIn(0, (pages - 1).coerceAtLeast(0)) * width)
            .coerceIn(0, maxScroll())
        scroller.abortAnimation()
        notifyOnSettle = false
        if (animated) {
            scroller.startScroll(scrollX, 0, target - scrollX, 0, PROGRAMMATIC_MS)
            postInvalidateOnAnimation()
        } else {
            scrollTo(target, 0)
        }
    }

    private fun maxScroll(): Int = ((pages - 1).coerceAtLeast(0)) * width

    private fun beginVelocity() {
        velocity?.recycle()
        velocity = android.view.VelocityTracker.obtain()
    }

    private fun endVelocity() {
        velocity?.recycle()
        velocity = null
    }

    private companion object {
        const val SNAP_MS = 250
        const val PROGRAMMATIC_MS = 280
    }
}
