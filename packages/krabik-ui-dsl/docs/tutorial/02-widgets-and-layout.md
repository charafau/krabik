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
draw, or carries something only a view can: an accessibility label, or an animation of its own. This is `Style::needs_view` in `rust/src/style.rs`:

```rust,ignore
pub fn needs_view(&self) -> bool {
    self.accessibility_label.is_some()
        || self.animation.is_some()
        || self.enter.is_some()
        || self.exit.is_some()
        || self.background.is_some()
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

### Container: a box that can be tapped

`Container` is the one layout widget that always has a native view. It is a box around its
content, takes the same padding and decoration as the others, and can report a tap:

```rust,ignore
Container::new()
    .padding(16)
    .background(SURFACE)
    .corner_radius(12)
    .on_tap(Msg::Open(note.id))
    .child(
        Row::new()
            .child(Text::new(&note.title))
            .child(Spacer::new())
            .child(Button::new("Delete").on_click(Msg::Delete(note.id))),
    )
```

A tap anywhere on the box sends `Msg::Open`. The button inside keeps its own tap, as does any
other control. A container usually has one child; more than one are laid out as in a `Column`.
Without `on_tap` it is only a box, and `.enabled(false)` stops the taps. On Android the box shows
the theme's press ripple; iOS shows no highlight.

`.on_long_press(Msg::Select(note.id))` reports a press that is held, apart from the tap. A
container may have both.

### Stack: children on top of each other

Rows and columns never overlap their children. `Stack` does: every child takes the same area, the
first at the back.

```rust,ignore
Stack::new()
    .height(180)
    .corner_radius(16)
    .child(Image::new(ImageSource::asset("sunrise.svg")).scale(ImageScale::Fill))
    .child(
        Column::new()
            .padding(14)
            .child(Spacer::new())
            .child(Text::new("Caption over the art").color(WHITE)),
    )
```

A stack is as large as its largest child unless it has a size of its own. Each child fills it, so
a column in a stack can push its content to an edge with a `Spacer`, as above. With
`.align_items(Align::Center)` (or `Start`, `End`) the children keep their own size and are placed
there on both axes. Like a row or a column, a stack has no native view unless it paints something.

A child can also be held at distances from the stack's edges. It then lies on top of the others
and does not count for the stack's size:

```rust,ignore
Stack::new()
    .child(Image::new(ImageSource::asset("cover.webp")))
    .child(Text::new("NEW").positioned(Inset::new().top(8).right(8)))
    .child(caption.positioned(Inset::new().left(0).right(0).bottom(0)))
```

A side that is not set does not hold the child; two opposite sides stretch it between them.
`.positioned` only means something on a direct child of a `Stack`.

### Wrap: a row that continues on the next line

```rust,ignore
Wrap::new()
    .spacing(8)       // between children on a line
    .run_spacing(8)   // between lines
    .child(Chip::new("Rust"))
    .child(Chip::new("Kotlin"))
    .child(Chip::new("Swift"))
```

A `Wrap` needs a width to wrap at, which it has wherever a column stretches it. Each line is as
tall as its tallest child.

### Limits on a size

`.min_width(..)`, `.max_width(..)`, `.min_height(..)` and `.max_height(..)` bound a size that is
otherwise decided by the content or the parent: a button that is at least 220 points wide, a
text that wraps at 160 points whatever room it has.

### Sharing out the free space

`justify_content` takes `Justify::Start`, `Center`, `End`, `SpaceBetween` (free space between the
children, none at the edges), `SpaceAround` (half a gap at each edge) and `SpaceEvenly` (a full
gap at each edge).

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

### Pull to refresh

A vertical `ScrollView` and a `List` can be pulled down to refresh. The gesture and its spinner
are the platform's own (`SwipeRefreshLayout` on Android, `UIRefreshControl` on iOS):

```rust,ignore
ScrollView::new()
    .grow(1)
    .on_refresh(Msg::Refresh)
    .refreshing(self.refreshing)
    .child(content)

// in update:
Msg::Refresh => {
    self.refreshing = true;
    return Command::perform(|| Msg::Loaded(fetch_notes()));
}
Msg::Loaded(notes) => {
    self.refreshing = false;
    self.notes = notes;
}
```

`on_refresh` is the message for the pull; `refreshing` says whether the spinner shows. The app
owns that flag: set it when the refresh starts and clear it when the data arrives (chapter 11 has
`Command`). A pull whose message leaves `refreshing` false stops the spinner at once. Decide
`on_refresh` once for a given container: adding or removing it recreates the native view, and
the scroll position with it.

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
share four more methods, from the `text_builder!` macro in `rust/src/element.rs`:

```rust,ignore
Text::new("Notes").size(30).bold().color(Color::TEXT)
Text::new("Subtitle").weight(FontWeight::Medium)
```

`size` is in logical points and defaults to 16. `color` defaults to `Color::TEXT`. `weight`
takes one of nine weights, `FontWeight::Thin` (100) to `FontWeight::Black` (900), with `Normal`
(400) the default; `.bold()` is short for `.weight(FontWeight::Bold)` (`FontWeight` is in
`rust/src/style.rs`). The font is the system's, and it decides which weights it really has: a
weight it lacks is drawn with the nearest one. The controls that only know regular and bold
(`RadioGroup`, `TabBar`, `DatePicker` on Android) show semi-bold and heavier as bold.

`Text` alone has more, declared on `TextProps` in `rust/src/widgets/text.rs`:

```rust,ignore
Text::new(long_paragraph)
    .text_align(TextAlign::Center)   // Start, Center or End, inside the text's own frame
    .max_lines(2)
    .ellipsis()                      // "…" on the last visible line

Text::new("SECTION")
    .italic()
    .underline()                     // and .strikethrough()
    .letter_spacing(1.5)             // extra points between characters; may be negative
    .line_height(24)                 // points per line, instead of the font's own
```

Text wraps to the width it is given. `text_align` only has a visible effect when the text's frame
is wider than the text, so it usually goes together with `.width_percent(1.0)` or a stretching
parent.

`line_height` applies to every line, including the first and the last, and to a text of one
line: `Text::new("Title").size(16).line_height(40)` is 40 points tall with the text in the
middle. That makes it the way to get the line rhythm of a design.

Italics come in two weights on iOS, regular and bold; an italic text in another weight is drawn
with the nearer of the two. Android synthesises the slant for any weight.

Text follows the text size the user chose in the system settings, on both platforms: a `size`
of 16 is 16 points at the default setting and larger when the user asked for larger text, and
`letter_spacing` and `line_height` grow with it. Leave room for that: give a text's container a
width and let the height follow, instead of fixing a height around a line of text.

### A font from a file

The font is the system's unless you name a file. `Text` and `Button` take one, and so does a
`Span`:

```rust,ignore
Text::new("Notes").size(30).font(Font::asset("Display-Bold.ttf"))
```

The file is a `.ttf` or `.otf` that ships with each platform's app, in the same place an image
asset goes: `android/app/src/main/assets/` on Android, and a resource of the app target on iOS
(the examples list a `Resources` folder in `project.yml`). Nothing has to be registered, and no
`Info.plist` entry is needed.

A font file is one face. Its weight and slant are the file's own, so `.bold()` and `.italic()`
do nothing to a text in a font file: an app with a regular and a bold file names the one it
wants. If the file cannot be loaded the text is drawn in the system font and the app's log says
so once. The other text-bearing widgets (`TextField`, `Checkbox`, `RadioGroup`, `TabBar`,
`DatePicker`) use the system font.

### Several styles in one text

A sentence with one bold word is still one text: it wraps as a whole. `Text::rich` takes the
parts, and each `Span` says only what differs from the text around it:

```rust,ignore
Text::rich([
    Span::new("By continuing you accept the "),
    Span::new("terms").bold().underline().color(Color::ACCENT),
    Span::new("."),
])
.size(14)
.color(Color::TEXT)
```

A span can set `size`, `color`, `weight` (or `bold`), `italic`, `underline`, `strikethrough`
and `font`. Everything it does not set comes from the `Text`: here every part is 14 points,
and the first and the last have the text's colour. A string converts into a plain span, so
`"Total: ".into()` works in the list. Alignment, the line limit, letter spacing and the line
height belong to the whole text.

A rich text is one native view (a `TextView` with a spanned string, a `UILabel` with an
attributed string), and `content` of its properties is all the parts joined, which is what a
screen reader says and what a test finds. A span cannot be tapped; a link inside a sentence is
not available yet.

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
| `Container` | `Container::new()` plus `child` | `on_tap(msg)`, `on_long_press(msg)` | `containers.rs` |
| `TextField` | `TextField::new(value)` | `on_change(\|String\| msg)`, `on_submit(msg)`, `on_focus(\|bool\| msg)` | `text_field.rs` |
| `Switch` | `Switch::new(checked)` | `on_toggle(\|bool\| msg)` | `switch.rs` |
| `Checkbox` | `Checkbox::new("label", checked)` | `on_toggle(\|bool\| msg)` | `check.rs` |
| `RadioButton` | `RadioButton::new("label", checked)` | `on_toggle(\|bool\| msg)` | `check.rs` |
| `RadioGroup` | `RadioGroup::new(["A", "B"])` | `on_select(\|usize\| msg)` | `radio_group.rs` |
| `Slider` | `Slider::new(value)` | `on_change(\|f32\| msg)` | `slider.rs` |
| `RangeSlider` | `RangeSlider::new(low, high)` | `on_change(\|(f32, f32)\| msg)` | `range_slider.rs` |
| `NativeView` | `NativeView::new("name")` plus `prop(key, value)` | `on_event(\|String\| msg)` | `native_view.rs` |
| `WebView` | `WebView::new(url)` | `on_navigate(\|String\| msg)` | `native_view.rs` |
| `ProgressIndicator` | `::new(fraction)`, `::indeterminate()` | none | `progress.rs` |
| `Image` | `Image::new(ImageSource::asset("x.webp"))`, `Image::icon(icon)` | none | `image.rs` |
| `DatePicker` | `::new(date)`, `::time(time)`, `::date_time(date, time)` | `on_change(\|DatePickerValue\| msg)` | `date_picker.rs` |
| `TabBar` | `TabBar::new(["Day", "Week"])` | `on_select(\|usize\| msg)` | `tab_bar.rs` |
| `Pager` | `Pager::new()` plus one `child` per page | `on_select(\|usize\| msg)` | `pager.rs` |
| `List` | `List::new(items)` | rows carry theirs; a recycled list has `on_dismiss(\|&T\| msg)` and `on_reorder(\|usize, usize\| msg)` | `list.rs` |

### Menus

A menu is the platform's own: a `PopupMenu` on Android, a `UIMenu` on iOS. In Rust it is a list
of lines, each with the message it sends:

```rust,ignore
let menu = Menu::new()
    .item("Rename", Msg::Rename(id))
    .item_with_icon("Share", Icon::new().drawable("ic_share").sf("square.and.arrow.up"), Msg::Share(id))
    .destructive("Delete", Msg::Delete(id));
```

Three things open one:

```rust,ignore
MenuButton::new("Sort", menu)                      // a button with a text and an arrow
MenuButton::icon(Icon::more(), "More actions", menu)   // a button that is an icon
Container::new().context_menu(menu).child(row)     // a long press on anything
```

`Container::menu(menu)` opens the menu on a tap, which is what the two buttons are made of. A
container with a menu for a gesture does not also report that gesture: a tap menu replaces
`on_tap`, a context menu replaces `on_long_press`. On iOS the context menu is the system's, which
lifts the container while the menu is open.

An app bar has a menu of its own for the actions that do not deserve a button:
`AppBar::new("Notes").action("Add", Msg::Add).menu_item("Settings", Msg::Settings)`. Android shows
them behind the toolbar's three dots, iOS behind an ellipsis button.

### Tooltips

`.tooltip("Search every note")` on any widget explains it while it is pressed and held: Android
shows its own tooltip, iOS a small label above the widget for a moment. It is for controls whose
meaning is not written on them. A widget with a tooltip should not also react to a long press.

### Icons

An icon is an `Image` whose source is a name the platform knows, drawn in one color:

```rust,ignore
Image::icon(Icon::new().drawable("ic_share").sf("square.and.arrow.up"))
    .size(20)
    .tint(Color::ACCENT)
```

`drawable` names a drawable resource of the Android app, `sf` an SF Symbol or an image of the
iOS asset catalog; `Icon::new().shared("name")` uses one name on both. An icon is 24 points
square and in the text color unless told otherwise, and a name the platform does not have shows
nothing (and is logged once). The framework brings a few itself: `Icon::add()`, `remove()`,
`close()`, `check()`, `search()`, `chevron_down()`, `chevron_up()`, `chevron_right()` and
`more()`.

### Widgets assembled from others

These are builders that return containers, texts and images, so they cost no native code and
look the same on both platforms. Their state is the application's, as with every widget: a chip
does not remember that it is selected, the application says so. Each has the usual style
methods, which replace the look it comes with.

| Widget | What it is | Events | File |
| ------ | ---------- | ------ | ---- |
| `Divider` | a thin line; `.vertical()`, `.indent(16)` | none | `divider.rs` |
| `Card` | a rounded, raised surface; `child`, `spacing` | `on_tap(msg)` | `card.rs` |
| `ListTile` | a row: `leading`, title, `subtitle`, `trailing` | `on_tap(msg)` | `list_tile.rs` |
| `Avatar` | a circle: `Avatar::image(source)` or `Avatar::initials("RW")` | none | `avatar.rs` |
| `Badge` | `Badge::new(child).count(n)`, `.label("new")`, `.dot(true)` | none | `badge.rs` |
| `Chip` | a rounded label; `.selected(bool)`, `.icon(..)` | `on_tap(msg)`, `on_delete(msg)` | `chip.rs` |
| `IconButton` | `IconButton::new(icon, "what it does")`; `.text("Find")` | `on_tap(msg)` | `icon_button.rs` |
| `Button::filled`, `Button::outlined` | looks of the native `Button` | `on_click(msg)` | `button.rs` |
| `ExpansionTile` | a header that shows its children while `.expanded(bool)` | `on_toggle(msg)` | `expansion_tile.rs` |
| `Stepper` | numbered steps, the `.current(i)` one open | `on_step(\|usize\| msg)` | `stepper.rs` |
| `NumberStepper` | a number between minus and plus; `min`, `max`, `step` | `on_change(\|f32\| msg)` | `number_stepper.rs` |
| `SearchBar` | a rounded field with a search icon and a clear button | `on_change(\|String\| msg)`, `on_submit(msg)` | `search_bar.rs` |
| `Autocomplete` | a field with the matching options under it | `on_change(\|String\| msg)`, `on_select(\|String\| msg)` | `autocomplete.rs` |
| `Grid` | a `List` with `.columns(n)` equal cells on a line | none of its own | `grid.rs` |

| `MenuButton` | a button that opens a `Menu`; `MenuButton::icon(icon, "what it is for", menu)` | the menu's lines | `menu.rs` |
| `Dropdown` | a field that shows one option and opens a menu of all; `.selected(Some(i))`, `.placeholder(..)` | `on_select(\|usize\| msg)` | `menu.rs` |
| `SegmentedControl` | a few options side by side; `.selected(i)` | `on_select(\|usize\| msg)` | `segmented_control.rs` |

What to know before using them:

- The second argument of `IconButton::new` says what the button does. It is not shown; a screen
  reader announces it and UI tests find the button by it.
- A `Badge` is as large as its child and its mark lies a little outside. In a column, which
  stretches its children, wrap the badge in a `Row`.
- Several `Chip`s in a `Wrap` are how a choice of several is offered.
- The suggestions of an `Autocomplete` are part of the layout: they push what follows down.
  Nothing can float over other content yet.
- A `Grid` is a list whose rows are lines of cells, so it takes `key`, `cell` (in place of
  `row`), `recycled()` and `cell_height(..)`, and needs a bounded area as a list does. A line is
  as tall as its tallest cell.
- A `NumberStepper` button that would leave the range is dimmed and does nothing.

A few notes that save a trip to the source:

- `TextField` has the setters `placeholder("...")`, `multiline(true)`, `secure(true)`,
  `keyboard(KeyboardType::Email)` and `ime_action(ImeAction::Done)`. `on_submit` fires when the
  user presses the action key on the keyboard.
- `Slider` has `min`, `max`, `step` and `color`. The default range is 0 to 1. `RangeSlider` has
  the same and two thumbs; its handler receives both values as one pair, lower first, so the
  message is `Price((f32, f32))` or a closure `|(low, high)| ..`.
- `Text::new(..).selectable()` lets the user copy the text: by selecting any part of it on
  Android, by holding it and choosing "Copy" on iOS, where the whole text is copied.
- A `List` that is `.recycled()` takes three more things. `.on_dismiss(|item| Msg::Delete(item.id))`
  lets a row be swiped away; `.on_reorder(|from, to| Msg::Move(from, to))` lets it be dragged
  after a long press; `.sticky_header(|item| item.is_header)` keeps those rows at the top while
  their section scrolls. The list is your state: a row comes back, or returns to its place,
  unless `update` removes or moves the item.
- `NativeView` shows a view the application's own Kotlin and Swift code makes, such as a map;
  [the recipe](../recipes/embed-a-native-view.md) has the three steps. `WebView` is the one the
  framework brings.
- `RadioGroup`, `TabBar`, `Pager` take `.selected(index)`.
- `Tabs` (a tab bar above one child) and `TabsView` (a tab bar above a swipeable pager) are
  compositions of the widgets above, in `tab_bar.rs` and `pager.rs`.
- `Image` needs an explicit size or a stretching parent; its constructor's doc comment says
  intrinsic sizing is not available for remote images yet.
- An asset whose name ends in `.svg` is drawn by Rust (the `resvg` crate) at the size it is shown,
  the same on both platforms, so it stays sharp at any size. SVG filters work, including
  `feGaussianBlur`; text in an SVG does not, and remote SVGs are not supported. The renderer is
  behind a cargo feature so apps without SVGs do not carry it:
  `krabik-ui-dsl = { path = "...", features = ["svg"] }` in the app's `Cargo.toml`.
- Only the widgets in the table report events. A `Row`, a `Column` or a `Text` has no tap handler
  (`Actions::of` in `rust/src/host/events.rs` lists the cases exhaustively). To make a row of a
  list or a card tappable, wrap it in a `Container` with `on_tap`.

The showcase app has one page per widget. Its Rust side is `example/rust/src/pages.rs`; reading a
page function there is the fastest way to see a widget used correctly.

### Widgets hold no state

`TextField::new(value)` shows `value`. It does not remember what the user typed. When the user
types, the field reports the new text through `on_change`; your `update` stores it; your next
`view` passes it back in. The same goes for `Switch::new(checked)`, `Slider::new(value)` and every
`.selected(index)`. This is the "controlled component" pattern from React, and it follows from the
rule that Rust owns all state.

If you forget to store a value, the widget snaps back to the old one straight away: after every
change the user makes, the control is set to what your state says. That is also how you refuse
input. Strip the letters from a digits-only field in `update` and the field shows only digits. If you
leave out `on_change`, the field cannot be edited in any lasting way.

## Starting the Notes app

Replace the counter with the first version of Notes. The state is the list of notes and the text
being typed. In a project made by `krabik create`, keep the lines below `impl App` and change
`Counter` to `NotesApp` in them: `impl Default`, `krabik_ui_dsl::hot_reload!(NotesApp);` and the
two install functions.

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
fn update(&mut self, msg: Msg) -> Command<Msg> {
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
    Command::none()
}
```

Clearing `self.draft` is what empties the field: the next `view` builds `TextField::new("")`, the
diff sees that the field's value changed, and the native field is updated.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::command::Command;
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

    fn update(&mut self, msg: Msg) -> Command<Msg> {
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
        Command::none()
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
  `text_builder!` (`size`, `color`, `bold`, `weight`).
- `rust/src/widgets/text.rs`: `TextProps`, `Span` and `Text::rich`.
- `rust/src/style.rs`: `Style`, `Length`, `EdgeInsets`, `Align`, `Justify`, `Color`.
- `rust/src/widgets/containers.rs` and `scroll.rs`: `Column`, `Row`, `Spacer`, `ScrollView`.
- `rust/src/widgets/*.rs`: one file per widget; the properties struct at the top of each file is
  the list of what the widget can do.
- `rust/src/engine/layout.rs`: `taffy_style`, the whole mapping from a node to flexbox.
- `example/rust/src/pages.rs`: `layouts_page`, `text_page`, `styling_page`, `scroll_page`.

Next: [3. Lists and state](03-lists-and-state.md).
