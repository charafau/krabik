# 2. Elements and messages

This chapter covers `rust/src/element.rs` and `rust/src/message.rs`: the data structure `view()`
returns, and how a typed message such as `Msg::Add` travels through code that does not know the
type `Msg`. After reading it you will understand why `Element<M>` is generic but the engine is not,
why two closures can be "equal", and how `Element::map` composes components.

One example runs through the chapter:

```rust,ignore
Button::new("+1").on_click(Msg::Add)
```

## Two types for one thing: `Element<M>` and `RawElement`

An application's `view` returns `Element<Msg>`. The type parameter is what lets the compiler check
that every widget in the tree sends messages of the same type. But the engine, the session and both
renderers must work for *any* application, and making all of them generic over `M` would duplicate
their code for every message type and push a type parameter through every signature.

So the typed part is a thin wrapper around an untyped description:

```rust,ignore
// rust/src/element.rs
pub struct Element<M = ()> {
    raw: RawElement,
    message: PhantomData<fn() -> M>,
}

pub struct RawElement {
    pub(crate) kind: ElementKind,
    pub(crate) style: Style,
    pub(crate) key: Option<Cow<'static, str>>,
    pub(crate) children: Vec<RawElement>,
}
```

`Element<M>` stores no value of type `M`. `PhantomData<fn() -> M>` is a zero-sized marker that
makes the compiler treat the struct as if it produced `M` values. The `fn() -> M` form is chosen
(over plain `PhantomData<M>`) so that `Element<M>` is `Send` and `Sync` whatever `M` is, and so the
compiler does not assume the element owns an `M` that needs dropping.

`RawElement` is the real data. It has four fields:

- `kind`: what the element is (below).
- `style`: layout and decoration shared by every element: padding, size, background, animation and
  so on (`Style` in `rust/src/style.rs`).
- `key`: an optional identity among siblings. It is a `Cow<'static, str>` so that a literal
  (`.key("header")`) costs nothing and a computed `String` (`.key(item.id.to_string())`) also works.
- `children`: nested elements, already raw.

Because the marker is the only difference, `Element<M>` implements `Deref<Target = RawElement>`.
That is why tests can write `runtime.update(&element, viewport)` with an `Element` although
`Runtime::update` takes `&RawElement`. `Clone`, `PartialEq` and `Debug` are implemented by hand so
they do not require `M: Clone` and so on; they forward to the raw value.

The two crossings are `Element::from_raw` and `Element::into_raw`, both `pub(crate)`. Widgets call
`from_raw` when they convert into an element. The application layer calls `into_raw` at exactly one
place per hook, in the blanket `AnyApp` implementation (chapter 9):

```rust,ignore
// rust/src/app.rs
impl<A: App> AnyApp for A {
    fn view(&self) -> RawElement {
        Component::view(self).into_raw()
    }
    // ...
}
```

Everything below that line is non-generic.

## `ElementKind`

```rust,ignore
// rust/src/element.rs
pub(crate) enum ElementKind {
    Column,
    Row,
    Spacer,
    Pager(PagerProps),
    /// A leaf widget; see [`Widget`].
    Widget(Widget),
    Scroll(ScrollProps),
    List(ListProps),
}
```

There are two groups. `Column`, `Row`, `Spacer`, `Pager`, `Scroll` and `List` are structural: they
have children, or generate children, or exist only for layout. Everything else, every leaf that is
drawn by one native view, is a `Widget` (chapter 3). The split matters because the engine treats
the structural kinds individually (a list is expanded, a scroll container reports a content size, a
pager lays its children side by side), while all widgets go through the same code path and differ
only in their properties.

Each kind has a numeric tag:

```rust,ignore
// rust/src/element.rs
pub(crate) fn tag(&self) -> u64 {
    match self {
        Self::Column => 1,
        Self::Row => 2,
        Self::Spacer => 3,
        Self::Pager(_) => 20,
        Self::Widget(widget) => widget.tag(),
        Self::Scroll(props) => match props.axis {
            Axis::Vertical => 8,
            Axis::Horizontal => 9,
        },
        Self::List(_) => 11,
    }
}
```

The tag is mixed into the node's identifier (chapter 4). Two elements at the same position but of
different kinds therefore get different ids, and the diff sees "one removed, one created" instead
of trying to update a label into a button. The scroll axis is part of the tag for the same reason:
in the words of the comment on `tag`, "switching it rebuilds the subtree, because the native scroll
container changes class". Tags must never
be reused; the comment above the `widgets!` table in `rust/src/widgets/mod.rs` lists which numbers
are taken.

## The builder macros

Every widget builder is a tuple struct around a `RawElement`:

```rust,ignore
// what #[derive(Widget)] generates for Button, and what containers.rs writes by hand
pub struct Button<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);
```

All of them need the same thirty or so methods for padding, size, background and so on. These come
from one declarative macro, `style_builder!`, defined in `element.rs`. It expands to an
`impl<M> $component<M>` whose methods each set one field of `self.0.style` and return `self`:

```rust,ignore
// rust/src/element.rs, inside style_builder!
pub fn padding(mut self, value: impl $crate::style::IntoPoints) -> Self {
    self.0.style.padding = $crate::style::EdgeInsets::all(value.into_points());
    self
}
// ... margin, width, height, grow, background, border, opacity, elevation,
// animate, animate_enter, animate_exit, enabled, accessibility_label, key ...
```

`IntoPoints` is implemented for `f32` and `i32`, which is why both `.padding(16)` and
`.padding(16.0)` compile.

The macro also generates one conversion that is easy to miss:

```rust,ignore
impl From<$component<()>> for $crate::element::RawElement { /* ... */ }
```

A widget whose message type is `()` (it sends nothing, or the caller does not care) converts
straight into a `RawElement`. Engine tests use this to build trees without naming a message type.

`text_builder!` is the second macro. It adds `size`, `color` and `bold` to widgets that show text.
These cannot write to a fixed field, because each widget keeps its text properties in its own
properties struct. So they go through a small indirection:

```rust,ignore
// rust/src/element.rs
impl RawElement {
    fn text_style(&mut self) -> Option<crate::widgets::TextStyleMut<'_>> {
        match &mut self.kind {
            ElementKind::Widget(widget) => widget.text_style(),
            ElementKind::Column | ElementKind::Row | /* ... */ => None,
        }
    }

    pub(crate) fn set_text_size(&mut self, size: f32) {
        if let Some(text) = self.text_style() {
            *text.size = size;
        }
    }
    // set_text_color, set_text_weight
}
```

`TextStyleMut` is three mutable references (`size`, `color`, `weight`) that a widget's properties
hand out when it has them (chapter 3). The macros are exported with `pub(crate) use` at the bottom
of the file so that widget modules and the derive's output can call them.

Why declarative macros and not a trait with default methods? A trait would need every builder to
expose its inner `RawElement` through a method, and callers would need the trait in scope to call
`.padding()`. Inherent methods need no import and show up directly in each builder's documentation.

## `Message`, `Handler<A>` and where the type is erased

```rust,ignore
// rust/src/message.rs
pub(crate) type Message = Box<dyn Any>;

pub struct Handler<A>(Option<Arc<dyn Fn(A) -> Message + Send + Sync>>);
```

A `Handler<A>` answers one question: "an event with payload `A` happened; what should the
application be told?" The payload type is the event's data: `()` for a click, `String` for a text
change, `bool` for a toggle, `usize` for a selection, `f32` for a slider. The answer is a
`Message`, a box holding a value of a type only the application knows.

There are three states of knowledge about the message type, and the handler is the hinge between
them:

1. The widget builder knows `M`. `Button<M>::on_click` takes an `M`.
2. `Handler::new` is generic over `M` but its *result* is not. This is the erasure point.
3. Everything after that holds `Handler<()>` and sees only `Box<dyn Any>`.

```rust,ignore
// rust/src/message.rs
pub(crate) fn new<M: 'static>(make: impl Fn(A) -> M + Send + Sync + 'static) -> Self {
    Self(Some(Arc::new(move |payload| Box::new(make(payload)))))
}
```

The closure passed in returns `M`. The closure stored returns `Box<dyn Any>`: it calls the first
one and boxes the result. From here on nothing has a type parameter for the message.

The function is behind an `Arc` for two reasons. Elements and nodes are cloned (the node tree
clones each widget's properties out of the element, and `Actions::of` clones handlers out of the
node), and cloning an `Arc` is a counter increment. And `Send + Sync` is required because an
application may build its view on any thread.

The rest of the API is small:

- `Handler::none()`: the widget does not report this event.
- `is_set()`: whether there is a function.
- `message(payload)`: call the function, if any. `pub(crate)`: only the session calls it.
- `map(&MessageMap)`: wrap the function in another one (see `Element::map` below).

### The trace, first half: from builder call to `RawElement`

`Button::new("+1").on_click(Msg::Add)` runs these steps. The first two functions are generated by
`#[derive(Widget)]` from `ButtonProps` in `rust/src/widgets/button.rs`.

1. `Button::new("+1")` calls `ButtonProps::new("+1")`, which fills in the defaults declared on the
   struct: `label: "+1".into()`, `on_click: Handler::none()`, `size: 16.0`, `color: Color::TEXT`,
   `weight: FontWeight::Normal`.
2. It wraps the properties:
   `Button(RawElement::leaf(ElementKind::Widget(Widget::Button(props))), PhantomData)`.
   `RawElement::leaf` sets a default `Style`, no key and no children.
3. `.on_click(Msg::Add)` is the generated event method for a `Handler<()>` field. Because the
   payload is `()`, it takes a message value, not a function, and requires
   `M: Clone + Send + Sync + 'static`:

   ```rust,ignore
   // generated; see rust/macros/src/widget.rs
   pub fn on_click(mut self, message: M) -> Self {
       if let ElementKind::Widget(Widget::Button(props)) = &mut self.0.kind {
           props.on_click = crate::message::Handler::new(move |()| message.clone());
       }
       self
   }
   ```

   The message is cloned each time the button is pressed, which is why `Msg` must be `Clone` for
   unit events. Handlers with a payload (`TextField::on_change(Msg::Name)`) take a function
   `Fn(String) -> M` instead, and an enum variant with one field is such a function.
4. When the button is passed to `.child(...)` or returned with `.into()`, the generated
   `From<Button<M>> for Element<M>` calls `Element::from_raw(self.0)`.

The result is a `RawElement` whose `kind` is `Widget(Button(ButtonProps { on_click: <set>, .. }))`.
The `Msg` type is gone from every type signature; it survives only inside the closure.

### The trace, second half: from click to `update`

Later, the user presses the native button. The steps on the core side (chapter 8 has the detail):

1. The platform calls `Session::node_event(cx, host, id, NodeEvent::Click)`.
2. `actions_for` finds the node with that id in the host's current tree and calls `Actions::of`,
   which clones `props.on_click` into `actions.click`.
3. `Actions::message(&NodeEvent::Click)` calls `self.click.message(())`.
4. `Handler::message` runs the stored closure: `Box::new((move |()| message.clone())(()))`. The
   result is a `Box<dyn Any>` holding `Msg::Add`.
5. `Session::update` passes it to `AnyApp::update`, which gets the type back:

   ```rust,ignore
   // rust/src/app.rs
   fn update(&mut self, message: Message) -> Jobs {
       // Every handler in this app's tree was built from `A::Msg`.
       match message.downcast::<A::Msg>() {
           Ok(message) => Component::update(self, *message).into_jobs(),
           Err(_) => Jobs::none(),
       }
   }
   ```

The downcast cannot fail in a correct program: the only way to put a handler into an
`Element<A::Msg>` is through a builder method that takes `A::Msg`. If it did fail, the message is
dropped silently. That is a deliberate choice of "ignore" over "panic", but it means a bug in the
erasure code would show up as a button that does nothing, not as a crash.

## Why handlers compare equal by presence only

`RawElement`, `Widget` and `UiNode` all derive `PartialEq`, and the diff relies on it: a node is
updated only when `previous.kind != node.kind || previous.style != node.style`. Properties structs
contain handlers, so `Handler` needs an `==`. Closures cannot be compared. The code picks the
weakest useful definition:

```rust,ignore
// rust/src/message.rs
impl<A> PartialEq for Handler<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_some() == other.0.is_some()
    }
}
```

Two handlers are equal when both are set or both are unset. This is not a shortcut; it is exactly
the information a native view needs. A native button does not hold the closure. It holds its node
id and reports "clicked". Whether it should report at all is the only thing it could need to know,
and that is the presence bit. Which message the click stands for is looked up in the latest tree at
the moment of the click.

The consequence that makes "always re-run `view`" affordable: every render creates new closures,
and none of them look like a change. A screen where nothing visible changed produces a tree equal
to the previous one, zero operations, and no layout pass.

There is one obligation that follows. Because the diff will not notice a changed closure, the
runtime must always keep the *new* tree, even when it equals the old one, so the next event finds
the new closures. `Runtime::update` does that, and a test pins it down
(`an_unchanged_render_still_takes_over_the_new_handlers` in `rust/src/engine/runtime.rs`).

`Debug` for a handler prints `Handler(set)` or `Handler(none)`, for the same reason.

## `Element::map` and `MessageMap`

A child component has its own message type. To place its view inside a parent's, the child's
messages must become the parent's:

```rust,ignore
self.counter.view().map(Msg::Counter)   // Element<CounterMsg> -> Element<Msg>
```

By this point the child's handlers are already erased; they return `Box<dyn Any>` holding a
`CounterMsg`. `map` cannot change their types, so it wraps each of them in a function that unboxes,
converts and reboxes:

```rust,ignore
// rust/src/element.rs
pub fn map<N: 'static>(self, wrap: impl Fn(M) -> N + Send + Sync + 'static) -> Element<N>
where
    M: 'static,
{
    let mut raw = self.raw;
    let wrap: MessageMap = Arc::new(move |message: Message| -> Message {
        match message.downcast::<M>() {
            Ok(message) => Box::new(wrap(*message)),
            // Not produced by this subtree; pass it through untouched.
            Err(other) => other,
        }
    });
    raw.map_messages(&wrap);
    Element::from_raw(raw)
}
```

`MessageMap` is `Arc<dyn Fn(Message) -> Message + Send + Sync>` (in `message.rs`). `map_messages` walks
the subtree and calls `Handler::map` on every handler it finds:

```rust,ignore
// rust/src/message.rs
pub(crate) fn map(&mut self, wrap: &MessageMap) {
    if let Some(make) = self.0.take() {
        let wrap = wrap.clone();
        self.0 = Some(Arc::new(move |payload| wrap(make(payload))));
    }
}
```

The walk itself is in `RawElement::map_messages`:

```rust,ignore
// rust/src/element.rs
pub(crate) fn map_messages(&mut self, wrap: &MessageMap) {
    match &mut self.kind {
        ElementKind::Widget(widget) => widget.map_messages(wrap),
        ElementKind::Pager(props) => props.on_select.map(wrap),
        ElementKind::List(props) => props.map_messages(wrap),
        ElementKind::Column | ElementKind::Row | ElementKind::Spacer | ElementKind::Scroll(_) => {}
    }
    for child in &mut self.children {
        child.map_messages(wrap);
    }
}
```

Three details are worth noticing.

**The match is exhaustive.** There is no `_ =>` arm. A new `ElementKind` that carries a handler
cannot be added without the compiler stopping here. For widgets the same guarantee comes from the
`WidgetProps` trait: every widget must implement `map_messages` (chapter 3).

**Lists are mapped lazily.** A list does not hold its rows; it holds a closure that builds a row on
demand. `ListProps::map_messages` in `rust/src/widgets/list.rs` replaces that closure with one that
builds the row and then maps it. So rows built long after `map` returned still produce the parent's
messages. The test `mapping_also_covers_rows_a_list_builds_later` in `element.rs` checks this.

**Nesting composes.** Mapping twice wraps twice; a click three components deep runs three small
closures, each doing one downcast. The `Err(other) => other` branch is defensive: it lets a message
that is not of type `M` pass through unchanged instead of being lost.

A cost to be aware of: `map` is eager for everything except list rows. It visits every element of
the child's subtree and allocates one new `Arc` per set handler, on every render. For ordinary
screens that is negligible next to building the tree.

## Rough edges

- `RawElement::container` and `RawElement::leaf` have identical bodies. The two names document
  intent at the call site but nothing enforces the difference.
- `ElementKind` derives `PartialEq`, and for `List` that goes through a hand-written
  `ListProps::eq` which compares the key closure with `Arc::ptr_eq`. Two lists built by two calls
  to `view` are therefore never equal as *elements*. The engine does not compare elements (it
  compares the node trees built from them), so this only affects tests that compare `RawElement`s.
- A failed downcast in `AnyApp::update` drops the message without a log line.

## Where this lives

- `rust/src/element.rs`: `Element`, `RawElement`, `ElementKind`, `style_builder!`,
  `text_builder!`, and two tests that exercise `map` end to end.
- `rust/src/message.rs`: `Message`, `Handler`, `MessageMap`. The whole file is under a hundred lines and
  worth reading in full.
- `rust/src/app.rs`, `impl<A: App> AnyApp for A`: where `into_raw` and `downcast` are called.
- `rust/src/host/events.rs`, `Actions`: where handlers are read back out of a node.

Next: [3. Widgets](03-widgets.md).
