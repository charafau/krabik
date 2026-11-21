# Compared to Flutter and React Native

This part explains `krabik-ui-dsl` by setting it next to two frameworks you may already know. It
is written first for a Flutter developer: every piece of the engine is matched with the Flutter
concept that does the same job, or with a plain statement that no such piece exists. A second
chapter does the same for React Native, which turns out to be the closer relative.

After reading this page you will know the seven ideas that matter most and have a one-table map
between the three frameworks. The two chapters then go through the internals in detail.

| Chapter | What it explains |
|---|---|
| [1. Compared to Flutter](01-compared-to-flutter.md) | The trees, one update step by step, identity and keys, state, layout, painting and threads, input, lists, navigation, interop, build and tooling, each against its Flutter counterpart. |
| [2. Compared to React Native](02-compared-to-react-native.md) | The same engine against React, Fiber, the Fabric shadow tree, Yoga, JSI and the mounting layer. |

The comparisons describe Flutter with its own renderer (Impeller, with Skia on older devices) and
React Native with the new architecture (Fabric, TurboModules, JSI, Hermes). Details of those
frameworks change between releases; where a detail depends on the version, the text says "recent
versions" and you should check the official documentation.

## The seven things to remember

If you read nothing else, read these. Each one links to the place that explains it.

### 1. It creates native views; it does not draw

Flutter owns every pixel: it paints your widgets itself on a surface the platform hands it. This
framework never paints. A `Text` becomes a real `TextView` on Android and a real `UILabel` on iOS;
a `Switch` is the platform's own switch. Rust only decides *which* views exist, *what* their
properties are and *where* they go.

That one fact puts the framework in React Native's family, not Flutter's. It is also why there is
no paint phase, no layer tree, no compositor and no raster thread anywhere in the code.
([Flutter chapter, "Painting"](01-compared-to-flutter.md#7-painting-threads-and-scheduling).)

### 2. There are two trees, not three

Flutter keeps a widget tree, an element tree and a render-object tree. Here:

- **`Element<M>`** is the description, the same idea as a Flutter `Widget`: an immutable value that
  `view()` builds from scratch every time.
- **`UiNode`** is the description with an id attached to every node. The engine keeps the one from
  the last render and compares the next one with it.

That is all. There is nothing like Flutter's long-lived `Element` object and nothing like a
`RenderObject`. Layout uses a Taffy tree that lives for the length of one function call, and the
"render objects" are the platform's own views, which Rust only knows by id.
([Flutter chapter, "The trees"](01-compared-to-flutter.md#2-the-trees).)

### 3. Identity is a number computed from position, not an object

In Flutter a piece of UI keeps its identity because an `Element` object stays mounted. Here a
node's identity is a 64-bit hash of three things: its parent's id, its kind, and its key (or, with
no key, its index among its siblings). Build the same description twice and you get the same ids,
so the same native views are kept. Change a node's parent, kind or position and it is a different
node: its view is removed and a new one created.

A key does one thing: it replaces the index in that hash. There is no `GlobalKey`.
([Flutter chapter, "Identity and keys"](01-compared-to-flutter.md#4-identity-and-keys).)

### 4. Everything is rebuilt after every message

There is no `setState` on a subtree, no dirty list, no `const` widgets and no `shouldRebuild`.
After each message the framework calls `view()` on the whole application and builds the whole node
tree again. The saving comes afterwards: if the new tree equals the old one, layout is skipped and
nothing is sent to the platform; otherwise only the nodes that differ produce operations.
([Flutter chapter, "One update"](01-compared-to-flutter.md#3-one-update-step-by-step).)

### 5. One state, one `update` function

State lives in one Rust struct. The only way to change it is a message handled by
`update(&mut self, msg) -> Command<Msg>`. Widgets have no state of their own, there are no hooks,
and there is no `BuildContext`, `InheritedWidget` or `Theme.of`. If you have used BLoC or Redux you
already know the shape: events in, new state out, UI is a function of state.
([Flutter chapter, "State"](01-compared-to-flutter.md#5-state).)

### 6. Layout is computed in Rust, with flexbox

Positions and sizes come from [Taffy](https://github.com/DioxusLabs/taffy), a flexbox library, in
Rust. The Android and iOS layout systems are bypassed: every view is simply given a frame. The one
thing Rust cannot work out alone is how big a piece of text is, so during layout it asks the
platform, synchronously, to measure it. The model is CSS flexbox, not Flutter's "constraints go
down, sizes go up", although `Column`, `Row` and `Spacer` will feel familiar.
([Flutter chapter, "Layout"](01-compared-to-flutter.md#6-layout).)

### 7. One thread, no frame scheduler, no bridge

A tap arrives on the platform's main thread, calls straight into Rust, runs `update`, rebuilds,
lays out, diffs and changes the native views before that call returns. There is no vsync-driven
frame, no separate UI thread and no message channel between a framework language and a host
language: Rust is compiled into the app and calls the platform's APIs directly (JNI on Android,
the Objective-C runtime on iOS). Background work runs on ordinary threads and comes back as a
message.
([Flutter chapter, "Painting, threads and scheduling"](01-compared-to-flutter.md#7-painting-threads-and-scheduling);
[React Native chapter, "Threads and the bridge"](02-compared-to-react-native.md#5-threads-and-the-bridge).)

## The three pipelines side by side

```text
  FLUTTER                     REACT NATIVE (Fabric)        KRABIK-UI-DSL

  setState / stream           setState / hook / store      native event -> message
        |                           |                            |
        v                           v                            v
  mark Element dirty          schedule a React render      update(&mut state, msg)
        |                           |                            |
   wait for vsync              (JS thread)                  (same call, main thread)
        |                           |                            |
        v                           v                            v
  BUILD   dirty subtrees      RENDER  components -> Fiber  view()   whole app
          Widget -> Element           -> shadow nodes (C++)         -> Element -> UiNode
        |                           |                            |
        v                           v                            v
  LAYOUT  RenderObjects       COMMIT  Yoga layout          LAYOUT   Taffy (skipped when
          constraints/sizes           shadow tree promoted          the tree is unchanged)
        |                           |                            |
        v                           v                            v
  PAINT   layer tree          MOUNT   diff shadow trees    DIFF     old UiNode vs new
        |                             -> mutations                  -> Vec<Op>
        v                           |                            |
  RASTER  Impeller/Skia             v                            v
          on raster thread    native views changed         native views changed
        |                     on the UI thread             (Android View / UIKit)
        v                           |                            |
  pixels on a surface         the platform draws           the platform draws
```

The left column ends in pixels the framework produced. The other two end in native views, and the
platform does the drawing. The right column has no waiting step: it is one synchronous call.

## Cheat sheet

Read a row as "the thing that does this job". "None" means the job is not done at all, or is done
by the platform.

| Concept | Flutter | React Native (new architecture) | krabik-ui-dsl |
|---|---|---|---|
| Language of the app | Dart | JavaScript / TypeScript | Rust |
| UI description | `Widget` | React element (JSX) | `Element<M>` (`rust/src/element.rs`) |
| Syntax sugar | none (constructors) | JSX | `view!` macro; expands to builder calls |
| Where you write a piece of UI | `build(BuildContext)` | function component | `Component::view(&self)` |
| Long-lived instance behind a description | `Element` | Fiber | none; a `NodeId` hash stands in for it |
| Retained tree that is diffed | element tree | Fiber tree, then shadow tree | `UiNode` tree of the last render |
| Object that does layout | `RenderObject` | shadow node (Yoga node) | Taffy node, built and dropped per layout |
| Layout model | box constraints; slivers | flexbox (Yoga) | flexbox (Taffy) |
| Text measurement | the engine's own text layout | native, called from layout | native `TextView` / `UILabel`, called from layout |
| What ends up on screen | pixels drawn by Impeller/Skia | native views | native views |
| Painting, compositing | `paint()`, layer tree, raster thread | the platform | the platform |
| Output of a render | a `Scene` (layer tree) | mount instructions | `Vec<Op>` (`rust/src/engine/render.rs`) |
| Local state | `State<T>` + `setState` | `useState`, class state | none |
| App-level state pattern | your choice (BLoC, Riverpod, …) | your choice (Redux, Zustand, …) | built in: `update(msg) -> Command` |
| Passing data down implicitly | `InheritedWidget`, `Provider` | Context | none; pass arguments |
| What gets rebuilt | dirty elements and below | the component that changed and below | everything, every time |
| Skipping work | `const`, `==` on widgets, repaint boundaries | `memo`, `useMemo` | whole-tree equality skips layout; per-node equality skips ops |
| Local key | `ValueKey`, `ObjectKey` | `key` prop | `.key("…")`, replaces the index in the id hash |
| Global key | `GlobalKey` | none (refs for access) | none |
| Frame scheduler | `SchedulerBinding`, vsync | React scheduler + UI frame | none; synchronous |
| Threads | UI, raster, platform, I/O | JS, UI, background | platform main thread; one OS thread per command |
| Background work | `Isolate`, `compute` | native modules, JS promises | `Command::perform` / `future` / `dispatcher` |
| Timers, streams | `Timer`, `Stream` | `setInterval`, effects | `Subscription::every` |
| Talking to the platform | platform channels, `dart:ffi` | JSI, TurboModules | direct calls: JNI, C functions, `objc2` |
| Embedding a native view | platform view (special case) | every host component | every widget |
| Gestures | own hit testing, gesture arena | native responders + JS responder system | the native view's own listeners |
| Text field value | `TextEditingController` | controlled component | value in state; "control follows state" |
| Long lists | `ListView.builder`, slivers | `FlatList` (virtualised in JS) | `List`: windowed, or `.recycled()` on `RecyclerView` / `UICollectionView` |
| Navigation | `Navigator` (routes in one tree) | React Navigation; native stack | `App::navigation()`; each screen is its own native root |
| App bar, tabs, dialogs, sheets | Flutter widgets | JS or native, by library | the platform's own, mirrored from `Scaffold`, `Dialog`, `BottomSheet` |
| Animation | controllers, tickers, implicit widgets | `Animated`, Reanimated | implicit only: `.animate(ms)`, enter/exit |
| Theme | `Theme.of(context)` | your choice | `Color::adaptive(light, dark)`; no theme object |
| Accessibility | semantics tree | accessibility props on native views | native views' own, plus `accessibility_label` |
| Create and run a project | `flutter create`, `flutter run` | the community CLI's `init`, `run-android`, `run-ios` | `krabik create --with-dsl`, `krabik run` |
| Hot reload | yes | Fast Refresh | `krabik watch`: on emulator and simulator, for `view` and `update` code ([how](../design/hot-reload.md)) |
| Tests without a device | widget tests, goldens | Jest, component tests | Rust tests on ops with a fake platform; no goldens |
| What ships | AOT snapshot + Flutter engine | JS bytecode + Hermes + native code | one native library with your app and the framework |

## What this part does not do

It does not rank the three frameworks. Flutter and React Native are mature, with tooling,
ecosystems and features this framework does not have: explicit animations, a semantics
tree, golden tests, thousands of packages. Each chapter ends with a plain list of what you gain
and what you give up.

## Where to go next

- For the engine in its own words, start at [The big picture](../internals/01-the-big-picture.md).
- For the reasons behind the choices compared here, read
  [Why it is built this way](../design/decisions.md).

Next: [1. Compared to Flutter](01-compared-to-flutter.md).
