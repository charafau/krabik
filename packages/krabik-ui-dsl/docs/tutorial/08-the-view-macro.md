# 8. The view macro

This chapter rewrites the Notes views with `view!`, a macro for writing a widget tree in a
nested, declarative form. It lists the macro's rules precisely, shows the builder and macro
versions of the same function side by side, and discusses when each reads better. Nothing new
happens at run time: by the end you will be able to translate between the two forms in your head.

## The same tree, written differently

Here is the note row as it stands, with builders:

```rust,ignore
fn note_row(note: &Note) -> Element<Msg> {
    Row::new()
        .width_percent(1.0)
        .height(ROW_HEIGHT)
        .padding_each(EdgeInsets::symmetric(12, 6))
        .spacing(8)
        .align_items(Align::Center)
        .background(palette::CARD)
        .border(1, palette::CARD_EDGE)
        .corner_radius(10)
        .child(
            Text::new(note.text.clone())
                .color(palette::ON_CARD)
                .max_lines(1)
                .ellipsis()
                .grow(1),
        )
        .child(
            Button::new("Open")
                .on_click(Msg::Open(note.id))
                .color(palette::ON_CARD)
                .accessibility_label(format!("Open {}", note.text)),
        )
        .into()
}
```

and with `view!`:

```rust,ignore
fn note_row(note: &Note) -> Element<Msg> {
    view! {
        Row(
            width_percent: 1.0,
            height: ROW_HEIGHT,
            padding_each: EdgeInsets::symmetric(12, 6),
            spacing: 8,
            align_items: Align::Center,
            background: palette::CARD,
            corner_radius: 10,
        ).border(1, palette::CARD_EDGE) {
            Text(
                note.text.clone(),
                color: palette::ON_CARD,
                max_lines: 1,
                ellipsis,
                grow: 1,
            )
            Button(
                "Open",
                on_click: Msg::Open(note.id),
                color: palette::ON_CARD,
                accessibility_label: format!("Open {}", note.text),
            )
        }
    }
}
```

The second form separates two things the first one mixes: a node's own settings go in the
parentheses, its children go in the braces. The nesting on the page is the nesting of the tree.
There is no `.child(` and no trailing `.into()`.

The macro does not interpret any of this. It rewrites it, token for token, into the builder
calls of the first form. The header of `rust/macros/src/view.rs` states the design:

```rust,ignore
//! The macro is syntax only. Every node becomes the builder calls an
//! application could write by hand, so any builder works without being
//! registered here, including ones an application defines itself.
```

That is why `max_lines`, `ellipsis` and `border` work in the macro without the macro knowing
about them: `max_lines: 1` becomes `.max_lines(1)` and the compiler checks that such a method
exists. There is no list of widgets or properties inside the macro to keep in sync. A widget
added to the framework tomorrow, or a builder type you write yourself, works on the first day.

The framework's own tests pin the equivalence down. The test
`the_view_macro_builds_the_same_tree_as_the_builders` in `rust/src/ui.rs` writes one tree both
ways and asserts the results are equal.

To use the macro, import it:

```rust,ignore
use krabik_ui_dsl::view;
```

The widget types the macro names (`Row`, `Text`, ...) must be in scope as well, exactly as if you
had written `Row::new()` yourself.

## The rules

These are the rules from the macro's documentation in `rust/macros/src/lib.rs`, each with what
it expands to.

### A node

`Name(...)` calls `Name::new(...)`.

| You write | It becomes |
| --------- | ---------- |
| `Spacer` | `Spacer::new()` |
| `Text("Hello")` | `Text::new("Hello")` |
| `Checkbox(title, done)` | `Checkbox::new(title, done)` |

A name is treated as a node when it is capitalized. More exactly, when any segment of its path
starts with an uppercase letter, so `ui::Column` is a node too (`starts_with_node` in `view.rs`).
Builders are types, and types are capitalized; that is the whole convention.

### Arguments and settings

Inside the parentheses, plain expressions come first and go to the constructor. After them,
`name: value` calls `.name(value)`:

| You write | It becomes |
| --------- | ---------- |
| `Text("Hello", size: 20)` | `Text::new("Hello").size(20)` |
| `Column(padding: 16, spacing: 8)` | `Column::new().padding(16).spacing(8)` |
| `Button("Add", on_click: Msg::Add)` | `Button::new("Add").on_click(Msg::Add)` |

The order of settings is the order of the method calls. For builders that does not matter.

A method without arguments is written as a bare name, **after at least one `name: value`**:

| You write | It becomes |
| --------- | ---------- |
| `Text("Hello", size: 20, bold)` | `Text::new("Hello").size(20).bold()` |
| `List(rows, item_height: 56, recycled)` | `List::new(rows).item_height(56).recycled()` |

The restriction exists because a bare name before any setting is indistinguishable from a
constructor argument: in `Text("Hello", bold)` the macro has to assume `bold` is a variable you
are passing to `new`. If a flag is the only setting, use a method call instead (next rule):
`Text("Hello").bold()`.

Once the settings have started, a constructor argument is an error, with the message
"constructor arguments must come before `name: value` settings".

### Method calls after a node

Any chain of method calls may follow the parentheses:

```rust,ignore
Text("Hello").bold()
Button("Add", on_click: Msg::Add).enabled(false)
Row(spacing: 8).border(1, palette::CARD_EDGE) { ... }
```

This is the way out for methods that do not fit `name: value`: a method with two arguments
(`border`), or a flag on a node with no other settings.

### Another constructor

When the last segment of the path is lowercase, it is taken as the constructor's name and called
as written:

| You write | It becomes |
| --------- | ---------- |
| `ScrollView::horizontal()` | `ScrollView::horizontal()` |
| `DatePicker::time(self.time)` | `DatePicker::time(self.time)` |
| `ProgressIndicator::indeterminate()` | `ProgressIndicator::indeterminate()` |

### Children

A `{ ... }` block after a node adds each thing inside it with `.child(...)`. Children are
separated by line breaks; commas and semicolons between them are allowed and ignored.

```rust,ignore
Column(spacing: 8) {
    Text("One")
    Text("Two")
}
```

becomes, in effect:

```rust,ignore
{
    let mut __node = Column::new().spacing(8);
    __node = __node.child(Text::new("One"));
    __node = __node.child(Text::new("Two"));
    __node
}
```

The children are added one statement at a time, not in one chained expression. The comment on
`child_statements` in `view.rs` gives the reason: "so `if`, `for` and `match` can simply be the
Rust control flow they look like".

A node can only have a block if its builder has a `child` method. `Column`, `Row`, `ScrollView`,
`Pager`, `Tabs` and `TabsView` do; a `Text` with a block is a compile error from the missing
method, as it would be by hand.

### Ordinary Rust as a child

Anything in a block that is not capitalized is an ordinary Rust expression. It is evaluated and
passed to `.child(...)`:

```rust,ignore
Column {
    caption(&self.summary())                 // a view function
    self.composer.view().map(Msg::Composer)  // a child component
    note_row(note)                           // another view function
}
```

This is how the pieces from chapter 4 fit in: view functions and mapped child components are
embedded as they are. The expression must produce something `child` accepts, which is a widget
builder or an `Element`.

Two edges to know. A `{ ... }` block on its own, with no node before it, is rejected with "a
`{ ... }` block here has no node to belong to". And a child that *is* capitalized is always read
as a node, so a constant such as `FOOTER` would be turned into `FOOTER::new()`. Wrap such a value
in a function call.

### Control flow

`if`, `if let`, `else`, `for` and `match` work inside a block, with nodes in their bodies:

```rust,ignore
view! {
    Column(spacing: 8, width_percent: 1.0) {
        if self.notes.is_empty() {
            caption("Nothing here yet.")
        } else {
            caption(&self.summary())
        }
        for note in self.notes.iter() {
            Text(note.text.clone(), key: note.id.to_string(), padding: 12)
        }
        match self.open {
            Some(id) => Text(format!("Note {id} is open")),
            None => {}
        }
    }
}
```

Compare the `for` with chapter 2, where the same loop needed a `let mut` and a reassignment. In
a `match`, an arm's body is one child, or a `{ ... }` block of any number of children; `{}` means
"nothing in this case".

Control flow is only allowed *inside* a container. The root of `view!` must be a single node or
expression. Two roots, or an `if` at the top, are errors with messages that say what to do:
"view! takes one root node; wrap several in a `Column { ... }` or `Row { ... }`".

### The result

`view!` produces an `Element`. The expansion ends by wrapping the root in
`::krabik_ui_dsl::ui::Element::from(...)`, which replaces the `.into()` at the end of a builder
chain. Two consequences:

- The macro refers to the framework by the crate name `krabik_ui_dsl`. If you rename the
  dependency in `Cargo.toml`, the macro stops finding it.
- The root must be something that converts into an `Element`. A `Scaffold`, a `Dialog` and a
  `BottomSheet` do not, so they are still written with builders. Their *contents* can use the
  macro: `Scaffold::new().body(view! { ... })`.

## When something goes wrong

Because the macro only rewrites, most mistakes surface as the compiler errors you would get from
the hand-written form. A misspelt setting, `Text("x", sise: 20)`, is "no method named `sise`
found for struct `Text`". A wrong argument type is a type mismatch on that argument. Read the
error as if you had written `.sise(20)`.

The macro's own errors are few and are about shape: no root, two roots, a constructor argument
after a setting, a dangling block, or a `.name` without parentheses ("expected a method call such
as `.bold()`"). Each is tested at the bottom of `view.rs`.

## Which form to use

Neither is better everywhere. Both forms can be mixed freely in one file, and in one function.

**`view!` reads better when the code is mostly structure**: a screen body with containers inside
containers, settings on each, and children that come and go with `if`. The Notes list body and
detail body are like that. It is also the form to prefer when people who do not write much Rust
will read the views.

**Builders read better when the code is mostly logic.** A function that returns one configured
widget (`caption`) gains nothing from a macro. Code that builds a container incrementally with
early returns, or picks between builders and then applies common settings, is plain Rust and
should look like it. And `Scaffold`, `AppBar`, `Dialog`, `BottomSheet` and friends are builders
only.

Some practical differences to weigh:

- **Tooling.** A builder chain is ordinary Rust: `rustfmt` formats it and an editor completes
  method names after the dot. The inside of `view!` is custom syntax; expect `rustfmt` to leave
  it as you typed it and completion to be weaker. The showcase (`example/rust/src/pages.rs`, some
  1,300 lines of views) is written entirely with builders, and the minimal todo example entirely
  with the macro, so you can judge both at size.
- **Methods with several arguments** need the `.method(a, b)` escape in the macro and are natural
  in a chain.
- **Compile time.** `view!` is a procedural macro and runs at every build of the crate. For the
  sizes in this tutorial that is not noticeable.

A reasonable default: `view!` for the body of each screen and each component, builders for small
helpers and for everything around the body.

## The final Notes app

The four view functions (`Composer::view`, `list_body`, `note_row`, `detail_body`) are now
written with the macro. Everything else is as in chapters 6 and 7. This is the complete file.

`notes/rust/src/lib.rs`:

```rust
use std::sync::Arc;

use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::navigation::{Navigation, Route, ScreenTransition};
use krabik_ui_dsl::presentation::{BottomSheet, Dialog};
use krabik_ui_dsl::scaffold::{AppBar, Drawer, DrawerItem, Fab, Scaffold, Snackbar};
use krabik_ui_dsl::ui::{
    Align, Button, Column, EdgeInsets, Element, Icon, List, Row, ScrollView, Text, TextField,
};
use krabik_ui_dsl::view;

/// The height the list reserves for each note.
const ITEM_HEIGHT: f32 = 64.0;
/// The height of the card drawn inside it; the rest is the gap between cards.
const ROW_HEIGHT: f32 = 56.0;

/// The app's colours. Each has a value for the light and for the dark
/// appearance; the framework picks one when it builds the tree.
mod palette {
    use krabik_ui_dsl::ui::Color;

    /// Captions and counts. The framework's own secondary text colour.
    pub const MUTED: Color = Color::SECONDARY_TEXT;
    /// The card a note sits on: paper yellow by day, dim brown by night.
    pub const CARD: Color = Color::adaptive(0xFFFF_F4C2, 0xFF3A_3320);
    /// The hairline around a card.
    pub const CARD_EDGE: Color = Color::adaptive(0xFFE8_D98A, 0xFF5A_5030);
    /// Text on a card. Dark brown on yellow, pale yellow on brown.
    pub const ON_CARD: Color = Color::adaptive(0xFF3E_2F00, 0xFFF3_E7B3);
}

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

    fn update(&mut self, msg: ComposerMsg) {
        match msg {
            ComposerMsg::Draft(text) => self.draft = text,
            // What a submitted draft becomes is the parent's decision;
            // see `NotesApp::update`.
            ComposerMsg::Submit => {}
        }
    }

    fn view(&self) -> Element<ComposerMsg> {
        view! {
            TextField(
                self.draft.clone(),
                placeholder: "Write a note",
                on_change: ComposerMsg::Draft,
                on_submit: ComposerMsg::Submit,
                width_percent: 1.0,
            )
        }
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
        view! {
            Column(padding: 16, spacing: 12, width_percent: 1.0, height_percent: 1.0) {
                self.composer.view().map(Msg::Composer)
                caption(&self.summary())
                List(
                    self.visible(),
                    item_height: ITEM_HEIGHT,
                    key: |note| note.id.to_string(),
                    row: note_row,
                )
            }
        }
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
    }

    /// Not shown: with navigation, the first screen is the root.
    fn view(&self) -> Element<Msg> {
        self.list_body()
    }
}

impl App for NotesApp {
    fn navigation(&self) -> Option<Navigation> {
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
    view! {
        Row(
            width_percent: 1.0,
            height: ROW_HEIGHT,
            padding_each: EdgeInsets::symmetric(12, 6),
            spacing: 8,
            align_items: Align::Center,
            background: palette::CARD,
            corner_radius: 10,
        ).border(1, palette::CARD_EDGE) {
            Text(
                note.text.clone(),
                color: palette::ON_CARD,
                max_lines: 1,
                ellipsis,
                grow: 1,
            )
            Button(
                "Open",
                on_click: Msg::Open(note.id),
                color: palette::ON_CARD,
                accessibility_label: format!("Open {}", note.text),
            )
        }
    }
}

/// The content of the detail screen.
fn detail_body(note: &Note) -> Element<Msg> {
    view! {
        ScrollView(width_percent: 1.0, height_percent: 1.0) {
            Column(padding: 16, spacing: 12, width_percent: 1.0, align_items: Align::Start) {
                Text(note.text.clone(), size: 18)
                Button("Details", on_click: Msg::ShowDetails)
            }
        }
    }
}

/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(palette::MUTED)
}
```

The tests from chapters 4 to 6 pass unchanged against this file. That is the practical proof
that the macro changed how the views are written and nothing about what they are.

## Where this lives

- `rust/macros/src/lib.rs`: the `view!` entry point and its documented rules.
- `rust/macros/src/view.rs`: the parser (`Child`, `Node`, `Setting`) and the expansion
  (`node_expression`, `child_statements`), with tests for every rule and every error message.
- `rust/src/ui.rs`: `the_view_macro_builds_the_same_tree_as_the_builders`.
- `rust/src/lib.rs`: `extern crate self as krabik_ui_dsl`, which lets the macro's
  `::krabik_ui_dsl::` paths work inside the framework itself, and the re-export
  `pub use krabik_ui_dsl_macros::view`.
- `example-todo-minimal/rust/src/lib.rs`: a whole app written with `view!`.

The other macro in that crate, `#[derive(Widget)]`, is for the framework's own widgets and is
covered in [`../internals/`](../internals/).

Next: [9. Testing](09-testing.md).
