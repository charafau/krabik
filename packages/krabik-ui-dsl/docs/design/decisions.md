# Why it is built this way

This chapter lists the decisions that shape the framework. Each one says what was decided, what
problem it answers, which alternatives were on the table, and what it costs. It is the place to
look when a piece of code seems more roundabout than necessary: the reason is usually here.

The companion chapter, [What was done, in order](history.md), tells the same story as a
timeline.

## 1. The app describes, the framework draws

**Decision.** An application never touches a native view. It returns a description of the whole
screen from `view()`, and the framework works out which native views to create, change, move and
remove.

**Why.** The alternative is an imperative API ("create a label, set its text, add it to that
container"). That is what the platforms offer, twice, in two different shapes. A description is
the same on both platforms, is plain data that a unit test can inspect, and removes a whole class
of bugs where the screen and the state drift apart because one update was forgotten.

**Cost.** The framework has to find out what changed. That is the job of the tree, the layout
and the diff, and it is most of the engine. See [the big picture](../internals/01-the-big-picture.md).

## 2. Native widgets, flexbox layout in Rust

**Decision.** Widgets are the platform's own (`TextView`, `UILabel`, `UISwitch`, Material and
UIKit controls). Their position and size come from one flexbox implementation in Rust, Taffy, and
are applied as absolute frames.

**Why.** Native widgets bring the platform's look, text input, accessibility and behaviour for
free. Letting each platform also lay them out would mean two layout systems (Android's layout
passes, Auto Layout) that never agree exactly. One layout engine gives the same geometry
everywhere and lets the engine know every frame without asking the platform.

**Cost.** The engine must know how big native content wants to be. Text is measured by asking
the platform (`Measure`), and each widget states its intrinsic size by hand
(`IntrinsicSize`). When those numbers are wrong, a control is clipped or has too much room. This
is the least automatic part of adding a widget.

## 3. Typed messages instead of string actions

**Decision.** A widget carries the message to send: `Button::new("+1").on_click(Msg::Add)`.
`Msg` is the application's own enum, and `update(&mut self, msg: Msg)` handles it.

**Before.** Widgets carried a string (`.action("counter:add")`) and one `handle(event)` function
matched on strings. Rows in a list built strings with `format!("note:delete:{index}")` and parsed
them back.

**Why.** With strings, a typo is a button that does nothing, found at run time. With an enum the
compiler checks every event, a `match` tells you when a case is missing, and payloads are real
values instead of text to parse. It also makes components possible: a child has its own message
type, and the parent lifts it with `Element::map`.

**How it stays out of the engine.** `Element<M>` is generic, but nothing below it is. Each handler
is stored as `Handler<A>`, a shared closure from the event's payload (`A`) to a boxed
`dyn Any`. The box is opened again only in `AnyApp::update`, which knows the application's
message type. So the tree, the diff, the session and both renderers are compiled once, not once
per application.

**A consequence worth knowing.** Two handlers compare equal when both are set or both are unset,
whatever they would send. Closures cannot be compared, and every `view()` creates new ones, so
any other rule would make every render look like a change. The engine therefore never trusts an
old handler: it always reads handlers from the newest tree.

**Alternatives rejected.** A callback API (`on_click(|state| ...)`) needs shared mutable access
to the state from inside the view, which Rust makes awkward on purpose. Keeping strings but
generating constants would have kept the parsing.

## 4. One list of widgets, matched without a catch-all

**Decision.** Every leaf widget is a variant of one `Widget` enum. Every place that treats widgets
differently is a `match` with no `_ =>` arm.

**Before.** The description and the node tree each had their own enum with the same variants,
copied by hand, and the platform files matched on them in about a dozen places with catch-all
arms.

**Why.** One of those places had been missed: on Android, five widgets were configured when
created and then ignored every later change. A catch-all arm made that compile. Without one, a
new variant is a compile error at each place that must handle it, and the error list is the
checklist.

**Cost.** A widget cannot be added from outside the crate. That is accepted: a widget needs
native code on two platforms anyway.

## 5. One session shared by both platforms

**Decision.** Everything that is not actually platform code lives in `host/`: which hosts exist,
when to render them, how navigation, chrome, dialog and sheet are kept in step with the
application, and how a native event becomes a message. The platforms plug in through three small
traits (`Platform`, `Host`, `Controller`).

**Before.** `android.rs` and `ios.rs` each had their own session, their own event dispatch and
their own row-action bookkeeping. They had already drifted: iOS did not pass the dialog's dismiss
action, and the two re-rendered on focus changes under different rules.

**Why.** Logic that exists twice is tested half as well and diverges. Behind a trait, the same
logic runs against a fake platform in ordinary unit tests, with no emulator. Those tests cover
navigation diffing, re-showing a dialog the application still wants, and each scaffold slot.

**Cost.** One level of indirection: a platform entry point decodes its arguments and calls the
session, and the session calls back through the controller.

## 6. Rust owns the state; native chrome mirrors it

**Decision.** The navigation stack, whether a dialog is showing, whether the drawer is open: all
of it is application state. Native controllers are told what to show and report what the user
did. They never decide.

**Why.** If the native side could pop a screen by itself, Rust would have to be told afterwards
and the two could disagree in between. So on Android the fragment back stack is not used at all
(screens are fragments added and removed by hand), and on iOS the navigation bar's own back
button is replaced by one that asks Rust. The iOS swipe-back gesture cannot be replaced, so it is
armed only while the application handles back and falls under the rule below.

**The one place the platform acts first.** A user can swipe a sheet away, tap outside an alert,
let a snackbar time out, drag a drawer shut or, on iOS, swipe a screen back. The platform has already done it by the time Rust
hears. The rule for all of them is the same: record what the platform now shows, send the
application its message, render, and if the application still says the thing should be visible,
show it again.

**Widgets follow the same rule.** A switch has flipped and a slider has moved by the time Rust
hears. If `update` does not accept the value, the description is unchanged and a plain diff would
send nothing, leaving the control showing a value the state does not hold. So after a value event
the session has that one control updated from the state regardless (`Runtime::reapply`). The
update functions compare against the live native value, so this costs nothing when the
application did accept it.

**Why chrome is not in the widget tree.** The app bar, tab bar, alert, sheet, floating button,
drawer and snackbar are placed, sized and animated by the platform. They are described on the
`Scaffold` (or by `dialog()` / `sheet()`), not as children of a `Column`, because Taffy has
nothing to say about them.

## 7. Always re-run `view`, and make that cheap

**Decision.** After every message, `view()` runs again for every host. `update` has no way to
say "nothing changed".

**Why.** Any such signal becomes one more thing an application can get wrong, and the wrong
answer is a stale screen. The cost belongs in the engine, where it can be handled once: when the
rebuilt tree equals the previous one, the frames are still valid, so layout (and the native text
measurement behind it) is skipped and no operations are produced. A recycled list row does the
same against the tree it showed last.

**Measured.** Typing in a 200-row recycled form on the emulator, per keystroke, in a debug build:
event handling went from 200 to 82 microseconds and one unchanged row's rebind from 49 to 9.

**Cost that remains.** The description itself is rebuilt every time. For very large screens that
is allocation the engine cannot avoid; lists exist so that only visible rows are described.

## 8. Colours carry both appearances and are resolved in the engine

**Decision.** A `Color` has a light and a dark value. The runtime picks one while it builds the
tree. Renderers only ever see resolved colours.

**Why.** The alternative was to pass the appearance into `view()`. That adds an argument to
every component for something most views do not care about. With adaptive colours a view stays a
pure function of the state, and a change of appearance is an ordinary diff: only nodes whose
resolved colour differs are updated.

**Platform handling differs, on purpose.** iOS keeps its views and tells Rust to re-render.
Android recreates the activity, because Material chrome created under the light theme does not
re-theme itself; the Rust application is kept in a `ViewModel` across the recreation and the new
session shows its screens again without replaying their animations.

**Cost.** A view cannot branch on the appearance (for example to pick a different image). That
would need the appearance to be readable, which is not there yet.

## 9. Macros generate boilerplate, not behaviour

**Decision.** `#[derive(Widget)]` writes what every widget file used to repeat: the builder type,
its constructor, setters, event methods and message mapping. `view!` rewrites a declarative
syntax into the same builder calls a person would write.

**Why limited.** A macro that also generated platform code or intrinsic sizes would have to
understand Android and UIKit, and the errors it produced would be far from the code that caused
them. What it generates now is code you could read in a hand-written widget; widgets with an
unusual shape skip the derive and write their builders by hand.

**Why `view!` knows no widgets.** It turns `Name(args) { children }` into
`Name::new(args).child(...)`. Because it has no list of widgets, a builder an application defines
works in it without registration, and a mistake inside it is reported by the compiler against the
generated builder call.

## 10. One copy of each shell, one layout for every app

**Decision.** The Kotlin half is an Android library module (`dev.krabik.ui`), the Swift half is
one folder compiled into each app, and every app is `rust/` + `android/` + `ios/`.

**Before.** Four apps each carried their own copy of the Swift host and a 390-line Gradle file,
and the Kotlin package name was written into 26 exported function names.

**Why.** A fix in a shell file had to be made four times. The package name now appears in one
Rust macro, and an app's Gradle file is about 45 lines.

## 11. Slow work is a value returned from `update`; its result is a message

**Decision.** `update` returns a `Command`. The framework runs it on another thread and delivers
what it produces as an ordinary message. Things that keep happening (a clock) are a
`Subscription` the application returns from its state, compared by key after every message.

**Before.** `update` returned nothing and nothing could send a message into a running app, so an
application could not load data or run a timer without blocking the UI thread.

**Why a return value.** Three shapes were on the table. A context argument (`cx.spawn(..)`)
avoids writing `Command::none()` but makes forwarding to a child component awkward, because the
child needs a context of its own message type. A dispatcher handed to the app once at start is
the least framework, but every app then manages threads itself and `update` stops being a pure
function of state and message. Returning a value keeps the state change and the request it
starts in one match arm, composes with `.map` exactly like a child's view does, and leaves
`update` testable without starting anything: a command is only a description until the
framework runs it. A dispatcher still exists (`Command::dispatcher`) for work that reports more
than once or runs on the app's own async runtime.

**Why results are messages.** The state lives on the UI thread and the screen is redrawn after a
message. Making a result a message means there is one way state changes, one place to test it,
and no locking in application code.

**Why the mailbox belongs to the app.** A session can end while work runs (Android recreates the
activity on a theme change). The queue of results is kept with the application object, so the
next session finds them.

**Why subscriptions are declared, not started.** "Start the timer" and "stop the timer" are the
kind of paired calls that get out of step. Returning what should run from the current state is
the same idea as `view()`, and the framework does the comparison. They pause in the background
because a timer for a screen nobody sees costs battery for nothing.

**What it costs.** `update` ends with `Command::none()` in the common case. Messages that cross
threads must be `Send`. There is no cancellation, no thread pool and no HTTP client; see
[Commands and the mailbox](../internals/11-commands-and-the-mailbox.md).

## 12. No blur style; blur lives in the illustration

A `.blur(radius)` style was built and tried on both platforms, then removed.

**What the platforms offer.** Android 12 and later blur a view and its children with
`View.setRenderEffect`: a real Gaussian blur at the radius asked for. Older versions have nothing,
and the minimum here is Android 7. UIKit has no public blur of a view's own content. Its
`UIVisualEffectView` blurs what is *behind* it, at the fixed strength of a system material.

**What the prototype showed.** The iOS half laid an ultra-thin material over the view. With
`blur(6)` on an image, Android showed a soft image that was still recognisable; iOS showed a grey
wash in which the image could not be made out. With `blur(3)` on a text, Android blurred it and
iOS erased it. The radius had no effect on iOS. One description, two different pictures: the
opposite of what a shared style is for.

**What to use instead.** Blur in an illustration belongs in the illustration: an `.svg` asset
with an `feGaussianBlur` filter is drawn by the same renderer on both platforms (the `svg`
feature), and a `Stack` puts content over it. A frosted panel over live content (the iOS
material, on purpose) would be a widget of its own with platform-specific looks, in the spirit of
the "platform-specific extensions" item in the backlog, and not a style value.

## Limits to keep in mind

- The CI workflow has never run: the repository's remote is not GitHub. It parses, and every
  command in it was run locally in the same form, but treat its first run as a test.
- Device builds for iOS compile and link. They have not been run on a phone, and signing needs a
  team id in an untracked `Local.xcconfig`.
- iOS has no native snackbar or navigation drawer. Both are small custom views with the same
  behaviour.
- Creating a widget's native view is still one `match` per platform. It is exhaustive, so it
  cannot be forgotten, but it is one more file to touch.
- A screen's description is rebuilt after every message; see decision 7.
