package com.example.krabik_ui_dsl

import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.TextView
import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.closeSoftKeyboard
import androidx.test.espresso.action.ViewActions.scrollTo
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.matcher.ViewMatchers.withHint
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Verifies that the frames Rust sends actually reach the native views.
 *
 * The end-to-end test covers behaviour; this one inspects the hierarchy, so a
 * regression in the layout path (a dropped translation, a missing layout
 * params update) fails even when the text still looks right.
 */
@RunWith(AndroidJUnit4::class)
class LayoutTest {

    @Test
    fun framesAreAppliedToTheNativeViews() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            addNote("First note")
            addNote("Second note")

            scenario.onActivity { activity ->
                val root = activity.window.decorView

                val first = findText(root, "• First note")
                val second = findText(root, "• Second note")
                assertNotNull("first note row missing", first)
                assertNotNull("second note row missing", second)

                val firstRow = first!!.parent as View
                val secondRow = second!!.parent as View

                // A node whose frame was never applied stays at 0x0.
                assertTrue(
                    "first row has no size: ${firstRow.width}x${firstRow.height}",
                    firstRow.width > 0 && firstRow.height > 0,
                )
                assertTrue(
                    "second row has no size: ${secondRow.width}x${secondRow.height}",
                    secondRow.width > 0 && secondRow.height > 0,
                )

                // Rows are positioned by translation, so they must not overlap.
                val firstBottom = firstRow.y + firstRow.height
                assertTrue(
                    "rows overlap: first=${firstRow.y}..$firstBottom second=${secondRow.y}",
                    secondRow.y >= firstBottom - 1,
                )

                // The input keeps its measured size after the rows around it
                // were re-laid out.
                val field = findHint(root, "Add a note")
                assertNotNull("note input missing", field)
                assertTrue(
                    "note input has no size: ${field!!.width}x${field.height}",
                    field.width > 0 && field.height > 0,
                )

                // The summary sits above the notes, in the same coordinate space.
                val summary = findText(root, "2 notes")
                assertNotNull("summary missing", summary)
                assertTrue(
                    "summary overlaps the first note row",
                    summary!!.y + summary.height <= firstRow.y,
                )
            }
        }
    }

    private fun addNote(text: String) {
        onView(withHint("Add a note")).perform(
            scrollTo(),
            click(),
            typeText(text),
            closeSoftKeyboard(),
        )
        onView(withText("Add")).perform(click())
    }

    /**
     * The batch API addresses views by node id, so the registry must gain a
     * view when Rust creates one and lose it when Rust removes it.
     */
    @Test
    fun registryTracksCreatedAndRemovedViews() {
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            var baseline = 0
            scenario.onActivity { activity ->
                baseline = findRustRoot(activity.window.decorView)!!.registeredViewCount()
            }
            assertTrue("no views registered", baseline > 0)

            addNote("Tracked note")
            scenario.onActivity { activity ->
                val count = findRustRoot(activity.window.decorView)!!.registeredViewCount()
                assertTrue("note did not register views: $count <= $baseline", count > baseline)
            }

            onView(withText("Clear notes")).perform(click())
            scenario.onActivity { activity ->
                val count = findRustRoot(activity.window.decorView)!!.registeredViewCount()
                assertEquals("removed views stayed registered", baseline, count)
            }
        }
    }

    private fun findRustRoot(view: View): RustRootView? {
        if (view is RustRootView) return view
        if (view is ViewGroup) {
            for (index in 0 until view.childCount) {
                findRustRoot(view.getChildAt(index))?.let { return it }
            }
        }
        return null
    }

    private fun findText(root: View, text: String): TextView? {
        if (root is TextView && root.text?.toString()?.contains(text) == true) {
            return root
        }
        if (root is ViewGroup) {
            for (index in 0 until root.childCount) {
                findText(root.getChildAt(index), text)?.let { return it }
            }
        }
        return null
    }

    private fun findHint(root: View, hint: String): EditText? {
        if (root is EditText && root.hint?.toString() == hint) {
            return root
        }
        if (root is ViewGroup) {
            for (index in 0 until root.childCount) {
                findHint(root.getChildAt(index), hint)?.let { return it }
            }
        }
        return null
    }
}
