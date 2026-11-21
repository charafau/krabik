package dev.krabik.ui

import android.annotation.SuppressLint
import android.content.Context
import android.util.Log
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout

/**
 * A view of the application's own inside a Rust-described screen.
 *
 * Rust lays [view] out and calls [update] with the properties the
 * description gives, at first and whenever they change.
 */
interface KrabikNativeView {
    val view: View

    fun update(props: Map<String, String>) {}
}

/**
 * The views an application supplies for `NativeView` nodes, by name.
 *
 * Register a factory before the first screen is shown, for example in
 * `Application.onCreate` or at the top of the activity's `onCreate`:
 *
 * ```kotlin
 * KrabikNativeViews.register("rating") { context, emit ->
 *     object : KrabikNativeView {
 *         override val view = RatingBar(context).apply {
 *             setOnRatingBarChangeListener { _, rating, fromUser ->
 *                 if (fromUser) emit(rating.toString())
 *             }
 *         }
 *         override fun update(props: Map<String, String>) {
 *             view.rating = props["value"]?.toFloatOrNull() ?: 0f
 *         }
 *     }
 * }
 * ```
 *
 * `emit` sends a string to the application's `on_event`.
 */
object KrabikNativeViews {
    fun interface Factory {
        fun create(context: Context, emit: (String) -> Unit): KrabikNativeView
    }

    /** The name the framework's own web view is registered under. */
    const val WEB_VIEW = "krabik.webview"

    private val factories = HashMap<String, Factory>()
    private val missing = HashSet<String>()

    init {
        register(WEB_VIEW) { context, emit -> KrabikWebView(context, emit) }
    }

    /** Makes `factory` the source of the views named `name`. */
    @JvmStatic
    fun register(name: String, factory: Factory) {
        factories[name] = factory
    }

    /** Creates the host view of one `NativeView` node. Called by Rust. */
    @JvmStatic
    fun create(context: Context, root: RustRootView, nodeId: Long): View =
        Host(context, root, nodeId)

    /** Gives a host its name and properties. Called by Rust. */
    @JvmStatic
    fun update(host: View, name: String, keys: Array<String>, values: Array<String>) {
        (host as? Host)?.update(name, keys.zip(values).toMap())
    }

    /** The frame Rust positions; the application's view fills it. */
    private class Host(
        context: Context,
        private val root: RustRootView,
        private val nodeId: Long,
    ) : FrameLayout(context) {
        private var name: String? = null
        private var content: KrabikNativeView? = null

        fun update(name: String, props: Map<String, String>) {
            if (name != this.name) {
                this.name = name
                removeAllViews()
                val factory = factories[name]
                if (factory == null && missing.add(name)) {
                    Log.w("krabik", "no native view is registered as \"$name\"")
                }
                content = factory?.create(context) { event -> root.onNativeEvent(nodeId, event) }
                content?.let {
                    addView(
                        it.view,
                        LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
                    )
                }
            }
            content?.update(props)
        }
    }
}

/** The framework's own native view: the platform's web view. */
private class KrabikWebView(context: Context, emit: (String) -> Unit) : KrabikNativeView {
    private var shown: String? = null

    override val view = WebView(context).apply {
        webViewClient = object : WebViewClient() {
            override fun onPageFinished(view: WebView, url: String) {
                emit(url)
            }
        }
    }

    @SuppressLint("SetJavaScriptEnabled")
    override fun update(props: Map<String, String>) {
        view.settings.javaScriptEnabled = props["javascript"] != "false"
        val url = props["url"].orEmpty()
        // Only an address the application newly asks for is loaded, so the
        // pages the user reached by following links are left alone.
        if (url != shown) {
            shown = url
            if (url.isEmpty()) view.loadUrl("about:blank") else view.loadUrl(url)
        }
    }
}
