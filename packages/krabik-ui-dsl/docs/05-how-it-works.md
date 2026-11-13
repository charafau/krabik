# 5. How it works inside

This chapter follows one tap from the native button to the screen updating.
The file names are under `rust/src/`.

```
            your app                    the engine                      a platform
  ┌───────────────────────┐  ┌─────────────────────────────┐  ┌────────────────────────┐
  │ view() -> Element<Msg>│─▶│ tree ─▶ layout ─▶ diff ─▶ ops│─▶│ create / update / move │
  │ update(msg)           │◀─│ handlers ◀─ node id + event │◀─│ native views           │
  └───────────────────────┘  └─────────────────────────────┘  └────────────────────────┘
```

## 1. The description: `Element`

`view()` returns an `Element<Msg>`: a plain tree of values. Each node says
what it is (`ElementKind`: a container, a list, or a `Widget` with its
properties), how it is styled (`Style`) and what its children are
(`element.rs`, `widgets/`, `style.rs`).

Nothing native exists yet. Building this tree is cheap, so the app rebuilds
all of it after every message.

### Messages inside the tree

`Button::new("+1").on_click(Msg::Increment)` stores a *handler* in the
button's properties: a small function that produces the message when called
(`message.rs`). The handler is type-erased as soon as it is stored. That is
why the engine and both platforms contain no generics: they pass around
"some message" and only your app's `update` sees the real type again.
`Element<Msg>` keeps the type on the outside so the compiler can check your
view.

## 2. The tree: stable identities

The engine turns the description into a tree of `UiNode`s (`engine/tree.rs`).
The important addition is a **node id**: a hash of the parent's id, the
node's position or key, and its kind. The same widget in the same place gets
the same id on every rebuild; this is how the engine knows that "the button"
before and after are one button.

A `List` is expanded here: only the rows near the viewport become nodes; the
rest is empty space of the right height.

## 3. Layout: Taffy

`engine/layout.rs` mirrors the node tree into a
[Taffy](https://github.com/DioxusLabs/taffy) flexbox tree and computes a
frame for every node. Leaf widgets report the size of their content
(`Widget::intrinsic_size`, defined next to each widget in `widgets/`). Text
cannot be measured in Rust, so the platform provides a `Measure`
implementation that asks an off-screen native label
(`engine/measure.rs`, `android/measure.rs`, `ios/measure.rs`).

## 4. The diff: operations

`engine/render.rs` compares the new tree with the previous one, by id, and
produces a short list of operations:

| Operation | Meaning |
|---|---|
| `Create(node)` | A node appeared: make its native view. |
| `Update(node)` | Its properties or style changed. |
| `Reparent { id, parent }` | It now sits under another native view. |
| `Remove { id, exit }` | It is gone, optionally with an exit animation. |
| `Layout { id, frame, .. }` | Its frame changed. |

If nothing changed, the list is empty and the platform does nothing. This is
what makes "rebuild everything after every message" efficient.

`engine/runtime.rs` ties steps 2 to 4 together for one native root view and
remembers what it needs between renders: the previous tree, scroll offsets,
which field has focus.

## 5. The platform applies the operations

Each platform is a Rust module that calls native APIs directly: JNI on
Android (`android/`), UIKit through `objc2` on iOS (`ios/`). For every
operation it creates, updates, moves or removes a real native view
(`android/host.rs`, `ios/host.rs`). Chapter 6 covers this half.

## 6. An event comes back

The user taps the button.

1. The native click listener calls into Rust with the session, the host and
   the **node id** of the button (`android/entry.rs`, `ios/events.rs`).
2. The session looks the node up in the current tree and takes its handler
   (`host/events.rs`). Handlers are always read from the latest tree, so a
   native view never holds a stale closure.
3. The handler produces the message; the session calls your `update`.
4. The session calls `view()` again and the cycle repeats from step 1.

## The session

One running app is a `Session` (`host/session.rs`). It owns your app and one
*host* per native root view: the main view, each navigation screen and the
bottom sheet each have their own tree and their own runtime. After every
message the session re-renders all hosts and then reconciles the native
chrome with what your app returns from `navigation()`, `dialog()`, `sheet()`
and `back()`: pushing and popping screens, updating the toolbar, showing or
dismissing the alert and the sheet.

The session is the same code on both platforms. A platform only supplies two
things (`host/platform.rs`):

- a `Host`: render this description into that native view;
- a `Controller`: push, pop, set the toolbar, show the alert, present the
  sheet.

Because of that split, the session logic is unit-tested on your development
machine with a fake platform, with no device involved.

## Threading

Everything runs on the platform's main thread: native events, `update`,
`view`, layout and applying operations. There is no background Rust thread
and nothing to synchronise in your app.

A global lock guards the session table. While a render is applying its
operations, a native widget may echo a programmatic change back as an event
("Rust set the switch" reported as "the user toggled it"). Android drops an
event that arrives while the render holds the lock; iOS delivers widget
events on the next main-queue turn, after the render has finished, and its
widgets are updated without firing their change actions.

Next: [the Android and iOS shells](06-platforms.md).
