package com.example.krabik_ui_dsl

import android.widget.ProgressBar
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.GeneralLocation
import androidx.test.espresso.action.GeneralSwipeAction
import androidx.test.espresso.action.Press
import androidx.test.espresso.action.Swipe
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.assertion.ViewAssertions.doesNotExist
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withContentDescription
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** Drives one page per widget in the showcase. */
@RunWith(AndroidJUnit4::class)
class ShowcaseTest {

    @Test
    fun catalogsNewControls() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            openPage("Radio group")
            onView(withText("Selected: Medium")).check(matches(isDisplayed()))
            onView(withText("Large")).perform(click())
            onView(withText("Selected: Large")).check(matches(isDisplayed()))
            tapBack()

            openPage("Slider")
            onView(withText("Volume: 40")).check(matches(isDisplayed()))
            onView(withContentDescription("Stepped volume")).perform(
                GeneralSwipeAction(
                    Swipe.FAST,
                    GeneralLocation.CENTER,
                    GeneralLocation.CENTER_RIGHT,
                    Press.FINGER,
                ),
            )
            onView(withText("Stepped: 100")).check(matches(isDisplayed()))
            tapBack()

            openPage("Date picker")
            onView(withText("Compact: 2026-11-12")).check(matches(isDisplayed()))
            onView(withText("2026-11-12")).perform(click())
            onView(withText("OK")).perform(click())
            onView(withText("Compact: 2026-11-12")).check(matches(isDisplayed()))
            onView(withContentDescription("Inline date"))
                .perform(androidx.test.espresso.action.ViewActions.scrollTo())
                .check(matches(isDisplayed()))
            tapBack()

            openPage("Progress")
            onView(withText("Set 100%")).perform(click())
            onView(withText("100%")).check(matches(isDisplayed()))
            tapBack()

            openPage("Spinner")
            onView(withContentDescription("Loading spinner")).check(matches(isDisplayed()))
            scenario.onActivity { activity ->
                val spinner = findProgressBar(activity.window.decorView)
                assertTrue("spinner is not indeterminate", spinner!!.isIndeterminate)
            }
            tapBack()

            openPage("Image")
            onView(withContentDescription("Krabik app icon")).check(matches(isDisplayed()))
            tapBack()

            openPage("Recycled list")
            onView(withText("Row taps: 0")).check(matches(isDisplayed()))
            onView(withContentDescription("Tap Recycled row 1")).perform(click())
            onView(withText("Row taps: 1")).check(matches(isDisplayed()))
            tapBack()
        }
    }

    @Test
    fun nativeScreensAndPresentations() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Text")
            onView(withText("Display 34 / bold")).check(matches(isDisplayed()))
            tapBack()
            onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))

            openPage("Dialog")
            onView(withText("Delete task?")).check(matches(isDisplayed()))
            onView(withText("Cancel")).perform(click())
            onView(withText("Delete task?")).check(doesNotExist())

            openPage("Bottom sheet")
            onView(withText("Filters")).check(matches(isDisplayed()))
            onView(withText("Close")).perform(click())
            onView(withText("Filters")).check(doesNotExist())
        }
    }

    @Test
    fun systemBackReturnsToTheCatalog() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Spinner")
            androidx.test.espresso.Espresso.pressBack()
            onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun windowedListMaterializesRows() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Windowed list")
            onView(withText("Windowed row 1")).check(matches(isDisplayed()))
            // The 200-row list only materializes the visible window, so a row
            // outside it must not exist in the native hierarchy.
            onView(withText("Windowed row 200")).check(doesNotExist())
            tapBack()
        }
    }

    private fun findProgressBar(view: android.view.View): ProgressBar? {
        if (view is ProgressBar) return view
        if (view is android.view.ViewGroup) {
            for (index in 0 until view.childCount) {
                findProgressBar(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }
}
