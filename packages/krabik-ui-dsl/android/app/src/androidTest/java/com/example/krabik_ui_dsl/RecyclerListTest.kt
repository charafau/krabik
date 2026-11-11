package com.example.krabik_ui_dsl

import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Checks that a `recycled` list is backed by a RecyclerView, that holder
 * recycling keeps the attached row count bounded, and that buttons inside
 * bind-time rows still dispatch their actions.
 */
@RunWith(AndroidJUnit4::class)
class RecyclerListTest {

    @Test
    fun recyclerRecyclesRowsAndDispatchesActions() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(withText("List (recycled)")).perform(scrollTo(), click())
            onView(withText("Taps: 0")).check(matches(isDisplayed()))

            var recycler: RecyclerView? = null
            scenario.onActivity { activity ->
                recycler = findRecycler(activity.window.decorView)
                assertNotNull("recycler missing", recycler)
                assertTrue(
                    "too many rows attached: ${recycler!!.childCount}",
                    recycler!!.childCount in 1..30,
                )
                firstTapButton(recycler!!)?.performClick()
            }

            onView(withText("Taps: 1")).check(matches(isDisplayed()))

            scenario.onActivity { recycler!!.scrollToPosition(900) }
            Thread.sleep(1_000)

            scenario.onActivity {
                val list = recycler!!
                assertTrue("recycler grew to ${list.childCount}", list.childCount <= 30)
                val rows = rowNumbers(list)
                assertTrue("no late rows materialized: $rows", (rows.maxOrNull() ?: 0) > 850)
                firstTapButton(list)?.performClick()
            }

            onView(withText("Taps: 2")).check(matches(isDisplayed()))
        }
    }

    private fun findRecycler(view: View): RecyclerView? {
        if (view is RecyclerView) return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                findRecycler(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }

    private fun firstTapButton(view: View): Button? {
        if (view is Button && view.text == "Tap") return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                firstTapButton(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }

    private fun rowNumbers(view: View): List<Int> {
        val numbers = mutableListOf<Int>()
        collectRows(view, numbers)
        return numbers
    }

    private fun collectRows(view: View, numbers: MutableList<Int>) {
        if (view is TextView) {
            val text = view.text?.toString().orEmpty()
            val match = ROW.find(text)
            if (match != null) numbers.add(match.groupValues[1].toInt())
        }
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                collectRows(view.getChildAt(index), numbers)
            }
        }
    }

    companion object {
        private val ROW = Regex("""Row (\d+)""")
    }
}
