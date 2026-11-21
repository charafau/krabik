package com.example.krabik_ui_dsl

import android.net.Uri
import android.view.View
import android.widget.RatingBar
import android.widget.VideoView
import dev.krabik.ui.KrabikNativeView
import dev.krabik.ui.KrabikNativeViews

/**
 * The views this application makes itself, for the `NativeView` nodes of the
 * showcase: a star rating, which the DSL has no widget for, and a video.
 */
object NativeViews {
    fun register() {
        KrabikNativeViews.register("rating") { context, emit ->
            object : KrabikNativeView {
                private val bar = RatingBar(context).apply {
                    numStars = 5
                    stepSize = 1f
                    setOnRatingBarChangeListener { _, rating, fromUser ->
                        if (fromUser) emit(rating.toInt().toString())
                    }
                }
                override val view: View = bar

                override fun update(props: Map<String, String>) {
                    val stars = props["stars"]?.toFloatOrNull() ?: 0f
                    if (bar.rating != stars) bar.rating = stars
                }
            }
        }
        KrabikNativeViews.register("video") { context, emit ->
            object : KrabikNativeView {
                private val video = VideoView(context).apply {
                    setOnPreparedListener { player ->
                        player.isLooping = true
                        start()
                        emit("playing")
                    }
                    setOnErrorListener { _, _, _ ->
                        emit("failed")
                        true
                    }
                }
                private var shown: String? = null
                override val view: View = video

                override fun update(props: Map<String, String>) {
                    val url = props["url"].orEmpty()
                    if (url != shown) {
                        shown = url
                        video.setVideoURI(Uri.parse(url))
                    }
                }
            }
        }
    }
}
