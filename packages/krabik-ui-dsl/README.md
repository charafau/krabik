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

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Name(name) => self.name = name,
        }
        Command::none()
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

The [documentation](docs/README.md) is in five parts:

1. [Tutorial](docs/tutorial/README.md): build a Notes app step by step.
2. How it works: [the engine](docs/internals/README.md) and
   [the platforms](docs/platforms/README.md), file by file.
3. Design: [why it is built this way](docs/design/decisions.md),
   [hot reload](docs/design/hot-reload.md) and
   [what was done, in order](docs/design/history.md).
4. [Recipes](docs/recipes/README.md): add a widget, add native chrome, write tests, debug a render.
5. [Comparisons](docs/comparisons/README.md): the internals next to
   [Flutter](docs/comparisons/01-compared-to-flutter.md) and
   [React Native](docs/comparisons/02-compared-to-react-native.md).

## Apps in this directory

| Directory | What it is |
|---|---|
| [`harness/`](harness) | The gallery the framework is developed and UI-tested against |
| [`example/`](example) | A showcase with one screen per widget, navigation, dialog and sheet |
| [`example-todo/`](example-todo) | A todo app with filtering, sorting and a recycled list |
| [`example-todo-minimal/`](example-todo-minimal) | The smallest complete app; `krabik create <name> --with-dsl` starts your own from its shells |

## Quick start

Your own app, with the [krabik CLI](../krabik-cli) installed from this checkout:

```sh
krabik create my-app --with-dsl   # a counter app with Android and iOS shells
cd my-app
krabik run                        # build, install and start it on the emulator
krabik watch                      # hot reload: every saved change, state kept
```

Add `--platform ios` to `run` and `watch` for the simulator. The
[tutorial](docs/tutorial/README.md) starts from exactly this project.

The framework and the apps in this directory:

```sh
# the Rust targets are installed automatically from rust-toolchain.toml

# unit tests
cargo test --manifest-path rust/Cargo.toml --workspace --all-features

# Android (emulator running)
cd example/android && ./gradlew :app:installDebug -PkrabikAbis=arm64-v8a

# iOS: open example/ios/KrabikUiDslExample.xcodeproj and run
```

Status and roadmap: [TODO.md](TODO.md).
