package com.example.krabik_ui_dsl

import dev.krabik.ui.*
import dev.krabik.ui.testing.findRustRoot
import android.content.pm.ActivityInfo
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.widget.ProgressBar
import androidx.lifecycle.Lifecycle
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.GeneralLocation
import androidx.test.espresso.action.GeneralSwipeAction
import androidx.test.espresso.action.Press
import androidx.test.espresso.action.Swipe
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.swipeLeft
import androidx.test.espresso.action.ViewActions.swipeUp
import androidx.test.espresso.assertion.ViewAssertions.doesNotExist
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.isEnabled
import androidx.test.espresso.matcher.ViewMatchers.isNotChecked
import androidx.test.espresso.matcher.ViewMatchers.isRoot
import androidx.test.espresso.matcher.ViewMatchers.isSelected
import androidx.test.espresso.matcher.ViewMatchers.withContentDescription
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.hamcrest.Matchers.anyOf
import org.hamcrest.Matchers.not
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
            onView(withText("Time: 10:30")).perform(
                androidx.test.espresso.action.ViewActions.scrollTo(),
            )
            onView(withText("10:30")).perform(click())
            onView(withText("OK")).perform(click())
            onView(withText("Time: 10:30")).check(matches(isDisplayed()))
            onView(isRoot()).perform(swipeUp())
            onView(withContentDescription("Date and time")).check(matches(isDisplayed()))
            onView(withContentDescription("Bounded date")).check(matches(isDisplayed()))
            onView(withText("Bounded: 2026-06-15")).check(matches(isDisplayed()))
            tapBack()

            openPage("Tabs")
            onView(withText("Overview: the first tab is selected by default."))
                .check(matches(isDisplayed()))
            onView(withText("Settings")).perform(click())
            onView(withText("Settings: tapping a tab sends the selected index."))
                .check(matches(isDisplayed()))
            tapBack()

            openPage("Frames & opacity")
            onView(withText("Panel is open.")).check(matches(isDisplayed()))
            onView(withText("Hide panel")).perform(click())
            onView(withText("Panel is closed.")).check(matches(isDisplayed()))
            onView(withText("Show panel")).perform(click())
            onView(withText("Panel is open.")).check(matches(isDisplayed()))
            tapBack()

            openPage("Decoration")
            onView(withText("Warm card")).check(matches(isDisplayed()))
            onView(withText("Cool palette")).perform(click())
            onView(withText("Cool card")).check(matches(isDisplayed()))
            tapBack()

            openPage("Enter & exit")
            onView(withText("2 cards")).check(matches(isDisplayed()))
            onView(withText("Add card")).perform(click())
            onView(withText("3 cards")).check(matches(isDisplayed()))
            onView(withText("Remove card")).perform(click())
            onView(withText("2 cards")).check(matches(isDisplayed()))
            tapBack()

            openPage("Curves & springs")
            onView(withText("Target: narrow")).check(matches(isDisplayed()))
            onView(withText("Run curves")).perform(click())
            onView(withText("Target: wide")).check(matches(isDisplayed()))
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
            onView(withContentDescription("Intrinsic asset icon")).check(matches(isDisplayed()))
            tapBack()

            openPage("Recycled list")
            onView(withText("Row taps: 0")).check(matches(isDisplayed()))
            onView(withContentDescription("Tap Recycled row 1")).perform(click())
            onView(withText("Row taps: 1")).check(matches(isDisplayed()))
            onView(withContentDescription("Shuffle rows")).perform(click())
            onView(withText("Recycled row 1 · updated")).check(matches(isDisplayed()))
            tapBack()

            openPage("Styling")
            onView(withText("Disabled action")).check(matches(not(isEnabled())))
            tapBack()
        }
    }

    /** The native chrome a Scaffold adds around a screen's content. */
    @Test
    fun scaffoldFloatingButtonSnackbarAndDrawer() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Scaffold")
            onView(withText("Items: 0")).check(matches(isDisplayed()))

            // The floating action button adds an item and raises a snackbar,
            // whose action takes the item back.
            onView(withText("Add")).perform(click())
            awaitText("Items: 1")
            awaitText("Added item 1")
            onView(withText("Undo")).perform(click())
            awaitText("Items: 0")
            awaitNoText("Added item 1")

            // The drawer opens from state and a destination closes it again.
            onView(withText("Open drawer")).perform(click())
            awaitText("Archive")
            onView(withText("Archive")).perform(click())
            awaitText("Section: Archive")
            Thread.sleep(600)
            onView(withText("Archive")).check(matches(not(isDisplayed())))

            // The system back action closes an open drawer before anything else.
            onView(withText("Open drawer")).perform(click())
            awaitText("Settings")
            androidx.test.espresso.Espresso.pressBack()
            Thread.sleep(600)
            onView(withText("Settings")).check(matches(not(isDisplayed())))
            onView(withText("Section: Archive")).check(matches(isDisplayed()))

            // Leaving the screen takes its chrome with it.
            tapBack()
            onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))
            onView(withText("Add")).check(doesNotExist())
        }
    }

    /**
     * The manifest handles rotation itself, so the activity, the Rust session
     * and the native views all survive it; only the layout changes.
     */
    @Test
    fun rotationKeepsTheActivityTheStackAndTheState() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            openPage("Radio group")
            onView(withText("Large")).perform(click())
            onView(withText("Selected: Large")).check(matches(isDisplayed()))

            var portrait: MainActivity? = null
            var portraitWidth = 0
            scenario.onActivity { activity ->
                portrait = activity
                portraitWidth = findRustRoot(activity.window.decorView)!!.width
                activity.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE
            }
            Thread.sleep(2000)

            try {
                scenario.onActivity { activity ->
                    assertTrue("rotation must not recreate the activity", activity === portrait)
                    assertTrue(
                        "the device is in landscape",
                        activity.resources.configuration.orientation ==
                            Configuration.ORIENTATION_LANDSCAPE,
                    )
                    val root = findRustRoot(activity.window.decorView)!!
                    assertTrue("Rust laid the screen out for the new width", root.width > portraitWidth)
                }
                // Same screen, same choice.
                onView(withText("Selected: Large")).check(matches(isDisplayed()))
            } finally {
                scenario.onActivity { activity ->
                    activity.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_PORTRAIT
                }
                Thread.sleep(2000)
            }

            // The stack is intact: back returns to the catalog.
            onView(withText("Selected: Large")).check(matches(isDisplayed()))
            tapBack()
            onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))
        }
    }

    /**
     * Slow work runs off the main thread: the screen stays usable while it
     * runs, and the result arrives as a message.
     */
    @Test
    fun backgroundWorkReportsBackWithoutBlockingTheScreen() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Async")
            onView(withText("Nothing loaded yet")).check(matches(isDisplayed()))

            onView(withText("Load")).perform(click())
            onView(withText("Loading…")).check(matches(isDisplayed()))
            // The request takes a couple of seconds; taps are handled meanwhile.
            onView(withText("Tap")).perform(click())
            onView(withText("Taps: 1")).check(matches(isDisplayed()))
            onView(withText("Loading…")).check(matches(isDisplayed()))

            awaitText("Loaded 3 items")
        }
    }

    /**
     * Work started before the activity is recreated delivers its result to
     * the activity that replaces it.
     */
    @Test
    fun backgroundWorkSurvivesRecreation() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            openPage("Async")
            onView(withText("Load")).perform(click())
            onView(withText("Loading…")).check(matches(isDisplayed()))

            scenario.recreate()

            // Same screen, and the request that was running still reports.
            awaitText("Loaded 3 items")
        }
    }

    /** A timer runs while the application asks for it and the app is in front. */
    @Test
    fun aSubscriptionTicksOnlyWhileItIsWanted() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            openPage("Async")
            onView(withText("Ticks: 0")).check(matches(isDisplayed()))

            onView(withContentDescription("Clock")).perform(click())
            awaitText("Ticks: 2")

            // In the background the clock stands still.
            scenario.moveToState(Lifecycle.State.CREATED)
            Thread.sleep(3500)
            scenario.moveToState(Lifecycle.State.RESUMED)
            val resumed = ticks()
            assertTrue("the clock ran in the background: $resumed", resumed <= 4)

            // Switched off, it stops. Right after the activity returns to the
            // front a tap can still go to the window that covered it, so the
            // tap is repeated until the switch has taken it.
            for (attempt in 1..5) {
                onView(withContentDescription("Clock")).perform(click())
                Thread.sleep(400)
                val off = try {
                    onView(withContentDescription("Clock")).check(matches(isNotChecked()))
                    true
                } catch (_: Throwable) {
                    false
                }
                if (off) break
            }
            onView(withContentDescription("Clock")).check(matches(isNotChecked()))
            Thread.sleep(500)
            val stopped = ticks()
            Thread.sleep(2500)
            val later = ticks()
            assertTrue(
                "the clock kept running after it was switched off: $stopped then $later",
                later == stopped,
            )
        }
    }

    /** The number the Async page shows after "Ticks: ". */
    private fun ticks(): Int {
        for (count in 0..60) {
            if (isVisible("Ticks: $count")) return count
        }
        throw AssertionError("no tick count on screen")
    }

    /**
     * A switch between light and dark recreates the activity, so the native
     * chrome picks up the new theme. The application and its screens must
     * come back as they were, repainted for the new appearance.
     */
    @Test
    fun switchingToDarkKeepsTheScreenAndItsState() {
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        fun night(mode: String) {
            automation.executeShellCommand("cmd uimode night $mode").close()
            Thread.sleep(2500)
        }
        fun screenBackground(scenario: ActivityScenario<MainActivity>): Int {
            var color = 0
            scenario.onActivity { activity ->
                val screen = activity.supportFragmentManager.fragments
                    .last { it.view != null && it is KrabikScreenFragment }
                color = (screen.requireView().background as ColorDrawable).color
            }
            return color
        }

        night("no")
        try {
            ActivityScenario.launch(MainActivity::class.java).use { scenario ->
                openPage("Radio group")
                onView(withText("Large")).perform(click())
                onView(withText("Selected: Large")).check(matches(isDisplayed()))
                assertTrue(Color.luminance(screenBackground(scenario)) > 0.5f)

                night("yes")

                // Still on the page, with the choice made before the switch.
                onView(withText("Selected: Large")).check(matches(isDisplayed()))
                assertTrue(
                    "the screen must be repainted dark",
                    Color.luminance(screenBackground(scenario)) < 0.5f,
                )
                // The stack came back too: back returns to the catalog.
                tapBack()
                onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))
            }
        } finally {
            night("no")
        }
    }

    @Test
    fun navigationScreensPaintAnOpaqueBackground() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            openPage("Text")
            scenario.onActivity { activity ->
                val screen = activity.supportFragmentManager.fragments
                    .last { it.view != null && it is KrabikScreenFragment }
                val background = screen.requireView().background as? ColorDrawable
                assertTrue(
                    "navigation screens must be opaque",
                    background != null && Color.alpha(background.color) == 255,
                )
            }
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
            awaitText("Delete task?")
            onView(withText("Cancel")).perform(click())
            awaitNoText("Delete task?")

            // The system alert's other button and the back button close it too.
            openPage("Dialog")
            onView(withText("Delete")).perform(click())
            awaitNoText("Delete task?")
            openPage("Dialog")
            androidx.test.espresso.Espresso.pressBack()
            awaitNoText("Delete task?")

            openPage("Bottom sheet")
            onView(withText("Open bottom sheet")).check(matches(isDisplayed()))
            openSheet()
            onView(withText("Close")).perform(click())
            awaitNoText("Filters")

            // A native dismissal reaches Rust, so the sheet can be reopened.
            openSheet()
            androidx.test.espresso.Espresso.pressBack()
            awaitNoText("Filters")
            openSheet()
            onView(withText("Close")).perform(click())
            onView(withText("Open bottom sheet")).check(matches(isDisplayed()))
            tapBack()
        }
    }

    /**
     * An alert has three button slots. With more choices than that, all of
     * them are offered as a list, and a sheet whose content grows shows
     * the new rows.
     */
    @Test
    fun manyChoicesAndAGrowingSheet() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Bottom sheet")
            onView(withText("Label: none")).check(matches(isDisplayed()))

            onView(withText("Choose a label")).perform(click())
            awaitText("Every choice is offered, in this order.")
            for (choice in listOf("Red", "Green", "Blue", "Yellow", "Cancel")) {
                onView(withText(choice)).check(matches(isDisplayed()))
            }
            onView(withText("Blue")).perform(click())
            awaitNoText("Every choice is offered, in this order.")
            awaitText("Label: Blue")

            openSheet()
            onView(withText("More filters")).perform(click())
            awaitText("From this week")
            onView(withText("Fewer filters")).check(matches(isDisplayed()))
            onView(withText("Close")).check(matches(isDisplayed())).perform(click())
            awaitNoText("Filters")
            tapBack()
        }
    }

    @Test
    fun poppingKeepsThePreviousScreenAlive() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            var home: android.view.View? = null
            scenario.onActivity { activity ->
                home = activity.supportFragmentManager.fragments
                    .first { it is KrabikScreenFragment }.view
            }
            openPage("Text")
            androidx.test.espresso.Espresso.pressBack()
            Thread.sleep(500)
            scenario.onActivity { activity ->
                val screens = activity.supportFragmentManager.fragments
                    .filterIsInstance<KrabikScreenFragment>()
                assertTrue("only the home screen remains", screens.size == 1)
                assertTrue("the home view was not rebuilt", screens[0].view === home)
            }
        }
    }

    @Test
    fun containersDemo() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("View pager")
            onView(withText("Page 1 of 3")).check(matches(isDisplayed()))
            onView(withContentDescription("View pager")).perform(swipeLeft())
            Thread.sleep(700)
            onView(anyOf(withText("Page 2 of 3"), withText("Page 3 of 3")))
                .check(matches(isDisplayed()))
            tapBack()

            openPage("Tabs view")
            onView(withText("Selected: Overview")).check(matches(isDisplayed()))
            onView(withText("Details")).perform(click())
            Thread.sleep(500)
            onView(withText("Selected: Details")).check(matches(isDisplayed()))
            onView(withText("Details page content.")).check(matches(isDisplayed()))

            // Swiping the pager moves the tab highlight: the tab bar is
            // updated in place from the application's new selection.
            onView(withText("Details page content.")).perform(swipeLeft())
            Thread.sleep(700)
            onView(withText("Selected: Settings")).check(matches(isDisplayed()))
            onView(withText("Settings")).check(matches(isSelected()))
            onView(withText("Details")).check(matches(not(isSelected())))
            tapBack()

            openPage("Bottom navigation")
            onView(withText("Home destination")).check(matches(isDisplayed()))
            onView(withContentDescription("Search")).perform(click())
            onView(withText("Search destination")).check(matches(isDisplayed()))
            tapBack()

            openPage("Toolbar")
            onView(withText("Toolbar taps: 0")).check(matches(isDisplayed()))
            onView(withText("Refresh")).perform(click())
            onView(withText("Toolbar taps: 1")).check(matches(isDisplayed()))
            onView(withText("Save")).perform(click())
            onView(withText("Toolbar taps: 2")).check(matches(isDisplayed()))
            tapBack()
        }
    }

    @Test
    fun pageTransitionsDemo() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Page transitions")

            onView(withText("Push with slide")).perform(click())
            onView(withText("Pushed with the native slide transition."))
                .check(matches(isDisplayed()))
            tapBack()

            onView(withText("Push with fade")).perform(click())
            onView(withText("Pushed with a cross-fade transition."))
                .check(matches(isDisplayed()))
            tapBack()

            onView(withText("Push with no animation")).perform(click())
            onView(withText("Pushed with no animation."))
                .check(matches(isDisplayed()))
            tapBack()
            tapBack()
        }
    }

    /** The device's language and appearance reach the application. */
    @Test
    fun theApplicationHearsAboutTheEnvironment() {
        val automation = InstrumentationRegistry.getInstrumentation().uiAutomation
        fun night(mode: String) {
            automation.executeShellCommand("cmd uimode night $mode").close()
            Thread.sleep(2500)
        }
        val language = java.util.Locale.getDefault().language

        night("no")
        try {
            ActivityScenario.launch(MainActivity::class.java).use {
                openPage("Environment")
                onView(withText("Language: $language")).check(matches(isDisplayed()))
                onView(withText("Appearance: light")).check(matches(isDisplayed()))

                // The switch recreates the activity; the application it
                // kept is told what changed.
                night("yes")
                onView(withText("Appearance: dark")).check(matches(isDisplayed()))
                onView(withText("Language: $language")).check(matches(isDisplayed()))
            }
        } finally {
            night("no")
        }
    }

    /**
     * The application shows dark while the device is light. The activity is
     * recreated for it, as for a change on the device, and comes back on the
     * same page with the toolbar, the system bars and the colours Rust
     * paints all dark.
     */
    @Test
    fun theApplicationChoosesItsAppearance() {
        fun screenBackground(scenario: ActivityScenario<MainActivity>): Int {
            var color = 0
            scenario.onActivity { activity ->
                val screen = activity.supportFragmentManager.fragments
                    .last { it.view != null && it is KrabikScreenFragment }
                color = (screen.requireView().background as ColorDrawable).color
            }
            return color
        }
        fun nightResources(scenario: ActivityScenario<MainActivity>): Boolean {
            var night = false
            scenario.onActivity { activity ->
                night = activity.resources.configuration.uiMode and
                    Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
            }
            return night
        }

        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            try {
                openPage("Environment")
                onView(withText("Appearance: light")).check(matches(isDisplayed()))

                onView(withText("Always dark")).perform(click())
                awaitText("Appearance: dark")
                assertTrue("the theme is dark", nightResources(scenario))
                assertTrue(
                    "the screen is painted dark",
                    Color.luminance(screenBackground(scenario)) < 0.5f,
                )

                onView(withText("Always light")).perform(click())
                awaitText("Appearance: light")
                assertTrue("the theme is light", !nightResources(scenario))

                onView(withText("Follow device")).perform(click())
                Thread.sleep(1000)
                onView(withText("Appearance: light")).check(matches(isDisplayed()))
            } finally {
                // The choice belongs to the process; do not leak it into
                // the tests that follow.
                scenario.onActivity {
                    androidx.appcompat.app.AppCompatDelegate.setDefaultNightMode(
                        androidx.appcompat.app.AppCompatDelegate.MODE_NIGHT_FOLLOW_SYSTEM,
                    )
                }
            }
        }
    }

    /** The platform's pull-to-refresh on a scroll view and on a recycled list. */
    @Test
    fun pullingDownRefreshes() {
        // A drag from the middle of the view to well below it: far enough
        // for the refresh layout, wherever the view sits on the screen.
        val pullDown = GeneralSwipeAction(
            Swipe.SLOW,
            GeneralLocation.CENTER,
            { view ->
                val origin = IntArray(2).also(view::getLocationOnScreen)
                val density = view.resources.displayMetrics.density
                floatArrayOf(
                    origin[0] + view.width / 2f,
                    origin[1] + view.height / 2f + 320 * density,
                )
            },
            Press.FINGER,
        )
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Pull to refresh")
            onView(withText("Scroll refreshes: 0")).check(matches(isDisplayed()))
            onView(withText("List refreshes: 0")).check(matches(isDisplayed()))

            onView(withText("Pull this box down")).perform(pullDown)
            awaitText("Scroll refreshes: 1")
            onView(withText("List refreshes: 0")).check(matches(isDisplayed()))

            // The list grows by a page of rows with every refresh.
            onView(withText("Message 6")).check(doesNotExist())
            onView(withText("Message 2")).perform(pullDown)
            awaitText("List refreshes: 1")
            onView(withText("Message 6")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun aContainerReactsToTaps() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Container")
            onView(withText("Nothing tapped yet.")).check(matches(isDisplayed()))

            // A tap on the card's text lands on the card.
            onView(withText("Inbox")).perform(click())
            onView(withText("Last tapped: Inbox (1 taps)")).check(matches(isDisplayed()))

            // The button inside the card keeps its own tap.
            onView(withText("Mute")).perform(click())
            onView(withText("Unmute")).check(matches(isDisplayed()))
            onView(withText("Last tapped: Inbox (1 taps)")).check(matches(isDisplayed()))

            // A container that paints nothing is tappable too.
            onView(withContentDescription("Archive row")).perform(click())
            onView(withText("Last tapped: Archive (2 taps)")).check(matches(isDisplayed()))

            // Disabled and plain containers ignore taps.
            onView(withText("Disabled card")).perform(click())
            onView(withText("A plain container ignores taps.")).perform(click())
            onView(withText("Last tapped: Archive (2 taps)")).check(matches(isDisplayed()))
        }
    }

    @Test
    fun systemBackReturnsToTheCatalog() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Spinner")
            androidx.test.espresso.Espresso.pressBack()
            Thread.sleep(400)
            // The catalog is revealed where the user left it, not rebuilt.
            onView(withText("Spinner")).check(matches(isDisplayed()))
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

    /**
     * Opens the bottom sheet and waits for it.
     *
     * A sheet that was just dismissed keeps its window on screen while it
     * animates out, and a tap during that time lands on its scrim instead of
     * the button underneath. Tap again once the window is gone.
     */
    private fun openSheet() {
        repeat(4) {
            onView(withText("Open bottom sheet")).perform(click())
            try {
                awaitText("Filters", timeoutMs = 1500)
                return
            } catch (_: Throwable) {
                // The previous sheet's window swallowed the tap.
            }
        }
        awaitText("Filters")
    }
}
