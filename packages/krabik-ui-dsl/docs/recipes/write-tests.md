# Write tests

This recipe shows how to test at each level of the framework, with the helpers that already exist
for each. It goes from the cheapest test (a function call on your app's state) to the most
expensive (driving a simulator). After it you will know which level a given bug belongs to, how
to write the test there, and how to run one test instead of a whole suite.

Background: what the engine, the session and the platform halves are is in
[the engine chapters](../internals/README.md) and [the platform chapters](../platforms/README.md).

## The levels

```text
 level                 what runs                         where it lives
 -----                 ---------                         --------------
 1 app state           your update(), your view()        <app>/rust/src/*.rs
 2 engine              tree -> layout -> diff -> ops      rust/src/engine/*.rs
 3 session             app + hosts + fake controller      rust/src/host/session.rs
 4 macros              token stream -> token stream       rust/macros/src/*.rs
 ------------------------ the line a host `cargo test` cannot cross ----------
 5 Android UI          real views on an emulator          */android/app/src/androidTest
 6 iOS UI              real views on a simulator          */ios/UITests
```

Levels 1 to 4 run on your development machine in seconds. Nothing under `rust/src/android` or
`rust/src/ios` is compiled there: `rust/src/lib.rs` puts both modules behind
`#[cfg(target_os = ...)]`. So everything the platform code does (creating views, applying
properties, delivering events) is covered only by levels 5 and 6.

Pick the lowest level that can see the bug:

| The question | Level |
|---|---|
| Does this message change the state correctly? | 1 |
| Does `view` return the right widgets for this state? | 1 |
| Does a state change produce the right ops, or none? | 2 |
| Is this node where I expect, at the size I expect? | 2 |
| Does the widget report this event as that message? | 2 |
| Is the native chrome told the right thing, in the right order? | 3 |
| Does the macro expand to the builder calls I expect? | 4 |
| Does the native view show it and react to a touch? | 5 and 6 |

## Level 1: your app is plain Rust

An application is a struct with `update` and `view`. Neither needs a device. The example's tests
create the app and send it messages:

```rust,ignore
// example/rust/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;
    use krabik_ui_dsl::widgets::Widget;

    fn app() -> ShowcaseApp {
        ShowcaseApp::new()
    }

    #[test]
    fn page_actions_push_and_back_pops() {
        let mut app = app();
        app.update(Msg::Open("radio-group"));
        assert_eq!(app.stack, vec!["home", "radio-group"]);
        assert!(app.screen("radio-group").is_some());
        app.update(Msg::Open("radio-group"));
        app.update(app.back().expect("back is handled"));
        assert_eq!(app.stack, vec!["home"]);
        // ...
    }
```

What you can call directly, because they are ordinary trait methods on your own type:

- `app.update(msg)` from `Component`.
- `app.view()` from `Component`, which returns an `Element<Msg>`.
- `app.navigation()`, `app.screen(key)`, `app.dialog()`, `app.sheet()`, `app.back()` from `App`.
  `screen` returns the typed `Scaffold<Msg>`, so chrome is testable as data:

```rust,ignore
// example/rust/src/lib.rs
    #[test]
    fn the_bottom_navigation_screen_declares_its_bar() {
        let mut app = app();
        app.update(Msg::Open("bottom-navigation"));
        // ...
        let screen = app.screen("bottom-navigation").expect("screen");
        let bar = screen.bottom_navigation.as_ref().expect("bar");
        assert_eq!(bar.items.len(), 3);
        assert!(bar.on_select.is_set());
    }
```

`Handler::is_set()` is public, so a test can check that a button, a bar or a sheet reports an
event at all. *Which* message it sends cannot be read from an app crate: `Handler::message` is
`pub(crate)`. To test the wiring from outside the framework, send the message yourself with
`app.update(...)` and assert on the state, as the tests above do. Inside the framework crate you
can go further; see level 2.

To look inside what `view` returned, build the tree and inspect it. This block is complete and
uses only public paths:

```rust
use krabik_ui_dsl::tree::{self, NodeKind};
use krabik_ui_dsl::ui::{Button, Column, Element, Text};
use krabik_ui_dsl::widgets::Widget;

fn view(count: u32) -> Element {
    Column::new()
        .child(Text::new(format!("Count: {count}")))
        .child(Button::new("+1").on_click(()))
        .into()
}

fn texts(element: &Element) -> Vec<String> {
    let root = tree::build(element);
    tree::preorder(&root)
        .into_iter()
        .filter_map(|node| match &node.kind {
            NodeKind::Widget(Widget::Text(props)) => Some(props.content.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_view_shows_the_count() {
    assert_eq!(texts(&view(3)), ["Count: 3"]);
}
```

`tree::build` takes a `&RawElement`; an `&Element<M>` works because `Element` dereferences to its
untyped description (`impl Deref for Element` in `rust/src/element.rs`). `tree::preorder` returns
every node, parents before children.

Keep logic out of `view` and you rarely need this. The minimal todo example tests methods such as
`add_todo` and `summary` directly (`example-todo-minimal/rust/src/lib.rs`).

Run:

```sh
cargo test --manifest-path example/rust/Cargo.toml
cargo test --manifest-path example/rust/Cargo.toml page_actions      # tests whose name contains this
```

## Level 2: the engine

The engine turns an element into a tree, lays it out, and diffs it against the previous tree into
a list of `Op`s. All of it is deterministic and platform-free, given one thing: something that
measures text.

### `ApproximateMeasure`

Layout must ask how large a text is, and on a device that question goes to a native label. On the
host, `ApproximateMeasure` in `rust/src/engine/measure.rs` answers it with arithmetic. Its doc
comment gives the formula: "an average glyph width of `0.55 * font size` and a line height of
`1.25 * font size`". With the default size of 16, `"Hello"` is `5 * 8.8 = 44` wide and `20` high,
which the module's own test asserts. Use it when you need frames that respond to content.

When a test needs fully predictable numbers, implement `Measure` yourself. The layout tests do:

```rust,ignore
// rust/src/engine/layout.rs, tests
    /// Measures every text as the same fixed box.
    struct FixedMeasure;

    impl Measure for FixedMeasure {
        fn measure(&self, _text: &TextProps, _constraints: Constraints) -> Size {
            Size::new(100.0, 20.0)
        }
    }

    fn layout(element: Element, viewport: Size) -> HashMap<NodeId, Rect> {
        let tree = tree::build(&element);
        let frames = compute(&tree, viewport, &FixedMeasure).frames;
        // ...
        frames
    }
```

Other fakes in the tree for specific questions: `WrappingMeasure` and `WidthMeasure` in the same
module (does text get the width its parent leaves?), `CountingMeasure` in
`rust/src/engine/runtime.rs` (was layout skipped?).

### `Runtime`

`Runtime` (`rust/src/engine/runtime.rs`) is the engine's front door: it keeps the current tree and
frames, and `update(&element, viewport)` returns the ops to apply. A test creates one and feeds it
elements:

```rust,ignore
// rust/src/engine/runtime.rs, tests
    fn viewport() -> Size {
        Size::new(300.0, 600.0)
    }

    fn screen(content: &str) -> Element {
        Column::new()
            .child(Text::new(content))
            .child(Button::new("Click").on_click(()))
            .into()
    }

    #[test]
    fn changing_text_emits_a_single_update() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen("Hello"), viewport());
        let update = runtime.update(&screen("Hello, Rust!"), viewport());

        assert_eq!(update.ops.len(), 1, "{:?}", update.ops);
        assert!(matches!(&update.ops[0], Op::Update(_)));
    }
```

The pattern for almost every engine test is the same three lines: first `update` to establish the
"before", second `update` with the "after", assert on `update.ops`. Always pass the ops to the
assertion message (`"{:?}", update.ops`); `Op` derives `Debug` and the dump is the fastest way to
see what went wrong.

After an update you can also read:

- `runtime.tree()`: the current `UiNode` tree. Find a node by walking `children`, or with
  `tree::find(runtime.tree(), id)`.
- `runtime.frames()`: `&HashMap<NodeId, Rect>`, the frame of every node, in logical points.
- `runtime.content_height()`, `runtime.content_sizes()`, `runtime.scroll_offset(id)`.

Two things `Runtime` does that a bare `tree::build` does not: it resolves adaptive colours for the
current scheme (`set_color_scheme`), and it skips layout when the rebuilt tree equals the current
one. Test colour behaviour through a `Runtime`, as
`a_new_color_scheme_updates_only_the_nodes_whose_colors_change` does.

### `RecordingRenderer`

A renderer is whatever applies ops. `RecordingRenderer` in `rust/src/engine/render.rs` stores
them instead:

```rust,ignore
// rust/src/engine/render.rs
/// Records every batch it receives, for tests and debugging.
#[derive(Clone, Debug, Default)]
pub struct RecordingRenderer {
    /// The batches received so far, oldest first.
    pub batches: Vec<Vec<Op>>,
}
```

Use it with `Runtime::render` when a test spans several renders and you want the history:

```rust,ignore
// rust/src/engine/runtime.rs, tests
    #[test]
    fn render_applies_one_batch() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let mut renderer = RecordingRenderer::default();

        runtime.render(&screen("Hello"), viewport(), &mut renderer);
        runtime.render(&screen("Hello"), viewport(), &mut renderer);

        assert_eq!(renderer.batches.len(), 2);
        assert!(!renderer.batches[0].is_empty());
        assert!(renderer.batches[1].is_empty());
    }
```

The same three pieces are public, so an app crate can use them too; there is a complete example
in [Debug a render](debug-a-render.md#step-3-which-ops-were-emitted).

### The diff alone

`render::diff(&old, &new)` compares two trees without a runtime. Start from `UiNode::empty()` for
"first build":

```rust,ignore
// rust/src/engine/render.rs, tests
    #[test]
    fn toggling_a_switch_updates_the_same_widget() {
        let old = tree(
            Column::new()
                .child(Switch::new(false).on_toggle(|_| ()))
                .into(),
        );
        let new = tree(/* the same with Switch::new(true) */);

        let ops = diff(&old, &new);
        assert_eq!(ops.len(), 1);
        match &ops[0] {
            Op::Update(node) => {
                assert_eq!(node.id, old.children[0].children[0].id);
                assert!(
                    matches!(&node.kind, NodeKind::Widget(Widget::Switch(props)) if props.checked)
                );
            }
            other => panic!("expected update, got {other:?}"),
        }
    }
```

Note how a node is addressed: `old.children[0].children[0]`. The tree always starts with a
synthetic root (`ROOT_ID`), whose single child is the element you passed, whose children are its
children.

### Events

Inside the framework crate, a test can ask what a widget reports. The runtime tests have a helper
for it:

```rust,ignore
// rust/src/engine/runtime.rs, tests
    /// What the widget `id` asks to be told about.
    fn actions(runtime: &Runtime, id: NodeId) -> Actions {
        tree::find(runtime.tree(), id)
            .and_then(Actions::of)
            .unwrap_or_default()
    }

    #[test]
    fn a_handler_returns_the_message_it_was_given() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        // The message is an ordinary value; here a string built at runtime.
        let element: Element<String> = Column::new()
            .child(Button::new("Delete").on_click(format!("note:delete:{}", 2)))
            .into();
        runtime.update(&element, viewport());

        let button = runtime.tree().children[0].children[0].id;
        let message = actions(&runtime, button)
            .click
            .message(())
            .expect("the button reports clicks");
        assert_eq!(
            message.downcast_ref::<String>().map(String::as_str),
            Some("note:delete:2")
        );
    }
```

A message is a `Box<dyn Any>`; `downcast_ref::<YourMsg>()` gets it back. For the full path from a
platform event, call `Actions::message(&NodeEvent::...)` instead of reading one handler; that is
where slider values are snapped and picker values clamped.

### One trap: recycled lists are not recycled on the host

`List::recycled()` becomes a native recycler only on a device:

```rust,ignore
// rust/src/engine/tree.rs, in `build_list`
    if props.mode == ListMode::Recycled && cfg!(any(target_os = "android", target_os = "ios")) {
        cx.info.recycled.insert(id, props.clone());
        return UiNode {
            id,
            kind: NodeKind::Recycler(RecyclerProps::new(props.item_count, props.item_height)),
            // ...
        };
```

In a host test the same list is built as a windowed list with real row nodes. So a host test
never sees a `NodeKind::Recycler`, and the whole row-binding path (`bind_row` on Android, the
collection-view data source on iOS) is level 5 and 6 only. The harness app has UI tests dedicated
to it (`harness/android/.../RecyclerListTest.kt`, the recycled sections of
`harness/ios/UITests/GalleryUITests.swift`).

Run:

```sh
cargo test --manifest-path rust/Cargo.toml --workspace          # library + macro crate
cargo test --manifest-path rust/Cargo.toml changing_text        # by name
cargo test --manifest-path rust/Cargo.toml engine::runtime      # by module path
```

## Level 3: session tests with the fake platform

The session is the part that owns the application, creates a host per native root view, and
keeps the native chrome in step with the application. It is generic over a `Platform` trait
(`rust/src/host/platform.rs`), and that is what makes it testable: the `tests` module of
`rust/src/host/session.rs` supplies a platform with no native code at all.

### The pieces

```rust,ignore
// rust/src/host/session.rs, tests
    /// Records what the session asks the platform to do.
    type Log = Rc<RefCell<Vec<String>>>;

    struct Fake;

    impl Platform for Fake {
        type Cx<'a> = ();
        type Error = Infallible;
        type Host = FakeHost;
        type Controller = FakeController;
    }
```

- `Cx` is what a native call needs (the JNI environment, the main-thread marker). The fake needs
  nothing, so every session call in a test takes `&mut ()`.
- `FakeHost` holds a real `Runtime` with `ApproximateMeasure` and counts its renders. Its
  `render` calls `runtime.update(element, Size::new(320.0, 640.0))` and increments `renders`. So
  the tree inside a fake host is real and can be inspected.
- `FakeController` implements every `Controller` method by pushing one line of text into the
  shared `Log`: `"push 1 home instantly"`, `"pop 2 Slide"`, `"root false"`, `"chrome HOME
  fab:Add"`, `"snackbar Deleted"`, `"dialog Sure?"`, `"sheet 3"`, `"back true"`.
- `TestApp` is an application whose state sits in an `Arc<Mutex<State>>`. The test keeps a clone,
  so it can change the state from outside "the way a timer or network reply would", then render.
  `State` has one field per thing a test wants to switch on: `stack`, `dialog`, `sheet`, `fab`,
  `drawer`, `snackbar`, and "sticky" variants that keep something up after the platform reports
  it gone.

### The helpers

```rust,ignore
// rust/src/host/session.rs, tests
    /// A session around a `TestApp`, the app itself and the controller's log.
    fn session(state: State) -> (Session<Fake>, TestApp, Log) { /* ... */ }

    /// Plays the controller's part: creates the host views it was asked for.
    fn attach_pending(session: &mut Session<Fake>) {
        let pending: Vec<u64> = session.pending.keys().copied().collect();
        for host in pending {
            session
                .attach_host(host, |kind| Ok(FakeHost::new(kind)))
                .unwrap();
        }
        session.render_all(&mut ()).unwrap();
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    /// A session one screen deep with the controller's first messages read.
    fn on_home(state: State) -> (Session<Fake>, TestApp, Log) { /* ... */ }
```

`attach_pending` needs a sentence of explanation. When the session pushes a screen it only *asks*
the controller for one and records the host as pending. On a device, the native controller then
creates a root view, which calls back into Rust (`nativeAttachHost`, `krabik_ios_attach_host`),
and only then does the screen have a host to render into. The fake controller creates nothing, so
the test plays that part by calling `attach_pending` after any render that pushed a screen or
presented a sheet. If you forget it, the screen's content is never rendered and tests that look
for a widget in it find no host.

`take` returns the log and empties it. The idiom is: do something, `assert_eq!(take(&log), [...])`,
do the next thing. An empty log is an assertion too, and often the important one.

### A complete example

```rust,ignore
// rust/src/host/session.rs, tests
    #[test]
    fn the_initial_stack_is_pushed_and_given_chrome() {
        let (mut session, _app, log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        // The stack the application starts with is shown, not animated in.
        assert_eq!(
            take(&log),
            ["push 1 home instantly", "root false", "chrome HOME"]
        );

        // Nothing changed, so a second render sends nothing.
        attach_pending(&mut session);
        assert!(take(&log).is_empty());
    }
```

Three kinds of input drive a session test:

1. **State changed from outside**: `app.state().stack.push("c"); session.render_all(&mut ())`.
2. **A widget event**: `session.node_event(&mut (), host, node_id, NodeEvent::Click)`. The test
   `a_click_reaches_the_application_and_rerenders` shows how to find the host id and a node id:
   iterate `session.hosts`, pick the one whose `kind()` is the screen, and search its tree with
   `tree::preorder(host.runtime().tree())`.
3. **A chrome event the platform reports**: `session.chrome_event(&mut (), ChromeEvent::Fab)`,
   `session.dialog_closed(&mut (), None)`, `session.sheet_dismissed(&mut ())`,
   `session.back(&mut ())`, `session.toolbar_action(...)`,
   `session.bottom_navigation_selected(...)`.

And three kinds of assertion: the log, the application's state (`app.state().clicks`), and the
hosts (`session.hosts.len()`, `session.hosts[&host].renders`, a host's runtime tree).

What this level cannot see: whether the real controllers do what the log line says. The fake
never dismisses anything on its own and never fails. See
[Add a chrome slot](add-a-chrome-slot.md#what-the-compiler-will-not-tell-you).

To extend the fake for a new feature you touch `State`, `Msg`, `TestApp::update`,
`TestApp::screen` (or `dialog`/`sheet`), and the `FakeController` method that should log it.

Run:

```sh
cargo test --manifest-path rust/Cargo.toml host::session
cargo test --manifest-path rust/Cargo.toml a_snackbar_the_application_keeps_is_shown_again
```

## Level 4: macro tests over token streams

The two macros are written as functions from `proc_macro2::TokenStream` to
`syn::Result<TokenStream>`, and the `#[proc_macro]` functions in `rust/macros/src/lib.rs` only
convert types and call them. That is deliberate; the crate's module comment says the logic lives
in functions over token streams "so it can be unit-tested without a compiler in the loop".

### `view!`

```rust,ignore
// rust/macros/src/view.rs, tests
    /// The expansion without the outer `Element::from(...)`, with whitespace
    /// normalized so it can be compared with hand-written code.
    fn expanded(input: TokenStream) -> String { /* ... */ }

    fn same(input: TokenStream, expected: TokenStream) {
        assert_eq!(expanded(input), expected.to_string());
    }

    fn error(input: TokenStream) -> String {
        expand(input).expect_err("fails").to_string()
    }

    #[test]
    fn a_node_calls_new_then_its_settings() {
        same(
            quote! { Text("Hello", size: 20, color: BLUE, bold) },
            quote! { Text::new("Hello").size(20).color(BLUE).bold() },
        );
        same(quote! { Spacer }, quote! { Spacer::new() });
    }
```

`same` compares the expansion with the builder code you would write by hand. Both sides go
through `quote!` and `to_string()`, so spacing differences disappear. Error messages are tested
with `error(...)` and `contains(...)`.

One more test sits in the main crate, because it needs the real widgets:
`the_view_macro_builds_the_same_tree_as_the_builders` in `rust/src/ui.rs` writes a view once with
`view!` and once with builders and asserts `from_macro.into_raw() == by_hand.into_raw()`. Add a
case there when you add syntax, so the expansion is known to compile and not only to look right.

### `#[derive(Widget)]`

The derive's output is long, so its tests assert on fragments:

```rust,ignore
// rust/macros/src/widget.rs, tests
    #[test]
    fn generates_the_builder_constructor_and_conversions() {
        let code = expanded(quote! {
            #[widget(name = Badge, about = "A badge.")]
            pub struct BadgeProps {
                #[widget(arg)]
                pub label: String,
                #[widget(default = 3, set)]
                pub count: u32,
            }
        });
        assert!(code.contains("pub struct Badge < M = () >"), "{code}");
        assert!(code.contains("pub fn new (label : impl Into < String >) -> Self"));
        // ...
    }
```

Mind the spacing in the expected strings. `TokenStream::to_string()` puts a space between tokens:
`Badge < M = () >`, `(label : impl Into < String >)`. Write the fragment the way the printer
prints it, and pass `"{code}"` as the assertion message on the first one so a failure shows the
real output to copy from.

Run:

```sh
cargo test --manifest-path rust/Cargo.toml -p krabik-ui-dsl-macros
```

## Level 5: Espresso on Android

The Espresso suites live in each app, for example
`example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/`. They launch the real
activity and drive the real views Rust created.

### Finding views that have no ids

Rust builds the views, so they have no resource ids. Tests find them three ways:

- by text: `onView(withText("Selected: Large"))`;
- by content description, which is the node's `.accessibility_label(...)`:
  `onView(withContentDescription("Stepped volume"))`;
- by walking the hierarchy, when the test needs the view object. `shared/android-test/` holds
  finders shared by every app (each app's `build.gradle.kts` adds the directory to its
  `androidTest` source set):

```kotlin
// shared/android-test/dev/krabik/ui/testing/ViewFinders.kt
/** The first view of type [T] under this one that satisfies [predicate], searching depth first. */
inline fun <reified T : View> View.findFirst(noinline predicate: (T) -> Boolean = { true }): T? =
    findFirst(T::class.java, predicate)

/** The native recycler a `List::recycled()` is rendered with. */
fun findRecycler(view: View): RecyclerView? = view.findFirst<RecyclerView>()

fun findRustRoot(view: View): RustRootView? =
    view.findFirst<RustRootView> { it.isShown && it.registeredViewCount() > 0 }
```

`findRustRoot` filters on `registeredViewCount() > 0` for the reason its comment gives: "A
navigation session also keeps the (hidden) main host alive; only the visible host has registered
widgets." Use the finders inside `scenario.onActivity { activity -> ... }`, starting from
`activity.window.decorView`. `LayoutTest.kt` is the model for asserting on real view sizes and
positions; its header says why that is worth a separate test: "a regression in the layout path (a
dropped translation, a missing layout params update) fails even when the text still looks right."

### The helpers and the pitfalls they encode

`UiHelpers.kt` in the example exists because the obvious Espresso calls do not work on these
views. Each helper documents a pitfall. Read them as a list of things that will bite you.

**`scrollTo` cannot find the target.** From `openPage`:

> The DSL scroll views position their content with translations, so Espresso's own `scrollTo`
> cannot always find the target. Swiping the root view mirrors what a user does and works for
> both the catalog and the widget pages.

So `openPage(label)` swipes until the label is visible, taps it, and waits.

**A full swipe skips things.** From `shortSwipeUp`:

> A full `swipeUp` flings through long catalogs and can skip a short button between two checks.
> Stepping by a fifth of the viewport guarantees a target below the fold becomes visible before
> it can pass above it.

**`swipeDown` starts on the toolbar.** From `contentSwipeDown`:

> Espresso's `swipeDown` starts at the top edge of the window, which is the status bar and
> toolbar, so it never scrolls the page underneath.

**A popped-to screen is where the user left it.** From `swipeToTop`: "A popped-to screen keeps
its scroll position, so the catalog is usually not at the top after going back." That is why
`tapBack()` ends with `swipeToTop()`, and why `openPage` starts with it (through `swipeTo`).

**A tap right after a fling stops the scroll.** The swipes are followed by a `Thread.sleep(150)`
or `Thread.sleep(400)`. A touch on a list that is still moving stops it and is not delivered as a
click.

**Dialogs and sheets are other windows.** From `awaitText`:

> A dialog or sheet lives in its own window, which appears a moment after the tap that asked for
> it; an immediate assertion would still be looking at the activity's window.

Use `awaitText("...")` and `awaitNoText("...")` (polling, 3 seconds by default) for anything that
appears through chrome: snackbars, drawers, alerts, sheets. Plain
`onView(...).check(matches(isDisplayed()))` is fine for content in the screen itself, because the
render is synchronous inside the event that caused it.

**A closing sheet swallows a tap.** From `openSheet` in `ShowcaseTest.kt`:

> A sheet that was just dismissed keeps its window on screen while it animates out, and a tap
> during that time lands on its scrim instead of the button underneath. Tap again once the
> window is gone.

The helper retries the tap up to four times with a short wait.

**Drawer animations are not waited for.** The scaffold test sleeps 600 ms after closing the
drawer before asserting `not(isDisplayed())`.

**Rotation and theme changes take seconds.** `rotationKeepsTheActivityTheStackAndTheState` sleeps
2 seconds after changing the orientation and restores portrait in a `finally` block, so a failure
does not leave the emulator rotated for the next test.

**Animations are on.** The CI job sets `disable-animations: false`. The tests are written to pass
with real animations, which is why the waits exist. Do not rely on them being disabled.

### The shape of a test

```kotlin
// example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/ShowcaseTest.kt
    @Test
    fun scaffoldFloatingButtonSnackbarAndDrawer() {
        ActivityScenario.launch(MainActivity::class.java).use {
            openPage("Scaffold")
            onView(withText("Items: 0")).check(matches(isDisplayed()))

            // The floating action button adds an item and raises a snackbar,
            // whose action takes the item back.
            onView(withText("Add")).perform(click())
            awaitText("Items: 1")
            awaitText("Added item 1")
            onView(withText("Undo")).perform(click())
            awaitText("Items: 0")
            awaitNoText("Added item 1")
            // ...
        }
    }
```

Assert on a text that is derived from application state (`"Items: 1"`) after every action. It
proves the whole loop: native event, Rust message, state change, render, native update.

### Running one class or one method

The Gradle task is `connectedDebugAndroidTest`, run from the app's `android` directory with an
emulator or device attached. The ABI must match it; `shared/android/rust.gradle.kts` defaults to
the architecture of the machine running Gradle, and `-PkrabikAbis=` overrides it.

```sh
cd example/android

# the whole suite of this app
./gradlew :app:connectedDebugAndroidTest -PkrabikAbis=arm64-v8a

# one class
./gradlew :app:connectedDebugAndroidTest -PkrabikAbis=arm64-v8a \
  -Pandroid.testInstrumentationRunnerArguments.class=com.example.krabik_ui_dsl.ShowcaseTest

# one method
./gradlew :app:connectedDebugAndroidTest -PkrabikAbis=arm64-v8a \
  -Pandroid.testInstrumentationRunnerArguments.class=com.example.krabik_ui_dsl.ShowcaseTest#scaffoldFloatingButtonSnackbarAndDrawer
```

The class name uses the test's package, `com.example.krabik_ui_dsl` (the `namespace` in
`example/android/app/build.gradle.kts`), not the `applicationId`, which is
`com.example.krabik_ui_dsl_example`. The Gradle build runs `cargo build` for the app's Rust crate
first, so a Rust change needs no separate step. The task uninstalls the app when it finishes.
Reports are written under `app/build/reports/androidTests`, which is the path the CI job uploads
on failure.

## Level 6: XCUITest on iOS

The suites live in each app's `ios/UITests/` directory, for example
`example/ios/UITests/ShowcaseUITests.swift`. XCUITest runs in a separate process and sees the app
only through accessibility, so everything is found by label and by element type.

### Finding elements

| Rust | XCUITest query |
|---|---|
| `Text::new("Selected: Large")` | `app.staticTexts["Selected: Large"]` |
| `Button::new("Large")` | `app.buttons["Large"]` |
| `Switch` with `.accessibility_label("Hide completed")` | `app.switches["Hide completed"]` |
| `Slider` with `.accessibility_label("Stepped volume")` | `app.sliders["Stepped volume"]` |
| `TextField` with a placeholder | `app.textFields["Add a note"]` |
| `TabBar` | `app.segmentedControls.firstMatch.buttons["Settings"]` |
| bottom navigation | `app.tabBars.firstMatch.buttons["Search"]` |
| a pager with a label | `app.scrollViews["View pager"]` |

The type comes from the UIKit class Rust created, the name from the visible text or the node's
accessibility label. If a query finds nothing, the first suspect is the element type.

### The pitfalls

**Always wait.** Every assertion in these suites is
`XCTAssertTrue(element.waitForExistence(timeout: 5))`, never a bare `.exists` for something that
should appear. Events on iOS are deferred to the next main-queue turn (`schedule` in
`rust/src/ios/mod.rs`), and the test runs in another process. A bare `XCTAssertFalse(x.exists)`
is used only for things that must be gone after an earlier wait already succeeded. For "becomes
absent", `ChecklistUITests` has `waitForAbsence`, built on an `exists == false` predicate
expectation.

**Scroll until hittable, not until it exists.** From the `open` helper: "Scrolls the catalog
until the page button is hittable, then opens it." An element below the fold exists but cannot be
tapped. `open` swipes up to eight times while `!button.isHittable`; `tapButton` swipes once.

**The catalog keeps its scroll position.** `backToCatalog` taps Back, calls `scrollToTop()` (eight
`swipeDown`s) and waits for the title. `open` starts with `scrollToTop()` too.

**Synthetic typing drops keystrokes.** From `ChecklistUITests`:

> Row text comes from synthetic typing, which can drop a keystroke, so the assertions use counts
> and summaries rather than exact strings.

After `typeText`, assert on something derived (`"2 of 3 tasks done · 1 note"`, a row count), not
on the typed string.

**Appearance changes need a pause and a reset.** `testSwitchingToDarkKeepsTheScreenAndItsState`
sets `XCUIDevice.shared.appearance`, restores it with `defer`, and sleeps one second before
taking the screenshot it measures.

**`continueAfterFailure = false`.** Set in `setUpWithError`. One failed wait ends the test,
because every later step depends on the screen the earlier one reached.

**They are slow.** A full run of the example suite takes on the order of ten minutes on a local
machine; the CI job allows 90 minutes for the four apps. Run one method while developing.

### Running one class or one method

The command is the one the CI workflow uses, with `-only-testing` added. The target name comes
from `example/ios/project.yml`: the scheme is `KrabikUiDslExample`, the UI test target is
`KrabikUiDslExampleUITests`.

```sh
cd example/ios

# the whole suite of this app
xcodebuild test -project KrabikUiDslExample.xcodeproj -scheme KrabikUiDslExample \
  -destination "platform=iOS Simulator,name=iPhone 17"

# one class
xcodebuild test -project KrabikUiDslExample.xcodeproj -scheme KrabikUiDslExample \
  -destination "platform=iOS Simulator,name=iPhone 17" \
  -only-testing:KrabikUiDslExampleUITests/ShowcaseUITests

# one method
xcodebuild test -project KrabikUiDslExample.xcodeproj -scheme KrabikUiDslExample \
  -destination "platform=iOS Simulator,name=iPhone 17" \
  -only-testing:KrabikUiDslExampleUITests/ShowcaseUITests/testScaffoldFloatingButtonSnackbarAndDrawer
```

Use the name of a simulator you have (`xcrun simctl list devices available`); the CI job picks
the first iPhone by id. The project builds the Rust library itself in a build phase
(`shared/build-rust-ios.sh`, declared in `project.yml` as "Build Rust library"), so a Rust change
needs no separate step here either. If you add a Swift file under `shared/ios/`, regenerate the
project with `xcodegen generate` in the app's `ios` directory; the header of `project.yml` says
the generated project is committed.

The other apps follow the same naming: `KrabikUiDsl` (harness), `KrabikUiDslTodo`,
`KrabikUiDslTodoMinimal`.

## Everything at once

What the CI workflow (`../../.github/workflows/ci.yml`, relative to the package) runs, as a local
checklist. The workflow has never run on a server, because the remote is not GitHub, so treat it
as documentation of the intended commands.

```sh
cargo fmt   --manifest-path rust/Cargo.toml --all --check
cargo clippy --manifest-path rust/Cargo.toml --workspace --all-targets -- -D warnings
cargo test  --manifest-path rust/Cargo.toml --workspace
cargo clippy --manifest-path rust/Cargo.toml --target aarch64-apple-ios-sim -- -D warnings
# then, for each of harness, example, example-todo, example-todo-minimal:
cargo test  --manifest-path example/rust/Cargo.toml
(cd example/android && ./gradlew :app:connectedDebugAndroidTest -PkrabikAbis=arm64-v8a)
(cd example/ios && xcodebuild test -project KrabikUiDslExample.xcodeproj \
    -scheme KrabikUiDslExample -destination "platform=iOS Simulator,name=iPhone 17")
```

Note the fourth line. It is the only host-side command that compiles `rust/src/ios`. There is no
matching clippy line for the Android target; the Android Rust code is compiled only as part of
the Gradle builds.

## Where this lives

- `example/rust/src/lib.rs`, `example-todo-minimal/rust/src/lib.rs`: app-level tests
- `rust/src/engine/measure.rs`: `Measure`, `ApproximateMeasure`
- `rust/src/engine/runtime.rs`: `Runtime`, and the `tests` module helpers (`actions`, `screen`)
- `rust/src/engine/render.rs`: `Op`, `diff`, `RecordingRenderer`
- `rust/src/engine/layout.rs`: `compute`, and `FixedMeasure` in its tests
- `rust/src/host/session.rs`: `Fake`, `FakeHost`, `FakeController`, `TestApp`, `session`,
  `attach_pending`, `take`, `on_home`
- `rust/macros/src/view.rs`, `rust/macros/src/widget.rs`: `expanded`, `same`, `error`
- `example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/UiHelpers.kt`,
  `ShowcaseTest.kt`, `LayoutTest.kt`
- `shared/android-test/dev/krabik/ui/testing/ViewFinders.kt`
- `example/ios/UITests/ShowcaseUITests.swift`, `ChecklistUITests.swift`
- `harness/android/app/src/androidTest/`, `harness/ios/UITests/GalleryUITests.swift`: lists,
  recycling and focus

Next: [Debug a render](debug-a-render.md).
