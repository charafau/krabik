package com.example.krabik_ui_dsl

import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.swipeDown
import androidx.test.espresso.action.ViewActions.swipeUp
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isCompletelyDisplayed
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.withText

/**
 * Scrolls the catalog until [label] is displayed, then taps it.
 *
 * The DSL scroll views position their content with translations, so Espresso's
 * own `scrollTo` cannot always find the target. Swiping the root view mirrors
 * what a user does and works for both the catalog and the widget pages.
 */
internal fun openPage(label: String) {
    swipeTo(label)
    onView(withText(label)).perform(click())
    Thread.sleep(300)
}

/** Swipes up until [label] is displayed. */
internal fun swipeTo(label: String) {
    repeat(12) {
        if (isVisible(label)) {
            return
        }
        onView(isRoot()).perform(swipeUp())
        Thread.sleep(400)
    }
}

/** Swipes down to the top of the current page. */
internal fun swipeToTop() {
    repeat(12) {
        if (isVisible("Krabik UI DSL")) {
            return
        }
        onView(isRoot()).perform(swipeDown())
        Thread.sleep(400)
    }
}

/** Taps the Back button of a widget page, scrolling to it if needed. */
internal fun tapBack() {
    repeat(12) {
        if (isVisible("Back")) {
            onView(withText("Back")).perform(click())
            swipeToTop()
            return
        }
        onView(isRoot()).perform(swipeDown())
    }
    onView(withText("Back")).perform(click())
    swipeToTop()
}

/** Whether a view with exactly [text] is currently displayed. */
internal fun isVisible(text: String): Boolean =
    try {
        onView(withText(text)).check(matches(isCompletelyDisplayed()))
        true
    } catch (_: Throwable) {
        false
    }
