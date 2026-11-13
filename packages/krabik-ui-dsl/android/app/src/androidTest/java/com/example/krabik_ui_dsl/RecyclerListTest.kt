package com.example.krabik_ui_dsl

import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.action.ViewActions.swipeUp
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.hasFocus
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.withHint
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.hamcrest.Matchers.allOf
import org.hamcrest.Matchers.not
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Checks that a `recycled` list is backed by a RecyclerView, that holder
 * recycling keeps the attached row count bounded, that buttons inside
 * bind-time rows still dispatch their actions, and that rebinds pick up state.
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

    @Test
    fun visibleRowsPickUpStateChangesWithoutScrolling() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(withText("List (recycled)")).perform(scrollTo(), click())
            onView(withText("Row 1 · v0")).check(matches(isDisplayed()))

            onView(withText("Bump rows")).perform(click())

            onView(withText("Row 1 · v1")).check(matches(isDisplayed()))
            onView(withText("Row 2 · v1")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun shrinkingTheListKeepsActionsWorking() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(withText("List (recycled)")).perform(scrollTo(), click())

            scenario.onActivity { it -> findRecycler(it.window.decorView)?.scrollToPosition(900) }
            Thread.sleep(1_000)

            onView(withText("Shrink")).perform(click())

            scenario.onActivity { activity ->
                val list = findRecycler(activity.window.decorView)
                assertNotNull(list)
                assertEquals(200, list!!.adapter?.itemCount)
                list.scrollToPosition(0)
            }
            Thread.sleep(500)

            scenario.onActivity { activity ->
                firstTapButton(findRecycler(activity.window.decorView)!!)?.performClick()
            }
            onView(withText("Taps: 1")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun variableHeightRowsScrollAndDispatchActions() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            // This demo is the last entry in the gallery; a swipe is more
            // reliable than `scrollTo` for a view at the very bottom edge.
            onView(isRoot()).perform(swipeUp())
            Thread.sleep(800)
            onView(withText("List (recycled, variable)")).perform(click())
            onView(withText("Taps: 0")).check(matches(isDisplayed()))

            scenario.onActivity { it -> findRecycler(it.window.decorView)?.scrollToPosition(250) }
            Thread.sleep(1_000)

            scenario.onActivity { activity ->
                val list = findRecycler(activity.window.decorView)
                assertNotNull(list)
                assertTrue("no late rows materialized", (rowNumbers(list!!).maxOrNull() ?: 0) > 200)
                firstTapButton(list)?.performClick()
            }

            onView(withText("Taps: 1")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun aFocusedFieldSurvivesRecycling() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(isRoot()).perform(swipeUp())
            onView(isRoot()).perform(swipeUp())
            Thread.sleep(800)
            onView(withText("List (recycled, form)")).perform(click())

            onView(withHint("Note 1")).perform(click())
            Thread.sleep(500)
            // Focusing the field may scroll the list a little; act on whichever
            // field actually took focus.
            val field = onView(allOf(isAssignableFrom(EditText::class.java), hasFocus()))
            field.perform(typeText("hello"))
            Thread.sleep(500)
            onView(allOf(withText("hello"), hasFocus())).check(matches(isDisplayed()))

            // A revision-only rebind of the visible rows keeps the field.
            onView(withText("Refresh")).perform(click())
            onView(allOf(withText("hello"), hasFocus())).check(matches(isDisplayed()))

            // Scrolling the row out and back rebinds it and restores focus.
            scenario.onActivity { findRecycler(it.window.decorView)?.scrollToPosition(120) }
            Thread.sleep(1_000)
            scenario.onActivity { findRecycler(it.window.decorView)?.scrollToPosition(0) }
            Thread.sleep(1_000)
            onView(allOf(withText("hello"), hasFocus())).check(matches(isDisplayed()))

            // A real blur while the row is mounted is not restored later.
            scenario.onActivity { it.currentFocus?.clearFocus() }
            Thread.sleep(500)
            scenario.onActivity { findRecycler(it.window.decorView)?.scrollToPosition(120) }
            Thread.sleep(800)
            scenario.onActivity { findRecycler(it.window.decorView)?.scrollToPosition(0) }
            Thread.sleep(800)
            onView(withText("hello")).check(matches(not(hasFocus())))
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
