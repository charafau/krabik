# 2. Widgets and layout

This chapter covers how things are placed on screen and which widgets exist. Layout in krabik is
flexbox, computed by Taffy, and once you have the mental model the handful of layout methods
explain themselves. By the end the counter from chapter 1 is replaced by the first version of the
Notes app: a title, a text field, an Add button and the notes below.

## The mental model: everything is a flex container

There are two layout containers. A `Column` places its children from top to bottom. A `Row`
places them from left to right. That direction is the container's **main axis**; the other
direction is its **cross axis**.

```text
  Column                         Row
  main axis: down                main axis: right
  cross axis: across             cross axis: down

  +-----------+                  +-----+-----+-----+
  |  child 1  |                  |     |     |     |
  +-----------+                  |  1  |  2  |  3  |
  |  child 2  |                  |     |     |     |
  +-----------+                  +-----+-----+-----+
  |  child 3  |
  +-----------+
```

This is CSS flexbox, and not by analogy: the engine translates each node into a Taffy style and
lets Taffy compute the frames. The translation is one function, `taffy_style` in
`rust/src/engine/layout.rs`:

```rust,ignore
let direction = match node.kind {
    NodeKind::Row => FlexDirection::Row,
    // ...
    _ => FlexDirection::Column,
};
// ...
TaffyStyle {
    display: Display::Flex,
    flex_direction: direction,
    // ...
    flex_grow: node.style.grow,
    // ...
    align_items: align_items(node.style.align_items),
    justify_content: justify_content(node.style.justify_content),
    // ...
}
```

Every node is `Display::Flex`. If you know flexbox from the web, the table below is all you need.
If you do not, read the four rules after it.

| Method                     | Flexbox equivalent           | Default           |
| -------------------------- | ---------------------------- | ----------------- |
| `Column::new()`            | `flex-direction: column`     |                   |
| `Row::new()`               | `flex-direction: row`        |                   |
| `.spacing(8)`              | `gap: 8px`                   | 0                 |
| `.padding(16)`             | `padding`                    | 0                 |
| `.margin(8)`               | `margin`                     | 0                 |
| `.width(120)`, `.height(40)` | `width`, `height` in points | from content      |
| `.width_percent(1.0)`      | `width: 100%`                |                   |
| `.grow(1)`                 | `flex-grow: 1`               | 0                 |
| `.align_items(Align::..)`  | `align-items`                | `Align::Stretch`  |
| `.justify_content(Justify::..)` | `justify-content`       | `Justify::Start`  |
| `.aspect_ratio(1.5)`       | `aspect-ratio`               | none              |

These methods exist on every widget, containers and leaves alike. They are written once, in the
`style_builder!` macro in `rust/src/element.rs`, and every widget invokes it.

**Rule 1: a node is as big as its content unless you say otherwise.** A `Text` is as wide and as
tall as the platform measures its string. A `Column` is as tall as its children stacked up. To say
otherwise, give a fixed size (`.width(120)`), a share of the parent (`.width_percent(1.0)`; the
argument is a fraction from 0 to 1, not a number from 0 to 100) or let it grow (rule 2).

**Rule 2: `grow` hands out the space that is left over on the main axis.** After every child has
its natural size, the remaining room is divided among the children that have `grow` set, in
proportion to their values. One child with `.grow(1)` takes all of it. Two with `.grow(1)` split
it evenly.

**Rule 3: `align_items` places children on the cross axis.** The default is `Align::Stretch`:
children of a `Column` are stretched to its full width, children of a `Row` to its full height.
That default is why a `Button` directly inside a `Column` spans the screen. Use `Align::Start`,
`Align::Center` or `Align::End` on the *parent* to stop that.

**Rule 4: `justify_content` places children on the main axis** when there is room left and nobody
grows: `Justify::Start`, `Center`, `End` or `SpaceBetween`.

Some things are fixed. There is no wrapping. `flex-shrink` is always 1, except inside a scroll
container where it is 0 so content keeps its natural size. `align-self` is not exposed. A `Text`
gets a minimum width of zero so it can shrink and wrap next to a sibling; the comment in
`taffy_style` explains that without it a long label would refuse to shrink and push its neighbours
out of a row.

Sizes are **logical points**: dp on Android, points on iOS (`Length` in `rust/src/style.rs`). The
size methods accept both integers and floats (`.padding(16)` and `.padding(16.0)`) through the
`IntoPoints` trait.

### Spacer

`Spacer` is an empty node that grows. Its whole implementation is a leaf with `grow` set to 1:

```rust,ignore
pub fn new() -> Self {
    let mut element = RawElement::leaf(ElementKind::Spacer);
    element.style.grow = 1.0;
    Self(element, PhantomData)
}
```

(from `rust/src/widgets/containers.rs`)

Put one between two children of a `Row` to push them to opposite edges. Give it a fixed size
(`Spacer::new().height(8)`) to use it as a gap of a specific size; the showcase's checklist page
does that.

### Padding and margin on each side

`.padding(16)` is uniform. For different sides use `padding_each` with an `EdgeInsets`:

```rust,ignore
.padding_each(EdgeInsets::symmetric(12, 8))        // 12 left and right, 8 top and bottom
.padding_each(EdgeInsets::new(8.0, 0.0, 8.0, 0.0)) // top, right, bottom, left
```

`margin` and `margin_each` work the same way, outside the node's border.

### The root

The engine wraps whatever `view` returns in a synthetic root node that fills the viewport
(`root_style` in `rust/src/engine/tree.rs` sets its width and height to 100%). The root lays its
single child out as a column with the default `Align::Stretch`, so your top-level element is as
wide as the screen but only as *tall* as its content. A page that should fill the screen, for
example because something at the bottom should grow, says so with `.height_percent(1.0)`.

### Containers are free

A `Column` or `Row` that only arranges its children does not become a native view. The layout
engine computes frames for its children and the children are attached directly to the nearest
ancestor that has a view. A container gets a native view of its own only when it has something to
draw. This is `Style::needs_view` in `rust/src/style.rs`:

```rust,ignore
pub fn needs_view(&self) -> bool {
    self.background.is_some()
        || self.gradient.is_some()
        || self.corner_radius > 0.0
        || self.corner_radii.is_some()
        || self.border.is_some()
        || self.opacity.is_some_and(|opacity| opacity < 1.0)
        || self.elevation > 0.0
}
```

So nest containers as deeply as the layout needs. Unlike in an Android XML layout, depth costs
nothing at draw time.

## Scrolling

A `ScrollView` is a container whose content may be larger than itself. `ScrollView::new()` scrolls
vertically, `ScrollView::horizontal()` horizontally. It is a native scroll view (so it has native
physics, overscroll and scrollbars).

The one thing to remember is in its doc comment in `rust/src/widgets/scroll.rs`:

```rust,ignore
/// Give it a bounded size (`.grow(1)`, a fixed height, or a parent-provided
/// constraint) so its content can actually overflow.
```

A scroll view sized by its content is exactly as tall as that content and never scrolls. Bound it:
`.height(220)`, or `.grow(1)` inside a column that fills the screen, or `.height_percent(1.0)`.

For long lists of similar rows use `List` instead (chapter 3). A `ScrollView` creates a native
view for every child, visible or not.

## Decoration

The same shared builder has the methods that paint:

| Method                              | Effect                                                |
| ----------------------------------- | ----------------------------------------------------- |
| `.background(color)`                | fill                                                  |
| `.gradient(Gradient::vertical(a, b))` | two-colour gradient; wins over `background`         |
| `.corner_radius(12)`                | rounded corners; `corner_radii` for each corner       |
| `.border(1, color)`                 | stroke                                                |
| `.opacity(0.6)`                     | the node and its children                             |
| `.elevation(4)`                     | native shadow                                         |
| `.enabled(false)`                   | disables input and dims the decoration                |
| `.accessibility_label("...")`       | what a screen reader announces                        |
| `.key("...")`                       | identity among siblings; chapter 3                    |

A colour is either a number in `0xAARRGGBB` form or a `Color`. For now use the named colours such
as `Color::SURFACE` and `Color::SECONDARY_TEXT`; they adapt to dark mode. Chapter 7 is about
colours.

There are also implicit animations (`.animate(250)`, `.animate_enter(200)`, `.animate_exit(160)`
and their curves). This tutorial does not use them; the showcase pages `animations_page` and
`animation_enter_exit_page` in `example/rust/src/pages.rs` show each one.

## Text styling

Widgets that show text (`Text`, `Button`, `TextField`, `Checkbox`, `RadioButton`, `RadioGroup`)
share three more methods, from the `text_builder!` macro in `rust/src/element.rs`:

```rust,ignore
Text::new("Notes").size(30).bold().color(Color::TEXT)
```

`size` is in logical points and defaults to 16. `color` defaults to `Color::TEXT`. There are two
weights, normal and bold (`FontWeight` in `rust/src/style.rs`); there is no font family setting.

`Text` alone has three more, declared on `TextProps` in `rust/src/widgets/text.rs`:

```rust,ignore
Text::new(long_paragraph)
    .text_align(TextAlign::Center)   // Start, Center or End, inside the text's own frame
    .max_lines(2)
    .ellipsis()                      // "…" on the last visible line
```

Text wraps to the width it is given. `text_align` only has a visible effect when the text's frame
is wider than the text, so it usually goes together with `.width_percent(1.0)` or a stretching
parent.

## The widgets

Each widget lives in one file in `rust/src/widgets/`. A leaf widget is one native view. The event
methods are the ones to remember, because their argument shape tells you what the widget reports.

An event with no payload takes a **message**: `.on_click(Msg::Add)`. An event with a payload takes
a **function from the payload to a message**: `.on_change(Msg::Draft)`. An enum variant with one
field, such as `Draft(String)`, *is* such a function, which is why you rarely write a closure.

| Widget | Constructor | Events | File |
| ------ | ----------- | ------ | ---- |
| `Text` | `Text::new("...")` | none | `text.rs` |
| `Button` | `Button::new("label")` | `on_click(msg)` | `button.rs` |
| `TextField` | `TextField::new(value)` | `on_change(\|String\| msg)`, `on_submit(msg)`, `on_focus(\|bool\| msg)` | `text_field.rs` |
| `Switch` | `Switch::new(checked)` | `on_toggle(\|bool\| msg)` | `switch.rs` |
| `Checkbox` | `Checkbox::new("label", checked)` | `on_toggle(\|bool\| msg)` | `check.rs` |
| `RadioButton` | `RadioButton::new("label", checked)` | `on_toggle(\|bool\| msg)` | `check.rs` |
| `RadioGroup` | `RadioGroup::new(["A", "B"])` | `on_select(\|usize\| msg)` | `radio_group.rs` |
| `Slider` | `Slider::new(value)` | `on_change(\|f32\| msg)` | `slider.rs` |
| `ProgressIndicator` | `::new(fraction)`, `::indeterminate()` | none | `progress.rs` |
| `Image` | `Image::new(ImageSource::asset("x.webp"))` | none | `image.rs` |
| `DatePicker` | `::new(date)`, `::time(time)`, `::date_time(date, time)` | `on_change(\|DatePickerValue\| msg)` | `date_picker.rs` |
| `TabBar` | `TabBar::new(["Day", "Week"])` | `on_select(\|usize\| msg)` | `tab_bar.rs` |
| `Pager` | `Pager::new()` plus one `child` per page | `on_select(\|usize\| msg)` | `pager.rs` |
| `List` | `List::new(items)` | none of its own; rows carry theirs | `list.rs` |

A few notes that save a trip to the source:

- `TextField` has the setters `placeholder("...")`, `multiline(true)`, `secure(true)`,
  `keyboard(KeyboardType::Email)` and `ime_action(ImeAction::Done)`. `on_submit` fires when the
  user presses the action key on the keyboard.
- `Slider` has `min`, `max`, `step` and `color`. The default range is 0 to 1.
- `RadioGroup`, `TabBar`, `Pager` take `.selected(index)`.
- `Tabs` (a tab bar above one child) and `TabsView` (a tab bar above a swipeable pager) are
  compositions of the widgets above, in `tab_bar.rs` and `pager.rs`.
- `Image` needs an explicit size or a stretching parent; its constructor's doc comment says
  intrinsic sizing is not available for remote images yet.
- Only the widgets in the table report events. A `Row`, a `Column` or a `Text` has no tap handler
  (`Actions::of` in `rust/src/host/events.rs` lists the cases exhaustively). To make something
  tappable today, put a `Button` in it. This is a rough edge you will notice in list rows.

The showcase app has one page per widget. Its Rust side is `example/rust/src/pages.rs`; reading a
page function there is the fastest way to see a widget used correctly.

### Widgets hold no state

`TextField::new(value)` shows `value`. It does not remember what the user typed. When the user
types, the field reports the new text through `on_change`; your `update` stores it; your next
`view` passes it back in. The same goes for `Switch::new(checked)`, `Slider::new(value)` and every
`.selected(index)`. This is the "controlled component" pattern from React, and it follows from the
rule that Rust owns all state.

If you forget to store a value, the widget snaps back to the old one on the next render. If you
leave out `on_change`, the field cannot be edited in any lasting way.

## Starting the Notes app

Replace the counter with the first version of Notes. The state is the list of notes and the text
being typed:

```rust,ignore
pub struct NotesApp {
    notes: Vec<String>,
    draft: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The text in the field changed.
    Draft(String),
    /// Adds the draft as a note.
    Add,
}
```

The page is a column that fills the screen. Inside it: a title, a line that counts the notes, a
row with the field and the button, and a scrolling column of notes that takes the rest of the
height.

```text
  Column (fills the screen, padding 24, spacing 12)
  +------------------------------------------+
  | Text "Notes"                             |
  | Text "2 notes"                           |
  | Row (spacing 8, children centred)        |
  | +----------------------------+ +-------+ |
  | | TextField  (grow 1)        | |  Add  | |
  | +----------------------------+ +-------+ |
  | ScrollView (grow 1)                      |
  | +--------------------------------------+ |
  | | Column of notes                      | |
  | |                                      | |
  | +--------------------------------------+ |
  +------------------------------------------+
```

The row is the interesting part:

```rust,ignore
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
    .child(Button::new("Add").on_click(Msg::Add))
```

The button keeps its natural width. The field has `.grow(1)`, so it takes whatever is left (rule
2). `align_items(Align::Center)` centres both vertically; without it the default `Stretch` would
make the shorter one as tall as the taller one (rule 3). `on_submit(Msg::Add)` makes the
keyboard's action key do the same as the button.

The notes are added in a loop. A builder is an ordinary value, so an ordinary `for` works:

```rust,ignore
let mut notes: Column<Msg> = Column::new().spacing(8).width_percent(1.0);
for note in &self.notes {
    notes = notes.child(
        Text::new(note.clone())
            .padding(12)
            .background(Color::SURFACE)
            .corner_radius(8),
    );
}
```

`child` takes the column by value and returns it, so the loop reassigns. Each note is a `Text`
with padding, a fill and rounded corners; a leaf widget can be decorated like a container.

`update` is short. `Msg::Add` ignores a draft that is empty after trimming:

```rust,ignore
fn update(&mut self, msg: Msg) {
    match msg {
        Msg::Draft(text) => self.draft = text,
        Msg::Add => {
            let text = self.draft.trim().to_string();
            if !text.is_empty() {
                self.notes.push(text);
                self.draft.clear();
            }
        }
    }
}
```

Clearing `self.draft` is what empties the field: the next `view` builds `TextField::new("")`, the
diff sees that the field's value changed, and the native field is updated.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::ui::{Align, Button, Color, Column, Element, Row, ScrollView, Text, TextField};

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The text in the field changed.
    Draft(String),
    /// Adds the draft as a note.
    Add,
}

/// The state of the app.
pub struct NotesApp {
    notes: Vec<String>,
    draft: String,
}

impl NotesApp {
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
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
}

impl Component for NotesApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Draft(text) => self.draft = text,
            Msg::Add => {
                let text = self.draft.trim().to_string();
                if !text.is_empty() {
                    self.notes.push(text);
                    self.draft.clear();
                }
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let mut notes: Column<Msg> = Column::new().spacing(8).width_percent(1.0);
        for note in &self.notes {
            notes = notes.child(
                Text::new(note.clone())
                    .padding(12)
                    .background(Color::SURFACE)
                    .corner_radius(8),
            );
        }

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
            .child(ScrollView::new().grow(1).child(notes))
            .into()
    }
}

impl App for NotesApp {}
```

And the tests, replacing the counter's:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_trims_and_ignores_an_empty_draft() {
        let mut app = NotesApp::new();
        app.update(Msg::Add);
        assert!(app.notes.is_empty());

        app.update(Msg::Draft("  Buy milk  ".to_string()));
        app.update(Msg::Add);
        assert_eq!(app.notes, ["Buy milk"]);
        assert!(app.draft.is_empty());
        assert_eq!(app.summary(), "1 note");
    }
}
```

## Where this lives

- `rust/src/element.rs`: `style_builder!` (every layout and decoration method) and
  `text_builder!` (`size`, `color`, `bold`).
- `rust/src/style.rs`: `Style`, `Length`, `EdgeInsets`, `Align`, `Justify`, `Color`.
- `rust/src/widgets/containers.rs` and `scroll.rs`: `Column`, `Row`, `Spacer`, `ScrollView`.
- `rust/src/widgets/*.rs`: one file per widget; the properties struct at the top of each file is
  the list of what the widget can do.
- `rust/src/engine/layout.rs`: `taffy_style`, the whole mapping from a node to flexbox.
- `example/rust/src/pages.rs`: `layouts_page`, `text_page`, `styling_page`, `scroll_page`.

Next: [3. Lists and state](03-lists-and-state.md).
