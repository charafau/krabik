package com.example.krabik_ui_dsl

import android.widget.SeekBar
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.GeneralLocation
import androidx.test.espresso.action.GeneralSwipeAction
import androidx.test.espresso.action.Press
import androidx.test.espresso.action.Swipe
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isAssignableFrom
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withClassName
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.hamcrest.Matchers.equalTo
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Exercises the checkbox, radio buttons, slider and progress indicator through
 * their native widgets.
 */
@RunWith(AndroidJUnit4::class)
class ControlsTest {

    @Test
    fun checksAndRadiosDispatchTypedEvents() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Controls")).perform(scrollTo(), click())
            onView(withText("Notifications are off.")).check(matches(isDisplayed()))
            onView(withText("Wi-Fi is on.")).check(matches(isDisplayed()))

            onView(withText("Wi-Fi")).perform(scrollTo(), click())
            onView(withText("Wi-Fi is off.")).check(matches(isDisplayed()))

            onView(withText("Large")).perform(scrollTo(), click())
            onView(withText("Size: Large")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun sliderUpdatesTheValueAndProgressIndicator() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Controls")).perform(scrollTo(), click())
            onView(withText("Volume: 40")).check(matches(withText("Volume: 40")))

            onView(isAssignableFrom(SeekBar::class.java)).perform(
                scrollTo(),
                GeneralSwipeAction(
                    Swipe.FAST,
                    GeneralLocation.CENTER,
                    GeneralLocation.CENTER_RIGHT,
                    Press.FINGER,
                ),
            )

            onView(withText("Volume: 100")).check(matches(withText("Volume: 100")))
            onView(withClassName(equalTo("android.widget.ProgressBar")))
                .perform(scrollTo())
                .check(matches(isDisplayed()))
        }
    }
}
