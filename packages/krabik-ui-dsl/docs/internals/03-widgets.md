# 3. Widgets

This chapter covers `rust/src/widgets/`: what one widget file contains, which parts
`#[derive(Widget)]` writes and which are written by hand, how the `widgets!` table produces the
`Widget` enum, and why a handful of widgets do not use the derive. After reading it you can open
any file in the folder and tell the generated parts from the hand-written ones, and you know what
adding a widget touches on the core side.

The macro's implementation (`rust/macros/src/widget.rs`) has its own chapter in
[`../platforms/`](../platforms/). Here it is described only by what it produces.

## What "a widget" means in this code

A widget is a leaf of the tree that is drawn by one native view and has no children: text, a
button, a text field, an image, a switch, a slider and so on. Containers (`Column`, `Row`,
`ScrollView`, `Pager`, `List`) are *not* widgets in this sense. They live in the same folder
because applications use them the same way, but in `ElementKind` they are separate variants
(chapter 2).

Each widget is three things:

1. A **properties struct** (`ButtonProps`): plain data, everything the native view needs to show.
   This is what ends up in the node tree and what the diff compares.
2. A **builder** (`Button<M>`): the typed API an application calls. It wraps a `RawElement` and
   disappears once the element is built.
3. An **intrinsic size**: how much room the widget wants when the style does not say.

## The shape of a widget file: `button.rs`

Here is the whole of the declarative part of `rust/src/widgets/button.rs`:

```rust,ignore
// rust/src/widgets/button.rs
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(name = Button, text, about = "A native button.", new = "Creates a button with the given label.")]
pub struct ButtonProps {
    /// The label shown on the button.
    #[widget(arg)]
    pub label: String,
    /// Sends `message` when the button is pressed.
    pub on_click: Handler<()>,
    /// Font size in logical points.
    #[widget(default = 16.0)]
    pub size: f32,
    /// Text color.
    #[widget(default = Color::TEXT)]
    pub color: Color,
    /// Font weight.
    #[widget(default = FontWeight::Normal)]
    pub weight: FontWeight,
}
```

The attributes read as follows.

On the struct:

- `name = Button` is the name of the builder to generate. It is also the `Widget` variant the
  builder wraps the properties in, unless `variant = Other` says otherwise.
- `text` says the struct has `size`, `color` and `weight` fields and wants the shared `.size()`,
  `.color()` and `.bold()` methods.
- `about` and `new` are the doc comments for the builder and its constructor.

On a field:

- `#[widget(arg)]`: a constructor argument. A `String` argument is accepted as
  `impl Into<String>`; a `Vec<String>` as any iterator of things that convert into strings.
- `#[widget(default = expr)]`: the initial value. Without it a field starts as
  `Default::default()`, and a `Handler` field as `Handler::none()`.
- `#[widget(set)]`: generate a setter named after the field. `button.rs` has none; `slider.rs`
  below shows the options.
- A field of type `Handler<A>` needs no attribute. It always gets an event method named after the
  field, and the field's doc comment becomes the method's.

### What the derive generates

For `ButtonProps` the derive emits, in this order (see `expand` in `rust/macros/src/widget.rs`):

```rust,ignore
// generated (shown as source; names are real, bodies trimmed)

impl ButtonProps {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: crate::message::Handler::none(),
            size: 16.0,
            color: Color::TEXT,
            weight: FontWeight::Normal,
        }
    }
}

/// A native button.
pub struct Button<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Button<M> {
    pub fn new(label: impl Into<String>) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Button(ButtonProps::new(label)))),
            PhantomData,
        )
    }

    /// Sends `message` when the button is pressed.
    pub fn on_click(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Widget(Widget::Button(props)) = &mut self.0.kind {
            props.on_click = Handler::new(move |()| message.clone());
        }
        self
    }
}

impl<M> From<Button<M>> for Element<M> { /* Element::from_raw(component.0) */ }

crate::element::style_builder!(Button);   // padding, width, background, key, ...
crate::element::text_builder!(Button);    // size, color, bold, weight   (because of `text`)

impl crate::widgets::WidgetProps for ButtonProps {
    fn map_messages(&mut self, wrap: &MessageMap) {
        self.on_click.map(wrap);                       // one line per Handler field
    }
    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {   // because of `text`
        Some(TextStyleMut {
            size: &mut self.size,
            color: &mut self.color,
            weight: &mut self.weight,
        })
    }
    fn resolve_colors(&mut self, scheme: ColorScheme) {
        self.color = self.color.resolve(scheme);       // one line per `Color` field
    }
}
```

Three things the derive works out from field *types*, without an attribute:

- Every field whose type is written `Handler<...>` gets an event method and a line in
  `map_messages`. A unit payload (`Handler<()>`) produces a method that takes a message value; any
  other payload produces a method that takes `impl Fn(A) -> M`.
- Every field of type `Color`, `Option<Color>` or `Vec<Color>` gets its lines in `resolve_colors`.
- A `String` argument or setter accepts `impl Into<String>`.

These checks look at the type as written, not at the resolved type. `is_named` compares the last
path segment, so `Color` and `crate::style::Color` are both colours, and a colour inside an
`Option` or a `Vec` is resolved as well. What a macro cannot see through is a type alias: a
field declared as `Tint`, where `type Tint = Color`, is skipped silently. `#[widget(fixed_color)]`
opts a field out.

The setters all start with `if let ElementKind::Widget(Widget::Button(props)) = &mut self.0.kind`.
The pattern always matches, because the builder's constructor is the only way to make a `Button`
and it always puts a `Widget::Button` inside. The `if let` is there because the builder stores a
general `RawElement`; the type system does not know which variant it holds.

### What is written by hand

The derive stops at boilerplate. In `button.rs` the hand-written part is the intrinsic size:

```rust,ignore
// rust/src/widgets/button.rs
const BUTTON_PADDING_X: f32 = 16.0;
const BUTTON_PADDING_Y: f32 = 12.0;
const BUTTON_MIN_HEIGHT: f32 = 20.0;

impl IntrinsicSize for ButtonProps {
    /// The label plus the button's own padding.
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let inner_width = max_width.map(|width| (width - BUTTON_PADDING_X * 2.0).max(0.0));
        let label = TextProps {
            content: self.label.clone(),
            size: self.size,
            color: self.color,
            weight: self.weight,
            ..TextProps::new("")
        };
        let text = label.intrinsic_size(inner_width, measurer);
        Size::new(
            text.width + BUTTON_PADDING_X * 2.0,
            text.height.max(BUTTON_MIN_HEIGHT) + BUTTON_PADDING_Y * 2.0,
        )
    }
}
```

A button measures its label as if it were a `Text` with the same font, at the width left after its
own horizontal padding, then adds the padding back. The constants are the framework's model of a
native button. Nothing in the core checks that the native button really uses these insets; if a
platform's button padding differs, the label can be clipped or the button looks loose. This is the
general trade of doing layout in Rust: the engine needs sizes before any native view exists, so it
either asks the platform (text, switches, images; chapter 5) or assumes.

Also hand-written, and not in this folder at all: the code that creates and updates the native
view. That lives in `rust/src/android/widgets/button.rs` and `rust/src/ios/widgets/button.rs`.

## A second example with setters: `slider.rs`

```rust,ignore
// rust/src/widgets/slider.rs
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = Slider,
    about = "A native slider.",
    new = "Creates a slider with the given value and the default range `0..=1`."
)]
pub struct SliderProps {
    #[widget(arg)]
    pub value: f32,
    #[widget(default = 0.0, set)]
    pub min: f32,
    #[widget(default = 1.0, set)]
    pub max: f32,
    #[widget(set(with = positive_step, from = f32))]
    pub step: Option<f32>,
    pub on_change: Handler<f32>,
    #[widget(default = Color::ACCENT, set(into))]
    pub color: Color,
}

fn positive_step(step: f32) -> Option<f32> {
    (step > 0.0).then_some(step)
}
```

There is no `text` flag, so a slider has no `.size()` or `.bold()`. The setter options:

| Attribute | Generated method | Stored value |
|---|---|---|
| `set` on `min: f32` | `min(self, min: f32)` | `min` |
| `set(with = positive_step, from = f32)` on `step` | `step(self, step: f32)` | `positive_step(step)` |
| `set(into)` on `color: Color` | `color(self, color: impl Into<Color>)` | `color.into()` |
| (none) on `on_change: Handler<f32>` | `on_change(self, message: impl Fn(f32) -> M + ...)` | `Handler::new(message)` |

Other options used elsewhere: `set(name = text_align)` renames the method (`TextProps::align`),
`set(some)` wraps the argument in `Some` (`RadioGroupProps::selected`), and `set(flag = true)`
makes a method with no argument (`TextProps::ellipsis`).

One field option is not about a setter. A colour is resolved for light or dark mode wherever the
derive finds a `Color`, an `Option<Color>` or a `Vec<Color>` field. A list of values that carry
colours of their own is marked `#[widget(colors)]`, and the derive calls `resolve_colors(scheme)`
on each item; `TextProps::spans`, a `Vec<Span>`, is the one case.

### `Text` is more than its derive

`rust/src/widgets/text.rs` adds three things by hand to what the derive generates:

- `Text::rich(spans)`, a second constructor. It stores the spans and sets `content` to their
  texts joined, so everything that reads `content` keeps working.
- `TextProps::runs()`, which the renderers use: one `TextRun` per span, with what the span does
  not set filled in from the text and its place counted in UTF-16 code units, the way both
  platforms index a string.
- `TextProps::measure_key()`, the part of the platforms' measurement cache key that describes
  the text. Every property that changes the size of a text is in it (size, weight, slant, letter
  spacing, line height, line limit, the runs); a test changes each one and checks that the key
  changes. A property that is missing there would make two different texts share one cached
  size.

`set(into)` on the colour is what makes `.color(0xFF00_897B)` work: `u32` converts into a fixed
`Color` (chapter 9).

The file also holds a helper that is not part of the builder:

```rust,ignore
// rust/src/widgets/slider.rs
pub(crate) fn snap_slider_value(value: f32, min: f32, max: f32, step: Option<f32>) -> f32
```

The native slider reports a raw position. The session snaps it to the range and step before
building the message (`Actions::message` in `rust/src/host/events.rs`), so both platforms round the
same way and the application never sees an off-step value.

The intrinsic size is one line: a slider fills the width it is given, or `CONTROL_WIDTH` (200
points, from `widgets/mod.rs`) when the width is unbounded, and is 40 points tall.

## The `widgets!` table and the `Widget` enum

`rust/src/widgets/mod.rs` declares all widgets in one place:

```rust,ignore
// rust/src/widgets/mod.rs
// Tags must never be reused: they are part of a node's identity. The
// containers in `ElementKind` use 1, 2, 3, 8, 9, 11 and 20 to 24, and 26
// is a text that can be selected.
widgets! {
    /// A run of text rendered by a native label.
    Text(TextProps) = 4,
    /// A native button.
    Button(ButtonProps) = 5,
    /// A native text input.
    TextField(TextFieldProps) = 6,
    /// A native image.
    Image(ImageProps) = 7,
    /// A native on/off switch.
    Switch(SwitchProps) = 10,
    /// A native checkbox with a label.
    Checkbox(CheckProps) = 12,
    /// A native radio button with a label.
    RadioButton(CheckProps) = 13,
    /// A native slider.
    Slider(SliderProps) = 14,
    /// A native determinate progress indicator.
    Progress(ProgressProps) = 15,
    /// A native group of mutually exclusive radio buttons.
    RadioGroup(RadioGroupProps) = 16,
    /// A native indeterminate progress spinner.
    Spinner(SpinnerProps) = 17,
    /// A native date, time or date-and-time picker.
    DatePicker(DatePickerProps) = 18,
    /// A native tab bar.
    TabBar(TabBarProps) = 19,
}
```

`widgets!` is a `macro_rules!` macro defined a few lines above. From each line
`Variant(Props) = tag` it generates:

- the enum variant `Widget::Variant(Props)`;
- an arm of `Widget::tag()` returning the number;
- an arm of `Widget::props()` and `Widget::props_mut()` returning the properties as
  `&dyn WidgetProps`.

The number is the identity tag from chapter 2. It is part of every node id, so it must be unique
and must never be reused for a different widget. The gaps in the sequence are the tags of the
structural kinds.

### Why one enum, matched exhaustively

The rule in this codebase is: code that needs to treat widgets differently matches on `Widget`
with every variant listed and no `_ =>` arm. Adding a line to the table then makes the compiler
stop at each such match. There are three kinds of places:

- the event resolver, `Actions::of` in `rust/src/host/events.rs`;
- each platform's "create a view" and "update a view" matches (in `android/` and `ios/`);
- nothing else on the core side, because of the trait below.

The history behind the rule: before the restructure, Android ignored updates to five widgets
(`RadioGroup`, `TabBar`, `Pager`, `DatePicker` and `Spinner` kept whatever they were created with),
because a match site had been missed and nothing reported it. An exhaustive match cannot have that
bug.

### `WidgetProps`: one trait instead of an arm per method

Three things the engine needs from every widget do not depend on which widget it is. Without care
each would be a 13-arm match. Instead the enum forwards to a trait:

```rust,ignore
// rust/src/widgets/mod.rs
pub(crate) trait WidgetProps: IntrinsicSize {
    /// Re-wraps every message the widget can produce.
    fn map_messages(&mut self, wrap: &MessageMap);

    /// The text style, for a widget that shows text.
    fn text_style(&mut self) -> Option<TextStyleMut<'_>> {
        None
    }

    /// Picks the value for `scheme` in every colour the widget has.
    fn resolve_colors(&mut self, scheme: ColorScheme);
}

pub(crate) trait IntrinsicSize {
    fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size;
}
```

and `Widget` has four one-line methods that call them through `props()` / `props_mut()`:

```rust,ignore
impl Widget {
    pub(crate) fn map_messages(&mut self, wrap: &MessageMap) { self.props_mut().map_messages(wrap); }
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        self.props().intrinsic_size(max_width, measurer)
    }
    pub(crate) fn resolve_colors(&mut self, scheme: ColorScheme) { /* ... */ }
    pub(crate) fn text_style(&mut self) -> Option<TextStyleMut<'_>> { /* ... */ }
}
```

Who calls them:

| Method | Called from | Purpose |
|---|---|---|
| `map_messages` | `RawElement::map_messages` | `Element::map` (chapter 2) |
| `text_style` | `RawElement::set_text_size` and friends | the `.size()`, `.color()`, `.bold()` builders |
| `resolve_colors` | `tree::resolve_colors` | dark theme (chapter 4) |
| `intrinsic_size` | `layout::content_size` | layout (chapter 5) |

`IntrinsicSize` is a separate supertrait so the split between generated and hand-written is visible
in the types: the derive implements `WidgetProps`, a person implements `IntrinsicSize`, and the
compiler refuses a widget that lacks either. `max_width` is the width available to the content,
`None` when unbounded. `measurer` is the platform's `Measure` (chapter 5), needed by anything
that contains text.

## The widgets that are not derived

Seven widgets use the derive: `Text`, `Button`, `TextField`, `Image`, `Switch`, `Slider` and
`RadioGroup`. The rest are written by hand. Each has a reason, with one exception.

### `check.rs`: two builders, one properties struct

`Checkbox` and `RadioButton` have the same properties (`label`, `checked`, `on_toggle`, text
style), so they share `CheckProps`, and the enum has two variants holding the same type:
`Checkbox(CheckProps) = 12` and `RadioButton(CheckProps) = 13`. The derive generates one builder
per struct, so it cannot produce both. The file writes both builders by hand; their methods are
identical except for the variant they match:

```rust,ignore
// rust/src/widgets/check.rs
pub fn on_toggle(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
where
    M: 'static,
{
    if let ElementKind::Widget(Widget::Checkbox(props)) = &mut self.0.kind {
        props.on_toggle = Handler::new(message);
    }
    self
}
```

The constructors also do something the derive has no option for: they copy the label into
`style.accessibility_label`.

The intrinsic size reserves room for the indicator: `CHECK_INDICATOR` (20) plus `CHECK_GAP` (28)
plus the measured label, at least `CHECK_HEIGHT` (48) tall. `RadioGroup` reuses the same constants
and stacks one such row per option.

### `progress.rs`: one builder, two widgets

The reverse case. `ProgressIndicator::new(0.5)` builds a `Widget::Progress(ProgressProps)` and
`ProgressIndicator::indeterminate()` builds a `Widget::Spinner(SpinnerProps)`. They are different
native views with different sizes (a bar that fills the width and is 6 points tall; a 24-point
square), so they are two variants, but applications think of them as one widget with two modes. The
`color` setter therefore matches on both:

```rust,ignore
// rust/src/widgets/progress.rs
match &mut self.0.kind {
    ElementKind::Widget(Widget::Progress(props)) => props.color = color,
    ElementKind::Widget(Widget::Spinner(props)) => props.color = color,
    _ => {}
}
```

Neither has a handler, so both `map_messages` implementations are empty.

### `date_picker.rs`: several constructors and value types

`DatePicker::new(date)`, `DatePicker::time(time)` and `DatePicker::date_time(date, time)` build the
same `DatePickerProps` with a different `DatePickerValue` variant. The derive only writes `new`.
The `range` and `time_range` setters take two arguments each. Most of the file is not builder code
at all: it defines `DateValue`, `TimeValue`, `DatePickerValue` and `DatePickerValue::clamped`, the
function the session uses to force a reported value into the widget's range before it becomes a
message.

The intrinsic size is a table of assumed sizes: 200, 120 or 240 points wide for a compact date,
time or date-and-time field, 48 tall; and for the inline mode a fixed block (320 by 320 for a date)
that shrinks to the available width.

### `tab_bar.rs`: one widget and one composite

`TabBar` is a plain leaf widget. `Tabs` in the same file is not a widget: it is a `Column` whose
first child is a `TabBar`, with `selected` and `on_select` reaching into `children.first_mut()` to
set the bar's properties. `Tabs::child` appends content below the bar.

**Rough edge.** `TabBar` itself has nothing the derive could not express (a `Vec<String>` argument,
one `set` field, one handler, three text fields). It appears not to have been converted. Its
`size`, `color` and `weight` are set with the shared `.size()`, `.color()` and `.bold()` from
`text_builder!(TabBar)`, like any other widget that shows text; on iOS the segmented control only
honours the colour.

### `pager.rs`: a container with a handler

`Pager` has children (its pages) and so cannot be a `Widget`, which by definition is a leaf. It is
`ElementKind::Pager(PagerProps)`. `PagerProps` holds `pages`, `selected` and `on_select`. The
`pages` count is not set by the builder; `tree::build_node` fills it in from the number of
children when it creates the node.

`TabsView` in the same file is a composite like `Tabs`: a `Column` of a `TabBar` and a `Pager`.
Its `on_select` gives both the same function, so tapping a tab and swiping a page report the same
message:

```rust,ignore
// rust/src/widgets/pager.rs
// The tab bar and the pager report the same selection.
let message = Arc::new(message);
// ... props.on_select = Handler::new(move |index| message(index));   (twice)
```

Both composites address their parts by position (`children.first_mut()`, `children.get_mut(1)`).
That is safe because the constructor is the only code that arranges those children.

### `list.rs`: generic, lazy, and converted late

`List<T, M>` is the only builder with a second type parameter and the only one that is not a
wrapper around a `RawElement`. It keeps its configuration in fields and builds the element in
`From<List<T, M>> for Element<M>`:

```rust,ignore
// rust/src/widgets/list.rs
let key = list.key.expect("List::key must be set");
let row = list.row.expect("List::row must be set");
// ...
let keys = { /* Arc<dyn Fn(usize) -> String>: index -> key(&items[index]) */ };
let rows = { /* Arc<dyn Fn(usize) -> RawElement>: index -> row(&items[index]) */ };
let mut element = RawElement::container(ElementKind::List(ListProps { /* ... */ keys, rows }));
// A list is a scroll region: it grows into the space its flex parent
// leaves and spans the cross axis.
element.style.width = Length::Percent(1.0);
element.style.grow = 1.0;
```

The conversion erases the item type `T`: the engine sees only "a function from an index to a key"
and "a function from an index to a row element", plus a count. The items are held in an
`Arc<Vec<T>>` captured by both closures, which is why the docs recommend passing an `Arc` to avoid
copying the data on every render. How the engine uses these closures is the subject of chapter 4.

Two things to know:

- Forgetting `.key(...)` or `.row(...)` is a panic at conversion time, not a compile error.
- Because `List` does not go through `style_builder!`, it has none of the shared style methods. To
  size or decorate a list, wrap it in a `Column`.

`ListProps` implements `PartialEq` and `Debug` by hand because closures have neither. Equality
compares the configuration and `Arc::ptr_eq(&self.keys, &other.keys)`. It leaves out
`estimated_height` and the `rows` closure.

### `containers.rs` and `scroll.rs`

`Column`, `Row`, `Spacer` and `ScrollView` are the structural kinds. Each is a tuple struct with a
`new`, a `child` method that pushes `child.into().into_raw()`, a `From` conversion and
`style_builder!`. There are no properties structs (`Column` and `Row` have none; `ScrollProps` is
only the axis), no handlers and no intrinsic size: a container is sized by its children. `spacing`
on `Column` and `Row` writes `style.gap`. `Spacer::new` sets `style.grow = 1.0`.

`Column::child` takes `impl Into<Element<M>>`. That bound is the whole type-safety story for
messages: a `Button<Other>` does not convert into `Element<Msg>`, so it cannot be added to a
`Column<Msg>` without a `map`.

`Wrap` is not a kind of its own: it builds an `ElementKind::Row` and sets `style.wrap`, which the
layout turns into flexbox wrapping. `Container` has a properties struct with two handlers,
`on_tap` and `on_long_press`.

### Widgets assembled from others

`divider.rs`, `card.rs`, `list_tile.rs`, `avatar.rs`, `badge.rs`, `chip.rs`, `icon_button.rs`,
`expansion_tile.rs`, `stepper.rs`, `number_stepper.rs`, `search_bar.rs`, `autocomplete.rs` and
`grid.rs` add no `Widget` variant and no native code. Each is a builder whose conversion into an
`Element` assembles containers, texts and images. The engine and the renderers never learn that a
chip exists; they see a container with a row in it.

The simple ones (`Divider`, `Card`) are one element and are built eagerly, as `TabsView` is. The
others collect their parts and assemble on conversion, so the order of the builder calls does not
matter:

```rust,ignore
// rust/src/widgets/list_tile.rs (trimmed)
pub struct ListTile<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>, Parts);

impl<M: 'static> From<ListTile<M>> for Element<M> {
    fn from(component: ListTile<M>) -> Self {
        let ListTile(shell, _, parts) = component;
        // ... build a row from parts.title, parts.leading, parts.trailing ...
        finish(shell, Container::new().handler(parts.on_tap).child(row))
    }
}

style_builder!(ListTile);
```

The first field is a *shell*: an element that exists only so `style_builder!` has a style and a
key to write to. `finish` (`rust/src/element.rs`) moves both onto the assembled root. Two rules
follow. The style the root needs (a chip's height and radius, a tile's minimum height) is set on
the shell in `new`, where the application's own style calls replace it. And a style set on the
assembled root inside `from` is lost, so nothing is set there.

Handlers are stored already erased, as `Handler<()>`, which is why these builders need no
`M: Clone` to be converted. A widget that reports a value keeps the application's function in an
`Arc` and makes one handler per button from it, as `NumberStepper` does for minus and plus.

`Grid` is the one that wraps a widget with behaviour: it builds a `List` over line numbers whose
rows are `Row`s of `columns` cells, so windowing and the recyclers work without knowing about it.

## What adding a widget touches on the core side

1. A new file in `rust/src/widgets/` with the properties struct, the derive, and
   `impl IntrinsicSize`.
2. A `mod` and a `pub use` line in `rust/src/widgets/mod.rs`.
3. One line in the `widgets!` table with an unused tag.
4. An arm in `Actions::of` (`rust/src/host/events.rs`), which the compiler will demand.
5. If the widget reports a new kind of event: a `NodeEvent` variant, a field in `Actions`, and an
   arm in `Actions::message`.

Everything else is on the platform side, where the compiler will also point at each match.

## Where this lives

- `rust/src/widgets/mod.rs`: `widgets!`, `Widget`, `WidgetProps`, `IntrinsicSize`,
  `TextStyleMut`, `CONTROL_WIDTH`.
- `rust/src/widgets/button.rs`, `slider.rs`, `text.rs`, `text_field.rs`: derived widgets, good
  templates.
- `rust/src/widgets/check.rs`, `progress.rs`, `date_picker.rs`, `tab_bar.rs`: hand-written leaf
  widgets.
- `rust/src/widgets/containers.rs`, `scroll.rs`, `pager.rs`, `list.rs`: structural kinds.
- `rust/macros/src/widget.rs`: the derive; its tests show the exact generated text.
- `rust/src/engine/layout.rs`, the `new_controls_have_intrinsic_sizes` test and its neighbours:
  the intrinsic sizes checked through a real layout.

Next: [4. The tree and identity](04-the-tree-and-identity.md).
