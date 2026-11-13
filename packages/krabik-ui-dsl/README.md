# krabik-ui-dsl

A mobile UI framework in Rust. You describe your interface as a tree of
widgets; the framework lays it out with [Taffy](https://github.com/DioxusLabs/taffy)
and draws it with real native views: `TextView`, `Button`, `RecyclerView` on
Android; `UILabel`, `UIButton`, `UICollectionView` on iOS. Navigation, the
toolbar, the tab bar, alerts and sheets are the platform's own.

```rust
#[derive(Clone)]
enum Msg { Increment, Name(String) }

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Name(name) => self.name = name,
        }
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .child(Text::new(format!("{}: {}", self.name, self.count)).size(22))
            .child(TextField::new(&self.name).on_change(Msg::Name))
            .child(Button::new("+1").on_click(Msg::Increment))
            .into()
    }
}
```

State lives in your app, events are values of your own message type checked
by the compiler, and after every message the view is rebuilt and only what
changed is applied to the native views.

## Guide

1. [Your first app](docs/01-your-first-app.md): state, messages, view
2. [Widgets, layout and styling](docs/02-widgets-and-layout.md)
3. [Components](docs/03-components.md): growing an app without a giant `update`
4. [Navigation, toolbar, dialogs and sheets](docs/04-navigation.md)
5. [How it works inside](docs/05-how-it-works.md): tree, layout, diff, events
6. [The Android and iOS halves](docs/06-platforms.md)
7. [Adding a widget](docs/07-adding-a-widget.md)
8. [Project layout, building and testing](docs/08-project-layout.md)

## Apps in this directory

| Directory | What it is |
|---|---|
| [`harness/`](harness) | The gallery the framework is developed and UI-tested against |
| [`example/`](example) | A showcase with one screen per widget, navigation, dialog and sheet |
| [`example-todo/`](example-todo) | A todo app with filtering, sorting and a recycled list |
| [`example-todo-minimal/`](example-todo-minimal) | The smallest complete app; copy it to start your own |

## Quick start

```sh
rustup target add aarch64-linux-android aarch64-apple-ios-sim

# unit tests
cargo test --manifest-path rust/Cargo.toml

# Android (emulator running)
cd example/android && ./gradlew :app:installDebug -PkrabikAbis=arm64-v8a

# iOS: open example/ios/KrabikUiDslExample.xcodeproj and run
```

Status and roadmap: [TODO.md](TODO.md).
