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
 * recycler handles scrolling and holder reuse.
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
        val adapter = RowAdapter(context, root, nodeId, count, itemHeightPx)
        recycler.adapter = adapter
        adapter.recycler = recycler
        recycler.setHasFixedSize(true)
        return recycler
    }

    @JvmStatic
    fun update(view: ViewGroup, count: Int, itemHeightPx: Int) {
        val recycler = view as? RecyclerView ?: return
        val adapter = recycler.adapter as? RowAdapter ?: return
        if (adapter.count == count && adapter.itemHeightPx == itemHeightPx) return
        adapter.count = count
        adapter.itemHeightPx = itemHeightPx
        recycler.adapter?.notifyDataSetChanged()
    }
}

private class RowAdapter(
    private val context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
    var count: Int,
    var itemHeightPx: Int,
) : RecyclerView.Adapter<RowAdapter.Holder>() {

    var recycler: RecyclerView? = null

    class Holder(val host: FrameLayout) : RecyclerView.ViewHolder(host)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): Holder {
        val host = FrameLayout(context)
        host.layoutParams = RecyclerView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            itemHeightPx,
        )
        return Holder(host)
    }

    override fun onBindViewHolder(holder: Holder, position: Int) {
        holder.host.layoutParams = RecyclerView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            itemHeightPx,
        )
        // The row must be laid out against the recycler's width, not the root
        // view's (the root includes padding the list does not have).
        val width = recycler?.width?.takeIf { it > 0 }
            ?: holder.host.width.takeIf { it > 0 }
            ?: root.width.takeIf { it > 0 }
            ?: context.resources.displayMetrics.widthPixels
        root.bindRow(nodeId, position, holder.host, width)
    }

    override fun getItemCount(): Int = count
}
