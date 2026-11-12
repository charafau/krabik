package com.example.krabik_ui_dsl

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.fragment.app.Fragment

/** One native navigation screen hosting a Rust-rendered subtree. */
class KrabikScreenFragment : Fragment() {
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

    override fun onDestroyView() {
        // The session owns the host; a screen can recreate its view after a
        // pop, and a modal host is removed by the Rust presentation sync.
        host = null
        super.onDestroyView()
    }

    companion object {
        private const val SESSION = "session"
        private const val HOST = "host"

        fun newInstance(session: Long, host: Long): KrabikScreenFragment =
            KrabikScreenFragment().apply {
                arguments = Bundle().apply {
                    putLong(SESSION, session)
                    putLong(HOST, host)
                }
            }
    }
}
