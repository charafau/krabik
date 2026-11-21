# What was done, in order

This chapter is the timeline of the work on the `taffy-integration` branch: what each step
changed, why it was done at that point, and how it was checked. It is meant to be read next to
`git log`; every heading names the commit it describes, and `git show <hash> --stat` lists the
files.

The reasons behind the larger choices are collected in [Why it is built this way](decisions.md).

In numbers, from the commit before the first step to the end: 269 files changed, about 20,600
lines added and 19,500 removed. Most of that is code that moved; the framework did not double in
size.

## How every step was checked

Each step ended with the same checks before it was committed, so a later step never stood on an
unverified one:

- `cargo fmt --check`, `cargo clippy -D warnings` and `cargo test` for the library, the macro
  crate and each app crate, plus clippy for the Android and iOS targets (the renderers only
  compile there).
- All four Android apps assembled, all four iOS apps built.
- For anything that changes behaviour or wiring: the Espresso suites on a Pixel 10 emulator and
  the XCUITest suites on an iPhone 17 simulator.

Two things could not be checked and are stated where they come up: the CI workflow (the remote is
not GitHub) and running on a physical iPhone.

## Round 1: five native-UI bugs

### `5913c35` Native dialogs, sheets, tab bar, back icon, transitions

The starting point was a list of five bugs. All five had the same root: something that should be
the platform's own control was drawn by the framework instead.

- **Dialogs.** A `Dialog` was Rust-drawn content inside a window. It became a description (title,
  message, buttons with a role) shown as the system alert: `UIAlertController` on iOS, a Material
  alert dialog on Android.
- **Bottom sheets.** These became `UISheetPresentationController` on iOS and
  `BottomSheetDialogFragment` on Android. The sheet is sized to its content, which needed a new
  question for the engine, `Runtime::content_height`, because the root node always fills the
  viewport and so cannot say how tall the content is.
- **iOS bottom navigation.** It was a small Rust-drawn row. It became a real `UITabBar` owned by
  the host, pinned to the bottom edge at the system height.
- **Android back icon.** The toolbar now uses the Material arrow, tinted by the theme.
- **Android page transitions.** Screens were fragments on the fragment back stack, and a pop
  rebuilt the screen underneath, so transitions flickered. Screens are now added and removed by
  hand, a covered screen keeps its view, and a pop reveals it as it was.

This round also moved bottom navigation and the toolbar to the host level, which is where the
later `Scaffold` comes from.

## Round 2: the restructure

The bug fixes were followed by a review of the whole package. Its findings were the plan for this
round: adding a widget touched about twelve places in files of three to four thousand lines; the
Android and iOS files duplicated the session logic and had drifted apart; shell files existed four
times; and events were strings.

### `1978dec` Remove leftovers

UniFFI scaffolding, unused exports and unused Gradle dependencies were removed, and a session
leak on iOS was fixed (the Swift controller never told Rust it was gone). This went first so the
later moves carried less.

### `e000272` Android: updates go through the same path as creation

The first real bug the review found. On Android a widget was configured in one function when
created and in another when updated, and the second did not handle RadioGroup, TabBar, Pager,
DatePicker or Spinner. They ignored every change after creation. Creation now builds the view and
calls the same `configure_view` an update calls, so the two cannot differ.

### `f0f38f4` One module per widget, one `Widget` enum

`ui.rs` held every widget. It became `widgets/`, one file per widget with its properties, builder
and intrinsic size. The two parallel enums (description and node) became one `Widget` enum used by
both, and the catch-all match arms were removed, so the compiler now lists every place a new
widget must be handled.

### `c9d4d56` One session and one event path

The session moved out of the two platform files into `host/`, behind the `Platform`, `Host` and
`Controller` traits. Event resolution (which handler does this native event belong to) moved with
it. From this commit on, navigation and presentation logic is unit-tested against a fake
platform.

### `4408bdf` Typed messages and components

The one deliberately breaking change. String actions and `App::handle` were replaced by
`Component { type Msg; update; view }`, typed event methods on widgets, and `Element::map`. All
four apps were ported; the showcase's checklist became a child component to prove composition
works.

### `da47117` Split the platform renderers

`android.rs` (about 2,600 lines) and `ios.rs` (about 3,600) became directories of focused modules:
entry points, host, rows, style, measurement, controller, and one file per widget. Nothing was
redesigned in this step; it is a move, which is why it is its own commit.

### `5b81899` The Kotlin becomes a library module

The shared Kotlin sources were compiled into each app through a source-set trick, under the
example's package name. They became the `dev.krabik.ui` Android library, and the JNI function
names are generated from one macro instead of being written out with the package in each. An
app's Gradle file went from about 390 lines to about 45.

### `c976d12` One copy of each shell file

The Swift host existed once per app; it moved to `shared/ios`. The demo gallery that was compiled
into the library moved out into its own app, `harness/`, so the library ships no application.
Every app got the same three folders, and iOS projects are generated from a `project.yml`.

### `42a81b2` Android: rotation, predictive back, edge to edge

Three gaps in the Android shell. Rotation used to recreate the activity and break navigation; the
manifest now handles those configuration changes and Rust is only told the new size. The back
callback is enabled only while the application says it handles back, so the system's own
predictive animation works at the root. And the app draws behind the system bars.

### `59c4181` First guide

Eight chapters of documentation, since replaced by the documents you are reading.

## Round 3: follow-ups

The restructure left a list of things it knowingly did not do. This round did them, and added the
`Scaffold`.

### `83b3c61` Skip layout when nothing changed

Every message re-runs `view`, and every render made recycled lists rebind their visible rows. Both
laid the result out again even when it was identical. The runtime now compares the rebuilt tree
with the current one and keeps the frames when they are equal; a recycled row does the same.

Two things planned here were deliberately not done. Replacing `notifyItemRangeChanged` with a
direct rebind would have left `RecyclerView`'s off-screen view cache showing stale rows, so the
notification stayed and the saving comes from the Rust side. And `update` did not gain a "nothing
changed" return value; see decision 7.

### `c88d702` Each widget owns its platform code

The follow-through on "one module per widget" for the platforms. Each widget's module now has its
`create` next to its `update`, and on Android its cached JNI handles too, instead of one struct
with 112 fields. Calls to the Kotlin host controller go through cached method ids like the widget
calls. In the core, every widget's properties implement one trait, so the `Widget` enum forwards
instead of growing an arm per method.

### `5c99b69` Proc macros

A new crate with `#[derive(Widget)]` for the builder boilerplate and `view!` for writing trees
declaratively. Seven widgets use the derive, which removed about 480 lines from their files; the
`Widget` enum itself is declared by a small table macro. The minimal todo example was ported to
`view!` as the reference.

One change from the plan: an expression inside `view!` is written bare (`row(item)`), not in
braces, because `Text("A") {x}` would be ambiguous with "a `Text` that has children".

### `fd7e2b5` Dark theme

`Color` with a light and a dark value, resolved by the engine; DayNight themes on Android with the
application retained across the activity's recreation; system background colours and an appearance
callback on iOS. The default text colour, which was a fixed black, became adaptive. All four apps
got adaptive palettes, and both platforms got a test that switches appearance and checks the
screen and its state survive.

### `db4639f` Tests and builds

CI jobs that run every UI suite, a device-target build, and the macro crate added to the checks.
A rotation test. The view finders that four test classes had each copied moved to one shared
folder. iOS projects build for a device as well as a simulator, with the Rust target chosen from
what Xcode is building for.

Running the suites this often surfaced a flaky test: it tapped a button while a dismissed sheet's
window was still animating out, and the tap landed on that window. The test now waits; the
framework was behaving correctly.

### Scaffold

`App::screen` now returns a `Scaffold`: the body plus the native chrome around it. The toolbar
and bottom navigation that used to be configured on the navigation stack moved onto it, and three
slots were added: a floating action button, a navigation drawer and a snackbar. `Navigation`
shrank to a list of routes.

Android uses the Material controls for all three. iOS has a filled capsule button for the first
and small custom views for the other two, because UIKit has neither a snackbar nor a drawer.

## After the documentation: state, navigation, background work

Writing the documentation produced a list of rough edges. The next round worked through it,
starting with the ones an application would hit first.

### `7dff7dc` A control the application overrules returns to its state

A switch flips and a slider moves before Rust hears of it. If `update` did not accept the value,
the description was unchanged, nothing was sent, and the control went on showing something the
state did not hold. After a value event the session now has that one control updated regardless.

The new tests found two bugs that had nothing to do with the change. Neighbouring rows of a
recycled list shared a root id (`row_root` forced the lowest bit, so rows 0 and 1 collided), and
an event in one row was delivered to the row beside it. The old behaviour had hidden it: the
wrong row's state changed, the right row's field kept what was typed, and the screen looked
correct. And on iOS a text field with an accessibility label was no longer exposed as a text
field.

### `fce62ff` Back in one line, swipe-back, chrome without a stack

`Navigation::on_back` names the back message so an app with a stack does not have to implement
`App::back`. The iOS swipe-back gesture, which had been switched off because UIKit completes it
by itself, is enabled while the application handles back and reconciled afterwards with the same
rule as a dismissed dialog. `App::scaffold` gives a single-screen app native chrome. A stack with
a repeated key is refused.

### Background work

`update` returns a `Command`; results come back as messages through a mailbox owned by the
application; `App::subscriptions` declares timers. See decision 11 and
[Commands and the mailbox](../internals/11-commands-and-the-mailbox.md). Every `update` in the
four apps and in this documentation gained a return type.

One failure during this work was the test's, not the framework's: right after an activity
returns to the foreground, a tap can still go to the window that covered it. The test repeats
the tap until the switch has taken it.

### Safety nets

Things that used to go wrong silently now fail a test or a build. The chrome event numbers and
the transition duration, which Kotlin, Swift and the C header repeat, are compared with Rust's
by a test that reads those files. Both controllers destructure `Chrome`, so a new slot does not
compile until each mentions it. A failed JNI lookup names what it was looking for in logcat and
the app stops with a message, where it used to show an empty screen. The derive resolves a
colour however its type is written and inside `Option` and `Vec`. Duplicate keys are caught for
list rows too, and reported once in release builds. On iOS a new view gets its properties from
the same function that later updates them.

The largest part was the row binding of recycled lists. Android and iOS each had a copy of the
logic that decides what a row needs; it is now one `RowBinder` in `rust/src/host/rows.rs` with
host tests, and the platforms only apply its operations.

Two things turned up while testing. The duplicate-key check failed one of the framework's own
tests, which had been building a list with repeated keys all along. And under load a text field
that had just rejected a character lost the next one typed: resetting the text with `setText`
restarts the keyboard's input connection. The field's text is now edited in place.

### Build

The Gradle task that builds the Rust library now counts the macro crate among its inputs, writes
debug and release libraries to separate folders, and the Kotlin library ships keep rules for
minified builds. `jni` became an Android-only dependency of the framework. The keep rules were
checked by driving a minified build of the showcase by hand, because the instrumented tests
cannot run against a minified app.

### Parity and behaviour

The remaining rough edges, platform by platform. On iOS the back button shows the app bar's
back title, tab-bar labels take their size, weight and colour, a view's corners can have
different radii, and a sheet is resized when its content changes height. On Android an alert
with more choices than button slots lists all of them. In the engine a disabled container
disables everything in it, a container with an accessibility label or an animation gets a view,
a list is windowed for its own height instead of the whole screen, and a list whose content is
replaced tells the native scroll view to go back to the top. `Snackbar::id` lets the same
message be shown again.

Looking at the result on a simulator showed one more iOS bug, older than this work: a view
whose corner radius was larger than half its height (a pill written as `corner_radius(999)`) was
not drawn at all. The radius is now limited to what fits the view each time it is laid out.

Two things were left as they are, with the reason recorded in `TODO.md`: `UIDatePicker` has no
API for the size, weight or colour of its text, and a UIKit alert cannot be dismissed by tapping
outside it.

## Gaps an application rewrite found

Rewriting a real application on the framework produced a list of things it had to work around
(`MISSING.md`). They were closed one commit each: `d6a75a8` `App::init` returns the command an
application starts with; `571ed47` `Container`, a box that can be tapped; `a9a1249` and `5b92fd1`
the application is told the device's language and appearance and can choose light or dark;
`1753175` pull to refresh; `cddf0af` `Stack`, and SVG assets drawn in Rust. A blur style was
prototyped and removed (`536eb95`, decision 12).

## Hot reload and the command line

### `59d0350`, `ca01df8` Hot reload

A rebuilt application library is loaded into the running app, which hands its state over to the
new code. The plumbing was first proven on the host with a fake platform (`59d0350`) and then on
the Android emulator and the iOS simulator (`ca01df8`), driven by a shell script. The design, the
measurements and the limits are in [`hot-reload.md`](hot-reload.md).

### The `krabik` CLI learns krabik-ui-dsl projects

The CLI in `packages/krabik-cli` only knew projects with UniFFI bindings. It now has three things
for an app of this framework. `krabik create <name> --with-dsl` writes the app's crate and the
two shells from a template; the shells point at a krabik checkout by absolute path, because the
framework is not published. `krabik run` lets Gradle or Xcode build the Rust library, as
decision 10 arranged, instead of building it itself. `krabik watch` replaced the shell script:
when the app's crate enables the `hot-reload` feature it rebuilds the crate on every save and
delivers the library to the running app. It was tried end to end with a new project on the
emulator (state kept across reloads, a broken build survived) and on the simulator.

## What is still open

- Run the CI workflow for the first time and fix whatever the runner disagrees with.
- Run on a physical iPhone.
- Publish the framework, so that an app does not depend on the location of a checkout, and
  support release builds of such an app in the CLI.
- The gaps next to Flutter, ranked, are in [`MISSING.md`](../../MISSING.md).
- The remaining backlog is in [`TODO.md`](../../TODO.md).
