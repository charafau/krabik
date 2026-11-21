package dev.krabik.ui

import android.content.Context
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.graphics.Canvas
import androidx.recyclerview.widget.ItemTouchHelper
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView

/**
 * Creates and updates the native recycler used by a `recycled` list.
 *
 * The adapter owns no content: it asks Rust to build one row into a host view
 * on every bind, so the DSL tree remains the source of truth while the
 * recycler handles scrolling and holder reuse. Rust caches the row per holder,
 * so a rebind of the same item updates the existing widgets instead of
 * recreating them.
 */
object KrabikRecyclers {
    @JvmStatic
    fun create(
        context: Context,
        root: RustRootView,
        nodeId: Long,
        count: Int,
        itemHeightPx: Int,
        refreshable: Boolean,
    ): ViewGroup {
        val recycler = RecyclerView(context)
        recycler.layoutManager = LinearLayoutManager(context)
        // Animations detach and rebind holders on every range change, which
        // clears the row state and makes focus and actions flicker.
        recycler.itemAnimator = null
        val adapter = RowAdapter(context, root, nodeId, count, itemHeightPx)
        recycler.adapter = adapter
        adapter.recycler = recycler
        recycler.setHasFixedSize(itemHeightPx > 0)
        KrabikViews.clipToOwnBounds(recycler)
        return if (refreshable) KrabikRefreshLayout(context, root, recycler) else recycler
    }

    /**
     * Applies what the description says about the list. `dismissible` and
     * `reorderable` say which gestures on a row the application listens
     * for; `sticky` holds the positions of the rows that stay at the top
     * while their section scrolls, ascending.
     */
    @JvmStatic
    fun update(
        view: ViewGroup,
        count: Int,
        itemHeightPx: Int,
        revision: Long,
        dismissible: Boolean,
        reorderable: Boolean,
        sticky: IntArray,
    ) {
        val recycler = KrabikViews.scrolling(view) as? RecyclerView ?: return
        val adapter = recycler.adapter as? RowAdapter ?: return
        adapter.setGestures(dismissible, reorderable)
        adapter.setSticky(sticky)
        adapter.update(count, itemHeightPx, revision)
        recycler.setHasFixedSize(itemHeightPx > 0)
    }
}

/**
 * Swiping a row away and dragging a row to another place.
 *
 * The recycler shows the gesture; what the list holds is the application's
 * to say. So each gesture ends by telling Rust and then binding the rows
 * again from the state, which puts a row back when the application kept it.
 */
private class RowGestures(private val adapter: RowAdapter) : ItemTouchHelper.Callback() {
    private var draggedFrom = RecyclerView.NO_POSITION
    private var draggedTo = RecyclerView.NO_POSITION

    override fun getMovementFlags(recycler: RecyclerView, holder: RecyclerView.ViewHolder): Int {
        val drag = if (adapter.reorderable) ItemTouchHelper.UP or ItemTouchHelper.DOWN else 0
        val swipe = if (adapter.dismissible) ItemTouchHelper.START or ItemTouchHelper.END else 0
        return makeMovementFlags(drag, swipe)
    }

    override fun isLongPressDragEnabled(): Boolean = adapter.reorderable

    override fun isItemViewSwipeEnabled(): Boolean = adapter.dismissible

    override fun onMove(
        recycler: RecyclerView,
        holder: RecyclerView.ViewHolder,
        target: RecyclerView.ViewHolder,
    ): Boolean {
        if (draggedFrom == RecyclerView.NO_POSITION) {
            draggedFrom = holder.bindingAdapterPosition
        }
        draggedTo = target.bindingAdapterPosition
        adapter.notifyItemMoved(holder.bindingAdapterPosition, target.bindingAdapterPosition)
        return true
    }

    override fun onSwiped(holder: RecyclerView.ViewHolder, direction: Int) {
        val position = holder.bindingAdapterPosition
        if (position != RecyclerView.NO_POSITION) {
            adapter.dismissed(position)
        }
    }

    override fun clearView(recycler: RecyclerView, holder: RecyclerView.ViewHolder) {
        super.clearView(recycler, holder)
        val (from, to) = draggedFrom to draggedTo
        draggedFrom = RecyclerView.NO_POSITION
        draggedTo = RecyclerView.NO_POSITION
        if (from != RecyclerView.NO_POSITION && to != RecyclerView.NO_POSITION && from != to) {
            // The recycler is still laying out; tell Rust once it is done.
            recycler.post { adapter.moved(from, to) }
        }
    }
}

/**
 * Draws the current section's header over the top of the list.
 *
 * The header is an ordinary row. Its picture comes from a holder of its own
 * that is never attached: Rust binds the row into it as into any holder, and
 * the decoration draws that view at the top, pushed up by the next header.
 */
private class StickyHeaders(private val adapter: RowAdapter) : RecyclerView.ItemDecoration() {
    private var holder: RowAdapter.Holder? = null
    private var shown = RecyclerView.NO_POSITION
    private var shownRevision = Long.MIN_VALUE
    private var shownWidth = 0

    override fun onDrawOver(canvas: Canvas, recycler: RecyclerView, state: RecyclerView.State) {
        val layout = recycler.layoutManager as? LinearLayoutManager ?: return
        val first = layout.findFirstVisibleItemPosition()
        if (first == RecyclerView.NO_POSITION) return
        val header = adapter.sticky.lastOrNull { it <= first } ?: return
        // The header row itself is still fully in place: nothing to pin.
        val own = layout.findViewByPosition(header)
        if (own != null && own.top >= 0) return

        val pinned = holder ?: adapter.createViewHolder(recycler, 0).also { holder = it }
        if (header != shown || adapter.revision != shownRevision || recycler.width != shownWidth) {
            adapter.bindDetached(pinned, header, recycler.width)
            shown = header
            shownRevision = adapter.revision
            shownWidth = recycler.width
        }
        val view = pinned.host
        val height = view.layoutParams.height.coerceAtLeast(0)
        view.measure(
            View.MeasureSpec.makeMeasureSpec(recycler.width, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(height, View.MeasureSpec.EXACTLY),
        )
        view.layout(0, 0, recycler.width, height)

        // The next header pushes this one up as it arrives.
        val next = adapter.sticky.firstOrNull { it > first }
        val nextTop = next?.let { layout.findViewByPosition(it)?.top }
        val offset = if (nextTop != null && nextTop < height) nextTop - height else 0
        canvas.save()
        canvas.clipRect(0, 0, recycler.width, (height + offset).coerceAtLeast(0))
        canvas.translate(0f, offset.toFloat())
        view.draw(canvas)
        canvas.restore()
    }
}

private class RowAdapter(
    private val context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
    count: Int,
    itemHeightPx: Int,
) : RecyclerView.Adapter<RowAdapter.Holder>() {

    var recycler: RecyclerView? = null

    private var count = count
    private var itemHeightPx = itemHeightPx
    var revision = Long.MIN_VALUE
        private set

    var dismissible = false
        private set
    var reorderable = false
        private set
    var sticky = IntArray(0)
        private set
    private var gestures: ItemTouchHelper? = null
    private var headers: StickyHeaders? = null

    /** Says which gestures on a row the application listens for. */
    fun setGestures(dismissible: Boolean, reorderable: Boolean) {
        this.dismissible = dismissible
        this.reorderable = reorderable
        if ((dismissible || reorderable) && gestures == null) {
            gestures = ItemTouchHelper(RowGestures(this)).also { it.attachToRecyclerView(recycler) }
        }
    }

    /** Says which rows stay at the top while their section scrolls. */
    fun setSticky(positions: IntArray) {
        val changed = !positions.contentEquals(sticky)
        sticky = positions
        if (positions.isNotEmpty() && headers == null) {
            headers = StickyHeaders(this).also { recycler?.addItemDecoration(it) }
        }
        if (changed) recycler?.invalidateItemDecorations()
    }

    /** A row was swiped away: tell Rust, then show what the state says. */
    fun dismissed(position: Int) {
        root.onRowDismissed(nodeId, position)
        // The helper keeps a swiped view where the finger took it, because
        // it expects the row to be removed. This holder goes on to show
        // whatever the state has at its position, so the helper is made to
        // let go of it: detaching puts every view it holds back in place.
        gestures?.let {
            it.attachToRecyclerView(null)
            it.attachToRecyclerView(recycler)
        }
        if (position < count) notifyItemChanged(position)
    }

    /** A row was dropped at another place: tell Rust, then show what the state says. */
    fun moved(from: Int, to: Int) {
        root.onRowMoved(nodeId, from, to)
        if (count > 0) notifyItemRangeChanged(0, count)
    }

    /** Binds a row into a holder the recycler does not manage. */
    fun bindDetached(holder: Holder, position: Int, width: Int) {
        val measured = root.bindRow(nodeId, position, holder.id, holder.host, width)
        applyHeight(holder.host, rowHeight(measured))
    }

    class Holder(val host: FrameLayout) : RecyclerView.ViewHolder(host) {
        /** Identifies this holder to Rust, which caches the row it shows. */
        val id: Long = nextId++
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): Holder {
        val host = FrameLayout(context)
        host.layoutParams = RecyclerView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            rowHeight(0),
        )
        return Holder(host)
    }

    override fun onBindViewHolder(holder: Holder, position: Int) {
        // The row must be laid out against the recycler's width, not the root
        // view's (the root includes padding the list does not have).
        val width = recycler?.width?.takeIf { it > 0 }
            ?: holder.host.width.takeIf { it > 0 }
            ?: root.width.takeIf { it > 0 }
            ?: context.resources.displayMetrics.widthPixels
        val measured = root.bindRow(nodeId, position, holder.id, holder.host, width)
        applyHeight(holder.host, rowHeight(measured))
    }

    override fun onViewAttachedToWindow(holder: Holder) {
        super.onViewAttachedToWindow(holder)
        root.restoreRowFocus(nodeId, holder.id)
    }

    override fun onViewRecycled(holder: Holder) {
        super.onViewRecycled(holder)
        root.unbindRow(nodeId, holder.id)
    }

    override fun getItemCount(): Int = count

    /**
     * Applies a new count, height mode and render revision.
     *
     * The revision changes whenever the application description was rebuilt.
     * Rebinding the attached cells is what propagates an app state change into
     * rows the recycler already owns; Rust diffs each row, so unchanged widgets
     * are left alone.
     */
    fun update(count: Int, itemHeightPx: Int, revision: Long) {
        val oldCount = this.count
        val heightChanged = this.itemHeightPx != itemHeightPx
        this.itemHeightPx = itemHeightPx
        this.count = count

        if (count > oldCount) {
            notifyItemRangeInserted(oldCount, count - oldCount)
        } else if (count < oldCount) {
            notifyItemRangeRemoved(count, oldCount - count)
        }

        // Reordering a list can move items to different positions without
        // changing the count; the ranged notification above only covers the
        // appended or removed tail. Rebinding the whole range is cheap because
        // RecyclerView only binds the attached cells.
        if ((heightChanged || revision != this.revision) && count > 0) {
            this.revision = revision
            notifyItemRangeChanged(0, count)
        }
    }

    private fun rowHeight(measured: Int): Int =
        if (itemHeightPx > 0) itemHeightPx else measured.coerceAtLeast(1)

    private fun applyHeight(host: View, height: Int) {
        val params = host.layoutParams as? RecyclerView.LayoutParams
        if (params == null) {
            host.layoutParams = RecyclerView.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                height,
            )
        } else if (params.height != height || params.width != ViewGroup.LayoutParams.MATCH_PARENT) {
            params.width = ViewGroup.LayoutParams.MATCH_PARENT
            params.height = height
            host.layoutParams = params
        }
    }

    private companion object {
        var nextId = 0L
    }
}
