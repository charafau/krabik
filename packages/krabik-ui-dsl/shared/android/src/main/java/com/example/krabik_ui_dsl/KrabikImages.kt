package com.example.krabik_ui_dsl

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
}
