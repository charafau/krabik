package dev.krabik.ui

import android.content.Context
import android.graphics.BitmapFactory
import android.widget.ImageView
import coil3.load
import coil3.request.crossfade

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

        if (source.isEmpty()) {
            view.setImageDrawable(null)
            return
        }

        val data = if (isAsset) "file:///android_asset/$source" else source
        view.load(data) {
            crossfade(true)
        }
    }

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
}
