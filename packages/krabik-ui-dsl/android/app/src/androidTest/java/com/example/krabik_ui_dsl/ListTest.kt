package com.example.krabik_ui_dsl

import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.action.ViewActions.swipeUp
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Verifies that the virtualized list only materializes the rows around the
 * viewport, and that scrolling moves the window.
 */
@RunWith(AndroidJUnit4::class)
class ListTest {

    @Test
    fun onlyTheVisibleWindowExistsAsNativeViews() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(withText("List")).perform(scrollTo(), click())

            scenario.onActivity { activity ->
                val rows = rowTexts(activity.window.decorView)
                assertTrue("expected a bounded window, found ${rows.size}", rows.size in 1..40)
                assertTrue("first row missing", 1 in rows)
                assertTrue("window should not reach the end", rows.max() < 999)
            }

            repeat(6) { onView(isRoot()).perform(swipeUp()) }

            scenario.onActivity { activity ->
                val root = activity.window.decorView
                val rows = rowTexts(root)
                assertTrue("window grew to ${rows.size}", rows.size in 1..40)
                assertFalse("row 1 should have left the window", 1 in rows)
                assertTrue("scrolling did not advance the window", rows.max() > 10)

                val rustRoot = findRustRoot(root)
                assertNotNull("root view missing", rustRoot)
                assertTrue(
                    "detached views stayed registered: ${rustRoot!!.registeredViewCount()}",
                    rustRoot.registeredViewCount() < 80,
                )
            }
        }
    }

    private fun rowTexts(root: View): List<Int> {
        val rows = mutableListOf<Int>()
        collectRows(root, rows)
        return rows
    }

    private fun collectRows(view: View, rows: MutableList<Int>) {
        if (view is TextView) {
            val match = ROW.matchEntire(view.text?.toString().orEmpty())
            if (match != null) {
                rows.add(match.groupValues[1].toInt())
            }
        }
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                collectRows(view.getChildAt(index), rows)
            }
        }
    }

    private fun findRustRoot(view: View): RustRootView? {
        if (view is RustRootView) return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                findRustRoot(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }

    companion object {
        private val ROW = Regex("""Row (\d+)""")
    }
}
