# 3. Components: growing an app without a giant `update`

One `Msg` enum and one `update` are fine for a screen or two. Beyond that,
split the app into components. A component is the same three things again,
state, messages and a view, for one part of the app.

## A child component

```rust
pub struct Checklist {
    tasks: Vec<Task>,
    draft: String,
}

#[derive(Clone)]
pub enum Msg {
    Draft(String),
    Add,
    SetDone(usize, bool),
}

impl Component for Checklist {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) { /* ... */ }

    fn view(&self) -> Element<Msg> { /* ... */ }
}
```

It knows nothing about the app around it, and it can be tested on its own.

## Using it from a parent

The parent does three things.

**Keep the child in its state:**

```rust
pub struct MyApp {
    checklist: Checklist,
    // ...
}
```

**Give the child's messages a place in its own message type:**

```rust
#[derive(Clone)]
pub enum Msg {
    Checklist(checklist::Msg),
    OpenSettings,
    // ...
}
```

**Show the child's view and forward its messages:**

```rust
fn view(&self) -> Element<Msg> {
    Column::new()
        .child(self.checklist.view().map(Msg::Checklist))
        .child(Button::new("Settings").on_click(Msg::OpenSettings))
        .into()
}

fn update(&mut self, msg: Msg) {
    match msg {
        Msg::Checklist(msg) => self.checklist.update(msg),
        Msg::OpenSettings => { /* ... */ }
    }
}
```

`Element::map` is the piece that makes this work: it turns an
`Element<checklist::Msg>` into an `Element<Msg>` by wrapping every message
the child's widgets can send, including widgets in list rows that are only
built later while scrolling.

The compiler guards the boundary: a child's view cannot be placed in a parent
without `map`, because the message types differ.

## When the parent needs to react

Sometimes the parent must know that something happened in a child, for
example "the user tapped Done, go back". Let the parent look at the message
before forwarding it:

```rust
Msg::Checklist(msg) => {
    if matches!(msg, checklist::Msg::Done) {
        self.stack.pop();
    }
    self.checklist.update(msg);
}
```

The child stays ignorant of navigation; the parent decides what Done means.

## How this compares to MVVM

If you come from MVVM: the component's state is the view model's state,
`update` is where its commands live, and `view` is the binding. The
difference is that there are no observable properties and no two-way
bindings to keep in sync. The view is recomputed from the state after every
message, and the framework works out what actually changed.

The showcase app is built this way: `example/rust/src/lib.rs` is the parent,
`example/rust/src/checklist.rs` a child component.

Next: [navigation, dialogs and sheets](04-navigation.md).
