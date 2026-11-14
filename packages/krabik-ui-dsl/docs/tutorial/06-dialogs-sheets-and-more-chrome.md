# 6. Dialogs, sheets and more chrome

This chapter adds the remaining native pieces to Notes: a confirmation dialog before deleting, a
snackbar with Undo afterwards, a bottom sheet with a note's details, a floating action button for
adding, and a navigation drawer that chooses the sort order. All five follow one pattern, so the
chapter teaches the pattern first and then applies it five times. It also says plainly which of
these are real platform controls and which are stand-ins.

## One pattern: described by state

In most UI toolkits you *show* a dialog: you call a function, the dialog appears, and later a
callback fires. Then there are two sources of truth, your state and "is the dialog currently up",
and they can disagree.

Here a dialog is described the same way as a label. A hook on `App` returns what should be on
screen for the current state:

```rust,ignore
/// The system alert to show, if any. It stays up while this returns
/// `Some`; a button sends its message and the application closes the
/// alert by returning `None` afterwards.
fn dialog(&self) -> Option<Dialog<Self::Msg>> {
    None
}

/// The native bottom sheet to show, if any.
fn sheet(&self) -> Option<BottomSheet<Self::Msg>> {
    None
}
```

(from `rust/src/app.rs`)

The drawer, the snackbar and the floating button are fields of the `Scaffold` that `screen`
returns, and work the same way. From the module comment of `rust/src/scaffold.rs`:

```rust,ignore
//! The drawer and the snackbar are described by state, like a
//! [`Dialog`](crate::presentation::Dialog): the scaffold contains them while
//! they should be visible, and the platform's own dismissal (a swipe, a
//! timeout) is reported as a message so the application can update its
//! state.
```

So every one of these has the same three parts:

1. **A field in your state** that says whether it is showing (`Option<u64>`, `bool`, ...).
2. **A hook or scaffold slot** that returns `Some(description)` while the field says so.
3. **Messages** that set and clear the field. One of them is always "the user closed it
   natively".

The third part is the one that is easy to forget. The user can close a sheet by dragging it down,
a snackbar disappears after a few seconds, a drawer is swiped shut. The platform does that on its
own; your state did not change. The framework therefore reports it, with a message you chose, and
your `update` clears the field:

```text
   opening                                  closing natively

   user taps "Details"                      user drags the sheet down
        |                                        |
        v                                        v
   Msg::ShowDetails                         the platform closes the sheet
        |                                        |
        v                                        v
   update: details_open = true              framework: "the sheet is gone",
        |                                   sends your on_dismiss message
        v                                        |
   sheet() returns Some(..)                      v
        |                                   update: details_open = false
        v                                        |
   framework sees: wanted, not shown             v
   -> presents the native sheet             sheet() returns None
                                            framework sees: not wanted, not
                                            shown -> nothing to do
```

If you forget the dismissal message, state and screen disagree, and the framework resolves it in
favour of your state: it shows the thing again. The comment on `sheet_dismissed` in
`rust/src/host/session.rs` says so:

```rust,ignore
/// The platform has already closed the sheet, so the session forgets it
/// first: if the application still returns a sheet, it is presented again.
```

There is a test for each case, for example `a_dialog_the_app_keeps_is_shown_again_after_it_closes`
and `a_snackbar_the_application_keeps_is_shown_again`. A sheet that pops back up after being
swiped away is the symptom of a missing `on_dismiss`.

## What is native where

Before the code, the honest inventory. "Native" here means the platform's own control, with its
own look, animation and accessibility behaviour.

| Piece | Android | iOS |
| ----- | ------- | --- |
| App bar | `Toolbar` | `UINavigationBar` |
| Bottom navigation | Material bottom navigation bar | `UITabBar` |
| Dialog | Material alert dialog | `UIAlertController` |
| Bottom sheet | `BottomSheetDialogFragment` | `UISheetPresentationController`, sized to its content |
| Floating action button | Material `ExtendedFloatingActionButton` | **stand-in**: a filled capsule `UIButton` |
| Snackbar | Material `Snackbar` | **stand-in**: `KrabikSnackbarView` |
| Drawer | `DrawerLayout` with a Material `NavigationView` | **stand-in**: `KrabikDrawerView` |

The three stand-ins exist because UIKit has no such controls. The file comment of
`shared/ios/KrabikChrome.swift` is direct about it:

```swift
// Android has a Material snackbar and navigation drawer; iOS has neither, so
// these are small views with the same behaviour. They are plain UIKit and
// know nothing about Rust: `KrabikHostController` owns them and reports what
// the user does.
```

They behave like their Android counterparts (the snackbar leaves after four seconds or on a
downward swipe, the drawer dims the screen and closes on an outside tap or a leftward swipe), but
an iOS user will recognise them as Android idioms. For an app that should feel at home on both
platforms, prefer the pieces in the top four rows.

## Dialog: confirm before deleting

Deleting a note from the app bar is one tap away from a mistake. Ask first.

**State.** Which note the question is about, or `None` when no question is open:

```rust,ignore
/// The note the confirmation dialog asks about, while it is open.
confirm_delete: Option<u64>,
```

**Hook.**

```rust,ignore
fn dialog(&self) -> Option<Dialog<Msg>> {
    self.confirm_delete.map(|_| {
        Dialog::new("Delete note?")
            .message("You can undo this for a few seconds.")
            .cancel("Cancel", Msg::CancelDelete)
            .destructive("Delete", Msg::ConfirmDelete)
    })
}
```

**Messages.** The app bar action now asks instead of deleting:

```rust,ignore
AppBar::new("Note").action("Delete", Msg::AskDelete(id))
```

```rust,ignore
Msg::AskDelete(id) => self.confirm_delete = Some(id),
Msg::CancelDelete => self.confirm_delete = None,
Msg::ConfirmDelete => {
    if let Some(id) = self.confirm_delete.take() {
        self.delete(id);
    }
}
```

Both buttons clear `confirm_delete` (`take()` leaves `None` behind), so after either one the hook
returns `None`.

A `Dialog` (`rust/src/presentation.rs`) is a title, an optional message and buttons. There are
three kinds of button, and the kind matters more than it seems:

- `.button(label, msg)`: an ordinary choice.
- `.destructive(label, msg)`: a choice that cannot be undone. iOS draws it red; Android colours
  it with the theme's error colour.
- `.cancel(label, msg)`: backs out. **Its message is also sent when the platform dismisses the
  dialog itself**: the Android back button, or a tap outside the dialog.

That last rule decides whether the dialog can be dismissed natively at all:

```rust,ignore
/// Whether the platform may dismiss the dialog without a button press
/// (Android's back button or a tap outside): only when there is a cancel
/// button whose message can stand in for that.
pub fn is_cancelable(&self) -> bool {
    self.cancel_button().is_some()
}
```

Without a cancel button the dialog is modal in the strict sense: the user must pick a button.
That follows from the pattern. A native dismissal has to become *some* message, or your state
would be left saying "open".

Two limits come from the platforms. The doc comment says "up to three buttons": on Android a
dialog has three button slots, and `showDialog` in `KrabikHostController.kt` puts the cancel
button in the negative slot, the first other button in the positive slot, the second in the
neutral slot, and silently skips any further ones. On iOS only one button can have the cancel
style, and a dialog with no buttons gets an "OK" so it can be closed.

One more detail explains why buttons are matched by index. When the user presses a button, the
platform has already closed the alert and reports "button 1". The session then asks your app for
the dialog *again* and takes the handler from that (`dialog_closed` in `rust/src/host/session.rs`).
So the dialog you return must still be the same one at that moment. It is, as long as only
messages change your state.

## Snackbar: undo

A confirmation and an undo together are more than this app needs, but they show two different
tools. The snackbar is the brief bar at the bottom: "Deleted", with an Undo button.

**State.** The deleted note and where it was, for as long as Undo is on offer:

```rust,ignore
/// The note deleted last and its position, while Undo is on offer.
deleted: Option<(usize, Note)>,
```

`delete` fills it in:

```rust,ignore
fn delete(&mut self, id: u64) {
    let Some(index) = self.notes.iter().position(|note| note.id == id) else {
        return;
    };
    let note = Arc::make_mut(&mut self.notes).remove(index);
    self.deleted = Some((index, note));
    // Deleting the open note also leaves its screen.
    if self.open == Some(id) {
        self.open = None;
        self.details_open = false;
    }
}
```

**Slot.** A snackbar is part of a screen's scaffold. After a delete the list is the top screen,
so the list's scaffold gets it:

```rust,ignore
.snackbar(self.deleted.as_ref().map(|(_, note)| {
    Snackbar::new(format!("Deleted \"{}\"", note.text))
        .action("Undo", Msg::Undo)
        .on_dismiss(Msg::SnackbarGone)
}))
```

Every scaffold slot accepts an `Option`, so state can decide in one expression
(`impl Into<Option<Snackbar<M>>>` in `Scaffold::snackbar`).

**Messages.**

```rust,ignore
Msg::Undo => {
    if let Some((index, note)) = self.deleted.take() {
        let notes = Arc::make_mut(&mut self.notes);
        let index = index.min(notes.len());
        notes.insert(index, note);
    }
}
Msg::SnackbarGone => self.deleted = None,
```

`on_dismiss` is the "closed natively" message: the snackbar timed out or was swiped away. The
user has let the offer pass, so the app forgets the deleted note. Pressing the action also removes
the bar natively (`ChromeEvent::SnackbarAction` is documented as "which also removes it"), and
`Msg::Undo` clears the field with `take()`, so state and screen agree on both paths.

A subtlety: the framework shows a snackbar when the description *differs* from the one it is
showing (`sync_chrome` compares them). Two snackbars with the same text and buttons are equal, so
the second would not restart the timer. Putting the note's text into the message makes each
deletion a different snackbar. The showcase does the same with "Added item 3".

The snackbar is positioned by the platform code above the floating button or the bottom bar, if
either is showing.

## Bottom sheet: note details

A bottom sheet is a panel that slides up over the screen and holds your own content. Use it for
secondary content and for choices that need more than three buttons.

**State.** `details_open: bool`.

**Hook.**

```rust,ignore
fn sheet(&self) -> Option<BottomSheet<Msg>> {
    if !self.details_open {
        return None;
    }
    let note = self.note(self.open?)?;
    let words = note.text.split_whitespace().count();
    let characters = note.text.chars().count();
    Some(
        BottomSheet::new()
            .on_dismiss(Msg::CloseDetails)
            .child(Text::new("Details").size(20).bold())
            .child(caption(&format!("{words} words, {characters} characters")))
            .child(Button::new("Close").on_click(Msg::CloseDetails)),
    )
}
```

**Messages.** `Msg::ShowDetails` sets the flag, from a button on the detail screen;
`Msg::CloseDetails` clears it, from the sheet's own Close button *and* from `on_dismiss`.

A `BottomSheet` (`rust/src/presentation.rs`) is a container: `child` appends to a column that
already has 20 points of padding, 12 points of spacing and the full width. What goes in it are
ordinary widgets sending ordinary messages. Under the hood the sheet's content is rendered by a
separate native host, the `HostKind::Sheet` case you saw in `render_all` in chapter 5, so it is
rebuilt after every message like any screen.

The platform supplies everything around the content: the surface, the rounded corners, the dimmed
background, the drag handle, and the height. "Sizes the sheet to its content" in the doc comment
is literal: on iOS the framework renders the content first, asks Rust for its height and makes
that the sheet's detent (`krabikPresentSheet` in `shared/ios/KrabikHostController.swift`).

Unlike a dialog, a sheet can always be dismissed natively, so `on_dismiss` is not optional in
practice.

## Floating action button: add

The floating action button (FAB) is the screen's main action, floating over the bottom corner of
the body. For the list screen that is "add the note I typed".

There is no state for a FAB; it is there or not, per screen:

```rust,ignore
.floating_action_button(
    Fab::new("Add")
        .icon(Icon::new().drawable("ic_add").sf("plus"))
        .on_click(Msg::Composer(ComposerMsg::Submit)),
)
```

Look at the message. The FAB belongs to the app's scaffold, so it sends the app's `Msg`. But the
action it stands for is the composer's Submit. The app builds the wrapped message by hand:
`Msg::Composer(ComposerMsg::Submit)`, the same value that `map` would have produced for a button
inside the composer. `update` cannot tell the difference and does not need to. This is the
flexibility of messages being plain values: anything that can construct one can trigger the
behaviour.

With the FAB doing the adding, the Add button inside the composer is redundant, and the composer
shrinks to the text field. Its `on_submit` still adds from the keyboard.

`Fab` takes a label and an optional icon (`rust/src/scaffold.rs`). The label is always shown and
is also the accessibility label. The `ic_add` drawable has to exist in the Android app's
resources; chapter 10 adds it.

## Drawer: sort order

A navigation drawer is a panel of destinations that slides in from the leading edge. Normally its
items are the sections of an app. Notes has no sections, so it uses the drawer for the one global
choice it has, the sort order. Treat the mechanics as the lesson, not the use.

**State.** Two fields: whether it is open, and the choice it makes.

```rust,ignore
drawer_open: bool,
newest_first: bool,
```

**Slot.**

```rust,ignore
.drawer(
    Drawer::new([
        DrawerItem::new("Oldest first"),
        DrawerItem::new("Newest first"),
    ])
    .selected(usize::from(self.newest_first))
    .open(self.drawer_open)
    .on_select(Msg::Sort)
    .on_open(Msg::Drawer(true))
    .on_close(Msg::Drawer(false)),
)
```

**Messages.**

```rust,ignore
Msg::Drawer(open) => self.drawer_open = open,
Msg::Sort(index) => {
    self.newest_first = index == 1;
    self.drawer_open = false;
}
```

The drawer has *two* native-change messages because the user can both open and close it without
your code: `on_open` (they tapped the menu button or swiped from the edge) and `on_close` (they
swiped it shut or tapped outside). Both write the same field. The doc comment on `Drawer` states
the contract:

```rust,ignore
/// The application owns whether the drawer is open: set [`Drawer::open`]
/// from state, and update that state in the `on_open` and `on_close`
/// messages, which are sent when the user opens or closes it natively.
```

Picking an item does not close the drawer by itself. `Msg::Sort` closes it by clearing the flag,
which is the app's decision to make.

When a scaffold has a drawer, the root screen's app bar gets a menu button in the place where a
pushed screen has its back arrow. Both platforms do this (`updateNavigationIcon` in
`KrabikHostController.kt`, `updateMenuButton` in `KrabikHostController.swift`). On a pushed
screen that spot belongs to back, so give a drawer to root screens only. On Android the system
back action also closes an open drawer before it does anything else; the activity registers a
second back callback for that (`drawerBack` in `KrabikActivity.kt`), and the close is reported
through `on_close` as usual.

The sort order changes what the list is given:

```rust,ignore
/// The notes in the order the list shows them.
fn visible(&self) -> Arc<Vec<Note>> {
    if self.newest_first {
        Arc::new(self.notes.iter().rev().cloned().collect())
    } else {
        Arc::clone(&self.notes)
    }
}
```

Because every row is keyed by its note's id, reversing the order moves the existing native rows
instead of rewriting them (chapter 3).

## Back, again

With more things that can be open, back needs an order. The convention, taken from the showcase:
back closes whatever is on top.

```rust,ignore
Msg::Back => {
    if self.confirm_delete.is_some() {
        self.confirm_delete = None;
    } else if self.details_open {
        self.details_open = false;
    } else {
        self.open = None;
    }
}
```

```rust,ignore
fn back(&self) -> Option<Msg> {
    (self.open.is_some() || self.details_open || self.confirm_delete.is_some())
        .then_some(Msg::Back)
}
```

In practice the dialog and the sheet usually handle back themselves, because each is a native
window that receives the back action first and reports a dismissal. The ordering above is the
safety net that keeps the state consistent whichever path the platform takes.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use std::sync::Arc;

use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::navigation::{Navigation, Route, ScreenTransition};
use krabik_ui_dsl::presentation::{BottomSheet, Dialog};
use krabik_ui_dsl::scaffold::{AppBar, Drawer, DrawerItem, Fab, Scaffold, Snackbar};
use krabik_ui_dsl::ui::{
    Align, Button, Color, Column, EdgeInsets, Element, Icon, List, Row, ScrollView, Text,
    TextField,
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
    /// The keyboard's action key or the Add button was pressed.
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

    fn update(&mut self, msg: ComposerMsg) -> Command<ComposerMsg> {
        match msg {
            ComposerMsg::Draft(text) => self.draft = text,
            // What a submitted draft becomes is the parent's decision;
            // see `NotesApp::update`.
            ComposerMsg::Submit => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<ComposerMsg> {
        TextField::new(self.draft.clone())
            .placeholder("Write a note")
            .on_change(ComposerMsg::Draft)
            .on_submit(ComposerMsg::Submit)
            .width_percent(1.0)
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
    /// Asks whether the note with this id should be deleted.
    AskDelete(u64),
    /// The confirmation dialog was cancelled.
    CancelDelete,
    /// The confirmation dialog's Delete button.
    ConfirmDelete,
    /// The snackbar's Undo button.
    Undo,
    /// The snackbar went away by itself.
    SnackbarGone,
    /// Opens the details sheet of the open note.
    ShowDetails,
    /// The details sheet was closed, by its button or natively.
    CloseDetails,
    /// The drawer was opened or closed.
    Drawer(bool),
    /// A sort order was picked in the drawer.
    Sort(usize),
}

/// The state of the app.
pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    /// The id the next note gets. Ids are never reused.
    next_id: u64,
    composer: Composer,
    /// The note shown on the detail screen, if that screen is open.
    open: Option<u64>,
    /// The note the confirmation dialog asks about, while it is open.
    confirm_delete: Option<u64>,
    /// The note deleted last and its position, while Undo is on offer.
    deleted: Option<(usize, Note)>,
    /// Whether the details sheet is showing.
    details_open: bool,
    /// Whether the drawer is showing.
    drawer_open: bool,
    /// The sort order the drawer chooses.
    newest_first: bool,
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
            confirm_delete: None,
            deleted: None,
            details_open: false,
            drawer_open: false,
            newest_first: false,
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

    /// The notes in the order the list shows them.
    fn visible(&self) -> Arc<Vec<Note>> {
        if self.newest_first {
            Arc::new(self.notes.iter().rev().cloned().collect())
        } else {
            Arc::clone(&self.notes)
        }
    }

    /// Removes a note and remembers it for Undo.
    fn delete(&mut self, id: u64) {
        let Some(index) = self.notes.iter().position(|note| note.id == id) else {
            return;
        };
        let note = Arc::make_mut(&mut self.notes).remove(index);
        self.deleted = Some((index, note));
        // Deleting the open note also leaves its screen.
        if self.open == Some(id) {
            self.open = None;
            self.details_open = false;
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
                List::new(self.visible())
                    .item_height(ITEM_HEIGHT)
                    .key(|note| note.id.to_string())
                    .row(note_row),
            )
            .into()
    }

    /// The list screen: the body plus its native chrome.
    fn list_screen(&self) -> Scaffold<Msg> {
        Scaffold::new()
            .app_bar(AppBar::new("Notes"))
            .body(self.list_body())
            .floating_action_button(
                Fab::new("Add")
                    .icon(Icon::new().drawable("ic_add").sf("plus"))
                    .on_click(Msg::Composer(ComposerMsg::Submit)),
            )
            .drawer(
                Drawer::new([
                    DrawerItem::new("Oldest first"),
                    DrawerItem::new("Newest first"),
                ])
                .selected(usize::from(self.newest_first))
                .open(self.drawer_open)
                .on_select(Msg::Sort)
                .on_open(Msg::Drawer(true))
                .on_close(Msg::Drawer(false)),
            )
            .snackbar(self.deleted.as_ref().map(|(_, note)| {
                Snackbar::new(format!("Deleted \"{}\"", note.text))
                    .action("Undo", Msg::Undo)
                    .on_dismiss(Msg::SnackbarGone)
            }))
    }

    /// The detail screen of one note.
    fn detail_screen(&self, note: &Note) -> Scaffold<Msg> {
        Scaffold::new()
            .app_bar(
                AppBar::new("Note")
                    .back_title("Notes")
                    .action("Delete", Msg::AskDelete(note.id)),
            )
            .body(detail_body(note))
    }
}

impl Component for NotesApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            // The composer says when; the app decides what a note is.
            Msg::Composer(ComposerMsg::Submit) => {
                if let Some(text) = self.composer.take() {
                    let id = self.next_id;
                    self.next_id += 1;
                    Arc::make_mut(&mut self.notes).push(Note { id, text });
                }
            }
            Msg::Composer(msg) => return self.composer.update(msg).map(Msg::Composer),
            Msg::Open(id) => {
                if self.note(id).is_some() {
                    self.open = Some(id);
                }
            }
            // Back closes whatever is on top: the dialog, the sheet, the screen.
            Msg::Back => {
                if self.confirm_delete.is_some() {
                    self.confirm_delete = None;
                } else if self.details_open {
                    self.details_open = false;
                } else {
                    self.open = None;
                }
            }
            Msg::AskDelete(id) => self.confirm_delete = Some(id),
            Msg::CancelDelete => self.confirm_delete = None,
            Msg::ConfirmDelete => {
                if let Some(id) = self.confirm_delete.take() {
                    self.delete(id);
                }
            }
            Msg::Undo => {
                if let Some((index, note)) = self.deleted.take() {
                    let notes = Arc::make_mut(&mut self.notes);
                    let index = index.min(notes.len());
                    notes.insert(index, note);
                }
            }
            Msg::SnackbarGone => self.deleted = None,
            Msg::ShowDetails => self.details_open = true,
            Msg::CloseDetails => self.details_open = false,
            Msg::Drawer(open) => self.drawer_open = open,
            Msg::Sort(index) => {
                self.newest_first = index == 1;
                self.drawer_open = false;
            }
        }
        Command::none()
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
            return Some(self.list_screen());
        }
        let id: u64 = key.strip_prefix("note:")?.parse().ok()?;
        Some(self.detail_screen(self.note(id)?))
    }

    fn dialog(&self) -> Option<Dialog<Msg>> {
        self.confirm_delete.map(|_| {
            Dialog::new("Delete note?")
                .message("You can undo this for a few seconds.")
                .cancel("Cancel", Msg::CancelDelete)
                .destructive("Delete", Msg::ConfirmDelete)
        })
    }

    fn sheet(&self) -> Option<BottomSheet<Msg>> {
        if !self.details_open {
            return None;
        }
        let note = self.note(self.open?)?;
        let words = note.text.split_whitespace().count();
        let characters = note.text.chars().count();
        Some(
            BottomSheet::new()
                .on_dismiss(Msg::CloseDetails)
                .child(Text::new("Details").size(20).bold())
                .child(caption(&format!("{words} words, {characters} characters")))
                .child(Button::new("Close").on_click(Msg::CloseDetails)),
        )
    }

    /// Back is ours while something can be closed; on the list with nothing
    /// open, the platform leaves the app.
    fn back(&self) -> Option<Msg> {
        (self.open.is_some() || self.details_open || self.confirm_delete.is_some())
            .then_some(Msg::Back)
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
                .spacing(12)
                .width_percent(1.0)
                .align_items(Align::Start)
                .child(Text::new(note.text.clone()).size(18))
                .child(Button::new("Details").on_click(Msg::ShowDetails)),
        )
        .into()
}

/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(Color::SECONDARY_TEXT)
}
```

The hooks and the scaffold are public data, so the whole flow can be tested without a device.
These replace the navigation tests from chapter 5 that used `Msg::Delete`:

```rust,ignore
#[test]
fn deleting_asks_first_and_can_be_undone() {
    let mut app = NotesApp::new();
    app.update(Msg::Open(1));
    app.update(Msg::AskDelete(1));
    let dialog = app.dialog().expect("the confirmation");
    assert_eq!(dialog.title, "Delete note?");
    // A cancel button lets Android's back button dismiss it.
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
```

The last test is modelled on `the_drawer_follows_state_and_picks_a_section` in
`example/rust/src/lib.rs`.

## Where this lives

- `rust/src/presentation.rs`: `Dialog`, `DialogRole`, `BottomSheet`.
- `rust/src/scaffold.rs`: `Fab`, `Drawer`, `DrawerItem`, `Snackbar`, and the tests at the bottom.
- `rust/src/host/session.rs`: `sync` (what is compared after each render), `sync_chrome`,
  `dialog_closed`, `sheet_dismissed`, `chrome_event`.
- `rust/src/host/platform.rs`: the `Controller` trait, the complete list of things Rust can ask
  the native chrome to do, and `ChromeEvent`, the complete list of things it reports.
- `shared/ios/KrabikChrome.swift`: the iOS stand-ins for the snackbar and the drawer.
- `shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt`: `showDialog`,
  `showSnackbar`, `setDrawer`, `setFab`.
- `example/rust/src/pages.rs`: `dialog`, `sheet` and `scaffold_chrome`.

Next: [7. Colours and dark mode](07-colours-and-dark-mode.md).
