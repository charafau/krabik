# 4. Components

This chapter splits the Notes app into two components: the app itself and a *composer*, the part
with the text field and the Add button. You will see a child component with its own message type,
how `Element::map` places its view inside the parent's, and how the parent forwards messages in
`update`. The chapter ends with the question that matters more than the mechanics: when a
component is worth it, and when a function that returns an `Element` is enough.

## Two ways to split a view

A `view` function grows. There are two tools for breaking it up, and they are not the same thing.

**A view function** is any function that returns a widget or an `Element`. It has no state of its
own; it turns its arguments into UI. You already have one: `note_row(note: &Note) -> Element<Msg>`.
It sends the *app's* messages (`Msg::Delete`), so it only makes sense inside this app.

**A component** is a type that implements `Component`: it owns state, has its own message enum,
and has its own `update`. It knows nothing about the app around it.

Start with the cheap tool.

### A view function

The line that counts the notes is small, grey text. The app will have more of those. Give the
style one name:

```rust,ignore
/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(Color::SECONDARY_TEXT)
}
```

Two details make this function more useful than it looks.

It is generic over the message type `M`. A `Text` sends no messages, so it can be a `Text<M>` for
any `M`, and the caller's context decides. That lets every component in the app use the same
helper. The showcase has exactly this function (`caption` in `example/rust/src/pages.rs`) and
uses it from both the app and its checklist component.

It returns `Text<M>`, the builder, not `Element<M>`. The caller can keep configuring it:
`caption("...").grow(1)`. Return the builder when the result is one widget, and `Element<M>` when
the function assembles several, as `note_row` does.

## The composer as a component

The text field and the Add button belong together. They share one piece of state, the draft, and
that state is of no interest to the rest of the app until the user presses Add. Today the draft
sits in `NotesApp` next to the notes, and `Msg` has two variants that exist only for those two
widgets. That is the signal that a component is hiding in there: *state and messages that only
one part of the view cares about*.

Pull it out. The component gets its own message type:

```rust,ignore
/// Everything the user can do in the composer.
#[derive(Clone, Debug, PartialEq)]
pub enum ComposerMsg {
    /// The text in the field changed.
    Draft(String),
    /// The Add button or the keyboard's action key was pressed.
    Submit,
}
```

its own state:

```rust,ignore
/// The field a new note is typed into.
pub struct Composer {
    draft: String,
}
```

and its own `Component` implementation:

```rust,ignore
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
```

This is the same trait the app implements. There is no separate "child component" concept in the
framework: a component is a struct with `update` and `view`, and the root one additionally
implements `App`. Notice what the composer's `view` returns: `Element<ComposerMsg>`. Its widgets
send *its* messages. It could be compiled and tested with no `NotesApp` in existence.

Now that the button lives next to the draft, it can be disabled while there is nothing to add.
That is a small improvement that would have been easy to write before too, but it is easier to
*notice* when the state and the view that depend on each other sit in the same thirty lines.

## Placing the child's view: `Element::map`

The parent keeps the child in its state:

```rust,ignore
pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    next_id: u64,
    composer: Composer,
}
```

and wants to show it:

```rust,ignore
Column::new()
    // ...
    .child(self.composer.view())   // does not compile
```

This is a type error, and a useful one. The column is a `Column<Msg>`; it accepts children that
send `Msg`. The composer's view is an `Element<ComposerMsg>`. If this compiled, a `ComposerMsg`
would arrive in `NotesApp::update`, which has no idea what to do with one.

The fix has two halves. First, the parent's message type gets a variant that *wraps* the child's:

```rust,ignore
pub enum Msg {
    /// Something happened in the composer.
    Composer(ComposerMsg),
    /// Deletes the note with this id.
    Delete(u64),
}
```

Second, the child's view is converted with `map`:

```rust,ignore
.child(self.composer.view().map(Msg::Composer))
```

`map` takes a function from the child's message to the parent's and returns an `Element<Msg>`.
`Msg::Composer` is such a function, `fn(ComposerMsg) -> Msg`. From its definition in
`rust/src/element.rs`:

```rust,ignore
/// Changes the message type, so a child component's view can be placed
/// inside its parent's:
pub fn map<N: 'static>(self, wrap: impl Fn(M) -> N + Send + Sync + 'static) -> Element<N>
```

What it does is walk the child's element tree and wrap every handler in it: where a button would
have produced `ComposerMsg::Submit`, it now produces `Msg::Composer(ComposerMsg::Submit)`. Nothing
else about the elements changes. The same applies to the rows a `List` inside the child will
build later, when it scrolls; `ListProps::map_messages` wraps the row builder so future rows are
mapped too. (The test `mapping_also_covers_rows_a_list_builds_later` in `rust/src/element.rs`
pins that down.)

```text
   Composer::view()                  after .map(Msg::Composer)

   Row                               Row
   +-- TextField                     +-- TextField
   |     on_change: Draft(text)      |     on_change: Composer(Draft(text))
   |     on_submit: Submit           |     on_submit: Composer(Submit)
   +-- Button                        +-- Button
         on_click: Submit                  on_click: Composer(Submit)

   Element<ComposerMsg>              Element<Msg>
```

## Forwarding in `update`

Messages from the composer now arrive in the app's `update` wrapped in `Msg::Composer`. The basic
move is to unwrap and pass them on:

```rust,ignore
Msg::Composer(msg) => self.composer.update(msg),
```

That one line is the whole "protocol" between a parent and a child. The framework is not involved:
it calls the root's `update`, and the root calls its children's. The app's doc comment in
`rust/src/app.rs` describes the pattern in one sentence: "the parent keeps the child in its
state, shows `child.view().map(Msg::Child)` and forwards `Msg::Child(msg)` to
`child.update(msg)`." The showcase does exactly this for its checklist page:
`Msg::Checklist(msg) => self.checklist.update(msg)` in `example/rust/src/lib.rs`.

### When the parent needs to hear something

The composer has one event the app cares about: the user submitted a draft. `Component::update`
returns nothing, so a child cannot hand a result back through it. This is a real limit of the
trait as it stands; there is no "output event" mechanism. But the parent sees every child message
before the child does, and it can match on the ones it cares about:

```rust,ignore
fn update(&mut self, msg: Msg) {
    match msg {
        Msg::Composer(ComposerMsg::Submit) => {
            if let Some(text) = self.composer.take() {
                let id = self.next_id;
                self.next_id += 1;
                Arc::make_mut(&mut self.notes).push(Note { id, text });
            }
        }
        Msg::Composer(msg) => self.composer.update(msg),
        Msg::Delete(id) => Arc::make_mut(&mut self.notes).retain(|note| note.id != id),
    }
}
```

The first arm intercepts `Submit`. The second forwards everything else. Order matters: Rust tries
match arms from the top, so the specific one must come first.

`take` is an ordinary method on the composer that hands over the draft and clears the field:

```rust,ignore
impl Composer {
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
```

The division of knowledge is now clean. The composer knows what a valid draft is (not blank,
trimmed). The app knows what a note is (it has an id and lives in a list). Neither knows the
other's part.

This interception pattern is a convention of this tutorial, not something the framework
prescribes. It is plain Rust: a `match` on a nested enum and a method call. Other shapes work
too, for example an inherent method on the child that takes the message and returns an
`Option<Output>`, which the parent calls instead of `update`. Use whichever reads best; the
framework only requires that the root's `update(msg)` ends up changing the right state.

## When is a component worth it?

A component costs something: a message enum, a wrapping variant, a `map`, a forwarding arm. Here
is a way to decide.

**Use a view function when the piece has no state of its own.** `note_row` and `caption` are
view functions. A row shows a note; the note lives in the app. Turning each row into a component
would mean a vector of row components to keep in sync with the vector of notes, for no gain.
Most of an app's view code should be functions like these.

**Use a component when all of these hold:**

- The piece has state that the rest of the app does not read. The draft is the example; the
  showcase's checklist with its tasks, notes and "hide completed" switch is a larger one
  (`example/rust/src/checklist.rs`).
- It has several messages that only it handles. If the parent's `update` would only forward
  them, they belong to a child.
- You can describe what it does without mentioning the parent. "A field for typing a note" passes.
  "The second row of the notes screen" does not.

**Reasons that are not good enough on their own:**

- "The `view` function is long." Split it into view functions.
- "I want to reuse this look." That is a view function, perhaps generic over `M` like `caption`.
- "It is a different screen." Screens are a navigation concept (chapter 5). A screen's content
  can be a component, as the checklist is, but many screens are a function of the app's
  state, as every other showcase page is.

A component also makes testing more focused. The composer's rules can be tested with a
`Composer` alone, and the app's tests can then assume a submitted draft is valid.

One practical note on files. Everything in this tutorial stays in `lib.rs` so each listing is
complete. In a real project a component gets its own module; the showcase's `checklist.rs` is the
model, with `mod checklist;` and `use checklist::Checklist;` in `lib.rs` and a message variant
`Checklist(checklist::Msg)`.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use std::sync::Arc;

use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::ui::{
    Align, Button, Color, Column, EdgeInsets, Element, List, Row, Text, TextField,
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
    /// Deletes the note with this id.
    Delete(u64),
}

/// The state of the app.
pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    /// The id the next note gets. Ids are never reused.
    next_id: u64,
    composer: Composer,
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
        }
    }

    /// Registers the app with the framework. The host calls this at startup.
    pub fn install() {
        set_app_factory(Self::new);
    }

    fn summary(&self) -> String {
        match self.notes.len() {
            0 => "No notes yet".to_string(),
            1 => "1 note".to_string(),
            count => format!("{count} notes"),
        }
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
            Msg::Delete(id) => Arc::make_mut(&mut self.notes).retain(|note| note.id != id),
        }
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .child(Text::new("Notes").size(30).bold())
            .child(caption(&self.summary()))
            .child(self.composer.view().map(Msg::Composer))
            .child(
                List::new(Arc::clone(&self.notes))
                    .item_height(ITEM_HEIGHT)
                    .key(|note| note.id.to_string())
                    .row(note_row),
            )
            .into()
    }
}

impl App for NotesApp {}

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
            Button::new("Delete")
                .on_click(Msg::Delete(note.id))
                .accessibility_label(format!("Delete {}", note.text)),
        )
        .into()
}

/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(Color::SECONDARY_TEXT)
}
```

The tests now come in two groups, one per component:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_composer_hands_over_a_trimmed_draft_once() {
        let mut composer = Composer::new();
        assert_eq!(composer.take(), None);

        composer.update(ComposerMsg::Draft("  Buy milk  ".to_string()));
        assert_eq!(composer.take().as_deref(), Some("Buy milk"));
        // Taking clears the field.
        assert_eq!(composer.take(), None);
    }

    #[test]
    fn a_submitted_draft_becomes_a_note() {
        let mut app = NotesApp::new();
        app.update(Msg::Composer(ComposerMsg::Draft("Buy milk".to_string())));
        app.update(Msg::Composer(ComposerMsg::Submit));
        let added = app.notes.last().unwrap();
        assert_eq!((added.id, added.text.as_str()), (3, "Buy milk"));

        // Submitting an empty field adds nothing.
        app.update(Msg::Composer(ComposerMsg::Submit));
        assert_eq!(app.notes.len(), 3);
    }
}
```

The second test sends the wrapped messages, exactly what the framework would deliver after the
`map`.

## Where this lives

- `rust/src/element.rs`: `Element::map`, `RawElement::map_messages`, and the two tests at the
  bottom of the file that show a child lifted into a parent.
- `rust/src/message.rs`: `Handler::map`, the one place a handler is re-wrapped.
- `rust/src/widgets/list.rs`: `ListProps::map_messages`, mapping for rows built later.
- `example/rust/src/checklist.rs` and the `Msg::Checklist` arm in `example/rust/src/lib.rs`: a
  real child component.

Why handlers can be re-wrapped without the engine knowing any message type is explained in
[`../internals/`](../internals/).

Next: [5. Navigation and the scaffold](05-navigation-and-scaffold.md).
