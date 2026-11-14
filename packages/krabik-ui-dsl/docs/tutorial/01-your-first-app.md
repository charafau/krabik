# 1. Your first app

This chapter explains what the framework is and builds the smallest possible app with it: a
counter. You will meet the `Component` trait (state, messages, `update`, `view`), the `App` trait
and `set_app_factory`, and you will test the app on your own machine with no phone or emulator.
At the end you will understand the loop every krabik app runs in, which the rest of the tutorial
builds on.

## What the framework is

`krabik-ui-dsl` is a mobile UI framework written in Rust. Your app is a Rust library. It holds all
of the state and describes what the screen should look like for that state. The framework lays
that description out with [Taffy](https://github.com/DioxusLabs/taffy), a flexbox engine, and
draws it with real native widgets: `TextView`, `Button`, `RecyclerView` and friends on Android,
`UILabel`, `UIButton`, `UICollectionView` and friends on iOS. There is no web view and no custom
drawing. In that respect it works like React Native, with Rust in the place of JavaScript.

The division of work is strict. Rust decides everything: what is on screen, which screen is on
top, whether a dialog is open. The Kotlin and Swift code only carries those decisions out and
reports back what the user did ("this button was pressed", "this field now contains `abc`"). The
module documentation of the crate says it in one paragraph:

```rust,ignore
//! An application implements [`Component`] (state, messages, `update`, `view`)
//! and [`App`], builds its view from the widgets in [`ui`], and registers
//! itself with [`set_app_factory`]. The [`engine`] turns each view into a tree,
//! lays it out with Taffy and diffs it against the previous one; the platform
//! renderer applies the resulting operations to native views and reports
//! events back as the application's messages.
```

(from `rust/src/lib.rs`)

## An empty folder

The tutorial builds one app, called Notes. An app has three parts, each in its own directory:

```text
notes/
  rust/      the app itself: a Rust library crate
  android/   a small Gradle project that hosts it
  ios/       a small Xcode project that hosts it
```

For the first nine chapters you only need `rust/`. Everything in it runs and can be tested on your
development machine. The two host projects come in [chapter 10](10-running-on-android-and-ios.md).

Create the folder next to the existing examples, so the relative paths below are right:

```sh
cd packages/krabik-ui-dsl
mkdir -p notes/rust/src
```

`notes/rust/Cargo.toml`:

```toml
[package]
name = "notes"
version = "0.1.0"
edition = "2024"

[lib]
# The staticlib is linked into the iOS app; the cdylib is loaded by the
# Android app. Neither matters until chapter 10; `cargo test` works with both.
crate-type = ["staticlib", "cdylib"]

[dependencies]
krabik-ui-dsl = { path = "../../rust" }

# Keeps cargo from adopting a workspace further up the directory tree.
[workspace]
```

This mirrors `example-todo-minimal/rust/Cargo.toml`. The framework is an ordinary path dependency:
your crate compiles it in, so the native library that ends up on the phone contains both your app
and the framework.

## A component: state, messages, update, view

Everything in a krabik app is a *component*. A component has four parts:

1. **State**: a plain Rust struct. Here it is one number.
2. **A message type**: an enum that lists everything that can happen to the component. Here:
   the user pressed "+1", or the user pressed "Reset".
3. **`update`**: takes one message and changes the state.
4. **`view`**: looks at the state and returns a description of the UI.

The trait is short enough to quote whole:

```rust,ignore
/// A piece of UI with its own state and messages.
pub trait Component {
    /// Everything that can happen to this component.
    type Msg: 'static;

    /// Applies a message to the state. The UI is rebuilt afterwards.
    fn update(&mut self, msg: Self::Msg) -> Command<Self::Msg>;

    /// Describes the UI for the current state.
    fn view(&self) -> Element<Self::Msg>;
}
```

(from `rust/src/app.rs`)

Note the signatures. `view` takes `&self`: it cannot change anything, it can only describe.
`update` takes `&mut self`: it can only change state, it cannot touch the screen. The framework
connects the two.

`update` returns a `Command`, which describes work to do off the UI thread, such as a network
request. Nothing in the first ten chapters needs one, so every `update` you will see ends with
`Command::none()`. Chapter 11 explains what else it can return.

Here is the counter. Put it in `notes/rust/src/lib.rs`:

```rust
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Increment,
    Reset,
}

/// The state of the app.
pub struct Counter {
    count: i64,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    /// Registers the app with the framework. The host calls this at startup.
    pub fn install() {
        set_app_factory(Self::new);
    }
}

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Reset => self.count = 0,
        }
        Command::none()
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .child(Text::new(format!("Count: {}", self.count)).size(24).bold())
            .child(Button::new("+1").on_click(Msg::Increment))
            .child(Button::new("Reset").on_click(Msg::Reset))
            .into()
    }
}

impl App for Counter {}
```

Read `view` slowly, because every screen in this tutorial is written the same way.

- `Column::new()` creates a vertical container. `.padding(24)` and `.spacing(12)` are layout
  settings in logical points (dp on Android, points on iOS). Chapter 2 covers layout.
- `.child(...)` appends a child. `Text::new(...)` is a label; `.size(24).bold()` style it.
- `Button::new("+1").on_click(Msg::Increment)` is the important line. The button does not get a
  callback. It gets a *value*: the message to send when it is pressed.
- `.into()` converts the finished `Column` into an `Element<Msg>`, the type `view` returns.

Every widget is a *builder*: a value you create with `new`, configure with chained methods, and
hand to a parent with `child`. Nothing native exists yet when `view` returns. An `Element` is a
plain description, cheap to build and to throw away.

### Why messages are values

`Element<Msg>` carries the message type as a type parameter. A `Button<Msg>` can only be given a
`Msg`, and a `Column<Msg>` only accepts children that send `Msg`. So the compiler checks every
event in the app: you cannot attach a message from another screen by mistake, and when you add a
variant to `Msg` the `match` in `update` stops compiling until you handle it.

An earlier version of this framework used string actions (`"counter:increment"`) that `update`
matched at run time. A typo there compiled and did nothing. Typed messages replaced that; the
restructure is commit `4408bdf` in `git log`.

Two requirements on `Msg` follow from how handlers are stored. `on_click` takes the message by
value and gives a copy to `update` on every click, so `Msg` must be `Clone`. The handler is kept
behind an `Arc<dyn Fn(..) + Send + Sync>` (see `Handler` in `rust/src/message.rs`), so `Msg` must
be `Send + Sync`, which any enum of plain data is. This is the generated signature
(`rust/macros/src/widget.rs` writes it for every `Handler<()>` field of a widget):

```rust,ignore
pub fn on_click(mut self, message: M) -> Self
where
    M: Clone + Send + Sync + 'static,
```

Deriving `Clone, Debug, PartialEq` on every message enum is a good habit. `Debug` and `PartialEq`
are not required by the framework, but tests want them.

## `App` and `set_app_factory`

`Component` describes a piece of UI. `App` marks the one component at the root and adds the hooks
for native chrome around it:

```rust,ignore
pub trait App: Component + Send + 'static {
    fn navigation(&self) -> Option<Navigation<Self::Msg>> { None }
    fn screen(&self, _key: &str) -> Option<Scaffold<Self::Msg>> { None }
    fn dialog(&self) -> Option<Dialog<Self::Msg>> { None }
    fn sheet(&self) -> Option<BottomSheet<Self::Msg>> { None }
    fn back(&self) -> Option<Self::Msg> { None }
}
```

(from `rust/src/app.rs`, doc comments removed)

Every method has a default, so `impl App for Counter {}` is a complete single-screen app. The
other hooks are the subject of chapters 5 and 6. The `Send` bound means your state struct must be
safe to move to another thread; a struct of plain data is.

The framework cannot know your type, so you tell it how to create your app:

```rust,ignore
pub fn set_app_factory<A: App>(factory: impl Fn() -> A + Send + Sync + 'static) {
    let _ = APP_FACTORY.set(Box::new(move || Box::new(factory())));
}
```

(from `rust/src/app.rs`)

It takes a *factory* (a function that makes an app), not an app, because the native side may need
more than one instance: "each activity or scene then gets its own instance", as the doc comment
says. The factory is stored in a `OnceLock`, so only the first call counts; later calls are
ignored. If a native view is created before any factory is registered, `create_app` panics with
"no application registered; call set_app_factory before creating a root view". The host projects
in chapter 10 call your `install` function first thing at startup to avoid that.

## The update loop

Here is what happens from a tap to a changed label.

```text
  +---------------------------------------------------------------+
  |  your app                                                     |
  |                                                               |
  |     state ------ view(&self) ------> Element<Msg>             |
  |       ^                                    |                  |
  |       |  update(&mut self, msg)            |                  |
  +-------|------------------------------------|------------------+
          |                                    v
  +-------|------------------------------------|------------------+
  |  framework                                 |                  |
  |       |                          build tree, lay out (Taffy), |
  |   look up the handler            diff against the last tree   |
  |   of that widget -> Msg                    |                  |
  +-------^------------------------------------|------------------+
          |                                    v
  +-------|------------------------------------|------------------+
  |  native views (Android / iOS)              |                  |
  |   "widget 42 was clicked"        create / update / move /     |
  |                                  remove native views          |
  +---------------------------------------------------------------+
```

In words:

1. At startup the framework creates your app with the factory and calls `view`.
2. It turns the `Element` into its own tree, gives every node a stable identifier, computes a
   frame for every node with Taffy, and creates the native views.
3. The user taps "+1". The native button reports "the widget with this identifier was clicked".
4. The framework finds that widget in the tree it built from your last `view`, takes the handler
   you attached with `on_click`, and gets `Msg::Increment` from it.
5. It calls `update(Msg::Increment)`. Your state changes.
6. It calls `view` again, builds a new tree, compares it with the previous one, and applies only
   the differences. Here that is one operation: change the text of one label.

Steps 4 to 6 are three lines in the shared session code:

```rust,ignore
/// Applies a message to the application and shows the result.
fn update(&mut self, cx: &mut P::Cx<'_>, message: Message) -> Result<(), P::Error> {
    self.apply(message);
    self.render_all(cx)
}
```

(from `rust/src/host/session.rs`)

Two design decisions are visible here.

**`view` always runs again after a message.** There is no "did anything change" flag for you to
return, and no way to forget to refresh. This keeps apps simple. The cost is paid inside the
engine: when the new tree is equal to the previous one, layout is skipped entirely, and when it
differs only the differing nodes produce native calls. So write `view` as if it were free, and do
not cache elements.

**Handlers are read from the latest tree, never pushed to native views.** The native button only
knows its identifier. That is why two handlers compare equal whenever both are set
(`impl PartialEq for Handler` in `rust/src/message.rs`): a closure you re-create on every `view`
does not look like a change to the diff, and nothing has to be sent to the platform for it.

How the tree, the identifiers, the layout and the diff work is the subject of
[`../internals/`](../internals/). For writing apps, the loop above is all you need.

## Testing without a device

`update` is an ordinary method on an ordinary struct. Nothing about it needs Android or iOS, so
the app's logic is tested with plain `cargo test`. Add this at the bottom of `lib.rs`:

```rust,ignore
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counting_and_resetting() {
        let mut app = Counter::new();
        app.update(Msg::Increment);
        app.update(Msg::Increment);
        assert_eq!(app.count, 2);

        app.update(Msg::Reset);
        assert_eq!(app.count, 0);
    }

    #[test]
    fn the_view_has_a_label_and_two_buttons() {
        let app = Counter::new();
        assert_eq!(app.view().children().len(), 3);
    }
}
```

Run it from `notes/rust`:

```sh
cargo test
```

The first test drives the app exactly the way the framework does: it sends messages and looks at
the state. The test module is a child of the file's module, so it can read the private `count`
field directly. The second test calls `view` and inspects the description. `children()` is a
method of `RawElement`, the untyped description an `Element` dereferences to
(`rust/src/element.rs`). Chapter 9 shows how to look deeper into a view.

This is the main practical benefit of the architecture: almost all of an app's behaviour is in
`update`, and `update` is testable in milliseconds. The real example apps do the same; see the
`tests` module at the bottom of `example-todo-minimal/rust/src/lib.rs`.

The framework itself also compiles on your machine. Its Android and iOS renderers are behind
`#[cfg(target_os = "android")]` and `#[cfg(target_os = "ios")]` in `rust/src/lib.rs`, so on a
desktop only the platform-independent parts are built, and that is all a test needs.

## The complete code so far

`notes/rust/src/lib.rs`:

```rust
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    Increment,
    Reset,
}

/// The state of the app.
pub struct Counter {
    count: i64,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    /// Registers the app with the framework. The host calls this at startup.
    pub fn install() {
        set_app_factory(Self::new);
    }
}

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Reset => self.count = 0,
        }
        Command::none()
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .child(Text::new(format!("Count: {}", self.count)).size(24).bold())
            .child(Button::new("+1").on_click(Msg::Increment))
            .child(Button::new("Reset").on_click(Msg::Reset))
            .into()
    }
}

impl App for Counter {}
```

plus the `tests` module shown above.

## Where this lives

- `rust/src/app.rs`: `Component`, `App`, `set_app_factory`, and `AnyApp`, the type-erased form
  the runtime holds.
- `rust/src/message.rs`: `Handler`, how a typed message is stored without the framework knowing
  its type.
- `rust/src/element.rs`: `Element<M>` and `RawElement`.
- `rust/src/host/session.rs`: `Session::update` and `render_all`, the loop described above.
- `example-todo-minimal/rust/src/lib.rs`: a complete small app in one file.

Next: [2. Widgets and layout](02-widgets-and-layout.md).
