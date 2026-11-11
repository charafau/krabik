package com.example.krabik_ui_dsl

import android.view.View
import android.view.ViewGroup
import android.widget.SeekBar
import androidx.recyclerview.widget.RecyclerView
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.closeSoftKeyboard
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.GeneralLocation
import androidx.test.espresso.action.GeneralSwipeAction
import androidx.test.espresso.action.Press
import androidx.test.espresso.action.Swipe
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.assertion.ViewAssertions.doesNotExist
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withHint
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Drives the todo showcase through its native widgets. */
@RunWith(AndroidJUnit4::class)
class TodoTest {

    @Test
    fun addingCompletingFilteringAndClearing() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            onView(withText("Todo")).check(matches(isDisplayed()))
            onView(withText("1 of 4 done")).check(matches(isDisplayed()))

            scenario.onActivity { activity ->
                val recycler = findRecycler(activity.window.decorView)
                assertNotNull("recycler missing", recycler)
                assertTrue("too many rows attached", recycler!!.childCount in 1..20)
            }

            // Completing a row updates the summary and the progress bar. Do it
            // before opening the keyboard so the click target is stable.
            onView(withText("Wire the native renderer")).perform(click())
            onView(withText("2 of 4 done")).check(matches(isDisplayed()))

            // Add a task.
            onView(withHint("What needs doing?")).perform(click(), typeText("Write docs"))
            onView(withText("Add")).perform(click())
            closeSoftKeyboard()
            Thread.sleep(1_000)
            onView(withText("Write docs")).check(matches(isDisplayed()))
            onView(withText("2 of 5 done")).check(matches(isDisplayed()))

            // Filters show only the matching rows.
            onView(withText("Active")).perform(click())
            onView(withText("Write docs")).check(matches(isDisplayed()))
            onView(withText("Review the UI DSL")).check(doesNotExist())

            onView(withText("Done")).perform(click())
            onView(withText("Review the UI DSL")).check(matches(isDisplayed()))
            onView(withText("Write docs")).check(doesNotExist())

            onView(withText("All")).perform(click())

            // The slider snaps the priority label to its range.
            onView(isAssignableFrom(SeekBar::class.java)).perform(
                GeneralSwipeAction(
                    Swipe.FAST,
                    GeneralLocation.CENTER,
                    GeneralLocation.CENTER_RIGHT,
                    Press.FINGER,
                ),
            )
            onView(withText("Priority: 5")).check(matches(isDisplayed()))

            onView(withHint("What needs doing?")).perform(click(), typeText("High priority"))
            onView(withText("Add")).perform(click())
            closeSoftKeyboard()
            Thread.sleep(1_000)
            onView(withText("High priority")).check(matches(isDisplayed()))

            // Clear both completed tasks. (Per-row deletion is covered by the
            // Rust state tests and the checklist example.)
            onView(withText("Clear completed")).perform(click())
            onView(withText("Review the UI DSL")).check(doesNotExist())
            onView(withText("Wire the native renderer")).check(doesNotExist())
            onView(withText("0 of 4 done")).check(matches(isDisplayed()))
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
}
