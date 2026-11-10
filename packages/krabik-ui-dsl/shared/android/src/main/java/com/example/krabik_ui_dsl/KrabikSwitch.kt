package com.example.krabik_ui_dsl

import android.content.Context
import android.widget.Switch

/**
 * The native widget used for `Switch` nodes.
 *
 * It owns the checked-state listener so user toggles are forwarded to Rust,
 * while state applied from Rust does not echo back as another event.
 */
class KrabikSwitch(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : Switch(context) {

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
