package com.example.krabik_ui_dsl

import android.content.DialogInterface
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.fragment.app.DialogFragment

/** A native dialog hosting a Rust-rendered presentation. */
class KrabikDialogFragment : DialogFragment() {
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

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        dialog?.window?.apply {
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            val width = (resources.displayMetrics.widthPixels * 0.9).toInt()
            setLayout(width, ViewGroup.LayoutParams.WRAP_CONTENT)
        }
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

        fun newInstance(session: Long, host: Long): KrabikDialogFragment =
            KrabikDialogFragment().apply {
                arguments = Bundle().apply {
                    putLong(SESSION, session)
                    putLong(HOST, host)
                }
            }
    }
}
