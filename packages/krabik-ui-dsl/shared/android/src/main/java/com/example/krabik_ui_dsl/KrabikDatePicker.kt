package com.example.krabik_ui_dsl

import android.app.AlertDialog
import android.app.DatePickerDialog
import android.app.TimePickerDialog
import android.content.Context
import android.graphics.Typeface
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.DatePicker
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.TimePicker
import java.util.Calendar
import java.util.Locale

/**
 * The native widget used for `DatePicker` nodes.
 *
 * Supports date, time and date-and-time values in compact or inline form.
 * Compact fields open native dialogs; inline pickers stay visible.
 */
class KrabikDatePicker(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : FrameLayout(context) {

    private val field = TextView(context).apply {
        gravity = Gravity.CENTER_VERTICAL
        isClickable = true
        setPadding(dp(12), 0, dp(12), 0)
        setOnClickListener { showCompact() }
    }

    private var kind = KIND_DATE
    private var inline = false
    private var year = 1970
    private var month = 1
    private var day = 1
    private var hour = 0
    private var minute = 0
    private var minDate: IntArray? = null
    private var maxDate: IntArray? = null
    private var minTime: IntArray? = null
    private var maxTime: IntArray? = null
    private var size = 16f
    private var color = 0xFF00_0000.toInt()
    private var bold = false
    private var silent = false

    private var inlineDate: DatePicker? = null
    private var inlineTime: TimePicker? = null

    init {
        addView(
            field,
            LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
        )
    }

    /** Applies the value kind and components from Rust. */
    fun configure(
        kind: Int,
        year: Int,
        month: Int,
        day: Int,
        hour: Int,
        minute: Int,
        inline: Boolean,
    ) {
        val rebuild = this.kind != kind || this.inline != inline
        this.kind = kind
        this.inline = inline
        this.year = year
        this.month = month
        this.day = day
        this.hour = hour
        this.minute = minute
        clampValue()
        if (rebuild) {
            rebuildViews()
        } else {
            syncViews()
        }
    }

    /** Sets the inclusive date range; zeros mean no bound. */
    fun setDateRange(
        minYear: Int,
        minMonth: Int,
        minDay: Int,
        maxYear: Int,
        maxMonth: Int,
        maxDay: Int,
    ) {
        minDate = if (minYear > 0) intArrayOf(minYear, minMonth, minDay) else null
        maxDate = if (maxYear > 0) intArrayOf(maxYear, maxMonth, maxDay) else null
        applyDateRange(inlineDate)
        clampValue()
        syncViews()
    }

    /** Sets the inclusive time range; negative values mean no bound. */
    fun setTimeRange(minHour: Int, minMinute: Int, maxHour: Int, maxMinute: Int) {
        minTime = if (minHour >= 0) intArrayOf(minHour, minMinute) else null
        maxTime = if (maxHour >= 0) intArrayOf(maxHour, maxMinute) else null
        clampValue()
        syncViews()
    }

    /** Applies the compact field's text style. */
    fun setStyle(size: Float, color: Int, bold: Boolean) {
        this.size = size
        this.color = color
        this.bold = bold
        updateField()
    }

    private fun rebuildViews() {
        removeAllViews()
        inlineDate = null
        inlineTime = null
        if (!inline) {
            addView(
                field,
                LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT),
            )
            updateField()
            return
        }

        silent = true
        try {
            when (kind) {
                KIND_TIME -> addInlineTime(this)
                KIND_DATETIME -> {
                    val column = LinearLayout(context).apply {
                        orientation = LinearLayout.VERTICAL
                    }
                    addInlineDate(column, weight = 1f)
                    addInlineTime(column, weight = 1f)
                    addView(
                        column,
                        LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.MATCH_PARENT,
                        ),
                    )
                }
                else -> addInlineDate(this)
            }
        } finally {
            silent = false
        }
    }

    private fun addInlineDate(parent: ViewGroup, weight: Float? = null) {
        val picker = DatePicker(context)
        applyDateRange(picker)
        picker.init(year, month - 1, day, DatePicker.OnDateChangedListener { _, y, m, d ->
            if (!silent) {
                year = y
                month = m + 1
                day = d
                clampValue()
                if (inlineTime != null) {
                    silentUpdateTime()
                }
                notifyChanged()
            }
        })
        inlineDate = picker
        val params = if (weight != null) {
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, weight)
        } else {
            LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        }
        parent.addView(picker, params)
    }

    private fun addInlineTime(parent: ViewGroup, weight: Float? = null) {
        val picker = TimePicker(context)
        picker.setIs24HourView(true)
        picker.hour = hour
        picker.minute = minute
        picker.setOnTimeChangedListener { _, h, m ->
            if (!silent) {
                hour = h
                minute = m
                clampValue()
                notifyChanged()
            }
        }
        inlineTime = picker
        val params = if (weight != null) {
            LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, 0, weight)
        } else {
            LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        }
        parent.addView(picker, params)
    }

    private fun syncViews() {
        silent = true
        try {
            inlineDate?.updateDate(year, month - 1, day)
            inlineTime?.hour = hour
            inlineTime?.minute = minute
            updateField()
        } finally {
            silent = false
        }
    }

    private fun silentUpdateTime() {
        silent = true
        try {
            inlineTime?.hour = hour
            inlineTime?.minute = minute
        } finally {
            silent = false
        }
    }

    private fun updateField() {
        if (inline) {
            return
        }
        field.setTextSize(TypedValue.COMPLEX_UNIT_SP, size)
        field.setTextColor(color)
        field.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        field.text = fieldText()
        field.contentDescription = field.text
    }

    private fun fieldText(): String = when (kind) {
        KIND_TIME -> formatTime()
        KIND_DATETIME -> "${formatDate()} ${formatTime()}"
        else -> formatDate()
    }

    private fun showCompact() {
        when (kind) {
            KIND_TIME -> showTimeDialog()
            KIND_DATETIME -> showDateTimeDialog()
            else -> showDateDialog()
        }
    }

    private fun showDateDialog() {
        val dialog = DatePickerDialog(
            context,
            { _, y, m, d ->
                year = y
                month = m + 1
                day = d
                clampValue()
                updateField()
                notifyChanged()
            },
            year,
            month - 1,
            day,
        )
        applyDateRange(dialog.datePicker)
        dialog.show()
    }

    private fun showTimeDialog() {
        TimePickerDialog(
            context,
            { _, h, m ->
                hour = h
                minute = m
                clampValue()
                updateField()
                notifyChanged()
            },
            hour,
            minute,
            true,
        ).show()
    }

    private fun showDateTimeDialog() {
        val column = LinearLayout(context).apply { orientation = LinearLayout.VERTICAL }
        val datePicker = DatePicker(context)
        applyDateRange(datePicker)
        datePicker.init(year, month - 1, day, null)
        val timePicker = TimePicker(context).apply {
            setIs24HourView(true)
            hour = this@KrabikDatePicker.hour
            minute = this@KrabikDatePicker.minute
        }
        column.addView(datePicker)
        column.addView(timePicker)
        val scroll = ScrollView(context).apply { addView(column) }
        AlertDialog.Builder(context)
            .setTitle("Select date and time")
            .setView(scroll)
            .setPositiveButton("OK") { _, _ ->
                year = datePicker.year
                month = datePicker.month + 1
                day = datePicker.dayOfMonth
                hour = timePicker.hour
                minute = timePicker.minute
                clampValue()
                updateField()
                notifyChanged()
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun clampValue() {
        val date = intArrayOf(year, month, day)
        minDate?.let { if (compareDate(date, it) < 0) setDate(it) }
        maxDate?.let { if (compareDate(date, it) > 0) setDate(it) }
        if (minTime != null || maxTime != null) {
            var total = hour * 60 + minute
            minTime?.let { min ->
                val lower = min[0] * 60 + min[1]
                if (total < lower) total = lower
            }
            maxTime?.let { max ->
                val upper = max[0] * 60 + max[1]
                if (total > upper) total = upper
            }
            hour = total / 60
            minute = total % 60
        }
    }

    private fun setDate(value: IntArray) {
        year = value[0]
        month = value[1]
        day = value[2]
    }

    private fun applyDateRange(picker: DatePicker?) {
        picker ?: return
        minDate?.let { picker.minDate = calendar(it).timeInMillis }
        maxDate?.let { picker.maxDate = calendar(it).timeInMillis }
    }

    private fun calendar(value: IntArray): Calendar = Calendar.getInstance().apply {
        set(value[0], value[1] - 1, value[2], 0, 0, 0)
        set(Calendar.MILLISECOND, 0)
    }

    private fun compareDate(left: IntArray, right: IntArray): Int {
        for (index in 0..2) {
            if (left[index] != right[index]) {
                return left[index].compareTo(right[index])
            }
        }
        return 0
    }

    private fun notifyChanged() {
        root.onPickerChanged(nodeId, kind, year, month, day, hour, minute)
    }

    private fun formatDate(): String = String.format(Locale.US, "%04d-%02d-%02d", year, month, day)

    private fun formatTime(): String = String.format(Locale.US, "%02d:%02d", hour, minute)

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()

    private companion object {
        const val KIND_DATE = 0
        const val KIND_TIME = 1
        const val KIND_DATETIME = 2
    }
}
