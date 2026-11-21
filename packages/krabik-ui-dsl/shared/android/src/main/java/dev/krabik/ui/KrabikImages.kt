package dev.krabik.ui

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.BitmapFactory
import android.util.Log
import android.widget.ImageView
import coil3.dispose
import coil3.load
import coil3.request.crossfade
import java.util.WeakHashMap
import java.util.concurrent.Executors

/**
 * Loads image sources into native `ImageView`s.
 *
 * Coil handles threading, request cancellation when the source changes, and
 * both local assets and remote URLs.
 */
object KrabikImages {
    @JvmStatic
    fun load(
        view: ImageView,
        source: String,
        isAsset: Boolean,
        fill: Boolean,
    ) {
        view.scaleType = if (fill) {
            ImageView.ScaleType.CENTER_CROP
        } else {
            ImageView.ScaleType.FIT_CENTER
        }

        view.imageTintList = null
        if (source.isEmpty()) {
            svgs.remove(view)
            view.setImageDrawable(null)
            return
        }
        if (isAsset && isSvg(source)) {
            loadSvg(view, source, fill)
            return
        }
        svgs.remove(view)

        val data = if (isAsset) "file:///android_asset/$source" else source
        view.load(data) {
            crossfade(true)
        }
    }

    /**
     * Shows a drawable resource of the application by name, in one color
     * when `tinted`. A name the application does not have shows nothing and
     * is logged once.
     */
    @JvmStatic
    fun loadIcon(view: ImageView, name: String, tint: Int, tinted: Boolean) {
        svgs.remove(view)
        view.dispose()
        view.scaleType = ImageView.ScaleType.FIT_CENTER
        view.imageTintList = if (tinted) ColorStateList.valueOf(tint) else null
        val context = view.context
        val id = if (name.isEmpty()) {
            0
        } else {
            context.resources.getIdentifier(name, "drawable", context.packageName)
        }
        if (id == 0) {
            if (name.isNotEmpty() && missingIcons.add(name)) {
                Log.w("krabik", "no drawable named \"$name\" for an icon")
            }
            view.setImageDrawable(null)
        } else {
            view.setImageResource(id)
        }
    }

    private val missingIcons = HashSet<String>()

    /**
     * Returns the pixel dimensions of a bundled asset, packed as
     * `(width shl 32) or height`, or zero when the source is remote or cannot
     * be decoded.
     *
     * Only the header is read, so layout never decodes the full bitmap.
     */
    @JvmStatic
    fun size(context: Context, source: String): Long {
        if (source.isEmpty() || source.startsWith("http://") || source.startsWith("https://")) {
            return 0L
        }
        if (isSvg(source)) {
            return svgSize(context, source)
        }
        return try {
            val options = BitmapFactory.Options().apply { inJustDecodeBounds = true }
            context.assets.open(source).use { stream ->
                BitmapFactory.decodeStream(stream, null, options)
            }
            if (options.outWidth <= 0 || options.outHeight <= 0) {
                0L
            } else {
                (options.outWidth.toLong() shl 32) or (options.outHeight.toLong() and 0xffffffffL)
            }
        } catch (_: Exception) {
            0L
        }
    }

    // region SVG

    /** What an image view shows when its source is an SVG asset. */
    private class Svg(val source: String, val fill: Boolean, val bytes: ByteArray) {
        /** The size, in pixels, of the rendering shown or on its way. */
        var width = 0
        var height = 0
    }

    private val svgs = WeakHashMap<ImageView, Svg>()
    private val renderer = Executors.newSingleThreadExecutor()
    private var missingRenderer = false

    private fun isSvg(source: String): Boolean = source.endsWith(".svg", ignoreCase = true)

    private fun assetBytes(context: Context, source: String): ByteArray? = try {
        context.assets.open(source).use { it.readBytes() }
    } catch (_: Exception) {
        null
    }

    /**
     * An SVG is drawn by Rust at the size of the view, so it stays sharp at
     * any size, and again whenever that size changes.
     */
    private fun loadSvg(view: ImageView, source: String, fill: Boolean) {
        val current = svgs[view]
        if (current != null && current.source == source && current.fill == fill) {
            return
        }
        val bytes = assetBytes(view.context, source)
        if (bytes == null) {
            svgs.remove(view)
            view.setImageDrawable(null)
            return
        }
        if (current == null) {
            view.addOnLayoutChangeListener { changed, _, _, _, _, _, _, _, _ ->
                renderSvg(changed as ImageView)
            }
        }
        svgs[view] = Svg(source, fill, bytes)
        view.setImageDrawable(null)
        renderSvg(view)
    }

    private fun renderSvg(view: ImageView) {
        val svg = svgs[view] ?: return
        val width = view.width
        val height = view.height
        if (width <= 0 || height <= 0 || (width == svg.width && height == svg.height)) {
            return
        }
        svg.width = width
        svg.height = height
        renderer.execute {
            val png = try {
                nativeRenderSvg(svg.bytes, width, height, svg.fill)
            } catch (_: UnsatisfiedLinkError) {
                reportMissingRenderer()
                null
            }
            val bitmap = png?.let { BitmapFactory.decodeByteArray(it, 0, it.size) }
            view.post {
                // The view may show another image or have another size by now.
                if (svgs[view] === svg && svg.width == width && svg.height == height) {
                    view.setImageBitmap(bitmap)
                }
            }
        }
    }

    /** The natural size of an SVG asset, in pixels, packed like [size]. */
    private fun svgSize(context: Context, source: String): Long {
        val bytes = assetBytes(context, source) ?: return 0L
        val points = try {
            nativeSvgSize(bytes)
        } catch (_: UnsatisfiedLinkError) {
            reportMissingRenderer()
            0L
        }
        if (points == 0L) return 0L
        val density = context.resources.displayMetrics.density
        val width = ((points ushr 32).toInt() * density).toLong()
        val height = ((points and 0xffffffffL).toInt() * density).toLong()
        return (width shl 32) or (height and 0xffffffffL)
    }

    private fun reportMissingRenderer() {
        if (!missingRenderer) {
            missingRenderer = true
            Log.e("krabik", "an .svg image needs the `svg` feature of krabik-ui-dsl in the app's Cargo.toml")
        }
    }

    @JvmStatic
    private external fun nativeSvgSize(bytes: ByteArray): Long

    @JvmStatic
    private external fun nativeRenderSvg(
        bytes: ByteArray,
        width: Int,
        height: Int,
        fill: Boolean,
    ): ByteArray?

    // endregion
}
