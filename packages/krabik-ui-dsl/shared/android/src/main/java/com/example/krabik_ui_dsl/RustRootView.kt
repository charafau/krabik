package com.example.krabik_ui_dsl

import android.animation.ValueAnimator
import android.content.Context
import android.util.LongSparseArray
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.InputMethodManager
import android.widget.FrameLayout
import java.lang.ref.WeakReference
import java.util.WeakHashMap
import kotlin.math.roundToInt

/**
 * The only Android widget the Rust UI owns.
 *
 * Rust creates the native widgets as children of this group, gives them exact
 * pixel frames, and receives their clicks. No layout is done in Kotlin beyond
 * measuring the children at the sizes Rust has already calculated.
 *
 * The host must load the native renderer (`libkrabik_ui_dsl` or a shell that
 * statically includes it) before constructing this view; [KrabikActivity]
 * does that through its `loadNativeLibrary` hook.
 */
class RustRootView(
    context: Context,
    sessionHandle: Long = 0L,
    host: Long = 0L,
) : ViewGroup(context), View.OnClickListener {

    private val ownsSession = sessionHandle == 0L
    private val hostId: Long = host

    /** The session handle shared by every host in this Activity. */
    val session: Long
        get() = handle
    private var handle: Long = if (ownsSession) {
        nativeCreate(context, resources.displayMetrics.density)
    } else {
        sessionHandle
    }

    /** Views by node id, so a layout batch can address them without an array of objects. */
    private val nodeViews = LongSparseArray<WeakReference<View>>()

    /** Running frame animations, so a newer frame cancels the previous one. */
    private val frameAnimators = WeakHashMap<View, ValueAnimator>()

    init {
        if (!ownsSession) {
            nativeAttachHost(handle, hostId)
        }
    }
    private var renderedWidth = -1
    private var renderedHeight = -1

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val height = MeasureSpec.getSize(heightMeasureSpec)
        setMeasuredDimension(width, height)

        if (handle != 0L && width > 0 && height > 0 && (width != renderedWidth || height != renderedHeight)) {
            renderedWidth = width
            renderedHeight = height
            nativeRender(handle, hostId, width, height)
        }

        for (index in 0 until childCount) {
            val child = getChildAt(index)
            val params = child.layoutParams
            val childWidth = params?.width?.takeIf { it >= 0 } ?: width
            val childHeight = params?.height?.takeIf { it >= 0 } ?: height
            child.measure(
                MeasureSpec.makeMeasureSpec(childWidth, MeasureSpec.EXACTLY),
                MeasureSpec.makeMeasureSpec(childHeight, MeasureSpec.EXACTLY),
            )
        }
    }

    override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
        // Rust positions children with translation. Their own frame is anchored
        // at the origin and takes the size it calculated for the layout pass.
        for (index in 0 until childCount) {
            val child = getChildAt(index)
            val params = child.layoutParams
            val width = params?.width?.takeIf { it >= 0 } ?: child.measuredWidth
            val height = params?.height?.takeIf { it >= 0 } ?: child.measuredHeight
            if (child.measuredWidth != width || child.measuredHeight != height) {
                child.measure(
                    MeasureSpec.makeMeasureSpec(width, MeasureSpec.EXACTLY),
                    MeasureSpec.makeMeasureSpec(height, MeasureSpec.EXACTLY),
                )
            }
            child.layout(0, 0, width, height)
        }
    }

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        super.onSizeChanged(width, height, oldWidth, oldHeight)
        if (handle != 0L && width > 0 && height > 0 && (width != renderedWidth || height != renderedHeight)) {
            renderedWidth = width
            renderedHeight = height
            nativeRender(handle, hostId, width, height)
        }
    }

    override fun onClick(view: View) {
        val nodeId = view.tag as? Long ?: return
        nativeOnClick(handle, hostId, nodeId)
    }

    /**
     * Applies one node's frame, in pixels.
     *
     * Rust calls this once per moved node instead of issuing separate
     * translation and layout-params calls. The params are mutated in place and
     * only reassigned when the size changes, so a pure move does not schedule
     * another layout pass.
     */
    fun applyFrame(view: View, x: Int, y: Int, width: Int, height: Int) {
        applyFrameInternal(view, x, y, width, height, 0, 0)
    }

    /**
     * Applies a batch of frames.
     *
     * [ids] holds one node id per frame and [frames] four integers per id in
     * `x, y, width, height` order. [animations] holds two integers per id in
     * `duration, curve` order; zero means the frame moves immediately.
     * Unknown ids are skipped, which can only happen for a node whose view was
     * already removed.
     */
    fun applyLayouts(ids: LongArray, frames: IntArray, animations: IntArray) {
        var offset = 0
        var index = 0
        for (id in ids) {
            val view = nodeViews.get(id)?.get()
            if (view != null) {
                applyFrameInternal(
                    view,
                    frames[offset],
                    frames[offset + 1],
                    frames[offset + 2],
                    frames[offset + 3],
                    animations[index * 2],
                    animations[index * 2 + 1],
                )
            }
            offset += 4
            index += 1
        }
    }

    /** Remembers a node's view so a later batch can address it by id. */
    fun registerView(nodeId: Long, view: View) {
        nodeViews.put(nodeId, WeakReference(view))
    }

    /** Forgets a node's view after Rust removed it. */
    fun unregisterView(nodeId: Long) {
        val view = nodeViews.get(nodeId)?.get()
        nodeViews.remove(nodeId)
        if (view != null) {
            frameAnimators.remove(view)?.cancel()
        }
    }

    /** The number of registered views, for tests. */
    fun registeredViewCount(): Int = nodeViews.size()

    private fun applyFrameInternal(
        view: View,
        x: Int,
        y: Int,
        width: Int,
        height: Int,
        duration: Int,
        curve: Int,
    ) {
        frameAnimators.remove(view)?.cancel()

        if (duration <= 0) {
            setFrame(view, x, y, width, height)
            return
        }

        val startX = view.translationX
        val startY = view.translationY
        val startWidth = if (view.width > 0) view.width else width
        val startHeight = if (view.height > 0) view.height else height

        val animator = ValueAnimator.ofFloat(0f, 1f).apply {
            this.duration = duration.toLong()
            interpolator = KrabikStyle.interpolator(curve)
            addUpdateListener { animation ->
                val fraction = animation.animatedValue as Float
                view.translationX = startX + (x - startX) * fraction
                view.translationY = startY + (y - startY) * fraction
                setFrame(
                    view,
                    x = 0,
                    y = 0,
                    width = lerp(startWidth, width, fraction),
                    height = lerp(startHeight, height, fraction),
                    move = false,
                )
            }
        }
        frameAnimators[view] = animator
        animator.start()
    }

    /** Applies a frame directly. Set [move] to false when only the size changes. */
    private fun setFrame(
        view: View,
        x: Int,
        y: Int,
        width: Int,
        height: Int,
        move: Boolean = true,
    ) {
        val params = view.layoutParams
        if (params == null) {
            view.layoutParams = FrameLayout.LayoutParams(width, height)
        } else if (params.width != width || params.height != height) {
            params.width = width
            params.height = height
            view.layoutParams = params
        }
        if (move) {
            view.translationX = x.toFloat()
            view.translationY = y.toFloat()
        }
    }

    private fun lerp(start: Int, end: Int, fraction: Float): Int =
        (start + (end - start) * fraction).roundToInt()

    /** Forwards an edit of a native input back to Rust. */
    fun onTextChanged(nodeId: Long, text: String) {
        if (handle != 0L) {
            nativeOnTextChanged(handle, hostId, nodeId, text)
        }
    }

    /** Forwards a keyboard submit of a native input back to Rust. */
    fun onSubmitted(nodeId: Long) {
        if (handle != 0L) {
            nativeOnSubmitted(handle, hostId, nodeId)
        }
    }

    /** Forwards a focus change of a native input back to Rust. */
    fun onFocusChanged(nodeId: Long, focused: Boolean, attached: Boolean) {
        if (handle != 0L) {
            nativeOnFocusChanged(handle, hostId, nodeId, focused, attached)
        }
    }

    /** Restores focus after a recycled row with a logical focus is reattached. */
    fun restoreRowFocus(nodeId: Long, holderId: Long) {
        if (handle != 0L) {
            nativeRestoreRowFocus(handle, hostId, nodeId, holderId)
        }
    }

    /** Reopens the keyboard for a field that regained focus programmatically. */
    fun ensureKeyboardShown(view: View) {
        val manager = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        // Never ask for the IME while RecyclerView is laying out.
        view.post { manager.showSoftInput(view, 0) }
    }

    /** Forwards a toggle of a native switch back to Rust. */
    fun onCheckedChanged(nodeId: Long, checked: Boolean) {
        if (handle != 0L) {
            nativeOnCheckedChanged(handle, hostId, nodeId, checked)
        }
    }

    /** Forwards a slider value, in the slider's own range, back to Rust. */
    fun onSliderChanged(nodeId: Long, value: Float) {
        if (handle != 0L) {
            nativeOnSliderChanged(handle, hostId, nodeId, value)
        }
    }

    /** Forwards the selected index of a native radio group back to Rust. */
    fun onRadioGroupSelected(nodeId: Long, index: Int) {
        if (handle != 0L) {
            nativeOnRadioGroupSelected(handle, hostId, nodeId, index)
        }
    }

    /** Forwards the selected index of a native tab bar back to Rust. */
    fun onTabSelected(nodeId: Long, index: Int) {
        if (handle != 0L) {
            nativeOnTabSelected(handle, hostId, nodeId, index)
        }
    }

    /** Forwards a date, time or date-and-time selection back to Rust. */
    fun onPickerChanged(
        nodeId: Long,
        kind: Int,
        year: Int,
        month: Int,
        day: Int,
        hour: Int,
        minute: Int,
    ) {
        if (handle != 0L) {
            nativeOnPickerChanged(handle, hostId, nodeId, kind, year, month, day, hour, minute)
        }
    }

    /** Asks Rust to build one recycled-list row into [host], at [width] pixels. */
    fun bindRow(nodeId: Long, index: Int, holderId: Long, host: ViewGroup, width: Int): Int =
        if (handle != 0L) {
            nativeBindRow(handle, hostId, nodeId, index, holderId, host, width)
        } else {
            0
        }

    /** Tells Rust that [holderId] no longer shows a row. */
    fun unbindRow(nodeId: Long, holderId: Long) {
        if (handle != 0L) {
            nativeUnbindRow(handle, hostId, nodeId, holderId)
        }
    }

    /** Forwards the scroll offset, in pixels, of a native scroll container. */
    fun onScrolled(nodeId: Long, offset: Int) {
        if (handle != 0L) {
            nativeOnScroll(handle, hostId, nodeId, offset.toFloat())
        }
    }

    /** Dispatches the system back button to Rust; returns whether it was handled. */
    fun onBackPressed(): Boolean = if (handle != 0L) nativeOnBack(handle) else false

    /** Registers the native navigation/modal controller with the session. */
    fun setController(controller: Any) {
        if (handle != 0L) {
            nativeSetController(handle, controller)
        }
    }

    /** Asks the application to close the presented modal. */
    fun onModalDismiss() {
        if (handle != 0L) {
            nativeOnModalDismiss(handle)
        }
    }

    /** Releases the Rust state when the activity goes away. */
    fun destroy() {
        if (handle != 0L) {
            if (ownsSession) {
                nativeDestroy(handle)
            } else {
                nativeDestroyHost(handle, hostId)
            }
            handle = 0L
        }
    }

    private external fun nativeCreate(
        context: Context,
        density: Float,
    ): Long

    private external fun nativeRender(handle: Long, host: Long, widthPx: Int, heightPx: Int)

    private external fun nativeOnClick(handle: Long, host: Long, nodeId: Long)

    private external fun nativeOnTextChanged(handle: Long, host: Long, nodeId: Long, text: String)

    private external fun nativeOnSubmitted(handle: Long, host: Long, nodeId: Long)

    private external fun nativeOnFocusChanged(
        handle: Long,
        host: Long,
        nodeId: Long,
        focused: Boolean,
        attached: Boolean,
    )

    private external fun nativeRestoreRowFocus(handle: Long, host: Long, nodeId: Long, holderId: Long)

    private external fun nativeOnCheckedChanged(handle: Long, host: Long, nodeId: Long, checked: Boolean)

    private external fun nativeOnSliderChanged(handle: Long, host: Long, nodeId: Long, value: Float)

    private external fun nativeOnRadioGroupSelected(handle: Long, host: Long, nodeId: Long, index: Int)

    private external fun nativeOnTabSelected(handle: Long, host: Long, nodeId: Long, index: Int)

    private external fun nativeOnPickerChanged(
        handle: Long,
        host: Long,
        nodeId: Long,
        kind: Int,
        year: Int,
        month: Int,
        day: Int,
        hour: Int,
        minute: Int,
    )

    private external fun nativeOnScroll(handle: Long, host: Long, nodeId: Long, offset: Float)

    private external fun nativeOnBack(handle: Long): Boolean

    private external fun nativeSetController(handle: Long, controller: Any)

    private external fun nativeOnModalDismiss(handle: Long)

    private external fun nativeBindRow(
        handle: Long,
        host: Long,
        nodeId: Long,
        index: Int,
        holderId: Long,
        holderHost: ViewGroup,
        width: Int,
    ): Int

    private external fun nativeUnbindRow(handle: Long, host: Long, nodeId: Long, holderId: Long)

    private external fun nativeDestroy(handle: Long)

    private external fun nativeAttachHost(handle: Long, host: Long)

    private external fun nativeDestroyHost(handle: Long, host: Long)
}
