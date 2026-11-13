package dev.krabik.ui

import android.content.Context
import android.widget.CheckBox
import android.widget.RadioButton

/**
 * The native widget used for `Checkbox` nodes.
 *
 * It owns the checked-state listener so user toggles are forwarded to Rust,
 * while state applied from Rust does not echo back as another event.
 */
class KrabikCheckBox(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : CheckBox(context) {

    private var silent = false

    init {
        setOnCheckedChangeListener { _, isChecked ->
            if (!silent) {
                root.onCheckedChanged(nodeId, isChecked)
            }
        }
    }

    /** Applies the checked state from Rust without reporting it back. */
    fun setCheckedSilently(checked: Boolean) {
        if (isChecked == checked) {
            return
        }

        silent = true
        try {
            isChecked = checked
        } finally {
            silent = false
        }
    }
}

/**
 * The native widget used for `RadioButton` nodes.
 *
 * A radio button is an independent control; the application enforces mutual
 * exclusion by rendering one selected item and handling the action.
 */
class KrabikRadioButton(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : RadioButton(context) {

    private var silent = false

    init {
        setOnCheckedChangeListener { _, isChecked ->
            if (!silent) {
                root.onCheckedChanged(nodeId, isChecked)
            }
        }
    }

    /** Applies the checked state from Rust without reporting it back. */
    fun setCheckedSilently(checked: Boolean) {
        if (isChecked == checked) {
            return
        }

        silent = true
        try {
            isChecked = checked
        } finally {
            silent = false
        }
    }
}
