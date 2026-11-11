package com.example.krabik_ui_dsl

import android.content.Context
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
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
        return recycler
    }

    @JvmStatic
    fun update(view: ViewGroup, count: Int, itemHeightPx: Int, revision: Long) {
        val recycler = view as? RecyclerView ?: return
        val adapter = recycler.adapter as? RowAdapter ?: return
        adapter.update(count, itemHeightPx, revision)
        recycler.setHasFixedSize(itemHeightPx > 0)
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
    private var revision = Long.MIN_VALUE

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
