package com.example.krabik_ui_dsl

import android.content.DialogInterface
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import com.google.android.material.bottomsheet.BottomSheetDialogFragment

/**
 * A native Material bottom sheet around Rust-rendered content.
 *
 * Material supplies the surface, scrim and drag behaviour; the sheet is as
 * tall as its content. Swiping it away, tapping the scrim and the back button
 * all arrive in [onCancel].
 */
class KrabikSheetFragment : BottomSheetDialogFragment() {
    private var host: RustRootView? = null

    override fun onCreateView(
        inflater: LayoutInflater,
        container: ViewGroup?,
        savedInstanceState: Bundle?,
    ): View {
        val view = RustRootView(
            requireContext(),
            requireArguments().getLong(SESSION),
            requireArguments().getLong(HOST),
        )
        host = view
        return view
    }

    override fun onCancel(dialog: DialogInterface) {
        host?.onModalDismiss()
        super.onCancel(dialog)
    }

    override fun onDestroyView() {
        // The session owns the host; a screen can recreate its view after a
        // pop, and a modal host is removed by the Rust presentation sync.
        host = null
        super.onDestroyView()
    }

    companion object {
        private const val SESSION = "session"
        private const val HOST = "host"

        fun newInstance(session: Long, host: Long): KrabikSheetFragment =
            KrabikSheetFragment().apply {
                arguments = Bundle().apply {
                    putLong(SESSION, session)
                    putLong(HOST, host)
                }
            }
    }
}
