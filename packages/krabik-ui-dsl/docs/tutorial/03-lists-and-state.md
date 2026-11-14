# 3. Lists and state

This chapter gives each note an identity, adds a Delete button to every row, and replaces the
hand-built column of notes with `List`, the widget for long lists. Along the way it explains
messages that carry data, what a `key` is and why a list cannot work without one, and what
`item_height` and `revision` are for. Afterwards you will be able to show a collection of any
size and know what the framework does with it.

## Notes get an identity

So far a note is a `String` and the app can only add. To delete a specific note, a message has to
say *which* one. An index would work until the list is sorted or filtered, so give each note an id
that never changes:

```rust,ignore
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    id: u64,
    text: String,
}

pub struct NotesApp {
    notes: Vec<Note>,
    /// The id the next note gets. Ids are never reused.
    next_id: u64,
    draft: String,
}
```

## Messages with payloads

There are two shapes of message with data, and you have already used one.

**The widget supplies the data.** A text field knows the new text; the app does not. So
`on_change` takes a function from the payload to a message, and the widget calls it when the
event happens:

```rust,ignore
TextField::new(self.draft.clone()).on_change(Msg::Draft)
```

`Msg::Draft` here is the variant's constructor, a function `fn(String) -> Msg`. A closure works
too when the payload needs company: `on_toggle(move |done| Msg::SetDone(id, done))` is how the
minimal todo example marks one specific task (`todo_row` in
`example-todo-minimal/rust/src/lib.rs`).

**The app supplies the data.** A Delete button reports no payload; a click is a click. But when
you build the button you know which note it belongs to, so you build the complete message then:

```rust,ignore
Button::new("Delete").on_click(Msg::Delete(note.id))
```

`Msg::Delete(note.id)` is evaluated while `view` runs. Each row's button holds its own finished
message value, and a click hands a clone of it to `update`. There is nothing to look up at click
time and no way for a row to send another row's id.

The new message and its handler:

```rust,ignore
pub enum Msg {
    Draft(String),
    Add,
    /// Deletes the note with this id.
    Delete(u64),
}

// in update:
Msg::Delete(id) => self.notes.retain(|note| note.id != id),
```

## Keys: who is who between two renders

Suppose the notes are still drawn with the loop from chapter 2, now with a row per note:

```rust,ignore
for note in &self.notes {
    notes = notes.child(
        Row::new()
            .child(Text::new(note.text.clone()).grow(1))
            .child(Button::new("Delete").on_click(Msg::Delete(note.id))),
    );
}
```

The user deletes the first of three notes. `view` runs again and returns a column with two rows.
The framework now has an old tree with three rows and a new tree with two, and it has to decide
which native views to keep. It does that with **node identifiers**. Every node gets one, derived
from three things (`node_id` in `rust/src/engine/tree.rs`):

```rust,ignore
mix(parent.0);
mix(tag);
match key {
    Some(key) => mix(hash_string(key)),
    None => mix(index as u64),
}
```

The parent's identifier, a tag for the kind of widget, and then either the node's **key** or, if
it has none, its **position** among its siblings. Two nodes in two renders with the same
identifier are "the same node": the native view is kept and only its changed properties are
updated.

Without keys, identity is position:

```text
  before              after deleting A     what the diff concludes
  index 0: row "A"    index 0: row "B"     row 0: text changed A -> B
  index 1: row "B"    index 1: row "C"     row 1: text changed B -> C
  index 2: row "C"                         row 2: removed
```

The screen ends up correct, but the framework did more work than necessary, and it removed the
wrong native view: the one that showed C, not the one that showed A. Anything that lives in the
native view rather than in your state goes with the position, not with the note. A focused text
field in row 0 stays focused, now showing a different note's text. An exit animation plays on the
last row instead of the deleted one.

With a key, identity is the key:

```rust,ignore
Row::new()
    .key(note.id.to_string())
    // ...
```

```text
  before              after deleting A     what the diff concludes
  key 1: row "A"      key 2: row "B"       key 1: removed
  key 2: row "B"      key 3: row "C"       key 2: moved up
  key 3: row "C"                           key 3: moved up
```

`.key(...)` is available on every widget. It accepts a `&'static str` or an owned `String`. The
rules are in its doc comment in `rust/src/element.rs`: keys must be unique among siblings and
stable across rebuilds. A duplicate key would give two siblings the same identifier and silently
merge their native views, so debug builds stop with a panic instead (`build_children` in
`rust/src/engine/tree.rs`): "duplicate element key ... among the children of ...".

When do you need a key? Whenever siblings can be inserted, removed or reordered: anything built in
a loop over data. Fixed layouts ("a title, then a field, then a list") do not need keys, because
position is a perfectly good identity there. One conditional child in a fixed layout is the
borderline case: if the hint above a list comes and goes, everything after it shifts by one
position, so key the things after it or the hint itself.

## Why a ScrollView is not a list

The loop above creates a row for every note, and the engine creates a native view for every row,
on screen or not. For twenty notes that is fine. For two thousand it means two thousand layout
computations and several thousand native views at startup.

`List` solves that by building only the rows near the visible area:

```rust,ignore
List::new(self.notes.clone())
    .item_height(ITEM_HEIGHT)
    .key(|note| note.id.to_string())
    .row(note_row)
```

Four things are given: the items, the height of a row, how to get an item's key, and how to build
an item's row. Compare this with the loop. You no longer build rows; you hand the list a function
and it calls the function for the items it decides to show. From the doc comment of `List::row`
in `rust/src/widgets/list.rs`: "Called only for visible items."

`key` and `row` are both required. Converting a list without them into an element panics with
"List::key must be set" or "List::row must be set".

The row builder is a plain function:

```rust,ignore
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
```

The text has `.grow(1)` so it fills the row and pushes the button to the trailing edge, and it is
limited to one line with an ellipsis so every row has the same height. The button gets an
`accessibility_label` that names the note: twenty buttons that all announce "Delete" are useless
to a screen reader, and UI tests find views by that label too (chapter 9).

### What the framework builds from a List

A `List` is not a native widget of its own in the default mode. While the tree is built it is
expanded into a native scroll view that contains three things (`build_list` and
`build_fixed_rows` in `rust/src/engine/tree.rs`):

```text
  scroll view  (the List's own node)
  +---------------------------------+
  | spacer:                         |   stands in for the rows above
  |   start * item_height           |
  +---------------------------------+
  | row 14   key "14"               |
  | row 15   key "15"               |   the window: only these exist
  | ...                             |   as elements and native views
  | row 27   key "27"               |
  +---------------------------------+
  | spacer:                         |   stands in for the rows below
  |   (count - end) * item_height   |
  +---------------------------------+
```

The two spacers make the scroll view's content as tall as the whole list, so the scrollbar and
the fling distance are right, while only the rows in the window are real. The window is the rows
that fit in the viewport plus `buffer` extra rows on each side (3 by default; `.buffer(n)` changes
it), so a small scroll does not immediately need new rows.

When the user scrolls, the native scroll view reports its offset. The engine checks whether the
window has moved to different rows (`Runtime::scroll` in `rust/src/engine/runtime.rs`); only then
does it rebuild, calling your row function for the rows that entered.

### Why the list needs `key`

Each row in the window is wrapped in a keyed node:

```rust,ignore
row.key = Some(Cow::Owned((props.keys)(index)));
row.children.push((props.rows)(index));
```

(from `build_fixed_rows`)

This is the same mechanism as in the previous section, and here it is essential. When the window
moves from rows 14 to 27 to rows 15 to 28, the position of every row inside the window changes:
what was the second child is now the first. If identity were position, every native row would be
rewritten with its neighbour's content on every scroll step. With keys, thirteen rows keep their
identifiers and their native views untouched, one is removed and one is created. The doc comment
on `List` puts it this way: keys "keep a row's native view alive while it scrolls through the
window".

So the key function must return something unique per item and stable over time. An id is ideal.
The item's text works only if texts cannot repeat; the showcase's list pages use it because their
items are "Windowed row 1" to "Windowed row 200".

### `item_height`

With `.item_height(64)` every row is exactly 64 points tall. That makes the window pure
arithmetic (`fixed_window` in `rust/src/engine/tree.rs`): the first visible row is the scroll
offset divided by the height. Nothing has to be measured, and the list can hold any number of
items.

The list wraps each row in a container of that height. The Notes rows are 56 points tall inside a
64 point slot, and the 8 points left over at the bottom of each slot are the gap between rows:

```rust,ignore
/// The height the list reserves for each note.
const ITEM_HEIGHT: f32 = 64.0;
/// The height of the card drawn inside it; the rest is the gap between cards.
const ROW_HEIGHT: f32 = 56.0;
```

Without `item_height` rows may have different heights. The list then has to find out how tall
each row is: it lays out the rows in the window, records their heights under their keys, and uses
`estimated_height` (56 by default) for rows it has not seen yet. A render may take up to three
build-and-layout passes while the measured heights settle (the loop in `Runtime::update`). The
doc comment sums up the trade: this "tolerates mixed row heights at the cost of a measurement
pass". Use a fixed height whenever the design allows it.

### `revision`

`revision` is a hint that only matters for lists *without* `item_height`.

A variable-height list keeps a cache per list: the measured height of each row and the running
totals that give each row's offset. That cache is only valid for one particular sequence of items.
When the items change, it must be thrown away and the scroll position reset. The question is how
the engine knows the items changed, given that all it has is a count and two closures.

Without a revision it finds out the hard way: on every build it calls your key function for
*every* item, hashes the keys together and compares the result with last time
(`build_variable_rows` in `rust/src/engine/tree.rs`, the branch commented "No revision supplied").
That is correct but costs time proportional to the list length on every render, including renders
caused by an unrelated button elsewhere on the screen.

With a revision you tell it:

```rust,ignore
List::new(self.notes.clone())
    .revision(self.notes_revision)   // a u64 you bump whenever the notes change
    .key(|note| note.id.to_string())
    .row(note_row)
```

Now an unchanged number means "same items as last time", and the build only touches the rows in
the window. From the doc comment of `List::revision`:

```rust,ignore
/// Change the value whenever the items, their order, or the strings their
/// keys derive from change. [...] It must also change
/// when a different list takes over the same position in the tree (for
/// example after a screen swap).
///
/// Without it the list stays correct but is fully re-keyed on every build,
/// which caps it at hundreds of rows.
```

The contract is yours to keep. If you change the items and forget to change the revision, the
list uses stale heights.

One thing surprised me while reading the source: for a list *with* `item_height`, the revision is
stored and compared but never consulted when rows are built; `build_fixed_rows` does not read it.
A fixed-height list has no cache to invalidate. Notes uses a fixed height, so it does not set a
revision.

### `.recycled()`

The default mode is called *windowed*: Rust decides which rows exist. The alternative hands that
job to the platform's own list widget:

```rust,ignore
List::new(items)
    .item_height(ITEM_HEIGHT)
    .recycled()
    .key(|note| note.id.to_string())
    .row(note_row)
```

"On Android this swaps the windowed scroll container for a `RecyclerView` and on iOS for a
`UICollectionView`" (doc comment of `List::recycled`). The native widget owns scrolling and reuses
a small pool of row views; when it needs a row it asks Rust to build the item at that index, and
Rust diffs the new row against whatever the reused view showed before. `.row_transition(180)`
cross-fades a row when it is rebound to new content.

Which to use:

- **Windowed** (default) is simpler. The rows are ordinary nodes in the same tree as everything
  else. Good for lists up to a few hundred rows.
- **Recycled** scrolls long lists the way the platform's own apps do, because the platform's list
  drives it. The showcase uses it for its 1000-row page, and the minimal todo app uses it too.

The API is identical, so switching is one word. On your development machine there is no native
list, so a recycled list is built as a windowed one there; the condition is in `build_list`:

```rust,ignore
if props.mode == ListMode::Recycled && cfg!(any(target_os = "android", target_os = "ios")) {
```

That keeps unit tests working without any special case in your code.

### Three constraints worth knowing

**The list sizes itself.** Converting a `List` into an element sets its width to 100% and its
`grow` to 1 (`impl From<List<T, M>> for Element<M>`). So inside a column that fills the screen it
takes all the remaining height, which is what you want. The parent must have a bounded height for
that to mean anything; this is why the page column has `.height_percent(1.0)`.

**Items and closures must be `Send + Sync + 'static`.** The list keeps your key and row functions
and calls them later, during scrolling, long after `view` returned. So they cannot borrow from
`self`. A plain `fn` like `note_row` satisfies this automatically. A closure must `move` copies of
whatever it needs; the showcase's recycled page copies a `bool` out of the app before building the
closure (`let alt = app.recycled_alt;` in `recycled_list_page`).

**The list shares the items; it does not copy them per row.** `List::new` takes
`impl Into<Arc<Vec<T>>>`. Passing a `Vec` moves it into a new `Arc`. That is why the snippets
above pass `self.notes.clone()`: a full copy of the notes on every render. The doc comment on
`List::new` gives the fix: "Pass an `Arc<Vec<_>>` to avoid copying the data on every rebuild."

## Holding the notes in an `Arc`

Change the state so the list and the app share one vector:

```rust,ignore
use std::sync::Arc;

pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    next_id: u64,
    draft: String,
}
```

`view` now passes `Arc::clone(&self.notes)`, which copies a pointer. `update` changes the vector
through `Arc::make_mut`:

```rust,ignore
Msg::Delete(id) => Arc::make_mut(&mut self.notes).retain(|note| note.id != id),
```

`Arc::make_mut` gives mutable access to the contents. If something else still holds the same
`Arc` (the list from the last render does, inside its closures) it first clones the vector, so
the old render keeps seeing the old notes and the app gets a fresh copy to change. The copy
happens once per change instead of once per render, and renders that do not change the notes copy
nothing.

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

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The text in the field changed.
    Draft(String),
    /// Adds the draft as a note.
    Add,
    /// Deletes the note with this id.
    Delete(u64),
}

/// The state of the app.
pub struct NotesApp {
    notes: Arc<Vec<Note>>,
    /// The id the next note gets. Ids are never reused.
    next_id: u64,
    draft: String,
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
            draft: String::new(),
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

    fn add(&mut self) {
        let text = self.draft.trim().to_string();
        if text.is_empty() {
            return;
        }
        let id = self.next_id;
        self.next_id += 1;
        Arc::make_mut(&mut self.notes).push(Note { id, text });
        self.draft.clear();
    }
}

impl Component for NotesApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Draft(text) => self.draft = text,
            Msg::Add => self.add(),
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
            .child(
                Text::new(self.summary())
                    .size(14)
                    .color(Color::SECONDARY_TEXT),
            )
            .child(
                Row::new()
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(
                        TextField::new(self.draft.clone())
                            .placeholder("Write a note")
                            .on_change(Msg::Draft)
                            .on_submit(Msg::Add)
                            .grow(1),
                    )
                    .child(Button::new("Add").on_click(Msg::Add)),
            )
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
```

The tests:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_gives_each_note_a_new_id() {
        let mut app = NotesApp::new();
        app.update(Msg::Draft("  Buy milk  ".to_string()));
        app.update(Msg::Add);
        let added = app.notes.last().unwrap();
        assert_eq!(added.text, "Buy milk");
        assert_eq!(added.id, 3);
        assert!(app.draft.is_empty());
    }

    #[test]
    fn deleting_removes_only_that_note() {
        let mut app = NotesApp::new();
        app.update(Msg::Delete(1));
        assert_eq!(app.notes.len(), 1);
        assert_eq!(app.notes[0].id, 2);
        // An id that does not exist changes nothing.
        app.update(Msg::Delete(99));
        assert_eq!(app.summary(), "1 note");
    }
}
```

## Where this lives

- `rust/src/widgets/list.rs`: `List`, `ListMode`, `ListProps` and the conversion that requires
  `key` and `row`.
- `rust/src/engine/tree.rs`: `node_id` (identity), `build_children` (the duplicate-key check),
  `build_list`, `build_fixed_rows`, `build_variable_rows`, `fixed_window`.
- `rust/src/engine/runtime.rs`: `Runtime::scroll` (when a scroll rebuilds) and the measuring loop
  in `Runtime::update`.
- `example/rust/src/pages.rs`: `list_page` (windowed) and `recycled_list_page`.
- The tests in `rust/src/engine/tree.rs` named `keys_stabilize_identifiers_when_siblings_change`
  and `a_list_only_materializes_the_window_around_the_viewport` show both ideas in a few lines.

How recycled rows are bound and diffed on each platform is covered in
[`../internals/`](../internals/).

Next: [4. Components](04-components.md).
