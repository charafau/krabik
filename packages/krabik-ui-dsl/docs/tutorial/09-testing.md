# 9. Testing

This chapter covers the three levels at which a krabik app is tested: Rust unit tests on your
app with no device, the framework's own tests against a fake platform, and instrumented UI tests
that drive the real native widgets on an emulator and a simulator. For each level it says what
it can and cannot catch, and for the UI tests it collects the pitfalls the existing suites ran
into. Afterwards you will know where a given test belongs.

## The three levels

```text
   level                  runs on        speed     what it proves
   -----                  -------        -----     --------------
   1. app unit tests      your machine   ms        your update logic, your
      (cargo test)                                 hooks, the shape of your
                                                   views

   2. framework tests     your machine   ms        the engine and the session
      with a fake                                  do the right thing with
      platform                                     any app

   3. UI tests            emulator /     minutes   the native widgets really
      (Espresso,          simulator                appear and react, on a
      XCUITest)                                    real OS
```

Level 1 is yours to write and should hold most of your tests. Level 2 already exists; you read
it more than you write it. Level 3 is a thin layer on top: a few flows per app, to prove that the
whole stack is wired together.

## Level 1: unit tests on your app

Every chapter so far ended with tests of this kind. The reason they are possible is the
architecture: state changes only in `update`, and everything the screen shows is returned from
functions of `&self`. A test plays the framework's part. It sends messages and reads what the
app says should be visible.

There are three things to look at.

### State after messages

The most direct test: send messages, check fields.

```rust,ignore
/// Types a note and submits it, the way the field and the Add button do.
fn add(app: &mut NotesApp, text: &str) {
    app.update(Msg::Composer(ComposerMsg::Draft(text.to_string())));
    app.update(Msg::Composer(ComposerMsg::Submit));
}

#[test]
fn adding_trims_and_ignores_a_blank_draft() {
    let mut app = NotesApp::new();
    add(&mut app, "   ");
    assert_eq!(app.notes.len(), 2);

    add(&mut app, "  Buy milk  ");
    let added = app.notes.last().unwrap();
    assert_eq!((added.id, added.text.as_str()), (3, "Buy milk"));
    assert!(app.composer.draft.is_empty());
}
```

Send the *wrapped* messages (`Msg::Composer(..)`), not calls to private helpers. That is what
the framework delivers, so the test also covers the forwarding in `update`. The minimal todo
example has one test that does this for its whole flow, `messages_drive_the_same_flow`.

### What the hooks return

`navigation`, `screen`, `dialog`, `sheet` and `back` are ordinary methods returning ordinary
values with public fields. Chapter 5 and chapter 6 tested them:

```rust,ignore
app.update(Msg::Open(2));
assert_eq!(app.navigation().unwrap().routes()[1].key, "note:2");

app.update(Msg::AskDelete(2));
assert!(app.dialog().expect("the confirmation").is_cancelable());

let snackbar = app.screen("notes").unwrap().snackbar.expect("a snackbar");
assert_eq!(snackbar.action.as_deref(), Some("Undo"));
```

A `Scaffold`'s slots, an `AppBar`'s `title` and `actions`, a `Dialog`'s `buttons`, a `Drawer`'s
`open` and `selected` are all public fields (`rust/src/scaffold.rs`, `rust/src/presentation.rs`).
The handlers inside are opaque, but `Handler::is_set()` tells you one is attached, as in
`app.sheet().unwrap().on_dismiss.is_set()`.

For back, use the line from chapter 5. It sends whatever the app says back should do:

```rust,ignore
app.update(app.back().expect("back is handled"));
```

### What a view contains

Sometimes the thing to check is on screen, not in the state: "the list shows a row for the new
note". An `Element` can be turned into the engine's tree and searched, with the same engine the
devices use and a stand-in for text measurement. This is the pattern from the showcase's test
`tabs_view_selection_updates_the_existing_tree`:

```rust,ignore
/// Every text the element shows on a phone-sized screen, in tree order.
fn texts(element: &Element<Msg>) -> Vec<String> {
    use krabik_ui_dsl::geometry::Size;
    use krabik_ui_dsl::measure::ApproximateMeasure;
    use krabik_ui_dsl::runtime::Runtime;
    use krabik_ui_dsl::tree::{self, NodeKind};
    use krabik_ui_dsl::widgets::Widget;

    let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
    runtime.update(element, Size::new(390.0, 800.0));
    tree::preorder(runtime.tree())
        .into_iter()
        .filter_map(|node| match &node.kind {
            NodeKind::Widget(Widget::Text(text)) => Some(text.content.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_list_shows_a_row_for_a_new_note() {
    let mut app = NotesApp::new();
    add(&mut app, "Buy milk");
    let texts = texts(&app.list_body());
    assert!(texts.contains(&"3 notes".to_string()));
    assert!(texts.contains(&"Buy milk".to_string()));
}
```

What the pieces are:

- `Runtime` (`rust/src/engine/runtime.rs`) is the engine's state for one native host: the
  current tree, the layout, scroll offsets. `update` builds the tree from an element for a
  viewport size and lays it out. Passing a real size matters for lists: a `List` only builds the
  rows that fit the viewport (chapter 3), so with no viewport you would see almost none.
- `ApproximateMeasure` (`rust/src/engine/measure.rs`) measures text without a platform. Its doc
  comment says what it assumes: "an average glyph width of `0.55 * font size` and a line height
  of `1.25 * font size`". Good enough to build a tree; not a statement about real pixels.
- `tree::preorder` walks the tree. A node's `kind` says what it is; for a widget, the properties
  struct inside is the one from `rust/src/widgets/`, so `text.content`, `button.label`,
  `field.value` and so on are readable.

The `_ =>` arm in the `match` is fine in a test. Inside the framework a match on `Widget` never
has one, so that the compiler lists every place a new widget must be handled; a test that looks
for one kind of node has no such duty.

Keep these tests few. A view test that asserts the exact structure of a screen breaks every time
the design changes. Assert that the important thing is present.

### The complete test module

For the final app from chapter 8, at the bottom of `notes/rust/src/lib.rs`:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    /// Types a note and submits it, the way the field and the Add button do.
    fn add(app: &mut NotesApp, text: &str) {
        app.update(Msg::Composer(ComposerMsg::Draft(text.to_string())));
        app.update(Msg::Composer(ComposerMsg::Submit));
    }

    /// Every text the element shows on a phone-sized screen, in tree order.
    fn texts(element: &Element<Msg>) -> Vec<String> {
        use krabik_ui_dsl::geometry::Size;
        use krabik_ui_dsl::measure::ApproximateMeasure;
        use krabik_ui_dsl::runtime::Runtime;
        use krabik_ui_dsl::tree::{self, NodeKind};
        use krabik_ui_dsl::widgets::Widget;

        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(element, Size::new(390.0, 800.0));
        tree::preorder(runtime.tree())
            .into_iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Widget(Widget::Text(text)) => Some(text.content.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_composer_hands_over_a_trimmed_draft_once() {
        let mut composer = Composer::new();
        assert_eq!(composer.take(), None);
        composer.update(ComposerMsg::Draft("  Buy milk  ".to_string()));
        assert_eq!(composer.take().as_deref(), Some("Buy milk"));
        assert_eq!(composer.take(), None);
    }

    #[test]
    fn adding_trims_and_ignores_a_blank_draft() {
        let mut app = NotesApp::new();
        add(&mut app, "   ");
        assert_eq!(app.notes.len(), 2);

        add(&mut app, "  Buy milk  ");
        let added = app.notes.last().unwrap();
        assert_eq!((added.id, added.text.as_str()), (3, "Buy milk"));
        assert!(app.composer.draft.is_empty());
    }

    #[test]
    fn the_list_shows_a_row_for_a_new_note() {
        let mut app = NotesApp::new();
        add(&mut app, "Buy milk");
        let texts = texts(&app.list_body());
        assert!(texts.contains(&"3 notes".to_string()));
        assert!(texts.contains(&"Buy milk".to_string()));
    }

    #[test]
    fn opening_a_note_pushes_its_screen_and_back_pops_it() {
        let mut app = NotesApp::new();
        // On the list, back belongs to the platform.
        assert_eq!(app.back(), None);

        app.update(Msg::Open(2));
        let navigation = app.navigation().unwrap();
        assert_eq!(navigation.routes()[1].key, "note:2");
        let screen = app.screen("note:2").expect("the detail screen");
        assert_eq!(screen.app_bar.expect("an app bar").title, "Note");

        app.update(app.back().expect("back is handled"));
        assert_eq!(app.navigation().unwrap().routes().len(), 1);
        // A note that does not exist cannot be opened.
        app.update(Msg::Open(99));
        assert_eq!(app.back(), None);
    }

    #[test]
    fn deleting_asks_first_and_can_be_undone() {
        let mut app = NotesApp::new();
        app.update(Msg::Open(1));
        app.update(Msg::AskDelete(1));
        let dialog = app.dialog().expect("the confirmation");
        assert_eq!(dialog.title, "Delete note?");
        assert!(dialog.is_cancelable());

        app.update(Msg::ConfirmDelete);
        assert!(app.dialog().is_none());
        assert!(app.note(1).is_none());
        // The detail screen is gone and the list offers Undo.
        assert_eq!(app.navigation().unwrap().routes().len(), 1);
        let snackbar = app.screen("notes").unwrap().snackbar.expect("a snackbar");
        assert_eq!(snackbar.action.as_deref(), Some("Undo"));

        app.update(Msg::Undo);
        assert_eq!(app.notes[0].id, 1);
        assert!(app.screen("notes").unwrap().snackbar.is_none());
    }

    #[test]
    fn a_snackbar_that_times_out_keeps_the_note_deleted() {
        let mut app = NotesApp::new();
        app.update(Msg::AskDelete(1));
        app.update(Msg::ConfirmDelete);
        app.update(Msg::SnackbarGone);
        assert!(app.screen("notes").unwrap().snackbar.is_none());
        app.update(Msg::Undo);
        assert!(app.note(1).is_none());
    }

    #[test]
    fn back_closes_the_sheet_before_the_screen() {
        let mut app = NotesApp::new();
        app.update(Msg::Open(2));
        app.update(Msg::ShowDetails);
        assert!(app.sheet().expect("the sheet").on_dismiss.is_set());

        app.update(app.back().expect("back is handled"));
        assert!(app.sheet().is_none());
        assert_eq!(app.navigation().unwrap().routes().len(), 2);

        app.update(app.back().expect("back is handled"));
        assert_eq!(app.back(), None);
    }

    #[test]
    fn the_drawer_follows_state_and_picks_the_order() {
        let mut app = NotesApp::new();
        let drawer = |app: &NotesApp| app.screen("notes").unwrap().drawer.expect("a drawer");
        assert!(!drawer(&app).open);

        app.update(Msg::Drawer(true));
        assert!(drawer(&app).open);

        app.update(Msg::Sort(1));
        assert_eq!(drawer(&app).selected, Some(1));
        assert!(!drawer(&app).open);
        assert_eq!(app.visible()[0].id, 2);
    }
}
```

Run them from `notes/rust` with `cargo test`.

### What level 1 cannot see

- Whether a native widget actually shows what the tree says.
- Real text measurement, and so real wrapping and truncation.
- Anything the platform does on its own: the keyboard, the animation of a push, a sheet being
  dragged, the system alert's layout.
- The platform halves of the framework. `rust/src/android/` and `rust/src/ios/` are not compiled
  on a desktop. To at least type-check and lint your crate for the devices, run clippy for a
  device target, as the minimal example's README suggests:

  ```sh
  cargo clippy --target aarch64-linux-android -- -D warnings
  cargo clippy --target aarch64-apple-ios-sim -- -D warnings
  ```

## Level 2: the framework's fake-platform tests

Between your app and the native widgets sits the *session*: the code that calls your hooks after
every message and tells the platform what to push, pop, show and dismiss. It is the part of the
framework with the most rules, and it is shared by Android and iOS behind a `Platform` trait
(`rust/src/host/platform.rs`). Because it only talks to the platform through that trait, it can
be run against a platform that does nothing but write down what it was asked.

That is what the `tests` module of `rust/src/host/session.rs` does. Its fake controller appends
a line to a log for every request:

```rust,ignore
fn pop(
    &self,
    _cx: &mut (),
    count: usize,
    transition: ScreenTransition,
) -> Result<(), Infallible> {
    self.0
        .borrow_mut()
        .push(format!("pop {count} {transition:?}"));
    Ok(())
}
```

and a test reads like a transcript of what the native side would have been told:

```rust,ignore
session.render_all(&mut ()).unwrap();
// The stack the application starts with is shown, not animated in.
assert_eq!(
    take(&log),
    ["push 1 home instantly", "root false", "chrome HOME"]
);
```

These tests are where the behaviour described in chapters 5 to 7 is pinned down. The names say
what each one guarantees:

| Test | Guarantee |
| ---- | --------- |
| `the_initial_stack_is_pushed_and_given_chrome` | a second identical render sends nothing |
| `a_restored_stack_is_shown_at_once_and_later_pushes_animate` | recreation does not replay pushes |
| `leaving_several_screens_is_one_pop` | one pop animation, hosts of popped screens dropped |
| `back_pops_until_the_root_and_then_declines` | back is handed to the platform at the root |
| `a_click_reaches_the_application_and_rerenders` | an event becomes a message and a render |
| `a_dialog_the_app_keeps_is_shown_again_after_it_closes` | state wins over a native dismissal |
| `a_sheet_can_be_reopened_after_a_native_dismissal` | a swipe is reported, not re-dismissed |
| `the_drawer_is_opened_by_state_and_follows_the_user` | native drawer changes reach the app |
| `a_snackbar_is_shown_once_and_can_be_dismissed_either_way` | timeout and action both report |
| `chrome_belongs_to_the_top_screen` | pushing and popping swaps the chrome |
| `the_color_scheme_reaches_every_host` | a theme change repaints all screens |

The history is the reason this level exists. Before the restructure (commit `c9d4d56`, "one
session and one event path shared by Android and iOS") each platform carried its own copy of the
session and event logic, and the two had drifted apart. Each copy could only be exercised on its
device. One shared session, tested with a fake platform, means a navigation bug is found by
`cargo test` in a second, and fixed once.

The rest of the engine is tested the same way, next to the code: tree building and keys in
`rust/src/engine/tree.rs`, layout in `layout.rs`, the diff in `render.rs`, lists and colour
schemes in `runtime.rs`, message mapping in `rust/src/element.rs`, the macros in
`rust/macros/src/`.

When do you touch this level? When you change the framework. And when something behaves
unexpectedly in your app: finding the test that covers the situation is usually faster than
reading the implementation, and it tells you whether what you see is intended.

## Level 3: UI tests on a device

A UI test installs the real app on an emulator or simulator, finds real native views and taps
them. It is the only level that proves a `Button` in your `view` became a button a user can
press. It is also slow and sensitive to timing, so keep it to a few end-to-end flows.

The existing suites to learn from:

| App | Android (Espresso) | iOS (XCUITest) |
| --- | ------------------ | -------------- |
| showcase | `example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/ShowcaseTest.kt`, `ChecklistTest.kt`, `LayoutTest.kt` | `example/ios/UITests/ShowcaseUITests.swift`, `ChecklistUITests.swift` |
| minimal todo | `example-todo-minimal/android/app/src/androidTest/.../MinimalTodoTest.kt` | `example-todo-minimal/ios/UITests/MinimalTodoUITests.swift` |

### Finding views

Rust creates the native views, so they have no Android resource ids and no storyboard
identifiers. Tests find them by what a user perceives:

| What | Espresso | XCUITest |
| ---- | -------- | -------- |
| a text | `onView(withText("2 notes"))` | `app.staticTexts["2 notes"]` |
| a button | `onView(withText("Add"))` | `app.buttons["Add"]` |
| a text field | `onView(withHint("Write a note"))` | `app.textFields["Write a note"]` |
| by accessibility label | `onView(withContentDescription("Open Buy milk"))` | `app.buttons["Open Buy milk"]` |
| the system alert | `onView(withText("Delete note?"))` | `app.alerts["Delete note?"]` |

The `accessibility_label` you set in Rust becomes the content description on Android and the
accessibility label on iOS (`apply_accessibility` in `rust/src/android/style.rs` and
`rust/src/ios/style.rs`). That is the second reason, after screen readers, to give repeated
buttons a label that names their row. On iOS a widget's kind decides which query finds it: the
minimal todo test finds its checkbox rows with `app.buttons["Wire the native renderer"]`.

For the cases where a test needs the native view object itself, there are shared helpers in
`shared/android-test/dev/krabik/ui/testing/ViewFinders.kt`: `findFirst<T>` walks the view
hierarchy for the first view of a type, `findRecycler` finds the `RecyclerView` of a recycled
list, and `findRustRoot` finds the root view that owns the widgets on screen. Each app adds that
directory to its test sources in `app/build.gradle.kts`:

```kotlin
// View finders shared by the instrumented tests of every app.
sourceSets["androidTest"].kotlin.srcDir(project(":krabik-ui").projectDir.resolve("../android-test"))
```

### A test for Notes

Modelled on the minimal todo tests, a flow for Notes looks like this. These two listings were
written against the same APIs the existing tests use but have not been run; treat them as a
starting point.

`notes/android/app/src/androidTest/java/com/example/notes/NotesTest.kt`:

```kotlin
package com.example.notes

import androidx.test.core.app.ActivityScenario
import androidx.test.espresso.Espresso.closeSoftKeyboard
import androidx.test.espresso.Espresso.onView
import androidx.test.espresso.Espresso.pressBack
import androidx.test.espresso.action.ViewActions.click
import androidx.test.espresso.action.ViewActions.typeText
import androidx.test.espresso.assertion.ViewAssertions.matches
import androidx.test.espresso.matcher.ViewMatchers.isDisplayed
import androidx.test.espresso.matcher.ViewMatchers.withContentDescription
import androidx.test.espresso.matcher.ViewMatchers.withHint
import androidx.test.espresso.matcher.ViewMatchers.withText
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Test
import org.junit.runner.RunWith

/** Drives Notes through its native widgets. */
@RunWith(AndroidJUnit4::class)
class NotesTest {

    @Test
    fun addOpenAndGoBack() {
        ActivityScenario.launch(MainActivity::class.java).use {
            onView(withText("2 notes")).check(matches(isDisplayed()))

            onView(withHint("Write a note")).perform(click(), typeText("Buy milk"))
            closeSoftKeyboard()
            onView(withText("Add")).perform(click())
            Thread.sleep(1_000)
            onView(withText("3 notes")).check(matches(isDisplayed()))

            onView(withContentDescription("Open Buy milk")).perform(click())
            Thread.sleep(500)
            onView(withText("Details")).check(matches(isDisplayed()))

            pressBack()
            Thread.sleep(500)
            onView(withText("3 notes")).check(matches(isDisplayed()))
        }
    }
}
```

`notes/ios/UITests/NotesUITests.swift`:

```swift
import XCTest

/// Drives Notes through the real UIKit widgets built by Rust.
final class NotesUITests: XCTestCase {
    private var app: XCUIApplication!

    override func setUpWithError() throws {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launch()
        XCTAssertTrue(app.staticTexts["2 notes"].waitForExistence(timeout: 10))
    }

    func testAddOpenAndGoBack() throws {
        let field = app.textFields["Write a note"]
        XCTAssertTrue(field.waitForExistence(timeout: 5))
        field.tap()
        field.typeText("Buy milk\n")
        XCTAssertTrue(app.staticTexts["3 notes"].waitForExistence(timeout: 5))

        app.buttons["Open Buy milk"].tap()
        XCTAssertTrue(app.buttons["Details"].waitForExistence(timeout: 5))

        app.buttons["Back"].tap()
        XCTAssertTrue(app.staticTexts["3 notes"].waitForExistence(timeout: 5))
    }
}
```

The iOS test submits with the return key (`"\n"`), which exercises `on_submit`; the Android test
presses the floating Add button. `app.buttons["Back"]` works because the framework's back button
has that fixed accessibility label (chapter 5).

### Pitfalls the existing tests record

The comments in the showcase tests are a list of things that went wrong. Each is worth knowing
before you write your own.

**Do not assert immediately after a tap that opens a dialog or a sheet.** From
`UiHelpers.kt`:

```kotlin
/**
 * Waits until a view with exactly [text] is displayed.
 *
 * A dialog or sheet lives in its own window, which appears a moment after
 * the tap that asked for it; an immediate assertion would still be looking
 * at the activity's window.
 */
internal fun awaitText(text: String, timeoutMs: Long = 3000) {
```

`awaitText` and `awaitNoText` poll every 100 ms until a deadline. On iOS the equivalent is built
in: `waitForExistence(timeout:)` and `waitForNonExistence(timeout:)`.

**Do not tap while a dismissed sheet is still animating out.** The sheet's window stays on
screen for the length of its exit animation and swallows the tap. From `ShowcaseTest.kt`:

```kotlin
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
```

The fix is to retry the tap until the sheet appears, not to sleep for a guessed duration.

**Do not fling.** A fast swipe throws the content, and it keeps moving after the gesture ends.
While it moves, a short target can pass through the visible area between two checks, and a tap
aimed at where a view was lands somewhere else. The helpers scroll in small, slow steps and pause
after each one. From `UiHelpers.kt`:

```kotlin
/**
 * A short upward swipe, relative to the view it is performed on.
 *
 * A full `swipeUp` flings through long catalogs and can skip a short button
 * between two checks. Stepping by a fifth of the viewport guarantees a target
 * below the fold becomes visible before it can pass above it.
 */
private val shortSwipeUp = GeneralSwipeAction(
    Swipe.SLOW,
    CoordinatesProvider { view -> floatArrayOf(view.width / 2f, view.height * 0.7f) },
    CoordinatesProvider { view -> floatArrayOf(view.width / 2f, view.height * 0.5f) },
    Press.FINGER,
)
```

`swipeTo` then repeats "is it visible? if not, one short swipe, wait 150 ms". Only when the
target is *completely* displayed does `openPage` tap it
(`allOf(withText(label), isCompletelyDisplayed())`).

**Espresso's `scrollTo` does not work here.** The comment on `openPage`: "The DSL scroll views
position their content with translations, so Espresso's own `scrollTo` cannot always find the
target. Swiping the root view mirrors what a user does". Swipe the root view instead.

**Espresso's `swipeDown` starts on the status bar.** It "starts at the top edge of the window,
which is the status bar and toolbar, so it never scrolls the page underneath". The helper
`contentSwipeDown` starts at 35% of the height.

**A screen you return to is where you left it.** After going back, the previous screen has kept
its scroll position, because it was never torn down (chapter 5). `tapBack` therefore scrolls to
the top again before the next step. If a test assumes a fresh screen after back, it will look
for a view that is scrolled out of sight.

**Give the keyboard a moment.** `MinimalTodoTest` calls `closeSoftKeyboard()` and then waits a
second before asserting; the layout changes while the keyboard slides away.

**Changing the appearance takes seconds on Android.** The dark mode test switches with
`cmd uimode night yes` and sleeps 2.5 seconds, because the activity is recreated (chapter 7).
It also resets the mode in a `finally` block; a test that leaves the emulator in dark mode
breaks the next one. The iOS test does the same with `defer`.

**On iOS, check hittability before tapping.** The showcase's `open` helper swipes up while
`!button.isHittable`, and `tapButton` does one extra swipe if needed. An element can exist in the
hierarchy and still be off screen.

A general rule sits under all of these: wait for a *condition*, not for a *time*, wherever the
toolkit lets you. The fixed sleeps that remain in the helpers are after gestures, where there is
no condition to wait for.

### Running them

Android, from the app's `android/` directory, with an emulator running or a device attached:

```sh
./gradlew :app:connectedDebugAndroidTest
```

iOS, from the app's `ios/` directory. Use the name of a simulator you have installed:

```sh
xcodebuild test -project KrabikUiDslTodoMinimal.xcodeproj -scheme KrabikUiDslTodoMinimal \
    -destination 'platform=iOS Simulator,name=iPhone 17'
```

Both build the Rust library first; chapter 10 explains how.

### In CI

`../../.github/workflows/ci.yml` (relative to the package root) has jobs for all three levels:
the unit tests and lints, APK and static-library builds, and two jobs named "android ui tests"
and "ios ui tests" that run the Espresso and XCUITest suites of every app on an emulator and a
simulator. One caveat has to be stated plainly: the workflow has never actually run, because the
repository's remote is not GitHub. The jobs describe the intended setup; nobody has seen them
pass or fail there. Also note that the older `docs/08-project-layout.md` still says CI "does not
run the UI tests", which predates those two jobs.

## Where a test belongs

- A rule about your data ("a blank note is not added"): level 1, on state.
- A rule about what is showing ("deleting offers Undo", "back closes the sheet first"): level 1,
  on the hooks.
- "This screen contains that text": level 1, on the tree. Sparingly.
- "The framework pops two screens in one animation": level 2. It is already there.
- "Typing in the field and pressing Add puts a row on the screen of a real phone": level 3. One
  test per main flow.

## Where this lives

- `example-todo-minimal/rust/src/lib.rs` and `example/rust/src/lib.rs`: the `tests` modules are
  models for level 1.
- `rust/src/host/session.rs`: the `tests` module with `Fake`, `FakeHost`, `FakeController` and
  `TestApp`.
- `rust/src/engine/measure.rs`: `ApproximateMeasure`.
- `example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/UiHelpers.kt`: the
  scrolling and waiting helpers quoted above.
- `shared/android-test/dev/krabik/ui/testing/ViewFinders.kt`: shared view finders.
- `example/ios/UITests/ShowcaseUITests.swift`: `open`, `tapButton`, `backToCatalog`.

Next: [10. Running on Android and iOS](10-running-on-android-and-ios.md).
