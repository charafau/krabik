package com.example.krabik_ui_dsl

import android.content.Context
import android.text.Editable
import android.text.TextWatcher
import android.view.Gravity
import android.widget.EditText

/**
 * The native input used for `TextField` nodes.
 *
 * It owns the text watcher so it can forward user edits to Rust and update the
 * text programmatically without echoing the change back.
 */
class KrabikTextInput(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : EditText(context) {

    private val watcher = object : TextWatcher {
        override fun beforeTextChanged(
            text: CharSequence?,
            start: Int,
            count: Int,
            after: Int,
        ) = Unit

        override fun onTextChanged(
            text: CharSequence?,
            start: Int,
            before: Int,
            count: Int,
        ) = Unit

        override fun afterTextChanged(text: Editable?) {
            root.onTextChanged(nodeId, text?.toString().orEmpty())
        }
    }

    init {
        setSingleLine(true)
        setBackground(null)
        setIncludeFontPadding(false)
        setPadding(0, 0, 0, 0)
        setGravity(Gravity.START or Gravity.CENTER_VERTICAL)
        addTextChangedListener(watcher)
    }

    /**
     * Sets the text from Rust without reporting the change back.
     *
     * The current selection is preserved so re-renders do not move the cursor
     * while the user is typing.
     */
    fun setTextSilently(value: String) {
        if (text.toString() == value) {
            return
        }

        val start = selectionStart.coerceAtLeast(0)
        val end = selectionEnd.coerceAtLeast(0)

        removeTextChangedListener(watcher)
        setText(value)
        val length = value.length
        setSelection(start.coerceAtMost(length), end.coerceAtMost(length))
        addTextChangedListener(watcher)
    }
}
