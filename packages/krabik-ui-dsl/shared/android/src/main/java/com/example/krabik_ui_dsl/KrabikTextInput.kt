package com.example.krabik_ui_dsl

import android.content.Context
import android.graphics.Rect
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.Gravity
import android.view.inputmethod.EditorInfo
import android.widget.EditText

/**
 * The native input used for `TextField` nodes.
 *
 * It owns the text watcher so it can forward user edits to Rust and update the
 * text programmatically without echoing the change back. It also owns the
 * focus and submit listeners, because both are native input concerns.
 */
class KrabikTextInput(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : EditText(context) {

    private var secure = false
    private var multiline = false
    private var keyboard = 0
    private var imeAction = 0
    private var configured = false

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
        setBackground(null)
        setIncludeFontPadding(false)
        setPadding(0, 0, 0, 0)
        addTextChangedListener(watcher)

        setOnEditorActionListener { _, actionId, event ->
            if (event == null || actionId != EditorInfo.IME_ACTION_UNSPECIFIED) {
                root.onSubmitted(nodeId)
            }
            // Let the platform handle the default action, such as hiding the
            // keyboard after "done".
            false
        }

        setOnFocusChangeListener { _, hasFocus ->
            // RecyclerView restores focus during a layout pass. Rendering from
            // here would notify its adapter mid-layout, so both directions are
            // reported on the next main-loop turn; Rust also sees whether the
            // field is still attached, which distinguishes recycling from a
            // real user blur.
            post {
                root.onFocusChanged(nodeId, hasFocus, isAttachedToWindow)
                if (hasFocus) {
                    // Ask the surrounding scroll view to reveal the field once
                    // the keyboard has changed the viewport.
                    requestRectangleOnScreen(Rect(0, 0, width, height), true)
                }
            }
        }
    }

    /** Applies the keyboard, line, and obscuring configuration of the field. */
    fun configure(
        secure: Boolean,
        multiline: Boolean,
        keyboard: Int,
        imeAction: Int,
    ) {
        if (configured &&
            this.secure == secure &&
            this.multiline == multiline &&
            this.keyboard == keyboard &&
            this.imeAction == imeAction
        ) {
            return
        }
        configured = true
        this.secure = secure
        this.multiline = multiline
        this.keyboard = keyboard
        this.imeAction = imeAction

        // Changing the input type or line mode can move the cursor, so the
        // current selection is restored afterwards.
        val start = selectionStart.coerceAtLeast(0)
        val end = selectionEnd.coerceAtLeast(0)

        setSingleLine(!multiline)
        setGravity(if (multiline) Gravity.START or Gravity.TOP else Gravity.START or Gravity.CENTER_VERTICAL)

        var inputType = when (keyboard) {
            1 -> InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
            2 -> InputType.TYPE_CLASS_NUMBER
            3 -> InputType.TYPE_CLASS_PHONE
            else -> InputType.TYPE_CLASS_TEXT
        }
        if (secure) {
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        } else if (multiline) {
            inputType = inputType or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        }
        this.inputType = inputType
        setMaxLines(if (multiline) Int.MAX_VALUE else 1)

        imeOptions = when (imeAction) {
            1 -> EditorInfo.IME_ACTION_DONE
            2 -> EditorInfo.IME_ACTION_NEXT
            3 -> EditorInfo.IME_ACTION_SEARCH
            4 -> EditorInfo.IME_ACTION_SEND
            else -> EditorInfo.IME_ACTION_UNSPECIFIED
        }

        val length = text?.length ?: 0
        setSelection(start.coerceAtMost(length), end.coerceAtMost(length))
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
