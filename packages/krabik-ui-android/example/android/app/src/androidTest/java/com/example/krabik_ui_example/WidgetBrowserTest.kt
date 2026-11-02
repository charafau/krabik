package com.example.krabik_ui_example

import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.Espresso.pressBack
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Test
import org.junit.runner.RunWith

/**
 * End-to-end tests for the Rust-built widget browser.
 *
 * Every view in these tests is created by Rust through JNI. The tests only drive the real Android
 * view hierarchy, so they exercise the bridge, the renderer, and the retained Rust state together.
 */
@RunWith(AndroidJUnit4::class)
class WidgetBrowserTest {

    @Test
    fun mainListRendersWidgetEntries() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Android widgets")).check(matches(isDisplayed()))
            onView(withText("Choose a widget")).check(matches(isDisplayed()))
            onView(withText("TextView")).check(matches(isDisplayed()))
            onView(withText("Button")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun backButtonReturnsToTheMainList() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("TextView")).perform(click())
            onView(withText("Back")).check(matches(isDisplayed()))
            onView(withText("Read-only text with configurable size, colour, gravity, padding, and background."))
                .check(matches(isDisplayed()))

            onView(withText("Back")).perform(click())
            onView(withText("Android widgets")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun systemBackReturnsToTheMainList() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Button")).perform(click())
            onView(withText("Back")).check(matches(isDisplayed()))

            pressBack()
            onView(withText("Android widgets")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun rustOwnedTapCounterUpdatesTheNativeTextView() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Button")).perform(click())
            onView(withText("Tapped 0 times")).check(matches(isDisplayed()))

            onView(withText("Tap me")).perform(click())
            onView(withText("Tapped 1 times")).check(matches(isDisplayed()))

            onView(withText("Tap me")).perform(click())
            onView(withText("Tapped 2 times")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun materialComponentsRenderFromJni() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Material Button")).perform(scrollTo(), click())
            onView(withText("Filled button")).check(matches(isDisplayed()))

            onView(withText("Back")).perform(click())
            onView(withText("Material Card")).perform(scrollTo(), click())
            onView(withText("Card title")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun materialAlertDialogShows() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Alert Dialog")).perform(scrollTo(), click())
            onView(withText("Show dialog")).perform(click())
            onView(withText("Delete item?")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun mainListKeepsItsScrollPosition() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Material Carousel")).perform(scrollTo(), click())
            onView(withText("Back")).perform(click())
            onView(withText("Material Carousel")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun materialSnackbarAndPickersOpen() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("Snackbar")).perform(scrollTo(), click())
            onView(withText("Show snackbar")).perform(click())
            onView(withText("Saved to your library")).check(matches(isDisplayed()))

            onView(withText("Back")).perform(click())
            onView(withText("Material Date Picker")).perform(scrollTo(), click())
            onView(withText("Pick a date")).perform(click())
            onView(withText("OK")).check(matches(isDisplayed()))
            pressBack()

            onView(withText("Back")).perform(click())
            onView(withText("Material Time Picker")).perform(scrollTo(), click())
            onView(withText("Pick a time")).perform(click())
            onView(withText("OK")).check(matches(isDisplayed()))
            pressBack()
        }
    }

    @Test
    fun everyMaterialDemoRenders() {
        val demos = listOf(
            "Material Button",
            "Material Toggle Group",
            "Material CheckBox",
            "Material Radio Group",
            "Material Switch",
            "Material TextView",
            "Material Text Field",
            "Material Dropdown",
            "Material Search Bar",
            "Material Search View",
            "Material Chips",
            "Material Chip Group",
            "Material Card",
            "Material Toolbar",
            "Material App Bar",
            "Collapsing Toolbar",
            "Bottom App Bar",
            "Floating Toolbar",
            "Docked Toolbar",
            "Material FAB",
            "Extended FAB",
            "Material Progress",
            "Loading Indicator",
            "Material Slider",
            "Material Divider",
            "Shapeable Image",
            "Material Tabs",
            "Bottom Navigation",
            "Navigation Rail",
            "Navigation Drawer",
            "Material Carousel",
            "Snackbar",
            "Alert Dialog",
            "Material Date Picker",
            "Material Time Picker",
        )

        ActivityScenario.launch(MainActivity::class.java).use {
            for (title in demos) {
                onView(withText(title)).perform(scrollTo(), click())
                onView(withText("Back")).check(matches(isDisplayed()))
                onView(withText("Back")).perform(click())
                // Returning from a demo keeps the list exactly where it was, so the row we
                // just opened should still be on screen.
                onView(withText(title)).check(matches(isDisplayed()))
            }
        }
    }
}
