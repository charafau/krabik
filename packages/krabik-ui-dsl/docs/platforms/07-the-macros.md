# The macros

This chapter explains the three macros in the project: `#[derive(Widget)]`, which writes a
widget's builder; `widgets!`, which declares the `Widget` enum from a table; and `view!`, which
lets an app write a widget tree declaratively. For each it shows a real input and the full code
it expands to, so that afterwards you can read `rust/macros/src/`, predict what any use of a
macro produces, and know what the macros deliberately leave to hand-written code.

The guiding rule, from the design notes: **macros generate boilerplate, not behaviour.**

## What a proc macro is

Rust has two kinds of macros. A *declarative* macro (`macro_rules!`) is a set of patterns: "when
the input looks like this, replace it with that". It lives in the crate that uses it and can only
rearrange what it was given. `widgets!` is one. A *procedural* macro is an ordinary Rust function
that the compiler runs **at compile time**. The compiler hands it the source tokens of the thing
the macro is attached to, the function returns new tokens, and the compiler compiles those in
place. Because it is real code, it can inspect its input: look at each field of a struct, read
its type and attributes, and generate a different method for each.

Procedural macros must live in their own crate marked `proc-macro = true`, which is why
`rust/macros/` exists (`krabik-ui-dsl-macros`). Such a crate can export only macros. Three
helper crates do the work: `syn` parses tokens into a syntax tree (`DeriveInput`, `Field`,
`Expr`), `quote` turns a template with `#variable` holes back into tokens, and `proc-macro2`
provides a token type that also works outside the compiler, which matters for testing. There are
three flavours: a *derive* macro (`#[derive(Widget)]`) adds code next to a struct without changing
it; a *function-like* macro (`view! { ... }`) replaces its invocation; an attribute macro
replaces the item it is on (not used here).

```rust,ignore
// rust/macros/src/lib.rs
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    view::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_derive(Widget, attributes(widget))]
pub fn derive_widget(input: TokenStream) -> TokenStream {
    widget::expand(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
```

`lib.rs` contains only these two thin wrappers. `attributes(widget)` tells the compiler that
`#[widget(...)]` is a helper attribute belonging to this derive, so it is allowed on the struct
and its fields. On an error, `into_compile_error` turns the `syn::Error` into a `compile_error!`
at the right place in the user's source.

## `#[derive(Widget)]`

### The problem it solves

Every widget is two types. A *properties struct* (`SliderProps`) is the plain data the engine
stores, compares and hands to the renderers. A *builder* (`Slider<M>`) is what an app writes:
`Slider::new(0.5).min(0.0).on_change(Msg::Volume)`. The builder is generic over the app's message
type `M`; the properties are not, because handlers are type-erased where an element enters the
engine. Writing the builder by hand means one constructor, one setter per property, one event
method per handler, a conversion into `Element<M>`, and a trait implementation. All of it is
mechanical, and all of it must stay in step with the struct. The derive writes it from the struct.

### Attributes

On the struct, inside `#[widget(...)]` (`struct_config` in `rust/macros/src/widget.rs`):

| Option | Meaning |
| --- | --- |
| `name = Slider` | The builder type to generate. Required. |
| `variant = Other` | The `Widget` enum variant, when it differs from `name`. |
| `about = "..."` | Doc comment for the builder type. |
| `new = "..."` | Doc comment for the constructor. |
| `text` | The widget shows text: add the shared `size`, `color` and `bold` setters. |

On a field (`field_config`):

| Option | Meaning |
| --- | --- |
| `arg` | The field is a constructor argument. |
| `default = expr` | Its initial value. Without it: `Default::default()`. |
| `set` | Generate a setter named after the field. |
| `set(name = other)` | Name the setter differently. |
| `set(into)` | Accept `impl Into<FieldType>`. Automatic for `String` fields. |
| `set(some)` | The field is an `Option<T>`; the setter takes a `T` and stores `Some`. |
| `set(with = function)` | Pass the argument through a function before storing. |
| `set(from = Type)` | The setter's parameter type, when it differs from the field's. |
| `set(flag = value)` | A setter with no argument that stores `value`. |

Two things need no attribute. A field of type `Handler<()>` gets an event method that takes a
message. A field of type `Handler<T>` gets an event method that takes a function from `T` to a
message. And every field of type `Color` is added to the generated `resolve_colors`.

The macro recognises these types **by name**: `is_named(ty, "Color")` checks that the last path
segment is `Color` with no type arguments, so `Color`, `style::Color` and `crate::style::Color`
all count, and `generic_payload` looks at the last path segment for `Handler`, `Option` or `Vec`.
A colour inside an `Option` or a `Vec` is resolved too. It cannot resolve types: a type alias
(`type Tint = Color`) is not recognised, and an unrelated type that happens to be named `Color`
is. That is a general limit of derive macros, which see tokens, not types. A field that holds a
colour which must not follow the appearance is marked `#[widget(fixed_color)]`.

### A real input

```rust,ignore
// rust/src/widgets/slider.rs
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = Slider,
    about = "A native slider.",
    new = "Creates a slider with the given value and the default range `0..=1`."
)]
pub struct SliderProps {
    /// The current value.
    #[widget(arg)]
    pub value: f32,
    /// The lowest selectable value.
    #[widget(default = 0.0, set)]
    pub min: f32,
    /// The highest selectable value.
    #[widget(default = 1.0, set)]
    pub max: f32,
    /// The interval values snap to. A non-positive step, or `None`, means a
    /// continuous slider.
    #[widget(set(with = positive_step, from = f32))]
    pub step: Option<f32>,
    /// Reports the new value whenever it changes.
    pub on_change: Handler<f32>,
    /// Color of the active track and thumb.
    #[widget(default = Color::ACCENT, set(into))]
    pub color: Color,
}

fn positive_step(step: f32) -> Option<f32> {
    (step > 0.0).then_some(step)
}
```

### What it expands to

Written out by hand from reading `expand` in `rust/macros/src/widget.rs`. The doc comments on the
fields are copied onto the generated methods; they are shortened here. `ACCESS` stands for the
pattern `crate::element::ElementKind::Widget(crate::widgets::Widget::Slider(props))`, which every
setter repeats in full.

```rust,ignore
// 1. A constructor on the properties struct.
impl SliderProps {
    #[doc = "Creates a slider with the given value and the default range `0..=1`."]
    pub fn new(value: f32) -> Self {
        Self {
            value,                                          // #[widget(arg)]
            min: 0.0,                                       // default = 0.0
            max: 1.0,                                       // default = 1.0
            step: ::core::default::Default::default(),      // no default given
            on_change: crate::message::Handler::none(),     // a Handler field
            color: Color::ACCENT,                           // default = Color::ACCENT
        }
    }
}

// 2. The builder type.
#[doc = "A native slider."]
pub struct Slider<M = ()>(
    pub(crate) crate::element::RawElement,
    pub(crate) ::core::marker::PhantomData<fn() -> M>,
);

impl<M> Slider<M> {
    #[doc = "Creates a slider with the given value and the default range `0..=1`."]
    pub fn new(value: f32) -> Self {
        Self(
            crate::element::RawElement::leaf(crate::element::ElementKind::Widget(
                crate::widgets::Widget::Slider(SliderProps::new(value)),
            )),
            ::core::marker::PhantomData,
        )
    }

    /// The lowest selectable value.                      // from `set`
    pub fn min(mut self, min: f32) -> Self {
        if let ACCESS = &mut self.0.kind {
            props.min = min;
        }
        self
    }

    /// The highest selectable value.
    pub fn max(mut self, max: f32) -> Self {
        if let ACCESS = &mut self.0.kind {
            props.max = max;
        }
        self
    }

    /// The interval values snap to. ...                  // from = f32, with = positive_step
    pub fn step(mut self, step: f32) -> Self {
        if let ACCESS = &mut self.0.kind {
            props.step = positive_step(step);
        }
        self
    }

    /// Reports the new value whenever it changes.        // Handler<f32>
    pub fn on_change(
        mut self,
        message: impl Fn(f32) -> M + Send + Sync + 'static,
    ) -> Self
    where
        M: 'static,
    {
        if let ACCESS = &mut self.0.kind {
            props.on_change = crate::message::Handler::new(message);
        }
        self
    }

    /// Color of the active track and thumb.              // set(into)
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        if let ACCESS = &mut self.0.kind {
            props.color = color.into();
        }
        self
    }
}

// 3. Conversion into the typed element an app's `view` returns.
impl<M> From<Slider<M>> for crate::element::Element<M> {
    fn from(component: Slider<M>) -> Self {
        crate::element::Element::from_raw(component.0)
    }
}

// 4. The style setters every builder has (padding, width, background, ...).
crate::element::style_builder!(Slider);

// 5. What the `Widget` enum forwards to.
impl crate::widgets::WidgetProps for SliderProps {
    fn map_messages(&mut self, wrap: &crate::message::Wrap) {
        let _ = wrap;
        self.on_change.map(wrap);
    }

    fn resolve_colors(&mut self, scheme: crate::style::ColorScheme) {
        let _ = scheme;
        self.color = self.color.resolve(scheme);
    }
}
```

Reading the pieces:

- **The builder is a newtype around `RawElement`.** `RawElement` is the untyped description the
  engine consumes. `PhantomData<fn() -> M>` records the message type without storing a value of
  it. The default `M = ()` lets a widget that sends no messages be written without a type
  annotation.
- **Every setter matches on the element kind** (`if let ACCESS = &mut self.0.kind`). The builder
  holds a general `RawElement`, so the setter has to reach inside it to the `SliderProps`. The
  pattern always matches for a builder created by `new`; the `if let` is there because the type
  system does not know that.
- **`on_change` takes `impl Fn(f32) -> M`.** In an app, `Msg::Volume` (a tuple variant
  constructor) is such a function. `Handler::new` erases the type. A `Handler<()>` field, as on
  a button, generates the other shape: `pub fn on_click(mut self, message: M)` with
  `M: Clone + Send + Sync + 'static`, storing `Handler::new(move |()| message.clone())`.
- **`map_messages`** lists every handler field. `Element::map` calls it to lift a child
  component's messages into its parent's. Forgetting a handler here would make that widget's
  events vanish inside a mapped component, which is exactly the kind of omission a generated list
  prevents.
- **`resolve_colors`** lists every `Color` field. The runtime calls it while building the tree,
  with the current light or dark scheme. This is how adaptive colours reach every widget without
  each one remembering to.
- `let _ = wrap;` and `let _ = scheme;` silence unused-variable warnings for a widget with no
  handlers or no colours.

With `text` on the struct, two more things are generated: a `text_style` method in the
`WidgetProps` impl that returns mutable references to the `size`, `color` and `weight` fields
(which must exist with those names), and `crate::element::text_builder!(Name);`, a declarative
macro in `rust/src/element.rs` that adds `size`, `color` and `bold` to the builder. `Text` uses
it; `Slider` does not, so its `color` setter comes from `set(into)` instead.

The generated code refers to the framework through `crate::`. The derive is therefore usable only
inside `krabik-ui-dsl`. An application cannot derive its own widget, and since a widget also needs
platform code inside the framework, that is consistent.

### Errors

The derive reports mistakes with a message at the offending place: a missing `name`, use on an
enum or a tuple struct, an unknown option, `arg` together with `default`, `set` on a `Handler`
field ("a `Handler` field already gets an event method; remove `set`"), and `set(some)` on a
field that is not an `Option`.

### Which widgets use it

`button.rs`, `image.rs`, `radio_group.rs`, `slider.rs`, `switch.rs`, `text.rs` and
`text_field.rs` derive their builder. `check.rs`, `date_picker.rs`, `progress.rs` and
`tab_bar.rs` still write the builder and the `WidgetProps` implementation by hand. The source
does not say why. Two visible differences from the derived ones: `CheckProps` backs two builders
(`Checkbox` and `RadioButton`), and the date picker has more than one constructor
(`DatePicker::time`), neither of which the derive supports. The containers, the list, the pager
and the scroll view are not `Widget` variants at all. So the hand-written `map_messages` and
`resolve_colors` in those four files must still be kept complete by hand.

## The `widgets!` table

`widgets!` is a declarative macro defined and used once, in `rust/src/widgets/mod.rs`.

```rust,ignore
// rust/src/widgets/mod.rs
// Tags must never be reused: they are part of a node's identity. The
// containers in `ElementKind` use 1, 2, 3, 8, 9, 11 and 20.
widgets! {
    /// A run of text rendered by a native label.
    Text(TextProps) = 4,
    /// A native button.
    Button(ButtonProps) = 5,
    // ...
    /// A native slider.
    Slider(SliderProps) = 14,
    // ...
}
```

One line per widget: the variant, its properties type, and a numeric tag. The pattern
`$($(#[$doc:meta])* $variant:ident($props:ty) = $tag:literal,)+` matches each line, and the body
produces:

```rust,ignore
// what widgets! expands to (shortened to three variants)
#[derive(Clone, Debug, PartialEq)]
pub enum Widget {
    /// A run of text rendered by a native label.
    Text(TextProps),
    /// A native button.
    Button(ButtonProps),
    /// A native slider.
    Slider(SliderProps),
}

impl Widget {
    pub(crate) fn tag(&self) -> u64 {
        match self {
            Self::Text(_) => 4,
            Self::Button(_) => 5,
            Self::Slider(_) => 14,
        }
    }

    fn props(&self) -> &dyn WidgetProps {
        match self {
            Self::Text(props) => props,
            Self::Button(props) => props,
            Self::Slider(props) => props,
        }
    }

    fn props_mut(&mut self) -> &mut dyn WidgetProps {
        match self {
            Self::Text(props) => props,
            Self::Button(props) => props,
            Self::Slider(props) => props,
        }
    }
}
```

The point is `props` and `props_mut`. `map_messages`, `intrinsic_size`, `resolve_colors` and
`text_style` on `Widget` are each one line that forwards through `props()` to the `WidgetProps`
trait. Without the table, each would be a `match` with one arm per widget, and adding a widget
would mean editing four matches whose arms all say the same thing. With it, adding a widget is
one line here.

The **tag** is mixed into node identifiers, so a node that changes from one widget type to
another gets a new identity and is recreated instead of updated. Tags are never reused, and the
numbers have gaps because the containers hold some of them.

Why a declarative macro and not a proc macro? The job is pure repetition of a fixed shape, which
`macro_rules!` does well, and it keeps the list of widgets readable in the file that defines the
enum.

What the table does *not* remove: the matches that do something *different* per widget. Those
stay hand-written and exhaustive, with no `_ =>` arm, so the compiler lists them when a line is
added: `Actions::of` in `rust/src/host/events.rs`, `create_widget` and `configure_view` in
`rust/src/android/widgets/mod.rs`, and `create_widget` and `update_content` in
`rust/src/ios/widgets/mod.rs`.

## `view!`

### What it is

`view!` is syntax and nothing else. It expands to the builder calls an app could write by hand.
It knows nothing about any particular widget: there is no list of widget names inside the macro.
That is why any builder works with it, including one an application defines itself, without
registering anything.

### Grammar

Inside `view! { ... }` there is exactly one root, which must be a node or an ordinary expression.
A **child** is one of five things (`enum Child` in `rust/macros/src/view.rs`):

```text
child      :=  node | embedded | if | for | match
node       :=  Path [ "(" arguments ")" ]
               { "." method "(" args ")" }
               [ "{" children "}" ]
arguments  :=  positional... , then  name ":" expr  or  bare-name ...
embedded   :=  any Rust expression that produces an element
if         :=  "if" expr "{" children "}" [ "else" (if | "{" children "}") ]
for        :=  "for" pattern "in" expr "{" children "}"
match      :=  "match" expr "{" arm... "}"
arm        :=  pattern [ "if" expr ] "=>" ( child | "{" children "}" )
children   :=  child...    (commas and semicolons between them are optional)
```

The rules for a node, with the code each produces:

| You write | It becomes |
| --- | --- |
| `Text("Hi")` | `Text::new("Hi")` |
| `Spacer` | `Spacer::new()` |
| `Text("Hi", size: 20)` | `Text::new("Hi").size(20)` |
| `Text("Hi", size: 20, bold)` | `Text::new("Hi").size(20).bold()` |
| `DatePicker::time(t, on_change: Msg::Time)` | `DatePicker::time(t).on_change(Msg::Time)` |
| `ui::Text("Hi")` | `ui::Text::new("Hi")` |
| `Text("Hi").bold().size(12)` | `Text::new("Hi").bold().size(12)` |
| `Column { ... }` | a block that calls `.child(...)` for each child |

Inside the parentheses, arguments come in two groups. Plain expressions first: they go to the
constructor. Then settings: `name: value` calls `.name(value)`, and a bare name calls `.name()`.
A bare name counts as a setting only **after** the first `name: value`; before that it is a
constructor argument, so `Text(title)` passes the variable `title`. A constructor argument after
a setting is an error ("constructor arguments must come before `name: value` settings").

How does the parser tell `name: value` from an expression such as `Msg::Add`? It peeks: an
identifier followed by a single colon, and not by `::`.

```rust,ignore
// rust/macros/src/view.rs
let named =
    content.peek(Ident) && content.peek2(Token![:]) && !content.peek2(Token![::]);
```

If the last segment of the path is lowercase (`DatePicker::time`), the path is already a
constructor and is called as written; otherwise `::new` is appended (`node_expression`).

### How a node is told apart from an ordinary expression

A child that is not `if`, `for` or `match` could be a node (`Text("Hi")`) or ordinary Rust
(`self.footer()`, `row(item)`). The macro has no list of widgets, so it uses a convention:

```rust,ignore
// rust/macros/src/view.rs
/// Builders are types, so their names are capitalized: `Text`, `ui::Column`,
/// `DatePicker::time`. Anything else, such as `self.footer()` or `row(item)`,
/// is ordinary Rust that produces an element.
fn starts_with_node(input: ParseStream) -> bool {
    let fork = input.fork();
    fork.call(Path::parse_mod_style).is_ok_and(|path| {
        path.segments.iter().any(|segment| {
            segment
                .ident
                .to_string()
                .chars()
                .next()
                .is_some_and(char::is_uppercase)
        })
    })
}
```

It looks ahead (on a `fork`, so nothing is consumed) for a path and asks whether any segment
starts with an uppercase letter. Rust's naming convention does the rest: types are capitalised,
functions and methods are not.

- `Text("Hi")`: one capitalised segment, a node.
- `row(item)`: lowercase, an embedded expression.
- `self.footer()`: the path is `self`, lowercase, embedded.

The convention has edges worth knowing:

- A capitalised name that is not a builder is still treated as a node. A constant used as a
  child, `HEADER`, becomes `HEADER::new()` and fails to compile with a confusing message. Writing
  it in parentheses, `(HEADER)`, makes it an ordinary expression, because a parenthesis cannot
  start a path.
- `Self::header(x)` works by accident of the rules: a capitalised segment makes it a node, and
  the lowercase last segment means it is called as written.
- A bare `{ ... }` block as a child is rejected with an explicit message, because it would be
  ambiguous with a node's children.

### Why children are added statement by statement

A node with children expands to a block with a mutable variable:

```rust,ignore
// rust/macros/src/view.rs
quote! {
    {
        let mut __node = #builder;
        #statements
        __node
    }
}
```

and each child becomes one statement, `__node = __node.child(...);`. The alternative would be a
single chained expression, `Column::new().child(a).child(b)`. That cannot express "add this child
only if a condition holds" or "add one child per item" without inventing helper methods on every
container. With statements, `if`, `for` and `match` are emitted as the Rust control flow they look
like, with child statements in their bodies:

```rust,ignore
// rust/macros/src/view.rs
Child::For { pattern, iterator, body } => {
    let body = child_statements(body);
    quote! { for #pattern in #iterator { #body } }
}
```

The comment on `child_statements` states it: children are added one at a time "so `if`, `for`
and `match` can simply be the Rust control flow they look like". The only requirement on a
container is a `child` method that takes and returns `self`, which `Column`, `Row` and the others
have:

```rust,ignore
// rust/src/widgets/containers.rs
pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
    self.0.children.push(child.into().into_raw());
    self
}
```

Nested nodes each get their own block, so the inner `__node` shadows the outer one only inside
that block. The double underscore keeps the name out of the way of anything an app would write.

### One input and its full expansion

The input, adapted from `example-todo-minimal/rust/src/lib.rs`:

```rust,ignore
view! {
    Column(padding: 24, spacing: 12) {
        Text("Todo", size: 30, color: COLOR_TEXT, bold)
        caption(&self.summary())
        Row(width_percent: 1.0, spacing: 8) {
            TextField(
                self.draft.clone(),
                placeholder: "What needs doing?",
                on_change: Msg::Draft,
                on_submit: Msg::Add,
                grow: 1.0,
            )
            Button("Add", on_click: Msg::Add)
        }
        if rows.is_empty() {
            caption("Nothing here yet.")
        }
        for row in &rows {
            todo_row(row)
        }
    }
}
```

The expansion, formatted:

```rust,ignore
::krabik_ui_dsl::ui::Element::from({
    let mut __node = Column::new().padding(24).spacing(12);
    __node = __node.child(Text::new("Todo").size(30).color(COLOR_TEXT).bold());
    __node = __node.child(caption(&self.summary()));
    __node = __node.child({
        let mut __node = Row::new().width_percent(1.0).spacing(8);
        __node = __node.child(
            TextField::new(self.draft.clone())
                .placeholder("What needs doing?")
                .on_change(Msg::Draft)
                .on_submit(Msg::Add)
                .grow(1.0),
        );
        __node = __node.child(Button::new("Add").on_click(Msg::Add));
        __node
    });
    if rows.is_empty() {
        __node = __node.child(caption("Nothing here yet."));
    }
    for row in &rows {
        __node = __node.child(todo_row(row));
    }
    __node
})
```

Things to notice:

- `caption(...)` and `todo_row(row)` are lowercase, so they are embedded as written. They are
  ordinary functions in the app that return a builder or an element.
- `bold` comes after settings, so it is `.bold()`. `self.draft.clone()` comes before any setting,
  so it is the constructor argument.
- `grow`, `padding` and `width_percent` are style setters from `style_builder!`; `placeholder`
  and `on_change` come from the derive. The macro neither knows nor cares which is which.
- The whole thing is wrapped in `::krabik_ui_dsl::ui::Element::from(...)`, so `view!` always
  yields an `Element`. The path is absolute so it works in any app. For the framework's own tests
  to use the macro, `rust/src/lib.rs` contains `extern crate self as krabik_ui_dsl;`, which lets
  the crate refer to itself by that name.
- Types are checked after expansion, by the ordinary compiler. A misspelled setting is reported
  as "no method named ... found" on the builder type, pointing at the name you wrote.

### Errors

`view!` reports structural mistakes itself: an empty macro, more than one root, a root that is
`if`/`for`/`match`, a constructor argument after a setting, a bare block as a child, and
`.name` without a call after a node.

## How the macros are tested without a compiler in the loop

A proc macro function receives `proc_macro::TokenStream`, a type that only exists while the
compiler is running a macro. Code that uses it cannot be called from a unit test. So all logic is
written against `proc_macro2::TokenStream`, the look-alike that works anywhere:

```rust,ignore
// rust/macros/src/view.rs
pub(crate) fn expand(input: TokenStream) -> syn::Result<TokenStream> { /* ... */ }
```

The two functions in `lib.rs` only convert between the token types. The tests in `view.rs` and
`widget.rs` build an input with `quote!`, call `expand`, and compare the output as text:

```rust,ignore
// rust/macros/src/view.rs
fn same(input: TokenStream, expected: TokenStream) {
    assert_eq!(expanded(input), expected.to_string());
}

#[test]
fn a_node_calls_new_then_its_settings() {
    same(
        quote! { Text("Hello", size: 20, color: BLUE, bold) },
        quote! { Text::new("Hello").size(20).color(BLUE).bold() },
    );
}
```

`expanded` strips the outer `Element::from( ... )` and returns the rest as a string. Both sides
go through the same token printer, so whitespace does not matter. The `view!` tests compare whole
expansions against hand-written code; the derive's tests check that the output `contains` the
important lines (the printer puts spaces between tokens, hence strings like `"pub struct Badge <
M = () >"`). Both files also have a test named `mistakes_are_reported_in_plain_words` that feeds
bad input and checks the error text.

These tests run in milliseconds with `cargo test --workspace` in `rust/`, on the host, with no
target toolchain. What they do not prove is that an expansion *compiles*. That is covered
elsewhere: seven widgets in the framework use the derive, a test in `rust/src/ui.rs` builds the
same tree with `view!` and with builders and compares them, and `example-todo-minimal` is written
with `view!`. There are no compile-fail tests (the `trybuild` kind), so the wording of errors
that come from the *compiler* after expansion is not pinned down.

## What the macros deliberately do not generate

- **Intrinsic size.** `impl IntrinsicSize for SliderProps` in `rust/src/widgets/slider.rs` is
  written by hand: "a slider fills the width it is given and has a fixed touch height". How big a
  widget wants to be is a decision, different for each one.
- **Platform code.** Nothing under `rust/src/android/widgets/` or `rust/src/ios/widgets/` is
  generated, and no Kotlin or Swift. How a slider maps onto `SeekBar` and `UISlider` is behaviour.
- **Event wiring.** `Actions::of` in `rust/src/host/events.rs` says which handler answers which
  native event, by hand.
- **The enum entry and tag.** One line in the `widgets!` table, chosen by a person, because tags
  must never be reused.
- **Anything in `view!` beyond syntax.** No keys, no diffing hints, no widget registry.

The reason is the same each time. Generated code is right for things that follow mechanically
from the struct: a setter per field, a list of handlers, a list of colours. It is wrong for
decisions. When a decision is hand-written and matched exhaustively, forgetting it is a compile
error with a file name. When it is generated from a default, forgetting it is a widget that
quietly does nothing, which is the failure this codebase already had once on Android and was
restructured to prevent.

## Where this lives

- `rust/macros/Cargo.toml`: `proc-macro = true`, `syn`, `quote`, `proc-macro2`.
- `rust/macros/src/lib.rs`: the two entry points and the user-facing documentation.
- `rust/macros/src/widget.rs`: `expand`, `struct_config`, `field_config`, `setter_method`,
  `constructor_argument`, and the tests.
- `rust/macros/src/view.rs`: `Child`, `Node`, `starts_with_node`, `node_expression`,
  `child_statements`, and the tests.
- `rust/src/widgets/mod.rs`: `widgets!`, `Widget`, `WidgetProps`, `IntrinsicSize`.
- `rust/src/element.rs`: `style_builder!`, `text_builder!`.
- `rust/src/widgets/slider.rs`: the worked example.
- `example-todo-minimal/rust/src/lib.rs`: an app written with `view!`.

Next: [One tap on each platform](08-one-tap-on-each-platform.md).
