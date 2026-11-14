# Recipes

Task-oriented walkthroughs for changing the framework itself. Each one is a checklist you can
follow from top to bottom: the files to touch in order, code modelled on an existing case, what
the compiler reports at each step, how to verify the result, and a section on what the compiler
will not catch.

They assume you know the vocabulary. If a term is unfamiliar, the tutorial in
[`../tutorial/`](../tutorial/) introduces the public API, the chapters in
[`../internals/`](../internals/README.md) explain the engine (elements, the tree, layout, the
diff, the session), and those in [`../platforms/`](../platforms/README.md) explain how Rust
reaches Android and iOS.

| Recipe | What it covers |
|---|---|
| [Add a widget](add-a-widget.md) | Adds a new leaf widget end to end, carrying an invented `Stepper` through the properties, the event path, Android (Rust and Kotlin), iOS, tests and the showcase. |
| [Add a chrome slot](add-a-chrome-slot.md) | Adds a piece of native chrome around a screen, following the floating action button for plain configuration and the snackbar for chrome the platform can dismiss on its own. |
| [Write tests](write-tests.md) | Shows how to test at each level, from an app's `update` to the engine, the session with its fake platform, the macros, Espresso and XCUITest, and how to run a single test. |
| [Debug a render](debug-a-render.md) | Gives a stage-by-stage procedure for finding out why something on screen is wrong, and the causes that have occurred in this codebase. |

## Three commands worth memorising

The Android and iOS renderers are compiled only for their targets, so a change to shared code
needs all three before it is known to build:

```sh
cargo check --manifest-path rust/Cargo.toml
cargo check --manifest-path rust/Cargo.toml --target aarch64-linux-android
cargo check --manifest-path rust/Cargo.toml --target aarch64-apple-ios-sim
```

## Where to start

- Changing what a screen can contain: [Add a widget](add-a-widget.md).
- Changing what surrounds a screen: [Add a chrome slot](add-a-chrome-slot.md).
- Something looks wrong and you do not know where: [Debug a render](debug-a-render.md), then
  [Write tests](write-tests.md) to pin it down at the stage you found.
