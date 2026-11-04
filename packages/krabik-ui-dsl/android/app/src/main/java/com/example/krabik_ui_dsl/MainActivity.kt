package com.example.krabik_ui_dsl

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import uniffi.krabik_ui_dsl.getGreeting

class MainActivity : Activity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv = findViewById<TextView>(R.id.sample_text)
        tv.text = getGreeting()
    }
}
