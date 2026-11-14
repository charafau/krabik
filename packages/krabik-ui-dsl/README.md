# krabik-ui-dsl

A mobile UI framework in Rust. You describe your interface as a tree of
widgets; the framework lays it out with [Taffy](https://github.com/DioxusLabs/taffy)
and draws it with real native views: `TextView`, `Button`, `RecyclerView` on
Android; `UILabel`, `UIButton`, `UICollectionView` on iOS. Navigation, the
app bar, the tab bar, alerts, sheets, the floating action button and the
drawer are the platform's own, and the app follows the device's light or
dark appearance.

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

## Documentation

The [documentation](docs/README.md) is in four parts:

1. [Tutorial](docs/tutorial/README.md): build a Notes app step by step.
2. How it works: [the engine](docs/internals/README.md) and
   [the platforms](docs/platforms/README.md), file by file.
3. Design: [why it is built this way](docs/design/decisions.md) and
   [what was done, in order](docs/design/history.md).
4. [Recipes](docs/recipes/README.md): add a widget, add native chrome, write tests, debug a render.

## Apps in this directory

| Directory | What it is |
|---|---|
| [`harness/`](harness) | The gallery the framework is developed and UI-tested against |
| [`example/`](example) | A showcase with one screen per widget, navigation, dialog and sheet |
| [`example-todo/`](example-todo) | A todo app with filtering, sorting and a recycled list |
| [`example-todo-minimal/`](example-todo-minimal) | The smallest complete app; copy it to start your own |

## Quick start

```sh
# the Rust targets are installed automatically from rust-toolchain.toml

# unit tests
cargo test --manifest-path rust/Cargo.toml --workspace

# Android (emulator running)
cd example/android && ./gradlew :app:installDebug -PkrabikAbis=arm64-v8a

# iOS: open example/ios/KrabikUiDslExample.xcodeproj and run
```

Status and roadmap: [TODO.md](TODO.md).
