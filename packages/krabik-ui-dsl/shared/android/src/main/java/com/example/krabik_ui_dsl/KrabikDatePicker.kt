package com.example.krabik_ui_dsl

import android.app.DatePickerDialog
import android.content.Context
import android.graphics.Typeface
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.DatePicker
import android.widget.FrameLayout
import android.widget.TextView
import java.util.Locale

/**
 * The native widget used for `DatePicker` nodes.
 *
 * Compact mode shows a date field and opens the platform's `DatePickerDialog`;
 * inline mode hosts an always-visible `DatePicker` calendar. Both report the
 * selected calendar date back to Rust.
 */
class KrabikDatePicker(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : FrameLayout(context) {

    private val compact = TextView(context).apply {
        gravity = Gravity.CENTER_VERTICAL
        isClickable = true
        setPadding(dp(12), 0, dp(12), 0)
        setOnClickListener { showDialog() }
    }

    private var inlinePicker: DatePicker? = null
    private var inlineMode = false
    private var year = 1970
    private var month = 1
    private var day = 1
    private var silent = false

    init {
        addView(
            compact,
            LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
        )
    }

    /** Applies the date, presentation mode and compact-field styling from Rust. */
    fun configure(
        year: Int,
        month: Int,
        day: Int,
        inline: Boolean,
        size: Float,
        color: Int,
        bold: Boolean,
    ) {
        this.year = year
        this.month = month
        this.day = day
        if (inline != inlineMode) {
            rebuild(inline)
        }
        silent = true
        try {
            if (inline) {
                inlinePicker?.updateDate(year, month - 1, day)
            } else {
                compact.setTextSize(TypedValue.COMPLEX_UNIT_SP, size)
                compact.setTextColor(color)
                compact.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
                compact.text = format(year, month, day)
                compact.contentDescription = compact.text
            }
        } finally {
            silent = false
        }
    }

    private fun rebuild(inline: Boolean) {
        removeAllViews()
        inlineMode = inline
        if (inline) {
            val picker = DatePicker(context)
            picker.init(year, month - 1, day, DatePicker.OnDateChangedListener { _, y, m, d ->
                if (!silent) {
                    apply(y, m + 1, d, notify = true)
                }
            })
            inlinePicker = picker
            addView(
                picker,
                LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
            )
        } else {
            inlinePicker = null
            addView(
                compact,
                LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
            )
        }
    }

    private fun showDialog() {
        val dialog = DatePickerDialog(
            context,
            { _, y, m, d -> apply(y, m + 1, d, notify = true) },
            year,
            month - 1,
            day,
        )
        dialog.show()
    }

    private fun apply(year: Int, month: Int, day: Int, notify: Boolean) {
        this.year = year
        this.month = month
        this.day = day
        silent = true
        try {
            inlinePicker?.updateDate(year, month - 1, day)
            compact.text = format(year, month, day)
            compact.contentDescription = compact.text
        } finally {
            silent = false
        }
        if (notify) {
            root.onDateChanged(nodeId, year, month, day)
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private fun format(year: Int, month: Int, day: Int): String =
        String.format(Locale.US, "%04d-%02d-%02d", year, month, day)
}
