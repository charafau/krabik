# 1. The big picture

This chapter shows the whole engine on one page: how the value returned by `view()` becomes native
views, and how a tap on one of those views comes back as a message. It also lists every module with
one line each and defines the words the later chapters use. After reading it you will know which
file to open for any step of the loop.

## The idea in four sentences

An application holds state and describes its UI as a tree of plain Rust values. The engine turns
that description into a tree of nodes with stable identifiers, asks Taffy (a flexbox library) where
each node goes, and compares the result with what is already on screen. The difference is a short
list of operations ("create this label", "move that button"), which a platform renderer applies to
real Android or iOS views. When the user touches a view, the platform reports "node 123 was
clicked"; the engine finds the message the application attached to that node, hands it to the
application's `update`, and runs the whole thing again.

Nothing in that loop is clever about *what* changed in the application. After every message the
engine calls `view()` again and rebuilds everything. That keeps applications simple: there is no
"mark dirty" call and no subscription to forget. The cost is paid back inside the engine, which
notices an identical tree and skips layout entirely (chapter 7).

## The loop

```text
  APPLICATION (your crate)                 rust/src/app.rs
  +---------------------------------------------------------+
  |  state --view()--> Element<Msg>      update(Msg) <--+   |
  +----------------------|------------------------------|---+
                         | into_raw()                   | downcast
                         v                              |
  SESSION                |              rust/src/host/session.rs
  +----------------------|------------------------------|---+
  |  render_all: for each host          node_event:     |   |
  |    host.render(&RawElement)           Actions::of   |   |
  |  sync: navigation, chrome,            -> Handler    |   |
  |        dialog, sheet, back            -> Box<dyn Any>   |
  +----------------------|------------------------------^---+
                         |                              |
  RUNTIME (one per host) v            rust/src/engine/  |
  +-------------------------------------------------+   |
  |  tree::build_with_focus   RawElement -> UiNode  |   |
  |     ids from parent + tag + key or index        |   |
  |  tree::resolve_colors     light or dark         |   |
  |  (equal to the last tree? skip layout)          |   |
  |  layout::compute          Taffy -> frames       |   |
  |     Measure: text, switch, image sizes  <-------+---+--- platform
  |  render::diff             Create/Update/        |   |
  |                           Reparent/Remove       |   |
  |  render::diff_layout      Layout (frames)       |   |
  +----------------------|--------------------------+   |
                         | Vec<Op>                      | NodeEvent
                         v                              |
  PLATFORM RENDERER      |        rust/src/android, ios |
  +----------------------|------------------------------|---+
  |  apply each Op to native views      listeners call  |   |
  |  (TextView, UILabel, ...)           the entry points|   |
  +---------------------------------------------------------+
                         |                              ^
                         v                              |
                    the screen  ---- user taps ---------+
```

Read it as two paths.

**Down (render).** `Session::render_all` asks the application for the description of each host (the
main view, each navigation screen, the bottom sheet) and calls `Host::render` on it. The host owns a
`Runtime`. `Runtime::update` builds the node tree, resolves colours, lays out, diffs, and returns a
`Vec<Op>`. The platform half of the host applies those operations to native views. After all hosts
have rendered, `Session::sync` compares the application's navigation stack, chrome, dialog and
sheet with what the native controller last received, and sends only the differences.

**Up (event).** A native listener calls a platform entry point, which calls
`Session::node_event(host, node_id, event)`. The session looks the node up in that host's current
tree, collects its handlers (`Actions::of`), and asks the matching handler for a message. The
message is a `Box<dyn Any>`; `AnyApp::update` downcasts it to the application's own message type
and calls `Component::update`. Then `render_all` runs again.

The two points where platform code takes over are the bottom edge of the runtime box (operations go
out) and the right edge (events come in, and the `Measure` trait is called during layout). Both are
described in [`../platforms/`](../platforms/).

## Why a tree of ids and not a tree of views

The engine never holds a pointer to a native view. It only knows nodes by a 64-bit `NodeId`. The
platform keeps a map from id to view. This has three consequences that shape the rest of the code:

1. **Ids must be stable.** If rebuilding the same description produced different ids, every render
   would destroy and recreate every view. So an id is a hash of the parent's id, the node's kind,
   and its key or position (chapter 4). The same description always hashes to the same ids.
2. **Everything is testable on a laptop.** The whole engine and the session run without a device.
   The tests use a fake `Measure` and a fake `Platform` and assert on the operations.
3. **Events are resolved late.** A native view only knows its id. The handler is looked up in the
   *latest* tree when the event arrives, so the engine never has to push a new closure to a view.
   This is why two handlers compare equal whenever both are set (chapter 2).

## The modules

All paths are under `rust/src/`.

| Module | What it is for |
|---|---|
| `lib.rs` | Declares the modules, re-exports `App`, `Component`, `set_app_factory`, `view!`, and gives the engine modules short paths such as `krabik_ui_dsl::runtime`. |
| `app.rs` | The `Component` and `App` traits an application implements; `AnyApp`, the same thing with the message type erased; the app factory. |
| `message.rs` | `Message` (a `Box<dyn Any>`), `Handler<A>` (an optional function from an event payload to a message) and `Wrap` (used by `Element::map`). |
| `element.rs` | `Element<M>` and `RawElement`, the description `view()` returns; `ElementKind`; the `style_builder!` and `text_builder!` macros. |
| `style.rs` | Values shared by all widgets: `Style`, `Length`, `EdgeInsets`, `Color`, `ColorScheme`, `Animation`, `Transition`, `Icon`. |
| `ui.rs` | A facade: re-exports everything an application needs to describe a UI. Most of the file is tests. |
| `widgets/mod.rs` | The `widgets!` table that declares the `Widget` enum, and the `WidgetProps` and `IntrinsicSize` traits. |
| `widgets/*.rs` | One file per widget: its properties struct, its builder and its intrinsic size. |
| `navigation.rs` | `Navigation`, `Route`, `ScreenTransition`, `BottomNavigation`. |
| `scaffold.rs` | `Scaffold` (one screen: body plus native chrome), `AppBar`, `Fab`, `Drawer`, `Snackbar`, and the erased `Chrome`. |
| `presentation.rs` | `Dialog` and `BottomSheet`. |
| `engine/geometry.rs` | `Size`, `Rect`, `Constraints`, `PixelFrame`. |
| `engine/tree.rs` | `RawElement` to `UiNode`: node ids, list windowing, colour resolution, tree walks. |
| `engine/measure.rs` | The `Measure` trait the platform implements, and `ApproximateMeasure` for tests. |
| `engine/layout.rs` | `UiNode` tree to Taffy and back: a frame for every node, a content size for every scroll container. |
| `engine/render.rs` | The `Op` enum and the two diffs that produce it. |
| `engine/runtime.rs` | `Runtime`: the state one host keeps between renders, and `update`, which ties the four previous files together. |
| `host/platform.rs` | The `Platform`, `Host` and `Controller` traits, `HostKind`, `ChromeEvent`. |
| `host/events.rs` | `NodeEvent` (what a native view reports) and `Actions` (the handlers one node carries). |
| `host/session.rs` | `Session<P>`: one running application, its hosts, and the mirror of navigation and chrome. Half the file is the fake-platform tests. |
| `animation.rs` | Evaluates timing curves in Rust. Only the iOS renderer uses it. |
| `frame_batch.rs` | Packs frames into flat integer arrays. Only the Android renderer uses it. |
| `android/`, `ios/` | The platform renderers. Not covered here. |

The dependency direction is strict and worth remembering: `widgets` and `element` know nothing
about the engine's tree; `engine` knows nothing about `host`; `host` knows nothing about Android or
iOS. Each layer can be tested with a fake of the layer below it.

## Glossary

**Element.** One piece of the description `view()` returns. `Element<M>` is the public, typed form:
`M` is the application's message type. Inside it is a `RawElement`, the untyped form the engine
consumes. An element is cheap, immutable data: a kind, a style, an optional key and children. It is
rebuilt from scratch on every render. See chapter 2.

**Node.** One entry of the engine's own tree, a `UiNode`. It is an element plus an identifier
(`NodeId`). A list element expands into several nodes, so the node tree is not always the same
shape as the element tree. The runtime keeps the node tree of the last render to diff against. See
chapter 4.

**Renderable node.** A node that has a native view of its own. Widgets, scroll containers, pagers
and recyclers always do. A `Column` or `Row` only does when it paints something (a background, a
border, a shadow). The others exist only for layout and never reach the platform.

**Runtime.** `engine::runtime::Runtime`. The engine state for one native root view: the last tree,
the last frames, scroll offsets, list measurements, focus. Its `update` method is the render
pipeline. See chapter 7.

**Host.** One native root view that Rust renders into, together with its `Runtime`. There is a main
host for the whole session, one host per navigation screen, and one for the bottom sheet while it
is open. The trait is `host::Host`; `HostKind` says which of the three a host is. See chapter 8.

**Session.** `host::Session<P>`. One running application instance and all its hosts, for one
Android activity or iOS scene. It owns the application, renders every host, mirrors navigation and
chrome to the controller, and routes events. It is generic over a `Platform`, so Android and iOS
share one copy of this logic. See chapter 8.

**Controller.** The native object that owns everything outside the widget tree: the screen stack,
the app bar, the bottom navigation bar, alerts, the sheet, the floating button, the drawer, the
snackbar. In Rust it is the `host::Controller` trait: a list of commands the session can send. The
controller never decides anything; it carries out what Rust says and reports what the user did.

**Chrome.** The native decoration around a screen's body: app bar, bottom navigation, floating
action button, drawer, snackbar. An application returns it as part of a `Scaffold`; the session
splits the scaffold into a body (rendered by the widget tree) and a `Chrome` value (sent to the
controller). Chrome is host-level because the platforms place and animate these pieces themselves.
See chapter 9.

**Op.** One instruction for a platform renderer: `Create`, `Update`, `Reparent`, `Remove` or
`Layout`. A render produces a `Vec<Op>`, called a batch. See chapter 6.

**Handler.** `message::Handler<A>`: "when an event with payload `A` happens, here is the message".
It holds an optional function from `A` to a boxed message. A button has a `Handler<()>`, a text
field a `Handler<String>`. See chapter 2.

**Message.** A value of the application's own message type (`Msg::Increment`). Inside the engine
it travels as `Box<dyn Any>`, because the engine does not know the type. The application gets it
back with its real type in `update`.

**Event.** What a native view reports before it is matched to a handler: `NodeEvent::Click`,
`NodeEvent::Text("abc")`, and so on. An event that no handler asked for is dropped.

**Key.** A string an application gives an element (`.key("row-7")`) so its id does not depend on
its position among its siblings.

**Logical points.** The unit of all layout numbers: dp on Android, points on iOS. Only the platform
renderer converts to pixels.

## Where this lives

- `rust/src/lib.rs`: the module list, with a comment on each conditional module.
- `rust/src/engine/mod.rs`: a five-line summary of the engine pipeline.
- `rust/src/host/mod.rs`: a summary of the session half.
- `Runtime::update` in `rust/src/engine/runtime.rs`: the down path in one function.
- `Session::node_event` in `rust/src/host/session.rs`: the up path in one function.

Next: [2. Elements and messages](02-elements-and-messages.md).
