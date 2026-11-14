# Add a chrome slot

This recipe adds a new piece of host-level chrome: something native that sits around a screen's
content and is not part of the widget tree, like the app bar or the floating action button. It
walks through the real code of the floating action button (the simple kind) and of the snackbar
(the kind the platform can dismiss on its own), so every excerpt below exists in the repository.
Afterwards you will know which files a slot touches, which of the two patterns your slot needs,
and where the compiler stops helping.

Background that this recipe does not repeat: why chrome is host-level at all, and what a
`Session`, a `Host` and a `Controller` are, is in [the engine chapters](../internals/README.md)
and [the platform chapters](../platforms/README.md). A one-line reminder of each appears where it
is first used.

## Chrome is not a widget

A widget is a node in the tree. Rust lays it out and gives it a frame. Chrome is different: the
platform places and animates it. A toolbar belongs to the window, a floating button floats above
whatever screen is on top, a snackbar slides in over everything. So chrome is described per
screen, next to the content, and carried out by one native object per session, the *controller*
(`KrabikHostController` in Kotlin and in Swift).

The application returns a `Scaffold` from `App::screen`. A scaffold is the body plus one optional
field per slot:

```rust,ignore
// rust/src/scaffold.rs
pub struct Scaffold<M = ()> {
    /// The native bar at the top, if the screen has one.
    pub app_bar: Option<AppBar<M>>,
    /// The content of the screen.
    pub body: Element<M>,
    /// The native bottom navigation bar, if any.
    pub bottom_navigation: Option<BottomNavigation<M>>,
    /// The floating action button, if any.
    pub floating_action_button: Option<Fab<M>>,
    /// The navigation drawer, if any.
    pub drawer: Option<Drawer<M>>,
    /// The snackbar being shown, if any.
    pub snackbar: Option<Snackbar<M>>,
}
```

The flow for every slot:

```text
 App::screen(key) -> Scaffold<Msg>
        |  Scaffold::erase()            (rust/src/scaffold.rs)
        v
 (RawElement body, Chrome)              message type erased
        |                 |
        v                 v
   Host::render      Session::sync_chrome   (rust/src/host/session.rs)
   (widget tree)          |  compare with what the controller last got
                          v
                 Controller::set_chrome(&Chrome)      (trait, host/platform.rs)
                    |                          |
     android/controller.rs               ios/controller.rs
     JNI -> KrabikHostController.kt      msg_send -> KrabikHostController.swift

 user taps the native thing
        |
        v
 RustRootView.onChromeEvent(kind, index)   /   krabik_ios_on_chrome_event(...)
        |
        v
 ChromeEvent::from_code  ->  Session::chrome_event  ->  app.update(message)
```

## The two patterns

Decide which one your slot is before you start. The difference is whether the platform can change
what is shown *without Rust asking*.

**Plain configuration.** The app bar, the bottom navigation bar and the floating action button.
Rust says what they look like; they stay that way until Rust says otherwise. The user can press
them, which sends a message, but pressing does not make them appear, disappear or change. The
session only needs to know "did the description change since I last sent it".

**Described by state, dismissible natively.** The drawer, the snackbar, the dialog and the bottom
sheet. The scaffold (or `App::dialog`, `App::sheet`) contains them *while they should be visible*.
But the user can also get rid of them without the application: swipe the drawer shut, let the
snackbar time out, tap outside the alert, drag the sheet down. Now the platform shows something
different from what the application last described, and the session must know that, or it will
compare the application's next description with a stale memory and conclude that nothing needs to
be sent.

The rule for the second kind, used four times in `rust/src/host/session.rs`:

> Remember what the platform shows. When the platform changes it by itself, update that memory
> first, then tell the application, then sync. If the application still disagrees, the sync
> re-applies the application's version.

The rest of this recipe does the first pattern as a checklist and then shows what the second one
adds.

## Pattern 1: a configuration slot, step by step

The model is the floating action button. Replace `Fab` and `floating_action_button` with your
slot's names.

| # | File | What you add |
|---|---|---|
| 1 | `rust/src/scaffold.rs` | the typed struct, its `erase`, the field in `Scaffold` and in `Chrome` |
| 2 | `rust/src/host/platform.rs` | a `ChromeEvent` variant and its code |
| 3 | `rust/src/host/session.rs` | an arm in `chrome_event`; the fake controller and a test |
| 4 | `rust/src/android/controller.rs` | a method handle and the call inside `set_chrome` |
| 5 | `shared/android/.../KrabikHostController.kt` | the view parameter, the setter, the event constant |
| 6 | `shared/android/.../KrabikActivity.kt` | create the native view and hand it to the controller |
| 7 | `rust/src/ios/controller.rs` | the `msg_send!` inside `set_chrome` |
| 8 | `shared/ios/KrabikHostController.swift` | the view, the `@objc` setter, the event constant |
| 9 | `shared/ios/KrabikIOS.h` | the new code in the comment of `krabik_ios_on_chrome_event` |
| 10 | example app and UI tests | a page that shows it, an assertion per platform |

### Step 1: describe it (`rust/src/scaffold.rs`)

A slot is a struct that is generic over the application's message type only through a marker:

```rust,ignore
// rust/src/scaffold.rs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fab<M = ()> {
    /// The text on the button, also its accessibility label.
    pub label: String,
    /// The optional icon shown before the label.
    pub icon: Icon,
    /// What to tell the application when the button is pressed.
    pub on_click: Handler<()>,
    marker: PhantomData<fn() -> M>,
}
```

`Handler<()>` is already type-erased: it stores a function that produces a boxed message
(`rust/src/message.rs`). The `M` exists so that the builder methods can check the message type at
compile time:

```rust,ignore
// rust/src/scaffold.rs
    /// Sends `message` when the button is pressed.
    pub fn on_click(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_click = Handler::new(move |()| message.clone());
        self
    }

    fn erase(self) -> Fab {
        Fab {
            label: self.label,
            icon: self.icon,
            on_click: self.on_click,
            marker: PhantomData,
        }
    }
```

`erase` turns `Fab<Msg>` into `Fab<()>` by rebuilding the struct with a new marker. Nothing else
changes; the handlers inside already produce erased messages. This is the point where the
application's type stops, so the session and both controllers are not generic.

Then four mechanical additions in the same file: the field in `Scaffold` (shown above), `None` in
`Scaffold::new`, a setter that accepts an `Option` so state can decide,

```rust,ignore
// rust/src/scaffold.rs
    pub fn floating_action_button(mut self, button: impl Into<Option<Fab<M>>>) -> Self {
        self.floating_action_button = button.into();
        self
    }
```

and the erased field in `Chrome` together with its line in `Scaffold::erase`:

```rust,ignore
// rust/src/scaffold.rs
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Chrome {
    pub app_bar: Option<AppBar>,
    pub bottom_navigation: Option<BottomNavigation>,
    pub floating_action_button: Option<Fab>,
    pub drawer: Option<Drawer>,
    pub snackbar: Option<Snackbar>,
}
```

Two derives on `Chrome` carry weight:

- `PartialEq`. The session decides whether to resend the chrome with `chrome != self.chrome`.
  Handlers compare equal "when both are set or both are unset" (`impl PartialEq for Handler` in
  `rust/src/message.rs`), so a rebuilt closure does not count as a change, and every other field
  you add does. Put only what the native side needs to draw into the struct.
- `Default`. An empty chrome means "hide everything". Screens without navigation and screens that
  return content alone get it.

**Compiler:** adding the field to `Chrome` without the line in `Scaffold::erase` gives `missing
field ... in initializer of Chrome` (E0063). Adding it to `Scaffold` without `Scaffold::new`
gives the same for `Scaffold`. The existing tests in the `tests` module at the bottom of the file
(`every_slot_keeps_its_configuration_and_messages`) are where you add assertions for the new slot.

### Step 2: name the event (`rust/src/host/platform.rs`)

If the user can act on your slot, it needs a `ChromeEvent`. All chrome events share one entry
point per platform and are told apart by a number:

```rust,ignore
// rust/src/host/platform.rs
pub(crate) enum ChromeEvent {
    /// The floating action button was pressed.
    Fab,
    /// A drawer destination was picked.
    DrawerSelected(usize),
    // ...
}

impl ChromeEvent {
    /// Decodes the event from the two numbers the platforms report.
    pub(crate) fn from_code(kind: i64, index: i64) -> Option<Self> {
        Some(match kind {
            0 => Self::Fab,
            1 => Self::DrawerSelected(usize::try_from(index).ok()?),
            2 => Self::DrawerOpened,
            3 => Self::DrawerClosed,
            4 => Self::SnackbarAction,
            5 => Self::SnackbarDismissed,
            _ => return None,
        })
    }
}
```

Add a variant and the next free number (6 today). The second argument, `index`, is there for
events that need one; the others pass 0.

Because the event path is shared, you add **no** JNI entry point, no `external fun`, and no C
function. `nativeOnChromeEvent` in `rust/src/android/entry.rs`, `RustRootView.onChromeEvent` in
Kotlin and `krabik_ios_on_chrome_event` in `rust/src/ios/entry.rs` already exist and already
decode through `from_code`. This is the main saving compared with a widget.

The `Controller` trait in the same file does not change for a configuration slot.
`set_chrome(&self, cx, chrome: &Chrome)` receives the whole struct. Do update its doc comment,
which lists what the call applies.

**Compiler:** adding the variant breaks `Session::chrome_event`, which matches on it exhaustively.
Forgetting the `from_code` line compiles; see the last section.

### Step 3: route it (`rust/src/host/session.rs`)

Reminder: a `Session` owns the application, one host per native root view, and mirrors the
application's navigation and chrome onto the controller.

Sending needs no new code. `sync_chrome` compares and sends the whole `Chrome`:

```rust,ignore
// rust/src/host/session.rs
    fn sync_chrome(&mut self, cx: &mut P::Cx<'_>, navigated: bool) -> Result<(), P::Error> {
        let Some(controller) = self.controller.as_ref() else {
            return Ok(());
        };
        let mut chrome = self.top_chrome();
        let snackbar = chrome.snackbar.take();
        if navigated || chrome != self.chrome {
            controller.set_chrome(cx, &chrome)?;
            self.chrome = chrome;
        }
        // ... the snackbar, see pattern 2
    }
```

`top_chrome` asks the application for the scaffold of the screen on top of the stack. Chrome
always belongs to the top screen; the test `chrome_belongs_to_the_top_screen` pins that down.
Note `navigated ||`: after any navigation change the chrome is resent even when equal, "so the
native bars never keep a previous screen's state".

Receiving is one arm:

```rust,ignore
// rust/src/host/session.rs, in `chrome_event`
        // Handlers are read from the application's current state.
        let chrome = self.top_chrome();
        let message = match event {
            ChromeEvent::Fab => chrome
                .floating_action_button
                .and_then(|fab| fab.on_click.message(())),
            // ...
        };
        if let Some(message) = message {
            self.app.update(message);
        }
        self.render_all(cx)
```

The handler is looked up in a *fresh* `top_chrome()`, not in `self.chrome`. `self.chrome` is the
copy last sent to the platform and may hold handlers from an older render. The same comment
appears in `toolbar_action`: "Handlers are read from the application's current state, never from
the copy last sent to the platform."

Now the test support, in the `tests` module of the same file. Reminder: the session is generic
over a `Platform`, and the tests plug in a fake one whose controller writes every call into a log
of strings. Three small additions:

1. `FakeController::set_chrome` appends a marker for the slot to its log line:

   ```rust,ignore
   // rust/src/host/session.rs, tests
               if let Some(fab) = &chrome.floating_action_button {
                   line += &format!(" fab:{}", fab.label);
               }
   ```

2. The test application's `State` gets a field (`fab: bool`), its `Msg` a variant (`Fab`), its
   `update` an arm, and its `screen` builds the slot from the field:

   ```rust,ignore
   // rust/src/host/session.rs, tests
                       .floating_action_button(
                           (state.fab && top).then(|| Fab::new("Add").on_click(Msg::Fab)),
                       )
   ```

3. A test that drives it. This is the whole one for the button:

   ```rust,ignore
   // rust/src/host/session.rs, tests
       #[test]
       fn the_floating_action_button_follows_state_and_reports_a_press() {
           let (mut session, app, log) = on_home(State::default());

           app.state().fab = true;
           session.render_all(&mut ()).unwrap();
           assert_eq!(take(&log), ["chrome HOME fab:Add"]);

           session.chrome_event(&mut (), ChromeEvent::Fab).unwrap();
           assert_eq!(app.state().clicks, 10);
           // The button itself did not change, so nothing is resent.
           assert!(take(&log).is_empty());

           app.state().fab = false;
           session.render_all(&mut ()).unwrap();
           assert_eq!(take(&log), ["chrome HOME"]);
       }
   ```

It checks the three things a configuration slot must do: appear when state says so, report a
press as the application's message without being resent, and disappear when state says so. The
helpers (`on_home`, `take`, `app.state()`) are explained in
[Write tests](write-tests.md#level-3-session-tests-with-the-fake-platform).

**Compiler:** `cargo check --manifest-path rust/Cargo.toml` passes and the host tests run. Neither
platform knows the slot yet, and both still compile, because they read only the `Chrome` fields
they know about.

```sh
cargo test --manifest-path rust/Cargo.toml the_floating_action_button
```

### Step 4: send it on Android (`rust/src/android/controller.rs`)

The Android controller is a Kotlin object reached through cached JNI method ids. Add one handle
to the `Handles` struct at the bottom of the file and resolve it in `Handles::load`:

```rust,ignore
// rust/src/android/controller.rs
    pub(crate) set_fab: Method,
    // ...
            set_fab: method!(
                env,
                &class,
                "setFab",
                "(Ljava/lang/String;Ljava/lang/String;Z)V",
                VOID
            ),
```

and make the call inside `set_chrome`:

```rust,ignore
// rust/src/android/controller.rs, in `set_chrome`
        let fab = chrome.floating_action_button.as_ref();
        let label = jcall!(
            env,
            env.new_string(fab.map_or("", |fab| fab.label.as_str()))
        )?;
        let icon = jcall!(
            env,
            env.new_string(fab.and_then(|fab| fab.icon.android_name()).unwrap_or(""))
        )?;
        ids.set_fab.call_void(
            env,
            self.0.as_obj(),
            &[
                JValue::Object(label.as_ref()),
                JValue::Object(icon.as_ref()),
                JValue::Bool(fab.is_some()),
            ],
        )?;
```

The convention across this file: the call is made on every `set_chrome`, present or not, and
absence is encoded in the arguments (`visible = false`, an empty array, `-1`). The native side
then always ends in the state Rust described, including "hidden". A slot that is only sent when
`Some` would stay on screen after the user navigates to a screen without it.

Only flat data crosses: strings, numbers, booleans, arrays of strings (the `string_array!` macro
in `rust/src/android/mod.rs`). Handlers never cross. The native side reports "the button was
pressed" and Rust finds the handler.

### Step 5: carry it out in Kotlin (`KrabikHostController.kt`)

Three additions. The view arrives through the constructor:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt
    /** The floating action button, above the bottom corner of the content. */
    private val fab: ExtendedFloatingActionButton,
```

the `init` block wires its listener to the shared event method:

```kotlin
        fab.visibility = View.GONE
        fab.setOnClickListener { main.onChromeEvent(EVENT_FAB) }
```

and the setter Rust calls applies the description:

```kotlin
    /** Applies the top screen's floating action button; hidden when not [visible]. */
    fun setFab(label: String, icon: String, visible: Boolean) {
        fab.text = label
        fab.contentDescription = label
        val resource = if (icon.isNotEmpty()) {
            activity.resources.getIdentifier(icon, "drawable", activity.packageName)
        } else {
            0
        }
        if (resource != 0) fab.setIconResource(resource) else fab.icon = null
        fab.visibility = if (visible) View.VISIBLE else View.GONE
    }
```

The event number is a constant in the companion object, under a comment that says where its twin
lives:

```kotlin
        // What `RustRootView.onChromeEvent` reports; Rust decodes the same
        // numbers in `ChromeEvent::from_code`.
        const val EVENT_FAB = 0
```

### Step 6: create the view (`KrabikActivity.kt`)

The controller does not build the window layout; the activity does, and passes the views in. For
the button that is one constructor call, one `addView` into the `content` frame (above both
containers, bottom end, with a margin), and one argument in the `KrabikHostController(...)` call:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt
        val fab = ExtendedFloatingActionButton(this)
        // ...
            // Above both, in the bottom corner the screen's main action lives in.
            val margin = (16 * resources.displayMetrics.density).toInt()
            addView(
                fab,
                FrameLayout.LayoutParams(
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    FrameLayout.LayoutParams.WRAP_CONTENT,
                    Gravity.BOTTOM or Gravity.END,
                ).apply { setMargins(margin, margin, margin, margin) },
            )
```

Where the view goes in that layout is the real design work of a slot: what it overlaps, what it
pushes, how it interacts with the system bars and with the other slots. The snackbar, for
example, anchors itself to the button when the button is visible (`showSnackbar` in the
controller).

### Step 7: send it on iOS (`rust/src/ios/controller.rs`)

The iOS controller is a Swift object reached by Objective-C message send. No handle to cache:

```rust,ignore
// rust/src/ios/controller.rs, in `set_chrome`
        let fab = chrome.floating_action_button.as_ref();
        let label = NSString::from_str(fab.map_or("", |fab| fab.label.as_str()));
        let icon = NSString::from_str(fab.and_then(|fab| fab.icon.ios_name()).unwrap_or(""));
        // SAFETY: see `controller_push`.
        let _: () = unsafe {
            objc2::msg_send![
                &*self.0,
                krabikSetFab: &*label,
                icon: &*icon,
                visible: fab.is_some()
            ]
        };
```

The selector is assembled from the labels: `krabikSetFab:icon:visible:`.

### Step 8: carry it out in Swift (`KrabikHostController.swift`)

The mirror image of step 5 and 6 in one file, because on iOS the controller also owns the view
hierarchy:

```swift
// shared/ios/KrabikHostController.swift
    /// The floating action button, a filled capsule above the bottom corner.
    private let fab = UIButton(configuration: .filled())

    /// Applies the top screen's floating action button; hidden when not `visible`.
    @objc func krabikSetFab(_ label: String, icon: String, visible: Bool) {
        fab.configuration?.title = label
        fab.configuration?.image = icon.isEmpty ? nil : toolbarImage(icon)
        fab.accessibilityLabel = label
        fab.isHidden = !visible
    }

    @objc private func fabTapped() {
        krabik_ios_on_chrome_event(session, Self.eventFab, 0)
    }
```

plus `installFloatingActionButton()` (styling, constraints, `addTarget`) and the constant
`private static let eventFab: Int64 = 0`. The Swift signature
`krabikSetFab(_ label: String, icon: String, visible: Bool)` with `@objc` is what produces the
selector Rust sends.

iOS has no floating action button control, so this one is a styled `UIButton`. The drawer and the
snackbar are small custom views for the same reason (`shared/ios/KrabikChrome.swift`). If your
slot has no UIKit equivalent, that file is where its view goes.

### Step 9: the header (`shared/ios/KrabikIOS.h`)

The C function already exists. Its comment enumerates the codes; extend it:

```c
/// Reports what the user did to the floating action button (kind 0), the
/// drawer (1 picked `index`, 2 opened, 3 closed) or the snackbar (4 action
/// pressed, 5 timed out or swiped away).
void krabik_ios_on_chrome_event(uint64_t handle, int64_t kind, int64_t index);
```

### Step 10: show and test it

Add the slot to the "Scaffold" page of the example (`scaffold_page` and the screen's scaffold in
`example/rust`), then extend the two UI tests named `scaffoldFloatingButtonSnackbarAndDrawer`
(Espresso) and `testScaffoldFloatingButtonSnackbarAndDrawer` (XCUITest). Both end with the check
that matters most for chrome, that leaving the screen removes it:

```kotlin
// example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/ShowcaseTest.kt
            // Leaving the screen takes its chrome with it.
            tapBack()
            onView(withText("Krabik UI DSL")).check(matches(isDisplayed()))
            onView(withText("Add")).check(doesNotExist())
```

## Pattern 2: what a dismissible slot adds

Take the snackbar. The application shows it by returning `Some(Snackbar)` in the scaffold. Android
removes it after a few seconds on its own. Three things are added on top of pattern 1.

### A memory of what the platform shows

```rust,ignore
// rust/src/host/session.rs, in `struct Session`
    /// The top screen's chrome as the controller last received it, without
    /// the snackbar.
    chrome: Chrome,
    /// The snackbar the controller is showing.
    snackbar: Option<Snackbar>,
    /// The alert the controller is showing.
    dialog: Option<Dialog>,
```

The snackbar is kept apart from the rest of the chrome on purpose. `set_chrome` is resent after
every navigation change even when nothing differs; resending a snackbar would restart its timer
and its animation. So it has its own pair of controller methods and its own comparison:

```rust,ignore
// rust/src/host/platform.rs, in `trait Controller`
    /// Shows a snackbar, replacing any that is up.
    fn show_snackbar(&self, cx: &mut P::Cx<'_>, snackbar: &Snackbar) -> Result<(), P::Error>;
    /// Removes the snackbar without reporting back.
    fn dismiss_snackbar(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
```

```rust,ignore
// rust/src/host/session.rs, the rest of `sync_chrome`
        if snackbar != self.snackbar {
            match &snackbar {
                Some(snackbar) => controller.show_snackbar(cx, snackbar)?,
                None => controller.dismiss_snackbar(cx)?,
            }
            self.snackbar = snackbar;
        }
```

**Compiler:** new methods on the `Controller` trait must be implemented by all three controllers:
`AndroidController`, `IosController` and the test's `FakeController`. A missing one is `not all
trait items implemented` (E0046). This is the one place where the compiler does force both
platforms to keep up, and a reason to prefer a dedicated method over a new argument to an
existing Kotlin setter when the behaviour is distinct.

### Correcting the memory when the platform acts

```rust,ignore
// rust/src/host/session.rs, in `chrome_event`
            ChromeEvent::SnackbarAction | ChromeEvent::SnackbarDismissed => {
                // The platform removed the snackbar; if the application keeps
                // returning it, the sync below shows it again.
                self.snackbar = None;
                chrome.snackbar.and_then(|snackbar| {
                    let handler = if event == ChromeEvent::SnackbarAction {
                        snackbar.on_action
                    } else {
                        snackbar.on_dismiss
                    };
                    handler.message(())
                })
            }
```

Follow the order. First `self.snackbar = None`: the platform no longer shows one, so the memory
says so. Then the application gets its message and usually clears its own state. Then
`render_all` runs `sync_chrome`:

- The application now returns `None`. Memory is `None`. Equal, nothing is sent. In particular
  `dismiss_snackbar` is *not* called for a snackbar that is already gone.
- The application still returns `Some`. Memory is `None`. Different, so `show_snackbar` is called
  and the snackbar comes back. The application's state wins.

Without the first line, the second case would compare `Some` with a stale `Some`, send nothing,
and leave the screen without a snackbar that the application believes is showing.

The drawer follows the same rule with a field instead of an option, because a drawer can be
present and closed:

```rust,ignore
// rust/src/host/session.rs, in `chrome_event`
            ChromeEvent::DrawerOpened | ChromeEvent::DrawerClosed => {
                // The platform changed the drawer by itself. Remember what it
                // shows now, so the sync below corrects it if the application
                // still says otherwise.
                let open = event == ChromeEvent::DrawerOpened;
                if let Some(shown) = &mut self.chrome.drawer {
                    shown.open = open;
                }
                // ... then the on_open / on_close handler
            }
```

The dialog (`dialog_closed`: `self.dialog = None;` first) and the sheet (`sheet_dismissed`:
`self.forget_sheet();` first) are the same idea. Their doc comments say it in one sentence each:
"The platform has already closed the alert, so the session forgets it first: if the application
still returns a dialog, it is shown again."

### A native side that reports only what the user did

The rule has a second half that lives in Kotlin and Swift. "Dismiss without reporting back" is in
the doc comment of every `dismiss_*` method on the trait. If a dismissal that Rust asked for were
reported as a user dismissal, the application would receive `on_dismiss` for something it did
itself, and a replaced snackbar would report the dismissal of its predecessor.

The Android snackbar tells the two apart by identity and by the reason code:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt, in showSnackbar
            override fun onDismissed(transientBottomBar: Snackbar, event: Int) {
                if (snackbar !== transientBottomBar) {
                    // Replaced or taken down by Rust; nothing to report.
                    return
                }
                snackbar = null
                if (event == DISMISS_EVENT_TIMEOUT || event == DISMISS_EVENT_SWIPE) {
                    main.onChromeEvent(EVENT_SNACKBAR_DISMISSED)
                }
            }
```

and `dismissSnackbar` clears the `snackbar` field *before* calling `bar.dismiss()`, so the
callback sees a different object and stays quiet. The drawer listener uses a flag that Rust's
`setDrawer` sets:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt, in init
            // Only a change the user made is reported; one Rust asked for is
            // already known there.
            override fun onDrawerOpened(drawerView: View) {
                if (!drawerOpen) {
                    drawerOpen = true
                    main.onChromeEvent(EVENT_DRAWER_OPENED)
                }
            }
```

On iOS the custom views make this simpler: `krabikDismissSnackbar` calls
`removeSnackbar(animated: false)` and reports nothing, while `snackbarLeftByItself` removes and
then reports.

### The tests that pin the rule down

For a dismissible slot write both outcomes. They exist for the snackbar:

```rust,ignore
// rust/src/host/session.rs, tests
    #[test]
    fn a_snackbar_the_application_keeps_is_shown_again() {
        let (mut session, app, log) = on_home(State {
            snackbar: true,
            sticky_snackbar: true,
            ..State::default()
        });

        // The platform took it down, but the application still returns it.
        session
            .chrome_event(&mut (), ChromeEvent::SnackbarDismissed)
            .unwrap();
        assert!(app.state().snackbar);
        assert_eq!(take(&log), ["snackbar Deleted"]);
    }
```

and, inside `a_snackbar_is_shown_once_and_can_be_dismissed_either_way`, the opposite: after a
timeout that the application accepts, `assert!(take(&log).is_empty());`. The "sticky" flag in the
test application's state exists only to produce the disagreeing case. Add one for your slot.

## What the compiler will not tell you

**A controller that never sends the slot.** `set_chrome` receives the whole `Chrome` and each
platform reads the fields it knows. If you add the field and forget step 4 or step 7, everything
compiles and the slot never appears on that platform. Only a UI test on that platform notices.
The fake controller has the same gap: if `FakeController::set_chrome` does not log the slot, a
session test cannot see whether it was sent.

**The event numbers.** They are written in four places by hand: `ChromeEvent::from_code` in Rust,
the `EVENT_...` constants in Kotlin, the `event...` constants in Swift, and the comment in
`KrabikIOS.h`. An unknown number is dropped silently (`_ => return None`, then the entry point
returns). A number that is swapped delivers the wrong event.

**The `from_code` line itself.** A `ChromeEvent` variant with an arm in `chrome_event` and no
line in `from_code` compiles. The session test still passes, because it constructs the variant
directly. On a device the event is never decoded.

**Method names and signatures on Android.** `"setFab"` and
`"(Ljava/lang/String;Ljava/lang/String;Z)V"` are strings. All controller handles are resolved in
`Handles::load`, which runs inside `nativeCreate`. A mismatch fails there, the session handle
comes back as `0`, and the app shows nothing at all. Look for `NoSuchMethodError` in logcat.

**Selectors on iOS.** `krabikSetFab:icon:visible:` is assembled inside `msg_send!` and is not
checked against the Swift class. A wrong selector, or a Swift method that lost its `@objc`,
raises an Objective-C "unrecognized selector" exception on the first `set_chrome`. The entry
points run inside `guard` (`rust/src/ios/mod.rs`), which catches it and prints `krabik ios: ...
handler raised an exception`, so the app keeps running with the rest of that sync skipped.

**Argument types across `msg_send!`.** The Rust expression types must match the Swift parameter
types (`i64` for `Int`, `bool` for `Bool`, `&NSString` for `String`). The existing calls convert
explicitly (`host as i64`); do the same.

**The quiet half of pattern 2.** Nothing verifies that a native dismissal Rust asked for is not
reported. The session tests cannot, because the fake controller never reports anything on its
own. This is checked only by the UI tests that open, close and reopen the slot.

**A stale class comment.** The header comment of `KrabikHostController.kt` lists the calls Rust
makes and stops at `presentSheet` / `dismissSheet`; it does not mention `setFab`, `setDrawer` or
`showSnackbar`. Update it when you add yours.

## Where this lives

- `rust/src/scaffold.rs`: `Scaffold`, `Scaffold::erase`, `Chrome`, `Fab`, `Drawer`, `Snackbar`
- `rust/src/host/platform.rs`: `Controller`, `ChromeEvent`, `ChromeEvent::from_code`
- `rust/src/host/session.rs`: `sync_chrome`, `top_chrome`, `chrome_event`, `dialog_closed`,
  `sheet_dismissed`, and the `tests` module (`FakeController`, `TestApp`, `on_home`)
- `rust/src/android/controller.rs`: `AndroidController::set_chrome`, `Handles`
- `rust/src/android/entry.rs`: `native_on_chrome_event`
- `shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt`, `KrabikActivity.kt`,
  `RustRootView.kt` (`onChromeEvent`)
- `rust/src/ios/controller.rs`: `IosController::set_chrome`
- `rust/src/ios/entry.rs`: `krabik_ios_on_chrome_event`
- `shared/ios/KrabikHostController.swift`, `shared/ios/KrabikChrome.swift`, `shared/ios/KrabikIOS.h`

Next: [Write tests](write-tests.md).
