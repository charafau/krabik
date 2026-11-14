# 5. Navigation and the scaffold

This chapter turns Notes into a two-screen app: the list, and a detail screen for one note. You
will implement `App::navigation`, `App::screen` and `App::back`, give each screen a native app bar
with a `Scaffold`, and move Delete into the app bar. The idea to take away is that Rust owns the
navigation stack as ordinary state and the platform only mirrors it.

## The stack is state

On Android and iOS, navigation is usually something you *do*: call `push`, call `pop`. Here it is
something you *describe*, like the rest of the UI. The app has a hook that returns the stack of
screens that should exist right now:

```rust,ignore
/// The native navigation stack, when the app uses one.
///
/// The application owns this state; the runtime mirrors it with native
/// navigation controllers. With navigation, [`Component::view`] is not
/// shown: the first screen is the root.
fn navigation(&self) -> Option<Navigation<Self::Msg>> {
    None
}
```

(from `rust/src/app.rs`)

A `Navigation` is a list of `Route`s, ordered from the root to the screen on top. A `Route` is a
key and a transition (`rust/src/navigation.rs`):

```rust,ignore
pub struct Route {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The native transition used when this screen is pushed.
    pub transition: ScreenTransition,
}
```

The framework calls `navigation()` after every message, compares the result with the stack it
showed last, and issues the native pushes and pops that turn one into the other.

```text
   your state                 navigation()                 native side
   ----------                 ------------                 -----------
   open: None                 ["notes"]                    [ list ]

        | update(Msg::Open(2))
        v
   open: Some(2)              ["notes", "note:2"]          push "note:2"
                                                           [ list | note 2 ]
        | update(Msg::Back)
        v
   open: None                 ["notes"]                    pop 1
                                                           [ list ]
```

So "going to the detail screen" is setting a field, and "going back" is clearing it. There is no
navigator object to call, and no way for the native stack and your state to disagree for longer
than one render.

For Notes the state is one field:

```rust,ignore
pub struct NotesApp {
    // ...
    /// The note shown on the detail screen, if that screen is open.
    open: Option<u64>,
}
```

and the stack follows from it:

```rust,ignore
fn navigation(&self) -> Option<Navigation<Self::Msg>> {
    let mut routes = vec![Route::new("notes")];
    if let Some(id) = self.open {
        routes.push(Route::new(format!("note:{id}")).transition(ScreenTransition::Slide));
    }
    Some(Navigation::new(routes))
}
```

The keys are yours to choose. `"notes"` is the list. The detail screen's key contains the note's
id, `"note:2"`, so the key alone says what the screen shows. A deeper app keeps a `Vec` of keys in
its state and returns it as is; the showcase does that with `stack: Vec<&'static str>`.
`Navigation::new` accepts plain strings as well as `Route`s, so `Navigation::new(["home",
"detail"])` works when every screen uses the default transition.

### How the mirror works

The comparison is in `sync_navigation` in `rust/src/host/session.rs`, and it is short:

```rust,ignore
let common = self
    .navigation
    .iter()
    .zip(screens)
    .take_while(|(old, new)| old.key == new.key)
    .count();

// Leaving several screens at once is one native pop, animated with
// the transition of the screen the user is looking at.
let popped = &self.navigation[common..];
if let Some(top) = popped.last() {
    controller.pop(cx, popped.len(), top.transition)?;
    // ...
}
// ...
for screen in &screens[common..] {
    // ...
    controller.push(cx, host, screen)?;
}
```

It finds how many screens at the bottom of the stack are unchanged, pops everything above that in
one step, and pushes whatever is new. This has consequences you can rely on:

- Going from `[a, b, c, d]` to `[a]` is one pop animation, not three.
- Replacing the top screen (`[a, b]` to `[a, c]`) is a pop followed by a push.
- A screen is identified by its key. If the key stays the same, the screen stays, whatever its
  content does.

One more case is handled for you. The first stack the native side receives is shown immediately
without animations, even if it is three screens deep. That happens when Android recreates the
activity with the same app instance (chapter 7): the user should come back to where they were,
not watch three screens slide in.

### Transitions

`ScreenTransition` has three values: `Slide` (the default; in from the trailing edge, reversed on
pop), `Fade` (a cross-fade) and `None`. A route's transition is used when that screen is pushed,
and again when it is popped. The showcase has one demo screen for each
(`"transition-fade"` and `"transition-none"` in `ShowcaseApp::navigation`).

## One screen: the Scaffold

The stack says *which* screens exist. A second hook says what one screen *is*:

```rust,ignore
/// One navigation screen, by its key: its content and the native chrome
/// around it. Content alone converts into a screen without chrome:
/// `Some(self.page().into())`.
fn screen(&self, _key: &str) -> Option<Scaffold<Self::Msg>> {
    None
}
```

A `Scaffold` is the layout of a screen: a body in the middle and slots for native pieces around
it (`rust/src/scaffold.rs`):

```rust,ignore
pub struct Scaffold<M = ()> {
    pub app_bar: Option<AppBar<M>>,
    pub body: Element<M>,
    pub bottom_navigation: Option<BottomNavigation<M>>,
    pub floating_action_button: Option<Fab<M>>,
    pub drawer: Option<Drawer<M>>,
    pub snackbar: Option<Snackbar<M>>,
}
```

```text
  +--------------------------------+
  | app bar                        |  native: Toolbar / UINavigationBar
  +--------------------------------+
  |                                |
  |  body                          |  an Element<Msg>, laid out and
  |                                |  drawn like everything so far
  |                      [ fab ]   |
  | [ snackbar                   ] |
  +--------------------------------+
  | bottom navigation              |  native: Material bar / UITabBar
  +--------------------------------+
```

The body is the kind of element you have been writing since chapter 1. Everything else is
*chrome*: native controls that belong to the host, not to the widget tree. The reason for the
split is that the platforms place, size and animate these themselves. An app bar extends under the
status bar, shows a back arrow, animates its title during a push. A tab bar sits above the home
indicator at a height the system decides. Rebuilding those out of rows and text would look wrong
on at least one platform. So you describe them, and the platform draws its own.

This chapter uses the app bar. Chapter 6 fills the other slots.

`screen` for Notes handles its two kinds of key:

```rust,ignore
fn screen(&self, key: &str) -> Option<Scaffold<Msg>> {
    if key == "notes" {
        return Some(
            Scaffold::new()
                .app_bar(AppBar::new("Notes"))
                .body(self.list_body()),
        );
    }
    let id: u64 = key.strip_prefix("note:")?.parse().ok()?;
    let note = self.note(id)?;
    Some(
        Scaffold::new()
            .app_bar(
                AppBar::new("Note")
                    .back_title("Notes")
                    .action("Delete", Msg::Delete(id)),
            )
            .body(detail_body(note)),
    )
}
```

Returning `None` for a key means "there is no such screen", and the framework renders nothing
for it. That is the right answer for `"note:7"` when note 7 has been deleted.

### What happens to `view`

Once `navigation()` returns `Some`, the root component's `view` is no longer shown. The first
route is the root screen instead. You can see the rule where each native host gets its content,
in `render_all` in `rust/src/host/session.rs`:

```rust,ignore
let element = match host.kind() {
    // A navigation app shows its root as the first screen.
    HostKind::Main if has_navigation => None,
    HostKind::Main => Some(self.app.view()),
    HostKind::Screen(key) => self.app.screen(key).map(|(body, _)| body),
    HostKind::Sheet => self.app.sheet().map(|sheet| sheet.into_content()),
};
```

`Component` still requires a `view`, so Notes returns the list from it; the showcase does the
same with a comment saying it is not shown. This is a rough edge: an app with navigation has a
required method that is never called on screen.

That loop also shows something useful. *Every* screen in the stack is rendered after every
message, not only the top one. The list screen under the detail screen is kept up to date, so
when the user goes back it already shows the right content, and its scroll position and native
views were never torn down.

Chrome is different: only the top screen's chrome is shown. `top_chrome` in the same file reads
the scaffold of the last route. An app without a navigation stack can have chrome too: it implements `App::scaffold`, which is
used in place of `view()` for the one screen it has. The minimal todo app does this for its app
bar:

```rust,ignore
// example-todo-minimal/rust/src/lib.rs
fn scaffold(&self) -> Option<Scaffold<Msg>> {
    Some(Scaffold::new().app_bar(AppBar::new("Todo")).body(self.view()))
}
```

## The app bar

`AppBar` (`rust/src/scaffold.rs`) is the Android toolbar and the iOS navigation bar.

```rust,ignore
AppBar::new("Note")                         // the title
    .subtitle("Edited today")               // an optional second line
    .back_title("Notes")                    // a label for the back button
    .action("Delete", Msg::Delete(id))      // a trailing text action
    .action_with_icon(                      // a trailing action with an icon
        "Share",
        Icon::new().drawable("ic_share").sf("square.and.arrow.up"),
        Msg::Share(id),
    )
```

An action is a label and a message, like a button. With `action_with_icon` the label is still
required; it is shown next to the icon and read by screen readers.

An `Icon` names a platform image (`rust/src/style.rs`). `.drawable("ic_share")` is an Android
drawable resource in the app's `res/drawable`; `.sf("square.and.arrow.up")` is an iOS SF Symbol
or asset-catalog image; `.shared("name")` is used on a platform that has no name of its own.
Icons are looked up by name at run time, so a misspelt name gives no icon and no error.

`back_title` deserves a caveat. On Android it becomes the accessibility description of the back
arrow. On iOS the framework replaces the system back button with its own chevron (see the next
section), and that button's accessibility label is fixed to "Back" in `krabikPush` in
`shared/ios/KrabikHostController.swift`. As far as the source shows, `back_title` therefore has
no visible effect on iOS today.

Actions are found again when tapped, not stored natively. The native bar reports "action number
1 was tapped", and the session asks your app for the top screen's scaffold *now* and takes the
handler from that:

```rust,ignore
// Handlers are read from the application's current state, never from
// the copy last sent to the platform.
let message = self
    .top_chrome()
    .app_bar
    .and_then(|bar| bar.actions.get(index)?.on_tap.message(()));
```

(from `toolbar_action` in `rust/src/host/session.rs`)

It is the same principle as for widgets in chapter 1: the platform holds no behaviour, only
indexes.

## Back

There are several ways for a user to go back: the system back button or gesture on Android, and
the back arrow in the app bar on both platforms. All of them end in one hook:

```rust,ignore
/// The message to send for the system back action, or `None` to let the
/// platform handle it (which leaves the app from its root screen).
fn back(&self) -> Option<Self::Msg> {
    None
}
```

For Notes:

```rust,ignore
fn back(&self) -> Option<Msg> {
    self.open.map(|_| Msg::Back)
}

// in update:
Msg::Back => self.open = None,
```

For the common case, "back leaves the top screen", there is a shorter form that needs no `back`
method at all. Name the message on the stack:

```rust,ignore
fn navigation(&self) -> Option<Navigation<Msg>> {
    Some(Navigation::new(self.routes()).on_back(Msg::Back))
}
```

`on_back` applies while more than one route is on the stack; on the root screen the platform
leaves the app. `App::back` is for other rules (close a dialog first, ask before discarding a
draft) and wins when it returns a message. The showcase uses both: `on_back` for screens and
`back` for an open dialog or sheet.

Read the two halves of the longer form separately.

`back()` answers "would you handle a back action right now, and with which message?". On the
detail screen: yes, `Msg::Back`. On the list: `None`, there is nothing to go back to.

The answer is not only used when the user presses back. The session asks after every render and
tells the platform whether the app currently handles back (`set_back_enabled` in `sync`). On
Android this arms or disarms the activity's back callback (`backCallback` in
`shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt`). While it is disarmed, the system
handles back itself, which is what makes the predictive "back to home" animation work on the root
screen. So return `None` whenever you would do nothing.

`update(Msg::Back)` then changes the state like any other message, and the stack follows.

Because back is a message, it can do whatever the situation calls for. The showcase closes an
open dialog first, then a sheet, then pops a screen (`Msg::Back` in `example/rust/src/lib.rs`).
Chapter 6 does the same for Notes.

### The framework never pops by itself

This is the consequence of "Rust owns the stack", and it is worth stating bluntly: **nothing goes
back unless your `update` removes a route.** The app bar's back arrow does not pop the screen. It
asks Rust:

- On Android the toolbar's navigation icon calls `main.onBackPressed()`, which calls the same
  session entry point as the system back button (`KrabikHostController.kt`, in the `init` block).
- On iOS the system back button is hidden and replaced with a chevron button whose action is
  `backTapped`, which calls `krabik_ios_on_back`. The comment in `krabikPush` gives the reason:
  "Rust owns the stack, so the system back button (which pops on its own) is replaced by one that
  asks Rust."

If you push a screen and neither set `Navigation::on_back` nor implement `back`, the arrow is
there and does nothing.

The one exception is the iOS swipe-from-the-edge gesture, which UIKit completes by itself. It is
enabled only while your app handles back, and when the user completes it the framework sends your
back message so the stack follows. If your `update` keeps the route, the screen is put back
(without animation), the same way a dialog you keep is shown again. A swipe the user abandons
sends nothing.

## Bottom navigation

Notes does not need tabs, but most apps do, so here is how they fit. A bottom navigation bar is a
scaffold slot, described by state like everything else. This is the showcase's
(`bottom_navigation` in `example/rust/src/pages.rs`):

```rust,ignore
BottomNavigation::new([
    BottomNavItem::new("Home", Icon::new().drawable("ic_home").sf("house.fill")),
    BottomNavItem::new(
        "Search",
        Icon::new().drawable("ic_search").sf("magnifyingglass"),
    ),
    BottomNavItem::new(
        "Settings",
        Icon::new().drawable("ic_settings").sf("gearshape.fill"),
    ),
])
.selected(app.bottom_dest.min(2))
.on_select(Msg::BottomDest)
```

The selected index is a field in your state. `on_select` reports the index the user tapped; your
`update` stores it; your screen's body shows the content for that index. The bar is the system
tab bar on iOS and a Material bottom navigation bar on Android.

Tabs are *not* routes. Switching tabs does not push anything; the root screen stays the root
screen and its body changes. The doc comment on `Scaffold::bottom_navigation` adds a practical
rule: "Give every top-level destination the same bar so it stays in place while the content
changes." A screen pushed on top (a detail screen) normally returns a scaffold without the bar,
and the bar goes away while that screen is on top.

## The detail screen's body, and opening it

The body is a scrolling column with the note's full text:

```rust,ignore
fn detail_body(note: &Note) -> Element<Msg> {
    ScrollView::new()
        .width_percent(1.0)
        .height_percent(1.0)
        .child(
            Column::new()
                .padding(16)
                .width_percent(1.0)
                .child(Text::new(note.text.clone()).size(18)),
        )
        .into()
}
```

Something has to open it. Chapter 2 mentioned that rows and text have no tap handler, so the
row gets an Open button where Delete used to be:

```rust,ignore
Button::new("Open")
    .on_click(Msg::Open(note.id))
    .accessibility_label(format!("Open {}", note.text))
```

```rust,ignore
// in update:
Msg::Open(id) => {
    if self.note(id).is_some() {
        self.open = Some(id);
    }
}
```

The check keeps the state honest: `open` only ever names a note that exists.

Delete now lives in the detail screen's app bar. Deleting the note you are looking at must also
leave its screen, and with the stack as state that is one more assignment in the same arm:

```rust,ignore
Msg::Delete(id) => {
    Arc::make_mut(&mut self.notes).retain(|note| note.id != id);
    if self.open == Some(id) {
        self.open = None;
    }
}
```

After this `update`, `navigation()` returns one route, and the framework pops the detail screen.
You did not call anything to make that happen.

The title "Notes" moves from the body into the app bar, so the list body loses its first line and
some padding.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use std::sync::Arc;

use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::navigation::{Navigation, Route, ScreenTransition};
use krabik_ui_dsl::scaffold::{AppBar, Scaffold};
use krabik_ui_dsl::ui::{
    Align, Button, Color, Column, EdgeInsets, Element, List, Row, ScrollView, Text, TextField,
};

/// The height the list reserves for each note.
const ITEM_HEIGHT: f32 = 64.0;
/// The height of the card drawn inside it; the rest is the gap between cards.
const ROW_HEIGHT: f32 = 56.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    id: u64,
    text: String,
}

// ----- The composer: the field a new note is typed into -----------------

/// Everything the user can do in the composer.
#[derive(Clone, Debug, PartialEq)]
pub enum ComposerMsg {
    /// The text in the field changed.
    Draft(String),
    /// The Add button or the keyboard's action key was pressed.
    Submit,
}

pub struct Composer {
    draft: String,
}

impl Composer {
    fn new() -> Self {
        Self {
            draft: String::new(),
        }
    }

    /// Takes the draft out, trimmed. `None` when there is nothing to add.
    fn take(&mut self) -> Option<String> {
        let text = self.draft.trim().to_string();
        if text.is_empty() {
            return None;
        }
        self.draft.clear();
        Some(text)
    }
}

impl Component for Composer {
    type Msg = ComposerMsg;

    fn update(&mut self, msg: ComposerMsg) {
        match msg {
            ComposerMsg::Draft(text) => self.draft = text,
            // What a submitted draft becomes is the parent's decision;
            // see `NotesApp::update`.
            ComposerMsg::Submit => {}
        }
    }

    fn view(&self) -> Element<ComposerMsg> {
        Row::new()
            .width_percent(1.0)
            .spacing(8)
            .align_items(Align::Center)
            .child(
                TextField::new(self.draft.clone())
                    .placeholder("Write a note")
                    .on_change(ComposerMsg::Draft)
                    .on_submit(ComposerMsg::Submit)
                    .grow(1),
            )
            .child(
                Button::new("Add")
                    .on_click(ComposerMsg::Submit)
                    .enabled(!self.draft.trim().is_empty()),
            )
            .into()
    }
}

// ----- The app ------------------------------------------------------------

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Something happened in the composer.
    Composer(ComposerMsg),
    /// Opens the detail screen of the note with this id.
    Open(u64),
    /// The system back action or the app bar's back arrow.
    Back,
    /// Deletes the note with this id.
    Delete(u64),
}

/// The state of the app.
pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    /// The id the next note gets. Ids are never reused.
    next_id: u64,
    composer: Composer,
    /// The note shown on the detail screen, if that screen is open.
    open: Option<u64>,
}

impl NotesApp {
    pub fn new() -> Self {
        Self {
            notes: Arc::new(vec![
                Note {
                    id: 1,
                    text: "Welcome to Notes".to_string(),
                },
                Note {
                    id: 2,
                    text: "Write something above and press Add".to_string(),
                },
            ]),
            next_id: 3,
            composer: Composer::new(),
            open: None,
        }
    }

    /// Registers the app with the framework. The host calls this at startup.
    pub fn install() {
        set_app_factory(Self::new);
    }

    fn note(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|note| note.id == id)
    }

    fn summary(&self) -> String {
        match self.notes.len() {
            0 => "No notes yet".to_string(),
            1 => "1 note".to_string(),
            count => format!("{count} notes"),
        }
    }

    /// The content of the list screen.
    fn list_body(&self) -> Element<Msg> {
        Column::new()
            .padding(16)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .child(self.composer.view().map(Msg::Composer))
            .child(caption(&self.summary()))
            .child(
                List::new(Arc::clone(&self.notes))
                    .item_height(ITEM_HEIGHT)
                    .key(|note| note.id.to_string())
                    .row(note_row),
            )
            .into()
    }
}

impl Component for NotesApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            // The composer says when; the app decides what a note is.
            Msg::Composer(ComposerMsg::Submit) => {
                if let Some(text) = self.composer.take() {
                    let id = self.next_id;
                    self.next_id += 1;
                    Arc::make_mut(&mut self.notes).push(Note { id, text });
                }
            }
            Msg::Composer(msg) => self.composer.update(msg),
            Msg::Open(id) => {
                if self.note(id).is_some() {
                    self.open = Some(id);
                }
            }
            Msg::Back => self.open = None,
            Msg::Delete(id) => {
                Arc::make_mut(&mut self.notes).retain(|note| note.id != id);
                // Deleting the open note also leaves its screen.
                if self.open == Some(id) {
                    self.open = None;
                }
            }
        }
    }

    /// Not shown: with navigation, the first screen is the root.
    fn view(&self) -> Element<Msg> {
        self.list_body()
    }
}

impl App for NotesApp {
    fn navigation(&self) -> Option<Navigation<Self::Msg>> {
        let mut routes = vec![Route::new("notes")];
        if let Some(id) = self.open {
            routes.push(Route::new(format!("note:{id}")).transition(ScreenTransition::Slide));
        }
        Some(Navigation::new(routes))
    }

    fn screen(&self, key: &str) -> Option<Scaffold<Msg>> {
        if key == "notes" {
            return Some(
                Scaffold::new()
                    .app_bar(AppBar::new("Notes"))
                    .body(self.list_body()),
            );
        }
        let id: u64 = key.strip_prefix("note:")?.parse().ok()?;
        let note = self.note(id)?;
        Some(
            Scaffold::new()
                .app_bar(
                    AppBar::new("Note")
                        .back_title("Notes")
                        .action("Delete", Msg::Delete(id)),
                )
                .body(detail_body(note)),
        )
    }

    /// Back is ours while the detail screen is open; on the list the
    /// platform leaves the app.
    fn back(&self) -> Option<Msg> {
        self.open.map(|_| Msg::Back)
    }
}

// ----- View functions -------------------------------------------------------

/// One note in the list.
fn note_row(note: &Note) -> Element<Msg> {
    Row::new()
        .width_percent(1.0)
        .height(ROW_HEIGHT)
        .padding_each(EdgeInsets::symmetric(12, 6))
        .spacing(8)
        .align_items(Align::Center)
        .background(Color::SURFACE)
        .corner_radius(10)
        .child(Text::new(note.text.clone()).max_lines(1).ellipsis().grow(1))
        .child(
            Button::new("Open")
                .on_click(Msg::Open(note.id))
                .accessibility_label(format!("Open {}", note.text)),
        )
        .into()
}

/// The content of the detail screen.
fn detail_body(note: &Note) -> Element<Msg> {
    ScrollView::new()
        .width_percent(1.0)
        .height_percent(1.0)
        .child(
            Column::new()
                .padding(16)
                .width_percent(1.0)
                .child(Text::new(note.text.clone()).size(18)),
        )
        .into()
}

/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(Color::SECONDARY_TEXT)
}
```

Navigation is testable like everything else, because it is a function of state. Add to the
`tests` module from chapter 4:

```rust,ignore
#[test]
fn opening_a_note_pushes_its_screen_and_back_pops_it() {
    let mut app = NotesApp::new();
    // On the list, back belongs to the platform.
    assert_eq!(app.back(), None);
    assert_eq!(app.navigation().unwrap().routes().len(), 1);

    app.update(Msg::Open(2));
    let navigation = app.navigation().unwrap();
    assert_eq!(navigation.routes()[1].key, "note:2");
    let screen = app.screen("note:2").expect("the detail screen");
    assert_eq!(screen.app_bar.expect("an app bar").title, "Note");

    app.update(app.back().expect("back is handled"));
    assert_eq!(app.navigation().unwrap().routes().len(), 1);
}

#[test]
fn deleting_the_open_note_leaves_its_screen() {
    let mut app = NotesApp::new();
    app.update(Msg::Open(2));
    app.update(Msg::Delete(2));
    assert_eq!(app.navigation().unwrap().routes().len(), 1);
    assert!(app.screen("note:2").is_none());
    // A note that does not exist cannot be opened.
    app.update(Msg::Open(2));
    assert_eq!(app.back(), None);
}
```

`app.update(app.back().expect(..))` is the line to copy into your own tests: it sends whatever
the app says back should do, the way the session does.

## Where this lives

- `rust/src/app.rs`: the `navigation`, `screen` and `back` hooks.
- `rust/src/navigation.rs`: `Navigation`, `Route`, `ScreenTransition`, `BottomNavigation`.
- `rust/src/scaffold.rs`: `Scaffold`, `AppBar`, `AppBarAction`.
- `rust/src/host/session.rs`: `render_all`, `sync`, `sync_navigation`, `top_chrome`, `back`,
  `toolbar_action`; the tests `leaving_several_screens_is_one_pop` and
  `back_pops_until_the_root_and_then_declines` show the mirror at work against a fake platform.
- `shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt` and
  `shared/ios/KrabikHostController.swift`: the native controllers that carry out pushes and pops.
- `example/rust/src/lib.rs`: `ShowcaseApp::navigation`, `screen` and `back`.

How the native controllers manage screens (fragments by hand on Android, a
`UINavigationController` on iOS) is covered in [`../internals/`](../internals/).

Next: [6. Dialogs, sheets and more chrome](06-dialogs-sheets-and-more-chrome.md).
