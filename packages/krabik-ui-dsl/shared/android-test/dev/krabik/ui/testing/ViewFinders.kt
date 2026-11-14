package dev.krabik.ui.testing

import android.view.View
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import dev.krabik.ui.RustRootView

/*
 * Helpers shared by the instrumented tests of every app in this package.
 * Each app adds this directory to its `androidTest` source set.
 *
 * Rust builds the views, so they have no resource ids for Espresso to match.
 * Tests that need the native view itself, rather than a text on screen, walk
 * the hierarchy with these.
 */

/** The first view of type [T] under this one that satisfies [predicate], searching depth first. */
inline fun <reified T : View> View.findFirst(noinline predicate: (T) -> Boolean = { true }): T? =
    findFirst(T::class.java, predicate)

fun <T : View> View.findFirst(type: Class<T>, predicate: (T) -> Boolean): T? {
    if (type.isInstance(this)) {
        val candidate = type.cast(this)!!
        if (predicate(candidate)) return candidate
    }
    if (this is ViewGroup) {
        for (index in 0 until childCount) {
            getChildAt(index).findFirst(type, predicate)?.let { return it }
        }
    }
    return null
}

/** The native recycler a `List::recycled()` is rendered with. */
fun findRecycler(view: View): RecyclerView? = view.findFirst<RecyclerView>()

/**
 * The root view that owns the widgets currently on screen.
 *
 * A navigation session also keeps the (hidden) main host alive; only the
 * visible host has registered widgets.
 */
fun findRustRoot(view: View): RustRootView? =
    view.findFirst<RustRootView> { it.isShown && it.registeredViewCount() > 0 }
