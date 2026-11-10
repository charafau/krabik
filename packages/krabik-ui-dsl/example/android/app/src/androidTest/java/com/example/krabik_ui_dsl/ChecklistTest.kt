package com.example.krabik_ui_dsl

import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.closeSoftKeyboard
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withContentDescription
import androidx.test.espresso.matcher.ViewMatchers.withHint
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Test
import org.junit.runner.RunWith

/**
 * End-to-end test for the checklist example on Android.
 *
 * Every view is created by Rust through JNI. The test drives the real view
 * hierarchy: toggling a task updates the summary, and the note flow exercises
 * per-row delete actions built at runtime, then removal.
 */
@RunWith(AndroidJUnit4::class)
class ChecklistTest {

    @Test
    fun checklistEndToEnd() {
        ActivityScenario.launch(MainActivity::class.java).use {
            // The summary reflects the initial state.
            onView(withText("1 of 3 tasks done · 0 notes")).check(matches(isDisplayed()))

            // Completing a task updates the summary in place. The switch is
            // matched by its accessibility label, like on iOS.
            onView(withContentDescription("Register the app factory")).perform(click())
            onView(withText("2 of 3 tasks done · 0 notes")).check(matches(isDisplayed()))

            // Adding a note appends a native row and updates the summary.
            onView(withHint("Add a note")).perform(
                scrollTo(),
                click(),
                typeText("First note"),
                closeSoftKeyboard(),
            )
            onView(withText("Add")).perform(click())
            onView(withText("2 of 3 tasks done · 1 note")).check(matches(isDisplayed()))

            // The per-row delete action, built dynamically by the app, removes
            // the note again.
            onView(withText("Delete")).perform(scrollTo(), click())
            onView(withText("2 of 3 tasks done · 0 notes")).check(matches(isDisplayed()))
        }
    }
}
