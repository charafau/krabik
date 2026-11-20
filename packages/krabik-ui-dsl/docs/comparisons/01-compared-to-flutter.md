# 1. Compared to Flutter

This chapter walks through the internals of `krabik-ui-dsl` in the order a Flutter developer would
ask about them: where are the three trees, what happens after `setState`, what is a key, where is
the state, who does layout, who paints, which thread am I on. For each question it first recalls
what Flutter does, then shows what this framework does instead, with the file to open.

After reading it you will be able to translate what you know about Flutter's pipeline into this
codebase, and you will know which Flutter ideas have no counterpart here at all.

## 1. The mental model

Start from the same sentence Flutter uses: **UI is a function of state.** That part is identical.
You hold state, you write a function that returns a tree of cheap immutable values describing the
UI, and the framework works out what to change on screen.

Here is a counter in both.

```dart
// Flutter
class Counter extends StatefulWidget {
  @override
  State<Counter> createState() => _CounterState();
}

class _CounterState extends State<Counter> {
  int count = 0;

  @override
  Widget build(BuildContext context) {
    return Column(children: [
      Text('$count'),
      ElevatedButton(
        onPressed: () => setState(() => count++),
        child: const Text('+1'),
      ),
    ]);
  }
}
```

```rust,ignore
// krabik-ui-dsl
struct Counter { count: u32 }

#[derive(Clone)]
enum Msg { Increment }

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Increment => self.count += 1,
        }
        Command::none()
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .child(Text::new(self.count.to_string()))
            .child(Button::new("+1").on_click(Msg::Increment))
            .into()
    }
}
```

Three differences are already visible, and they are the important ones:

1. **The button does not hold a closure that changes state.** It holds a *value*, `Msg::Increment`.
   `view` takes `&self`: it cannot change anything. Only `update` can.
2. **There is no `State` object attached to a widget.** `Counter` is an ordinary struct. Nothing in
   the framework creates it per widget, keeps it alive or disposes of it.
3. **There is no `BuildContext`.** `view` receives nothing but `self`.

What is *not* visible is the largest difference: Flutter turns that description into pixels, and
this framework turns it into a `TextView` and a `Button` on Android, a `UILabel` and a `UIButton`
on iOS. Everything below follows from those two facts: a single message-driven state, and native
views as the output.

## 2. The trees

### What Flutter has

Flutter keeps three trees alive between frames:

- **Widgets**: immutable configuration, rebuilt freely.
- **Elements**: one long-lived object per widget position. An element holds the `State`, knows its
  parent and children, and decides whether a new widget can update it (`Widget.canUpdate`: same
  `runtimeType` and same `key`).
- **Render objects**: long-lived objects that do layout, painting and hit testing.

Below those, painting produces a fourth structure, the layer tree, which the engine rasterises.

### What this framework has

```text
  FLUTTER                                KRABIK-UI-DSL

  Widget tree       (immutable,          Element<M> tree   (immutable, rebuilt
      |              rebuilt)                 |              from scratch, then dropped)
      v                                       v
  Element tree      (long-lived,         UiNode tree       (a value; the one from the
      |              holds State)             |              last render is kept to diff)
      v                                       v
  RenderObject tree (long-lived,         Taffy tree        (exists only inside
      |              layout + paint)          |              layout::compute)
      v                                       v
  Layer tree        (per frame)          Vec<Op>           (per render)
      |                                       |
      v                                       v
  engine rasterises                      native view hierarchy
                                         (Android Views / UIKit, kept by id)
```

| Flutter | Here | Notes |
|---|---|---|
| `Widget` | `Element<M>`, wrapping a `RawElement` | Same role. `M` is your message type. |
| `Element` | nothing | Replaced by a computed `NodeId`. See [section 4](#4-identity-and-keys). |
| `State<T>` | nothing | State is in your app struct. |
| `RenderObject` (layout half) | a Taffy node | Thrown away after each layout. |
| `RenderObject` (paint and hit-test half) | the native view | Owned by the platform. |
| Layer tree | nothing | The platform composites its own views. |

The naming is a trap: **this framework's `Element` is Flutter's `Widget`**, not Flutter's
`Element`.

The description is a small struct:

```rust,ignore
// rust/src/element.rs
pub struct RawElement {
    kind: ElementKind,
    style: Style,
    key: Option<Cow<'static, str>>,
    children: Vec<RawElement>,
}
```

The engine turns it into a `UiNode` tree, which is the same data with an id on every node:

```rust,ignore
// rust/src/engine/tree.rs
pub struct UiNode {
    pub id: NodeId,
    pub kind: NodeKind,
    pub style: Style,
    pub children: Vec<UiNode>,
}
```

A `UiNode` is not an object with a life of its own. Nothing mutates it, it has no parent pointer
and no methods that do layout or painting. After every message a complete new `UiNode` tree is
built and the old one is dropped. What survives between renders is the *ids*, and the native views
the platform keeps in a map under those ids.

Each native root view (the main view, each navigation screen, the bottom sheet) has its own
`Runtime` (`rust/src/engine/runtime.rs`) that keeps the last `UiNode` tree, the last frames,
scroll offsets and focus. That `Runtime` is the nearest thing to Flutter's `BuildOwner` and
`PipelineOwner` rolled into one: its `update` method is the whole pipeline, in one function.

### Not every node has a view

In Flutter, `Padding`, `Column` and `Center` all create render objects. Here a `Column`, `Row` or
`Spacer` takes part in layout but normally creates **no native view**. Its children's views are
attached to the nearest ancestor that does have one, with frames worked out relative to it. A
container only gets a view of its own when it has something only a view can carry:

```rust,ignore
// rust/src/style.rs
pub fn needs_view(&self) -> bool {
    self.accessibility_label.is_some()
        || self.animation.is_some()
        || self.enter.is_some()
        || self.exit.is_some()
        || self.background.is_some()
        || self.gradient.is_some()
        || self.corner_radius > 0.0
        || self.corner_radii.is_some()
        || self.border.is_some()
        || self.opacity.is_some_and(|opacity| opacity < 1.0)
        || self.elevation > 0.0
}
```

So deeply nested layout costs almost nothing on the native side: twenty nested `Column`s around a
`Text` produce one `TextView`. Give a `Column` a background and it gains a view; its children are
then moved under it with a `Reparent` operation, without being recreated.

More: [The tree and identity](../internals/04-the-tree-and-identity.md).

## 3. One update, step by step

### In Flutter

1. `setState` marks one `Element` dirty and asks the scheduler for a frame.
2. Nothing happens until the next vsync.
3. `BuildOwner.buildScope` rebuilds the dirty elements, in depth order. Each rebuild calls
   `build`, compares the new child widgets with the old ones and updates, creates or unmounts
   child elements.
4. `PipelineOwner.flushLayout` lays out the render objects that were marked as needing it.
5. `flushPaint` repaints the dirty ones into layers.
6. The layer tree goes to the raster thread, which draws it.

Work is proportional to what was marked dirty, and it is spread over a frame.

### Here

1. A native listener (say a click) calls a Rust entry point with a node id.
2. `Session::node_event` (`rust/src/host/session.rs`) finds that node in the latest tree, takes
   its handler and builds the message.
3. `update(&mut self, msg)` runs on your app struct.
4. `Session::render_all` calls `view()` (or `screen(key)`, or `sheet()`) for **every** host.
5. For each host, `Runtime::update` does:
   - `tree::build`: `RawElement` to `UiNode`, computing ids;
   - `tree::resolve_colors`: pick the light or dark value of each colour;
   - compare the new tree with the old one; if equal, skip the next step;
   - `layout::compute`: build a Taffy tree, run flexbox, collect a frame per node;
   - `render::diff`: which views to create, update, reparent or remove;
   - `render::diff_layout`: which frames changed.
6. The platform renderer applies the resulting `Vec<Op>` to native views.
7. The session compares navigation, app bar, dialog and sheet with what the platform last got and
   sends the differences.
8. The entry point returns. The platform draws the changed views on its own schedule.

All of it happens inside the one call from the native listener, on the main thread.

| | Flutter | Here |
|---|---|---|
| What triggers work | `markNeedsBuild` on an element | any message, a scroll that changes a list window, a size or colour-scheme change, a background result |
| When it runs | next vsync | immediately, synchronously |
| How much is rebuilt | dirty elements and their subtrees | the whole description, for every host |
| How work is avoided | only dirty parts run; `const` widgets; relayout and repaint boundaries | equal tree: no layout, no ops. Different tree: ops only for nodes that differ |
| Who owns the pipeline | `BuildOwner`, `PipelineOwner`, `SchedulerBinding` | `Session::render_all`, `Runtime::update` |

The shortcut is one comparison:

```rust,ignore
// rust/src/engine/runtime.rs, in Runtime::update
let (mut first, first_info) = self.build(element, viewport);
// ...
let unchanged =
    !suppress_animation && first_info.reset_scroll.is_empty() && first == self.tree;
```

`UiNode` derives `PartialEq`, so "did anything change?" is a structural comparison of two trees.
Because many messages change nothing on a given screen, this is what makes "rebuild everything"
affordable: building a tree of small structs is cheap, measuring text and running flexbox is not,
and the second part is skipped.

Keep the cost model in mind when you write `view`. It runs after every message, including each
character typed and each time a windowed list scrolls past a row boundary. There is no `const`
constructor to stop a subtree from being rebuilt and no `Selector` to narrow a rebuild. Do the
expensive work (sorting, filtering, formatting a thousand rows) in `update` and keep the result in
state, or hand a list an `Arc<Vec<T>>` and let it build only the visible rows.

More: [The runtime](../internals/07-the-runtime.md),
[decision 7, "Always re-run `view`"](../design/decisions.md).

## 4. Identity and keys

This is the part that differs most from Flutter while looking the same from outside.

### In Flutter

Identity is an object. An `Element` stays mounted across rebuilds. When its parent rebuilds, the
framework walks the old and new child lists and, for each slot, asks `Widget.canUpdate(old, new)`:
same `runtimeType` and same `key`? Then the element is kept and given the new widget. Otherwise it
is unmounted and a new one inflated. Keys let the framework match children that moved within the
list. A `GlobalKey` lets an element move anywhere in the tree and keep its state.

### Here

Identity is a number, recomputed on every build:

```rust,ignore
// rust/src/engine/tree.rs
fn node_id(parent: NodeId, index: usize, key: Option<&str>, tag: u64) -> NodeId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64; // FNV-1a offset basis
    // ...
    mix(parent.0);
    mix(tag);
    match key {
        Some(key) => mix(hash_string(key)),
        None => mix(index as u64),
    }
    NodeId(hash)
}
```

Three inputs: the **parent's id**, a **tag** that is fixed per kind of node (`Column`, `Text`,
`Button`, … each have their own number), and the **key** or, without one, the **index** among the
siblings.

The diff then needs no child-list algorithm at all. It puts every node of the old tree in a
`HashMap<NodeId, &UiNode>`, walks the new tree, and looks each node up by id
(`diff_reapplying` in `rust/src/engine/render.rs`):

- id not in the old tree: `Create`;
- id in both, something differs: `Update`;
- id in both, nothing differs: no operation;
- id only in the old tree: `Remove`.

Matching "by key" or "by position" is not a decision the diff makes. It is a consequence of how
the id was computed.

### What that means in practice

| Situation | Flutter | Here |
|---|---|---|
| Same widget type at the same slot, no keys | Element kept, updated | Same id: view kept, `Update` if anything differs |
| Different type at the same slot | Element replaced | Different tag, so a different id: one `Remove`, one `Create` |
| Unkeyed item inserted at the top of a list | Each element is matched with the wrong widget and updated; state sticks to positions | Same: each id now describes the next item; views are updated in place, one is created at the end |
| Keyed item inserted at the top | Elements matched by key, moved | Ids unchanged; the new item is created; the others only get new frames |
| Keyed siblings reordered | Render objects moved in the child list | Only `Layout` ops. Nothing is moved in the native child list; each view just gets a new frame |
| Subtree wrapped in a new parent | State lost, unless a `GlobalKey` is used | Every id below changes (the parent id is part of the hash): all views recreated. No way to prevent it |
| Keyed node moved to another parent | State lost, unless `GlobalKey` | Recreated. A key is only meaningful among siblings |
| Two siblings with the same key | Assertion in debug | `debug_assert!` panic in debug builds, one log line in release |

So a key here is exactly a Flutter `ValueKey<String>`: a *local* key, scoped to one parent. Use it
for the same reason, on anything in a list that can be inserted, removed or reordered:

```rust,ignore
Text::new(&note.title).key(format!("note-{}", note.id))
```

There is no `GlobalKey`, and nothing that plays its role: no way to move a view between parents
with its state, no `currentState`, no `currentContext`.

### What "state" a kept view still has

Since there is no `State` object, what is preserved when an id survives? Whatever the *native
view* remembers by itself: a scroll position, the cursor and selection in a text field, keyboard
focus, a running animation. Lose the id and you lose those, in the same situations where Flutter
would lose a `State`.

One detail worth knowing: there is no "move child" operation. Native children are appended in the
order they are created, and positions come entirely from frames. Reordering keyed siblings is
therefore very cheap, but it also means the native child order (which decides which of two
overlapping siblings is on top) follows creation order, not the current order in your
description.

More: [The tree and identity](../internals/04-the-tree-and-identity.md),
[Diffing and operations](../internals/06-diffing-and-operations.md).

## 5. State

### The shape

Flutter gives you a mechanism (`State` and `setState`) and leaves the architecture to you.
This framework gives you one architecture and no mechanism below it. It is the Elm architecture,
also called model–view–update:

```rust,ignore
// rust/src/app.rs
pub trait Component {
    type Msg: 'static;
    fn update(&mut self, msg: Self::Msg) -> Command<Self::Msg>;
    fn view(&self) -> Element<Self::Msg>;
}
```

If you use BLoC, the mapping is direct:

| BLoC | Here |
|---|---|
| Event class | a variant of your `Msg` enum |
| `bloc.add(Event())` in a callback | `.on_click(Msg::Something)` on a widget |
| Bloc state class | your app struct |
| `on<Event>((event, emit) { … })` | a `match` arm in `update` |
| `emit(newState)` | mutate `self` |
| `BlocBuilder` | `view(&self)`; always the whole thing |
| An `async` event handler | return a `Command` from `update` |
| A stream subscription in the bloc | `App::subscriptions()` |

### What is missing, and what you do instead

| Flutter | Here |
|---|---|
| `StatefulWidget` for a bit of local state (is this panel expanded?) | A field in your app struct, or in a child component's struct that the parent owns |
| `InheritedWidget`, `Provider`, `context.watch` | Pass the value as a function argument. `view` is plain Rust; helper functions take what they need |
| `Theme.of(context)`, `MediaQuery.of(context)` | No context. Colours carry both appearances (`Color::adaptive`); see [section 9](#9-lists-navigation-chrome-animation-theming-accessibility) |
| `initState` / `dispose` | None for a widget: nothing is mounted. The app has `App::init` for the work it starts with; start other work from `update` and keep timers in `subscriptions()` |
| `TextEditingController`, `ScrollController`, `FocusNode` | None. The text is a `String` in your state; scroll position belongs to the native view |
| `ValueNotifier`, streams feeding a builder | A message that updates state |

### Components

A reusable piece with its own state is a struct that implements `Component`, stored as a field of
its parent. The parent embeds the child's view and translates its messages:

```rust,ignore
// in the parent's view
self.counter.view().map(Msg::Counter)

// in the parent's update
Msg::Counter(msg) => self.counter.update(msg).map(Msg::Counter),
```

`Element::map` (`rust/src/element.rs`) wraps every handler in the child's subtree so the messages
come out as the parent's type. This is the only composition mechanism for state. The parent
always owns the child; nothing is created or destroyed by the framework as the tree changes shape.

### Handlers are values

A Flutter callback is a closure that runs when the button is pressed. Here a handler is "the
message to send" (`Handler<A>` in `rust/src/message.rs`), and two handlers compare equal whenever
both are set:

```rust,ignore
// rust/src/message.rs
impl<A> PartialEq for Handler<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0.is_some() == other.0.is_some()
    }
}
```

This matters for the diff. In Flutter a new closure on every build is harmless because the element
is updated anyway. Here, if closures were compared, every node with a handler would look changed
on every render. Instead, a native view only ever knows its node id; when it fires, the handler is
read from the *latest* tree. A new closure never has to be sent anywhere.

### Side effects

`update` runs on the main thread and must not block. Anything slow is returned as a value:

```rust,ignore
Msg::Refresh => Command::perform(|| Msg::Loaded(fetch_notes())),
```

| Dart | Here (`rust/src/command.rs`) |
|---|---|
| `await someFuture()` then `setState` | `Command::future(async { … })`; its output is a message |
| `compute(fn, arg)` / `Isolate.run` | `Command::perform(|| …)`; runs on its own thread |
| A `Stream` you listen to | `Command::dispatcher(|d| …)`; `d.send(msg)` any number of times, from any thread |
| `Timer.periodic` | `Subscription::every(key, interval, || Msg::Tick)` returned from `App::subscriptions()` |
| Cancelling a subscription in `dispose` | Stop returning it; subscriptions are compared by key after every message |

There is no event loop and no `async` runtime under your `update`. A command is a job on an OS
thread whose result is put in a mailbox; the main thread is woken, applies every waiting message,
and renders once. More: [Commands and the mailbox](../internals/11-commands-and-the-mailbox.md).

## 6. Layout

### In Flutter

"Constraints go down, sizes go up, the parent sets the position." Each `RenderBox` receives
`BoxConstraints`, lays out its children with constraints of its own choosing, picks a size and
positions the children. Every layout widget is its own algorithm (`RenderFlex`, `RenderStack`,
`RenderPadding`, slivers for scrolling). Layout is incremental: relayout boundaries stop a change
from spreading.

### Here

One algorithm for everything: CSS flexbox, run by Taffy in Rust.

```rust,ignore
// rust/src/engine/layout.rs
pub fn compute(root: &UiNode, viewport: Size, measurer: &dyn Measure) -> LayoutResult {
    let mut taffy = TaffyTree::new();
    // ...
}
```

Each `UiNode` becomes a Taffy node carrying a flexbox style: `Row` is a row-direction flex
container, everything else is column-direction, `padding`, `spacing`, `grow`, `width_percent`,
`align_items` and so on map to flexbox properties. Taffy returns a rectangle per node. The result
is a `HashMap<NodeId, Rect>` of frames in logical points (dp on Android, points on iOS).

| | Flutter | Here |
|---|---|---|
| Model | per-widget protocol, box constraints | one flexbox pass |
| `Column` / `Row` | `RenderFlex` | flex container; `Row` and `Column` exist with the same meaning |
| `Expanded(flex: 1)` | parent data on the child | `.grow(1.0)` on the child |
| `Padding` widget | its own render object | `.padding(…)` on any element |
| `SizedBox`, `ConstrainedBox` | render objects | `.width(…)`, `.height(…)`, `.width_percent(…)` on the element |
| `Stack` / `Positioned` | `RenderStack` | no general equivalent in the engine |
| Custom layout | `CustomMultiChildLayout`, your own `RenderBox` | none |
| Incremental layout | yes, relayout boundaries | no; all or nothing, per host |
| Where the tree lives | in the render objects, between frames | created and dropped inside `compute` |

"All or nothing" sounds worse than it is: when the node tree is unchanged the whole pass is
skipped, and when it runs, text sizes are cached by the platform measurers.

### Text is measured by the platform

Flutter lays out text itself, in the engine, which is how it can guarantee identical text metrics
everywhere. This framework draws text with the platform's `TextView` or `UILabel`, so only the
platform knows how big a string will be. Layout therefore calls out, in the middle of the flexbox
pass:

```rust,ignore
// rust/src/engine/measure.rs
pub trait Measure: Send + Sync {
    fn measure(&self, text: &TextProps, constraints: Constraints) -> Size;
    // ...
}
```

On Android the implementation sets the text on an off-screen `TextView` and calls its `measure`;
on iOS it asks an off-screen `UILabel` for `sizeThatFits`. Both cache results. In unit tests an
`ApproximateMeasure` stands in, which is why the engine's layout can be tested on a laptop.

Two consequences for a Flutter developer: text looks exactly like the rest of the platform
(fonts, shaping, emoji, dynamic type are the system's), and the same string can have slightly
different sizes on the two platforms.

### How the result reaches the views

Frames are sent as `Op::Layout`, each relative to the nearest ancestor that has a view. The
platform layout systems are not used: no `LinearLayout`, no `ConstraintLayout`, no Auto Layout. On
Android every frame of a render is packed into flat arrays and sent in a single JNI call
(`rust/src/frame_batch.rs`); on iOS the frames are set directly.

More: [Layout and measurement](../internals/05-layout-and-measurement.md).

## 7. Painting, threads and scheduling

### Painting

Flutter: render objects paint into a canvas, producing layers; the engine (Impeller, or Skia on
older devices) rasterises the layers on a raster thread to a surface. Flutter owns every pixel,
which gives it identical rendering everywhere, shaders, `CustomPaint`, and free-form transforms
and clips on anything.

Here: there is no paint. The last thing Rust produces is a list of instructions:

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

Six kinds of operation are the whole interface between the engine and a platform. A renderer
(`rust/src/android/`, `rust/src/ios/`) turns each into calls on real views: construct a
`TextView`, call `setText`, add it to a parent, set its frame. Backgrounds, borders, corner radii
and shadows are native drawables and layers configured by those calls. Drawing, compositing,
scrolling physics and ripple effects are the platform's.

What you give up is everything that needs pixel access: no `CustomPaint`, no `Canvas`, no shaders,
no `Transform` on arbitrary subtrees, no `ClipPath`. What you get is controls that are the
platform's own, with its accessibility, text selection, autofill and input methods, without the
framework reimplementing them.

In Flutter terms: the whole application is made of platform views, and there is no Flutter
surface between them.

### Threads

| | Flutter | Here |
|---|---|---|
| Where your code runs | UI thread (the root isolate) | the platform main thread |
| Rasterising | raster thread | the platform's render pipeline |
| Platform APIs | platform thread, reached by channels | the same main thread, called directly |
| Heavy work | another isolate (no shared memory) | another OS thread (shared memory, `Send` checked by the compiler) |

Flutter has historically run Dart on its own UI thread, separate from the platform's main thread;
recent versions merge the two on mobile. Either way a frame's rasterisation happens elsewhere.
Here there is exactly one thread for everything visible. `update`, `view`, layout, the diff and
the view mutations all run on the main thread inside the native event's call stack.

The consequence is the usual native one: **if `update` or `view` is slow, the app stutters**,
scrolling included. There is no second thread keeping animations smooth while your code runs.

### Scheduling

Flutter's `SchedulerBinding` runs everything from vsync: animation tickers first, then build,
layout, paint. Several `setState` calls before a frame collapse into one build.

Here there is no scheduler and no frame concept in the engine. One native event is one `update`
and one render, done at once. The only batching is for background results: a burst of messages
from worker threads causes a single wake-up, and `Session::drain` applies them all before
rendering once.

One place where the platforms differ, and which has no Flutter equivalent, is re-entrancy. Setting
a property on a native control can make it fire its listener synchronously, which would call back
into Rust in the middle of a render. Android discards such an event (the entry point finds the
state locked and treats the event as an echo of Rust's own write). iOS avoids the situation by
delivering every widget event on the next turn of the main queue. More:
[How Rust talks to the platforms](../platforms/01-how-rust-talks-to-the-platforms.md).

## 8. Input

### Gestures

Flutter does its own hit testing on the render tree and resolves competing gestures in the
gesture arena. Here there is neither. A `Button` is a native button with a native click listener;
a `Scroll` is a native scroll view with native physics. What reaches Rust is already a finished
event:

```text
  native listener  ->  entry point(session handle, host id, node id, payload)
                   ->  Session::node_event
                   ->  handler looked up in the latest tree
                   ->  message  ->  update
```

There is no `GestureDetector`, no `Listener`, no raw pointer events. A widget reports the events
its native control supports (click, text change, toggle, slider value, selection, scroll, focus)
and nothing else. What `InkWell` or a `GestureDetector` with `onTap` does for a card or a list
row is `Container::new().on_tap(msg)` here: a box with a native view, a native click listener on
Android and a tap recognizer on iOS.

### Controls follow state

In Flutter a `TextField` has a `TextEditingController` that owns the text; you read from it. A
`Switch` is drawn from the `value` you pass, so if you ignore `onChanged` it does not move.

Here the value lives in your state, as with the Flutter `Switch`. But a native control changes
itself *first* and tells you afterwards: by the time `update` sees `Msg::Name(text)`, the
`EditText` already shows the new text. If `update` accepts the value, the description matches the
control and nothing needs doing. If `update` rejects it (say it strips digits), the description is
unchanged, so a plain diff would see no difference and leave the control showing a value the
state does not hold.

The engine closes that gap. After any value-changing event that has a handler, the session marks
the node:

```rust,ignore
// rust/src/host/session.rs, in Session::node_event
if changes_value && message.is_some() {
    state.runtime_mut().reapply(id);
}
```

and the next diff emits an `Update` for that node even though nothing in the description changed.
The control snaps back to what the state says. This is the rule "control follows state", and it
makes every input behave like Flutter's `Switch`: what is on screen is what `view` returned.

More: [The session and events](../internals/08-the-session-and-events.md),
[One tap, end to end](../internals/10-one-tap-end-to-end.md).

## 9. Lists, navigation, chrome, animation, theming, accessibility

### Lists

Flutter's `ListView.builder` is a sliver: during layout it asks the builder for exactly the
children that fit the viewport, building widgets lazily inside the layout pass.

`List` here has two modes (`rust/src/widgets/list.rs`):

| Mode | How it works | Nearest Flutter idea |
|---|---|---|
| Windowed (default) | A native scroll view containing a top spacer, the visible rows plus a buffer, and a bottom spacer. Scroll offsets are reported to Rust; crossing a row boundary re-runs the render with a different window. | `ListView.builder` with a cache extent |
| `.recycled()` | A native `RecyclerView` / `UICollectionView`. The platform asks for a row when a cell is about to appear, and Rust binds the row's description into the recycled cell's views. | No Flutter equivalent; this is the platform's own cell reuse |

In both, you pass the data (`Arc<Vec<T>>`), a key function and a row function, and only the rows
near the viewport are built. In the windowed mode a scroll that changes the window goes through
the full path in [section 3](#3-one-update-step-by-step), `view()` included, which is another
reason to keep `view` cheap.

### Navigation

Flutter's `Navigator` keeps routes as entries in an `Overlay`, all inside one widget tree and one
render tree; transitions are Flutter animations.

Here navigation is declarative and close to the "pages" API of Navigator 2.0: `App::navigation()`
returns the current stack as a list of routes with keys, and the session diffs it against the
stack the platform is showing, turning the difference into native pushes and pops. But each
screen is a **separate native root** with its own `Runtime` and its own node tree: a fragment on
Android, a view controller in a `UINavigationController` on iOS. Transitions, the back gesture and
the back button are the platform's. When the user swipes back on iOS, the platform has already
popped; the session sends your back message and pushes the screen again if your state still lists
it.

### Chrome

A Flutter `Scaffold` is a widget that lays out other Flutter widgets (`AppBar`,
`BottomNavigationBar`, `FloatingActionButton`). Here `Scaffold` is a *description of native
chrome*: the app bar is a platform toolbar or navigation bar, dialogs are system alerts, the
bottom sheet is the platform's sheet. The session keeps a copy of what it last sent and forwards
only changes. You cannot put arbitrary widgets in an app bar the way you can in Flutter; you fill
the slots the platform offers. More:
[App, navigation, scaffold, presentation](../internals/09-app-navigation-scaffold-presentation.md).

### Animation

| Flutter | Here |
|---|---|
| `AnimationController`, `Ticker`, `Tween`, `AnimatedBuilder` | none |
| `AnimatedContainer`, `AnimatedOpacity` (implicit) | `.animate(ms)` / `.animation(…)`: when a node's frame or opacity changes, the platform animates to the new value |
| `AnimatedSwitcher`, route-style enter/exit | `.animate_enter(…)`, `.animate_exit(…)` |
| `Curves.*`, `SpringSimulation` | `AnimationCurve`: linear, ease variants, cubic Bézier, spring |
| `Hero` | none |

Only implicit animations exist. You change state, the description changes, and a node that asked
for it animates from its old frame to its new one, run by the platform's animators. There is no
value you can drive frame by frame from your code.

### Theming

No `Theme`, no `ThemeData`, no `MediaQuery`. A colour is a pair:

```rust,ignore
const COLOR_MUTED: Color = Color::adaptive(0xFF66_6666, 0xFFA0_A4A8);
```

and the engine picks the light or dark value while building the tree, before the diff. Switching
to dark mode is therefore an ordinary render in which only nodes whose resolved colour changed get
an `Update`. Shared styling is done with ordinary Rust: constants and helper functions.

### Accessibility

Flutter builds a semantics tree from its render tree and exposes that to TalkBack and VoiceOver,
because its pixels mean nothing to them. Here the views are native, so the platform's
accessibility already sees buttons as buttons and text as text. The framework adds one thing,
`.accessibility_label(…)`. There is no equivalent of `Semantics`, roles, custom actions or
`MergeSemantics`.

## 10. Engine, interop, build and tooling

### What "the engine" is

| | Flutter | Here |
|---|---|---|
| Runtime shipped with the app | the Flutter engine (C++): Dart VM, renderer, text layout | none; the framework is a Rust library compiled into your app |
| Host-side shell | the embedder (`FlutterActivity`, `FlutterViewController`) | `KrabikActivity`, `RustRootView` (Kotlin); `KrabikHostController`, `RustHostView` (Swift), under `shared/` |
| App binary | AOT snapshot of your Dart code plus the engine library | one `.so` (Android) or static library (iOS) containing your crate and the framework |
| Memory management | garbage collector | none; Rust ownership |

### Talking to the platform

In Flutter, calling a platform API means a platform channel (asynchronous, values serialised by a
codec) or `dart:ffi`, with a plugin wrapping it.

Here there is no channel because there are not two worlds. Rust calls Android through JNI, with
method ids looked up once and cached (`rust/src/android/handles.rs`), and calls UIKit directly
through the Objective-C runtime with the `objc2` crates. In the other direction, Kotlin calls
`external fun`s and Swift calls C functions declared in `shared/ios/KrabikIOS.h`. Every call is an
ordinary synchronous function call carrying numbers and strings: a session handle, a host id, a
node id, a payload.

The price is that this boundary is written by hand. JNI method signatures are strings checked at
run time, and the C header is not generated from the Rust side. A few shared constants are
compared by a test. More:
[How Rust talks to the platforms](../platforms/01-how-rust-talks-to-the-platforms.md).

### Build

`flutter build` hides the native toolchains. Here you use them directly: Gradle runs `cargo build`
for each Android ABI through a shared script (`shared/android/rust.gradle.kts`), and Xcode runs a
pre-build script (`shared/build-rust-ios.sh`). There is no CLI that wraps both. More:
[Building and CI](../platforms/06-building-and-ci.md).

### Tooling

| Flutter | Here |
|---|---|
| Hot reload, hot restart | hot reload of `view` and `update` code on the emulator and the simulator ([how](../design/hot-reload.md)); anything else is a native rebuild and reinstall |
| DevTools: widget inspector, performance overlay | none; the platform's own tools (Layout Inspector, Xcode view debugger) show the real native views |
| `debugPrint`, `flutter logs` | `adb logcat -s krabik`; the Xcode console |
| Widget tests (`pumpWidget`, `find`, `tap`) | Rust tests that build a session with a fake platform, send events and assert on the operations and messages |
| Golden tests | none |
| Integration tests | Espresso (Android) and XCUITest (iOS) suites |
| pub.dev packages | none; you write Rust, and add widgets to the framework itself ([recipe](../recipes/add-a-widget.md)) |

The widget-test equivalent is worth a second look, because it is where the design pays off. The
engine never touches a view, only ids, so the whole pipeline in
[section 3](#3-one-update-step-by-step) runs in `cargo test` on a laptop, and a test can assert "this message produces exactly one
`Update` for this node". More: [Tutorial 9: Testing](../tutorial/09-testing.md),
[Write tests](../recipes/write-tests.md).

## 11. What you gain, what you give up

**Gain, compared with Flutter**

- Real platform controls: system text editing, selection, autofill, accessibility, navigation
  transitions and sheets, with no reimplementation to keep up to date.
- No engine in the binary and no garbage collector.
- One state and one `update`, checked by the Rust compiler: a message with no `match` arm does not
  compile.
- A small engine you can read. The core pipeline is a handful of files under `rust/src/engine/`.
- Direct, synchronous access to platform APIs from the same language as your app.

**Give up**

- Hot reload as complete as Flutter's. Here it covers `view` and `update` code on the emulator
  and the simulator; a change to the shape of the state, or a device, means a reinstall.
- Pixel-identical UI across platforms, custom painting, shaders, free-form transforms and clips.
- Explicit animations, `Hero`, gesture recognisers.
- Local widget state, `InheritedWidget`, `GlobalKey`, lifecycle callbacks.
- A layout system you can extend; `Stack`-style overlap inside content.
- Partial rebuilds. You keep `view` cheap yourself.
- A second thread for smoothness: slow `update` or `view` blocks the UI.
- The ecosystem, the tooling and the maturity.

## Rough edges

- Statements about Flutter in this chapter describe its architecture in general terms. Thread
  layout and the default renderer have changed between Flutter releases; check the Flutter
  documentation for the version you use.
- The remark in [section 4](#4-identity-and-keys) that overlapping siblings stack in creation
  order follows from the renderers only ever appending children (`addView`, `addSubview`) and from
  the absence of a move operation. It is read from the code, not covered by a test.
- Some older chapters still list five operations; `ScrollTo` is the sixth and was added later.
- Recycled list rows are identified by list and item index (`row_root` in
  `rust/src/engine/tree.rs`), not by your key function, so the key advice in section 4 applies to
  windowed lists and ordinary children.

## Where this lives

- `rust/src/app.rs`: `Component` and `App`, the two traits an application implements.
- `rust/src/element.rs`: `Element<M>`, `RawElement`, `Element::map`.
- `rust/src/message.rs`: `Handler<A>` and why handlers compare by presence.
- `rust/src/engine/tree.rs`: `UiNode`, `node_id`, list windowing, colour resolution.
- `rust/src/engine/layout.rs`, `rust/src/engine/measure.rs`: Taffy and the `Measure` trait.
- `rust/src/engine/render.rs`: `Op`, `diff_reapplying`, `diff_layout`.
- `rust/src/engine/runtime.rs`: `Runtime::update`, the pipeline in one function.
- `rust/src/host/session.rs`: `render_all`, `node_event`, `drain`.
- `rust/src/command.rs`: `Command`, `Subscription`, the mailbox.
- `rust/src/android/`, `rust/src/ios/`, `shared/`: the renderers and the shells.

Next: [2. Compared to React Native](02-compared-to-react-native.md).
