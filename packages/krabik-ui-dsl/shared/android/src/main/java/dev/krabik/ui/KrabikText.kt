package dev.krabik.ui

import android.graphics.Paint
import android.graphics.Typeface
import android.os.Build
import android.text.SpannableString
import android.text.Spanned
import android.text.style.AbsoluteSizeSpan
import android.text.style.ForegroundColorSpan
import android.text.style.LineHeightSpan
import android.text.style.StrikethroughSpan
import android.text.style.StyleSpan
import android.text.style.TypefaceSpan
import android.text.style.UnderlineSpan
import android.util.Log
import android.util.TypedValue
import android.widget.TextView
import kotlin.math.roundToInt

/**
 * How a string is drawn: its font, its decorations and its spacing.
 *
 * Rust calls [apply] for a text it renders and for the off-screen view it
 * measures with, so a frame is always measured for what is drawn in it.
 */
object KrabikText {
    // The bits of `flags`; repeated in `rust/src/android/widgets/text.rs`.
    private const val ITALIC = 1
    private const val UNDERLINE = 1 shl 1
    private const val STRIKETHROUGH = 1 shl 2

    /** How many numbers describe one run; see `pack_runs` in Rust. */
    private const val RUN_FIELDS = 7

    /** Font files that were loaded, by asset path; null when loading failed. */
    private val fontFiles = HashMap<String, Typeface?>()

    private const val DECORATIONS = Paint.UNDERLINE_TEXT_FLAG or Paint.STRIKE_THRU_TEXT_FLAG

    /**
     * Sets the string of [view] and how it is drawn.
     *
     * [weight] is 100 to 900. [letterSpacing] and [lineHeight] are in the unit
     * of the text size (sp), and a [lineHeight] of zero means the font's own.
     * The text size of the view has to be set before.
     *
     * [runs] is null for a text in one style. For a rich text it holds, for
     * each of its parts, the start, the end, the flags, the weight, the colour,
     * the bits of the size and the font; the parts then bring their own
     * decorations.
     *
     * [fonts] are the asset paths of the font files in use. [font] and the
     * font of a run are indexes into it, or -1 for the system font.
     */
    @JvmStatic
    fun apply(
        view: TextView,
        text: String,
        weight: Int,
        flags: Int,
        letterSpacing: Float,
        lineHeight: Float,
        runs: IntArray?,
        fonts: Array<String>?,
        font: Int,
    ) {
        view.typeface = typeface(view, fonts, font, weight, (flags and ITALIC) != 0)

        var paintFlags = view.paintFlags and DECORATIONS.inv()
        if (runs == null) {
            if ((flags and UNDERLINE) != 0) paintFlags = paintFlags or Paint.UNDERLINE_TEXT_FLAG
            if ((flags and STRIKETHROUGH) != 0) {
                paintFlags = paintFlags or Paint.STRIKE_THRU_TEXT_FLAG
            }
        }
        if (view.paintFlags != paintFlags) view.paintFlags = paintFlags

        // Android counts letter spacing in ems.
        val textSize = view.textSize
        view.letterSpacing = if (textSize > 0f) scaled(view, letterSpacing) / textSize else 0f

        if (runs == null && lineHeight <= 0f) {
            view.text = text
            return
        }

        val styled = SpannableString(text)
        if (runs != null) {
            for (index in 0 until runs.size / RUN_FIELDS) {
                val at = index * RUN_FIELDS
                val start = runs[at].coerceIn(0, styled.length)
                val end = runs[at + 1].coerceIn(start, styled.length)
                if (start == end) continue
                fun span(what: Any) =
                    styled.setSpan(what, start, end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)

                val runFlags = runs[at + 2]
                val italic = (runFlags and ITALIC) != 0
                val runWeight = runs[at + 3]
                val runFont = runs[at + 6]
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                    span(TypefaceSpan(typeface(view, fonts, runFont, runWeight, italic)))
                } else {
                    // Before Android 9 a part of a text cannot have a font
                    // file of its own, only a style.
                    span(StyleSpan(style(runWeight, italic)))
                }
                span(ForegroundColorSpan(runs[at + 4]))
                val size = Float.fromBits(runs[at + 5])
                span(AbsoluteSizeSpan(scaled(view, size).roundToInt()))
                if ((runFlags and UNDERLINE) != 0) span(UnderlineSpan())
                if ((runFlags and STRIKETHROUGH) != 0) span(StrikethroughSpan())
            }
        }
        if (lineHeight > 0f) {
            // A span gives every line the height, the first and the last one
            // too; `TextView.setLineHeight` only adds space between lines.
            styled.setSpan(
                LineHeight(scaled(view, lineHeight).roundToInt()),
                0,
                styled.length,
                Spanned.SPAN_INCLUSIVE_INCLUSIVE,
            )
        }
        view.setText(styled, TextView.BufferType.SPANNABLE)
    }

    /** Sets the font of [view]: the system font in the given weight. */
    @JvmStatic
    fun applyTypeface(view: TextView, weight: Int, italic: Boolean) {
        view.typeface = typeface(weight, italic)
    }

    /**
     * The typeface of a text or of one of its parts: the font file at
     * [index] of [fonts], as it is, or the system font in [weight].
     */
    private fun typeface(
        view: TextView,
        fonts: Array<String>?,
        index: Int,
        weight: Int,
        italic: Boolean,
    ): Typeface {
        val path = fonts?.getOrNull(index) ?: return typeface(weight, italic)
        // A file that failed stays in the map as null, so it is tried and
        // reported once.
        if (!fontFiles.containsKey(path)) {
            fontFiles[path] = try {
                Typeface.createFromAsset(view.context.assets, path)
            } catch (error: RuntimeException) {
                Log.e("krabik", "font $path could not be loaded: ${error.message}")
                null
            }
        }
        return fontFiles[path] ?: typeface(weight, italic)
    }

    private fun typeface(weight: Int, italic: Boolean): Typeface = when {
        weight == 400 && !italic -> Typeface.DEFAULT
        weight == 700 && !italic -> Typeface.DEFAULT_BOLD
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.P ->
            Typeface.create(Typeface.DEFAULT, weight, italic)
        // Before Android 9 a typeface is regular or bold.
        else -> Typeface.create(Typeface.DEFAULT, style(weight, italic))
    }

    /** The nearest of the four styles every Android version has. */
    private fun style(weight: Int, italic: Boolean): Int {
        val bold = weight >= 600
        return when {
            bold && italic -> Typeface.BOLD_ITALIC
            bold -> Typeface.BOLD
            italic -> Typeface.ITALIC
            else -> Typeface.NORMAL
        }
    }

    /** A length in the unit of the text size, in pixels. */
    private fun scaled(view: TextView, value: Float): Float =
        TypedValue.applyDimension(
            TypedValue.COMPLEX_UNIT_SP,
            value,
            view.resources.displayMetrics,
        )

    /** Makes every line [height] pixels tall, with the text centred in it. */
    private class LineHeight(private val height: Int) : LineHeightSpan {
        override fun chooseHeight(
            text: CharSequence,
            start: Int,
            end: Int,
            spanstartv: Int,
            lineHeight: Int,
            metrics: Paint.FontMetricsInt,
        ) {
            val natural = metrics.descent - metrics.ascent
            if (natural <= 0) return
            val extra = height - natural
            // Half of the difference goes below the text, the rest above it.
            metrics.descent += extra / 2
            metrics.ascent = metrics.descent - height
            metrics.top = metrics.ascent
            metrics.bottom = metrics.descent
        }
    }
}
