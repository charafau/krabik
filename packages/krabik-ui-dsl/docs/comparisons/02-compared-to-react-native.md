# 2. Compared to React Native

This chapter sets `krabik-ui-dsl` next to React Native with the new architecture (Fabric,
TurboModules, JSI, Hermes). It assumes you have read
[the Flutter chapter](01-compared-to-flutter.md) or at least [the seven things to remember](README.md#the-seven-things-to-remember); it does not
explain the engine again from the start, it only places each piece next to its React Native
counterpart.

After reading it you will know why React Native is the closer relative, which of its layers this
framework also has under another name, and which layers it does not need.

If you come from Flutter and do not know React Native, read section 1 and the table in section 2.
They are enough to see where this framework sits between the two.

## 1. Why this is the closer relative

Flutter and React Native answer the question "what is on screen?" differently. Flutter draws
pixels itself. React Native creates the platform's own views and tells them what to show. This
framework gives React Native's answer.

It also shares React Native's second big decision: **layout is not done by the platform.** React
Native computes flexbox with Yoga, a C++ library, and hands each native view a finished frame.
This framework computes flexbox with Taffy, a Rust library, and does the same.

So the architecture has the same three stages:

```text
  REACT NATIVE (Fabric)                     KRABIK-UI-DSL

  React components (JS)                     view() (Rust)
        |  reconcile                              |  build, hash ids
        v                                         v
  shadow tree (C++), Yoga layout            UiNode tree, Taffy layout
        |  diff                                   |  diff
        v                                         v
  mount instructions                        Vec<Op>
        |                                         |
        v                                         v
  native views (UI thread)                  native views (main thread)
```

The difference is what sits *between* the stages. React Native has two languages (JavaScript for
your code, C++ and the platform languages for the renderer) and several threads, so it needs a
JavaScript engine, an interface between the languages, and rules about which thread does what.
This framework has one language for the app and the renderer, and one thread. Most of what
follows is a list of React Native machinery that has no counterpart here because there is nothing
for it to connect.

## 2. The trees

| React Native | Here | Notes |
|---|---|---|
| React element (what JSX produces) | `Element<M>` (`rust/src/element.rs`) | Immutable description, rebuilt on every render. |
| JSX | `view!` macro | Both are syntax only. `view!` expands to builder calls. |
| Function component | a `view` method or a plain function returning `Element<M>` | No component instance exists at run time. |
| Fiber | nothing | A `NodeId` hash stands in for it. |
| Hooks state stored on the fiber | nothing | State is in the app struct. |
| Shadow node (C++), with its Yoga node | `UiNode` (`rust/src/engine/tree.rs`), plus a Taffy node during layout | The tree that is laid out and diffed. |
| Host component (`<View>`, `<Text>`) | a widget or container (`Column`, `Text`, `Button`) | The things that can become native views. |
| Mount instruction (create, insert, update, remove, delete) | `Op` (`rust/src/engine/render.rs`) | See section 3 for the differences. |
| Host view | the native view, kept in a map by `NodeId` | Same thing: `TextView`, `UILabel`, … |
| React tag (an integer per host view) | `NodeId` | Both are how the native side names a view. |
| View flattening | view-less containers | See below. |

React keeps two retained trees in memory: the Fiber tree in JavaScript, which holds component
state and drives reconciliation, and the shadow tree in C++, which holds layout. This framework
keeps one, and it is a plain value: the `UiNode` tree of the last render.

### Shadow tree against `UiNode` tree

Both are immutable in the sense that matters. Fabric never mutates a committed shadow tree: an
update clones the changed nodes and their ancestors and shares the rest. Here nothing is shared
and nothing is cloned; the whole `UiNode` tree is built again from the description, and the old
one is dropped after the diff. Fabric's approach does less work per update; this one has less
machinery. The check that keeps it affordable is whole-tree equality: an unchanged tree skips
layout and produces no operations (`Runtime::update` in `rust/src/engine/runtime.rs`).

### View flattening is the default here

Fabric has an optimisation called view flattening: a `<View>` that only affects layout (no
background, no event handler, no opacity) gets no host view, and its children are attached to the
nearest ancestor that has one.

This framework does the same thing, and it is the rule, not an optimisation. A `Column`, `Row` or
`Spacer` has a native view only when `Style::needs_view()` (`rust/src/style.rs`) says it carries
something a view is needed for: a background, a border, rounded corners, a shadow, opacity, an
animation or an accessibility label. When a container gains or loses its view between renders, the
diff emits a `Create` or `Remove` for the container and a `Reparent` for each affected child, so
the children's views survive.

## 3. Reconciliation

### How identity works

React's rule: a child keeps its identity when it is at the same place under the same parent with
the same type and the same key. That rule is the same here. What differs is how it is carried
out.

React walks the old fibers and the new elements of each parent together, matching by key (or by
index without keys), and records what was added, moved or deleted. The fiber object persists and
is updated.

Here the rule is baked into a number. Each node's id is a hash of its parent's id, a tag for its
kind and its key or index (`node_id` in `rust/src/engine/tree.rs`). The diff
(`diff_reapplying` in `rust/src/engine/render.rs`) indexes the old tree by id and looks each new
node up. It never compares two child lists.

| Situation | React | Here |
|---|---|---|
| Same type, same position | Fiber kept, props updated | Same id: `Update` if anything differs |
| Different type, same position | Unmount, mount | Different id: `Remove` and `Create` |
| List item inserted, with keys | Others kept, moved | Others keep their ids and get new frames |
| List item inserted, no keys | Later items re-used with the wrong props | Same |
| Keyed child moved to another parent | Remount | Different id (the parent is in the hash): recreated |
| Duplicate keys among siblings | Warning | `debug_assert!` panic in debug, one log line in release |

A key therefore means what it means in React: scoped to siblings, used to keep identity when the
order changes.

### Phases

Fabric splits an update into three phases, which can run on different threads:

1. **Render**: React runs components and builds shadow nodes.
2. **Commit**: Yoga lays the new shadow tree out, and it becomes the next tree to mount.
3. **Mount**: the new tree is diffed against the one on screen, and the resulting mutations are
   applied to host views on the UI thread.

React can also interrupt a render, prioritise one update over another, and batch several state
changes into one render.

Here the three phases are consecutive statements in one function, `Runtime::update`: build,
layout, diff. Then the renderer applies the operations. Nothing can be interrupted or reordered,
there are no priorities, and one event produces one render.

### The operations

Fabric's mutations include inserting a child at an index and removing it from an index, because a
native parent's child order matters to it. `Op` has no index anywhere:

```rust,ignore
// rust/src/engine/render.rs
pub enum Op {
    Create(RenderNode),
    Update(RenderNode),
    Reparent { id: NodeId, parent: Option<NodeId> },
    Remove { id: NodeId, exit: Option<Transition> },
    Layout { id: NodeId, frame: Rect, content_size: Option<Size>, animation: Option<Animation> },
    ScrollTo { id: NodeId, offset: f32 },
}
```

A new view is appended to its parent, and its position comes only from its frame. Reordering
siblings is a set of `Layout` operations.

More: [Diffing and operations](../internals/06-diffing-and-operations.md).

## 4. State

React components hold state locally (`useState`, `useReducer`), share it through context, and
run effects tied to their lifetime. This framework has none of those. It has the pattern React
developers know from Redux, built in and without the store library:

| React / Redux | Here |
|---|---|
| Store state | your app struct |
| Action | a variant of your `Msg` enum |
| `dispatch(action)` in a handler | `.on_click(Msg::Something)` |
| Reducer `(state, action) => state` | `update(&mut self, msg)`; mutates in place |
| Thunk, saga, async action | the `Command` that `update` returns |
| `useEffect` with an interval and a cleanup | `Subscription::every`, returned from `App::subscriptions()` while it should run |
| `useSelector`, `memo`, `useMemo` | none; the whole `view` runs every time |
| `useState` for local UI state | a field in the app struct or in a child component's struct |
| Context | pass arguments |
| `useRef` to a host component | none; views are not reachable from app code |
| `useEffect` on mount / unmount | none for a component: nothing is mounted. `App::init` returns the work the app starts with |

`useReducer` at the root of an app, with every component a pure function of props, is a fair
picture of what an app here looks like. The reducer is `update`, and the compiler checks that
every action is handled.

Event handlers differ in one way worth knowing. In React a handler is a function that runs. Here
it is a description of the message to send, stored in the node tree, and the native view only
knows its node id. When the view fires, the handler is read from the latest tree
(`Session::node_event` in `rust/src/host/session.rs`). The problem of stale closures, which hooks
users know well, cannot occur, because no closure is ever handed to a view.

## 5. Threads and the bridge

This is where the two frameworks differ most.

### React Native

- Your code runs in a JavaScript engine (Hermes), normally on a dedicated **JS thread**.
- Native views can only be touched on the **UI thread**.
- Layout can run on a **background thread**.
- **JSI** lets JavaScript hold references to C++ objects and call them synchronously, replacing
  the older asynchronous, serialising bridge.
- **TurboModules** expose platform APIs to JavaScript through JSI, with **Codegen** producing the
  typed glue from a specification.

Much of the new architecture's design is about getting work safely from the JS thread to the UI
thread, and about allowing a synchronous render on the UI thread when an event needs one.

### Here

| | React Native | Here |
|---|---|---|
| Where app code runs | JS thread | platform main thread |
| Where layout runs | JS or background thread | main thread |
| Where views are mutated | UI thread | main thread |
| Getting from app code to views | commit, then a hop to the UI thread | a function call |
| Language runtime shipped | Hermes | none |
| Interface between app language and native | JSI | none needed; Rust is native code |
| Platform API access | TurboModules + Codegen | direct calls from Rust: JNI on Android, `objc2` on iOS |
| Glue between Rust and Kotlin/Swift | (generated) | written by hand; shared constants checked by a test |
| Background work | native modules; promises on the JS thread | `Command`: one OS thread per job, result posted to a mailbox |

Rust is compiled into the app as a native library. When a button is tapped, the platform calls a
Rust function on the main thread; that function runs `update`, the render and the view mutations,
and returns. There is no engine to enter, no value to convert between two object models, and no
thread to hop to.

The cost of that simplicity is the one every native developer knows: the main thread is the only
thread. In React Native a slow reducer blocks the JS thread while native scrolling stays smooth.
Here a slow `update` or `view` blocks scrolling too. Slow work belongs in a `Command`.

The return path from a background thread is the only asynchronous hop in the framework. The
worker puts its message in the `Mailbox` (`rust/src/command.rs`); the mailbox wakes the main
thread (a `Handler` post on Android, a main-queue dispatch on iOS); `Session::drain` applies every
waiting message and renders once. More:
[Commands and the mailbox](../internals/11-commands-and-the-mailbox.md).

### The boundary that does exist

There is still a boundary between Rust and the host's Kotlin and Swift shells, and it is cruder
than JSI:

- **Android**: Kotlin declares `external fun`s on `RustRootView`; Rust exports the matching JNI
  symbols (`rust/src/android/entry.rs`) and calls Android methods through cached method ids
  (`rust/src/android/handles.rs`). Signatures are strings, checked at run time.
- **iOS**: Swift calls C functions declared in a hand-written header (`shared/ios/KrabikIOS.h`);
  Rust creates and configures UIKit views itself through the `objc2` crates.

What crosses is small: a session handle, a host id, a node id and a primitive payload on the way
in; ordinary setter calls on views on the way out. There is no serialised tree and no generated
code. More: [How Rust talks to the platforms](../platforms/01-how-rust-talks-to-the-platforms.md).

## 6. Events and controlled components

A React Native `TextInput` with a `value` prop is a controlled component: the native field
changes first, sends `onChangeText`, and React then sets the value it wants. Everyone who has
filtered input in `onChangeText` has seen the consequences of the native view being one step
ahead.

This framework has the same situation and one explicit rule for it, "control follows state".
After a value-changing event (text, toggle, slider, selection, picker) that has a handler, the
session calls `Runtime::reapply(id)`, and the next diff emits an `Update` for that control even if
the description did not change. If `update` rejected or altered the value, the control is put back
to what the state holds. Every input is controlled; there is no uncontrolled mode with a default
value, except that a control with no handler is left as the user set it.

Gestures are simpler than in React Native. There is no responder system and no gesture-handler
library. Each widget reports the events of its native control and nothing else. The counterpart
of `Pressable` around a card or a row is `Container::new().on_tap(msg)`, which reports a tap and
nothing finer (no press-in, press-out or long press).

## 7. Lists, navigation, animation

### Lists

| React Native | Here |
|---|---|
| `ScrollView` with all children | `Scroll` |
| `FlatList`: virtualised in JavaScript, renders a window of items and blank space around it | `List` in its default, windowed mode: a native scroll view with a top spacer, the visible rows and a bottom spacer; the window is recomputed in Rust as the offset changes |
| Recycling list libraries | `List::recycled()`: a real `RecyclerView` / `UICollectionView`; the platform asks Rust to bind a row into a reused cell |

The windowed mode works like `FlatList` and shares its character: rows are created as they
approach the viewport. In the recycled mode the platform's own cell reuse does the work, and each
cell keeps a small node tree of its own that is diffed when another row is bound into it
(`RowBinder` in `rust/src/host/rows.rs`).

### Navigation

React Navigation's native stack renders each screen into a native container
(`UINavigationController` on iOS, fragments on Android), so transitions and gestures are the
platform's. This framework does the same and nothing else: there is no JavaScript-style stack
drawn by the framework. `App::navigation()` returns the stack as data, the session diffs it by
route key against what the platform shows, and each screen is a separate native root with its own
`Runtime`.

App bars, tab bars, alerts and sheets follow the same idea: they are the platform's, described by
`Scaffold`, `Dialog` and `BottomSheet` and mirrored by the session.

### Animation

| React Native | Here |
|---|---|
| `LayoutAnimation`: animate whatever the next layout changes | `.animate(ms)` / `.animation(…)` on a node: its frame and opacity changes are animated by the platform |
| Entering / exiting animations | `.animate_enter(…)`, `.animate_exit(…)` |
| `Animated` values, native driver | none |
| Reanimated worklets, gesture-driven animation | none |

Only the implicit kind exists.

## 8. Build and tooling

| React Native | Here |
|---|---|
| Metro bundles JavaScript; the native app loads the bundle | cargo compiles the app and the framework into one native library; Gradle and Xcode invoke cargo |
| `npx @react-native-community/cli init` | `krabik create <name> --with-dsl`; the project refers to a local checkout of the framework |
| `run-android`, `run-ios` | `krabik run`, `krabik run --platform ios` |
| Fast Refresh | `krabik watch`: hot reload of `view` and `update` code on the emulator and the simulator ([how](../design/hot-reload.md)) |
| Over-the-air JavaScript updates | not possible; everything is compiled native code |
| Flipper / React DevTools | the platform's view inspectors show the real views; logs under the `krabik` tag |
| Jest, React Native Testing Library | Rust tests against a fake platform that assert on operations and messages |
| Detox, Maestro | Espresso and XCUITest suites |
| Autolinking of native modules | none; one Gradle script and one Xcode pre-build script |
| npm ecosystem | none |

More: [Building and CI](../platforms/06-building-and-ci.md),
[Tutorial 9: Testing](../tutorial/09-testing.md).

## 9. What you gain, what you give up

**Gain, compared with React Native**

- No JavaScript engine, no bundle, no garbage collector, and no boundary between your code and
  the renderer.
- One thread and one synchronous path from event to view, which is easy to reason about and to
  step through in a debugger.
- Types end to end: messages, handlers and widget properties are checked by the Rust compiler.
- An engine that runs entirely in unit tests, without a device or a simulated host.
- Direct access to platform APIs from app code, without writing a module.

**Give up**

- Fast Refresh on devices and for every kind of change, and over-the-air updates.
- Local component state, hooks, context.
- Concurrent rendering, priorities, and a JS thread that leaves the UI thread free.
- Explicit and gesture-driven animation.
- Generated, type-checked glue between the languages.
- The ecosystem: navigation libraries, component libraries, native modules.
- Maturity. This is a young framework with a known list of rough edges ([`TODO.md`](../../TODO.md)).

## Rough edges

- Statements about React Native describe the new architecture in general terms. Thread use and
  defaults have changed between releases; check the React Native documentation for the version
  you use.
- The comparison of `FlatList` with the windowed list is about the mechanism, not about measured
  performance. Nothing in this repository benchmarks one against the other.

## Where this lives

- `rust/src/element.rs`, `rust/src/engine/tree.rs`: the description and the node tree.
- `rust/src/engine/layout.rs`: the Taffy pass that plays Yoga's role.
- `rust/src/engine/render.rs`: `Op`, the counterpart of mount instructions.
- `rust/src/engine/runtime.rs`: render, commit and diff in one function.
- `rust/src/host/session.rs`: events in, renders out, navigation and chrome mirrored.
- `rust/src/command.rs`: background work and the mailbox.
- `rust/src/android/entry.rs`, `rust/src/ios/entry.rs`: the entry points native code calls.

Back to [the index](README.md).
