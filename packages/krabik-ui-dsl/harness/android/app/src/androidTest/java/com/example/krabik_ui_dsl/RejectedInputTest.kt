package com.example.krabik_ui_dsl

import android.widget.SeekBar
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.closeSoftKeyboard
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.GeneralLocation
import androidx.test.espresso.action.GeneralSwipeAction
import androidx.test.espresso.action.Press
import androidx.test.espresso.action.Swipe
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.swipeUp
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.isNotChecked
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.withContentDescription
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import dev.krabik.ui.KrabikTabBar
import dev.krabik.ui.testing.findFirst
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Test
import org.junit.runner.RunWith

/**
 * A native control shows what the user did to it straight away. When the
 * application does not accept that value, the control has to go back to what
 * the state says.
 */
@RunWith(AndroidJUnit4::class)
class RejectedInputTest {

    @Test
    fun overruledControlsReturnToTheirState() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            // The entry is the last one; `scrollTo` stops short of the
            // system bar, so the page is swiped to its end instead.
            onView(isRoot()).perform(swipeUp())
            onView(isRoot()).perform(swipeUp())
            Thread.sleep(800)
            onView(withText("Rejected input")).perform(click())
            onView(withText("Refused: 0")).check(matches(isDisplayed()))

            // A switch the application keeps off.
            onView(withContentDescription("Locked switch")).perform(click())
            onView(withText("Refused: 1")).check(matches(isDisplayed()))
            onView(withContentDescription("Locked switch")).check(matches(isNotChecked()))

            // A tab bar the application keeps on the first tab.
            onView(withText("Second")).perform(click())
            onView(withText("Refused: 2")).check(matches(isDisplayed()))
            scenario.onActivity { activity ->
                val tabs = activity.window.decorView.findFirst<KrabikTabBar>()
                assertNotNull("tab bar missing", tabs)
                assertEquals(0, tabs!!.selectedIndex)
            }

            // A slider dragged to the end that the application caps at half.
            onView(withContentDescription("Capped")).perform(
                GeneralSwipeAction(
                    Swipe.FAST,
                    GeneralLocation.CENTER,
                    GeneralLocation.CENTER_RIGHT,
                    Press.FINGER,
                ),
            )
            onView(withText("Capped: 50")).check(matches(isDisplayed()))
            scenario.onActivity { activity ->
                val slider = activity.window.decorView.findFirst<SeekBar>()
                assertNotNull("slider missing", slider)
                assertEquals(slider!!.max / 2, slider.progress)
            }

            // A field that only keeps digits.
            onView(withContentDescription("Digits only")).perform(click(), typeText("a1b2"))
            closeSoftKeyboard()
            onView(withText("Digits: 12")).check(matches(isDisplayed()))
            onView(withContentDescription("Digits only")).check(matches(withText("12")))

            // The same rule inside a recycled row.
            onView(withContentDescription("Row switch 2")).perform(click())
            onView(withText("Refused: 3")).check(matches(isDisplayed()))
            onView(withContentDescription("Row switch 2")).check(matches(isNotChecked()))
        }
    }
}
