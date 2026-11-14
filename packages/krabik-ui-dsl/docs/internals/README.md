# How the engine works

This folder is a guided tour of the platform-independent Rust code of `krabik-ui-dsl`: everything
under `rust/src/` except `android/` and `ios/`. After reading it you should be able to open any of
those files and know what it is for, what calls it, and why it is shaped the way it is.

The Android and iOS renderers, the Kotlin and Swift shells, the build and the macros are covered
separately in [`../platforms/`](../platforms/). The chapters here say where that story takes over.

## Chapters

| Chapter | What it explains |
|---|---|
| [1. The big picture](01-the-big-picture.md) | The whole loop on one page: `view()` to native views and back, a table of modules and a glossary. |
| [2. Elements and messages](02-elements-and-messages.md) | What `view()` returns, how a typed message becomes a boxed one, and why handlers compare equal by presence. |
| [3. Widgets](03-widgets.md) | The shape of a widget file, what `#[derive(Widget)]` writes, the `Widget` enum and the widgets that are written by hand. |
| [4. The tree and identity](04-the-tree-and-identity.md) | How an element becomes a `UiNode` tree with stable ids, which nodes get a native view, and how lists expand. |
| [5. Layout and measurement](05-layout-and-measurement.md) | How the tree is handed to Taffy, how text and other intrinsic sizes feed in, and what the frames mean. |
| [6. Diffing and operations](06-diffing-and-operations.md) | The `Op` enum, the two diffs, the order of a batch, and the two small helper modules for animation and frame packing. |
| [7. The runtime](07-the-runtime.md) | The state one host keeps between renders, `update` step by step, scrolling, focus and recycled rows. |
| [8. The session and events](08-the-session-and-events.md) | The `Platform` traits, `Session`, how navigation and chrome are mirrored, how an event finds its message, and the fake-platform tests. |
| [9. App, navigation, scaffold, presentation](09-app-navigation-scaffold-presentation.md) | The public surface an application sees and how each typed piece is erased before the session touches it. |
| [10. One tap, end to end](10-one-tap-end-to-end.md) | Two traces through every core function: a button tap, and one character typed inside a recycled list row. |
| [11. Commands and the mailbox](11-commands-and-the-mailbox.md) | Work started by `update`, timers, and how a result gets from a worker thread back to the UI thread on each platform. |

## Suggested reading order

Read chapter 1 first; it defines the words the other chapters use.

Then there are two reasonable routes:

- **Bottom up** (the order of the numbers): 2, 3, 4, 5, 6, 7, 8, 9, then 10 as a recap. Each
  chapter only depends on earlier ones.
- **Trace first**: read 1, then 10, and use the links in 10 to jump into the chapter for whichever
  step you want to understand in depth. This is faster if you already know Elm- or React-style
  frameworks.

If you only have time for three chapters, read 1, 7 and 8. The runtime and the session are where
the design decisions live; the rest is the machinery they drive.

## Conventions

- File paths are relative to the package root (`packages/krabik-ui-dsl`).
- Code blocks marked as excerpts are copied from the source and trimmed with `// ...`. They show
  the code as it was when this was written; the item names are the stable reference, so search for
  the function name if a file has moved on.
- "Rough edge" marks a place where the code is correct but awkward, or has a limit you should know
  about before changing it.
