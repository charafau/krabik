# The iOS renderer

This chapter walks through `rust/src/ios/` file by file. On iOS, Rust drives UIKit directly: it
creates `UILabel`s and `UIButton`s itself, sets their frames, and receives their events without
Swift in between. The chapter explains the state and its lock, why every event waits one
main-queue turn, how each `Op` becomes UIKit calls, why frame animation uses a display link, and
how a collection-view data source is written in Rust.

A reminder of the terms (details in [`../internals/`](../internals/)): the engine produces `Op`s
(`Create`, `Update`, `Reparent`, `Remove`, `Layout`); a *host* is one native root view plus its
engine `Runtime`; the *session* owns the app and all hosts; the *controller* is the Swift object
that owns the chrome.

## The files

```text
rust/src/ios/
  mod.rs         helpers: colours, fonts, add_action, schedule,
                 with_session, guard
  entry.rs       the C entry points and the state map
  controller.rs  Platform/Host/Controller impls, msg_send! to Swift
  host.rs        IosState, Content, ViewHandle, applying each Op
  events.rs      a UIKit callback becomes a NodeEvent for the session
  motion.rs      frame animation (display link), style animation
  images.rs      bundled and remote images
  style.rs       decoration and accessibility
  measure.rs     IosMeasure: text, switch and image sizes
  rows.rs        recycled lists: the data source, row binding
  widgets/
    mod.rs       update_content, Site, create_widget
    button.rs, text.rs, text_field.rs, scroll.rs, ...   one file per widget
```

The crates involved: `objc2` (messaging, class definition, `MainThreadMarker`), `objc2-ui-kit`,
`objc2-foundation`, `objc2-quartz-core`, `objc2-core-foundation` and `objc2-core-graphics` (typed
bindings), `block2` (Objective-C blocks from Rust closures) and `dispatch2` (the main queue). They
are declared in `rust/Cargo.toml` under `[target.'cfg(target_os = "ios")'.dependencies]`, so they
are not compiled for any other target. Each UIKit class used needs its Cargo feature, which is why
that feature list is long.

## `entry.rs`: the C entry points

The state map is the same idea as on Android:

```rust,ignore
// rust/src/ios/entry.rs
pub(crate) static STATES: OnceLock<Mutex<HashMap<u64, IosSession>>> = OnceLock::new();
pub(crate) static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
```

`IosSession` is `Session<Ios>` (a type alias in `controller.rs`). Unlike Android there is no
wrapper struct: iOS needs no context object and works in points, so there is no density.

### `krabik_ios_create`

```rust,ignore
// rust/src/ios/entry.rs (trimmed)
let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
let mut main = IosState::new(handle, 0, HostKind::Main, &root, mtm);
main.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
let mut session = IosSession::new(crate::app::create_app(), main);
session.set_color_scheme(scheme(dark));
let mut cx = mtm;
guard("create", || infallible(session.render_all(&mut cx)));
states().lock().expect("ios states").insert(handle, session);
handle
```

Two differences from Android's `nativeCreate`. The handle is allocated *before* the host, because
every `IosState` stores its own session handle and host id: the widgets it creates capture those
numbers in their event closures. And the session renders once before it is put into the map,
because Swift passes the initial size along.

### The rest

Every other entry point has the same skeleton: get a `MainThreadMarker` or return; wrap the body
in `guard`; look the session up with `with_session`; do one thing; usually `render_all`.

- `krabik_ios_render` sets one host's viewport and renders everything.
- `krabik_ios_set_color_scheme` exists only on iOS. Here a switch between light and dark does not
  recreate anything: Swift reports it, the session stores the scheme, and the re-render produces
  ordinary `Update` ops for the nodes whose resolved colours changed. Android's equivalent is the
  `night` argument of `nativeCreate` after the activity was recreated.
- `krabik_ios_attach_host` wraps a new host view in an `IosState` and hands it to
  `session.attach_host`.
- `krabik_ios_destroy_host` and `krabik_ios_destroy` drop a host or the whole session.
  `krabik_ios_destroy` removes every view from its superview first.
- The chrome events (`on_back`, `on_toolbar_action`, `on_bottom_navigation_selected`,
  `on_chrome_event`, `on_dialog_action`, `on_modal_dismiss`) call the matching session method.
- `krabik_ios_content_height` returns `runtime.content_height()` for the sheet.

There is no retained-app mechanism. Nothing on iOS destroys the session while the app continues.

## `mod.rs`: helpers, the lock, `schedule` and `guard`

### Small helpers

- `system_font(size, weight)` returns the system font, regular or bold.
- `color(c)` converts a resolved engine `Color` (`0xAARRGGBB`) into a `UIColor`; `cg_color` gives
  the `CGColor` that layers want. The colours are fixed values, not dynamic system colours: the
  engine has already picked the light or the dark value. That is why Swift has to tell Rust about
  appearance changes.
- `parent_host_in`, `remove_subviews`, `inset`: view bookkeeping.
- `cast_view` and `as_any` are unchecked casts between Objective-C types. Each has a one-line
  safety note; they are sound as long as callers pass `UIView` subclasses.
- `add_action(control, events, identifier, mtm, handler)` attaches a Rust closure to a
  `UIControl` as a `UIAction`. This is how every control event is wired.

### State access

```rust,ignore
// rust/src/ios/mod.rs
pub(crate) fn with_session<T>(handle: u64, body: impl FnOnce(&mut IosSession) -> T) -> Option<T> {
    let mut states = states().lock().expect("ios states");
    states.get_mut(&handle).map(body)
}
```

`with_state(handle, host, body)` is the same for one host. Both **block** on the mutex. There is
no try-lock on iOS. So iOS must never reach `with_session` while the same thread already holds
the lock. The next two sections are how that is guaranteed.

### `schedule`

```rust,ignore
// rust/src/ios/mod.rs
/// Defers an event to the next main-queue turn.
///
/// UIKit can deliver another action synchronously while we are rebuilding the
/// tree (for example `EditingDidEnd` when a focused field is removed). Running
/// the handler asynchronously keeps the state lock from being re-entered.
pub(crate) fn schedule(work: impl FnOnce() + Send + 'static) {
    DispatchQueue::main().exec_async(work);
}
```

`exec_async` on the main queue means "run this after the current main-thread work has returned to
the run loop". By then any render has finished and released the lock. Every widget event handler
in `widgets/` wraps its dispatch in `schedule`. See `events.rs` below for the full argument.

### `guard`

```rust,ignore
// rust/src/ios/mod.rs
pub(crate) fn guard(label: &str, body: impl FnOnce()) {
    let result = objc2::exception::catch(std::panic::AssertUnwindSafe(body));
    if let Err(exception) = result {
        // ... format and eprintln!
    }
}
```

UIKit reports programmer errors by raising an Objective-C exception (an out-of-range index in a
collection view batch update, for instance). An exception that unwinds through a Rust
`extern "C"` frame aborts the process. `guard` catches it, logs it and returns. This needs the
`exception` feature of `objc2`. It catches Objective-C exceptions, not Rust panics; a Rust panic
inside an `extern "C"` function still aborts.

`infallible` unwraps a `Result<T, Infallible>`. The session's methods return `Result<_,
P::Error>`, and on iOS the error type has no values.

## `controller.rs`: plugging into the session

`Ios` implements `Platform` with `Cx = MainThreadMarker` and `Error = Infallible`. `IosState`
implements `Host<Ios>` by forwarding to its fields. `IosController` wraps a retained `NSObject`
(the Swift controller) and implements `Controller<Ios>`, one `msg_send!` per method, with
`set_chrome` sending four.

Two `unsafe impl Send` appear here and in `host.rs`: for `Session<Ios>` and for `IosState`. A
value in a `static Mutex` must be `Send`, and UIKit references are not. The justification written
next to them is that all access happens on the main thread, serialised by the mutex. The compiler
cannot check that; the `MainThreadMarker` checks at each entry point are what make it true.

`set_back_enabled` does nothing: iOS has no system back action. Its custom back button always
asks the session.

## `host.rs`: one root view and its operations

### `Content` and `ViewHandle`

```rust,ignore
// rust/src/ios/host.rs (trimmed)
pub(crate) enum Content {
    None,
    Label(Retained<UILabel>),
    Field(Retained<UITextField>),
    Button(Retained<UIButton>),
    Switch(Retained<UISwitch>),
    Check { button: Retained<UIButton>, radio: bool },
    // RadioGroup, DatePicker, TabBar, Pager, Slider, Progress, Spinner, Image ...
    Host(Retained<UIView>),
    Scroll { scroll: Retained<UIScrollView>, host: Retained<UIView>, _delegate: Retained<ScrollDelegate> },
    Recycler { collection: Retained<UICollectionView>, data_source: Retained<RecyclerDataSource> },
}

pub(crate) struct ViewHandle {
    pub(crate) view: Retained<UIView>,
    pub(crate) content: Content,
    pub(crate) gradient: Option<Retained<CAGradientLayer>>,
    pub(crate) parent: Option<NodeId>,
    pub(crate) frame: Option<Rect>,
    pub(crate) padding: EdgeInsets,
    pub(crate) frame_link: Option<Retained<CADisplayLink>>,
}
```

On Android a view handle is two untyped references, and every update goes through a cached
method id. On iOS the bindings are typed, so the handle keeps the concrete class in `Content`.
An update then needs no downcast: `Content::Label(label)` already is a `UILabel`.

`Content` also solves padding. UIKit views have no padding property. A text node with padding is
therefore two views: an outer `UIView` that carries the background, border and frame, and the
`UILabel` inside it, inset by the padding (`Content::inset_handle`). A decorated container is an
outer view plus a `Content::Host` view for its children. A scroll container has the
`UIScrollView` and a content host inside it. `ViewHandle::children_host` returns the right view to
add children to.

`gradient` is the `CAGradientLayer` behind the content when the node has a gradient. `frame_link`
is the display link of a running frame animation. `Content` also owns the delegates and the data
source, because UIKit holds those weakly.

### `IosState`

```rust,ignore
// rust/src/ios/host.rs (trimmed)
pub(crate) struct IosState {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) kind: HostKind,
    pub(crate) runtime: Runtime,
    pub(crate) root: Weak<UIView>,
    pub(crate) views: HashMap<NodeId, ViewHandle>,
    pub(crate) images: ImageLoader,
    // row_actions, row_action_ids, row_states, row_nodes: recycled lists
    pub(crate) pending_exits: HashMap<NodeId, Retained<UIView>>,
    pub(crate) viewport: Size,
}
```

`root` is weak so that Rust does not keep the Swift host view alive (chapter 1). `render` gives
up quietly if the root view is gone. `pending_exits` holds views that are animating out; Android
keeps the equivalent map in Kotlin.

### `render` and `apply_to`

`render` calls `runtime.update(element, viewport)` and applies the ops with `apply_to`. As on
Android, the view map is taken out of `self` so the same function can serve a recycled row's own
view set. There is no frame batch: a UIKit call from Rust is a direct Objective-C message, so
there is no expensive boundary to batch across.

**`Create`**: drop any view still animating out under this node id, build a `ViewHandle` with
`create_handle`, add its view to the parent's children host. If the node declares an enter
transition, set alpha 0 and a 0.96 scale, then animate to the target with `run_animation`.

**`Update`**:

```rust,ignore
// rust/src/ios/host.rs (trimmed)
let snapshot = animation.map(|_| style_snapshot(&handle.view, handle.gradient.as_ref()));
update_content(&mut handle.content, &node, self.handle, self.host, mtm);
apply_style(&handle.view, &node, &mut handle.gradient);
apply_accessibility(&handle.view, &node, mtm);
if let (Some(animation), Some(snapshot)) = (animation, snapshot) {
    animate_style_change(&handle.view, handle.gradient.as_ref(), &node, snapshot, animation, mtm);
}
```

The widget's own properties, then decoration, then accessibility. If the node has a style
animation, the old decoration is captured first and animated to the new one afterwards. Images
and recyclers are updated in two extra blocks after this, because they need `self` (downloads)
or the data source.

**`Reparent`**: remove from the old superview, add to the new children host.

**`Remove`**: cancel a running frame animation, then either remove the view at once or animate
it out. The exit animation's completion block removes the view and then, through `schedule`,
clears the `pending_exits` entry, but only if the entry still refers to this view.

**`Layout`**: `apply_layout`, covered under `motion.rs`.

`Op::Update` also copies the node's padding into `ViewHandle.padding`. A padding change alters
the layout too, so a `Layout` operation follows in the same batch and insets the content with
the new value. (Until this was noticed while writing this chapter, the padding was only set when
the handle was created, and a changed padding kept the old inset.) One difference from Android
remains: an update for a node with no known view is ignored here, where Android recreates the
view.

### `create_handle`

```rust,ignore
// rust/src/ios/host.rs (trimmed)
let mut handle = match &node.kind {
    NodeKind::Widget(widget) => self.create_widget(widget, node, &site),
    NodeKind::Pager(props) => pager::create(props, node, &site),
    NodeKind::Scroll(props) => scroll::create(props, node, &site),
    NodeKind::Recycler(props) => create_recycler(props, node, &site),
    NodeKind::Column | NodeKind::Row => { /* UIView + Content::Host */ }
    NodeKind::Spacer | NodeKind::Root => { /* plain UIView */ }
};
apply_style(&handle.view, node, &mut handle.gradient);
apply_accessibility(&handle.view, node, mtm);
```

Unlike Android's two-step "construct, then configure", an iOS `create` function receives the
properties and returns a configured, wired view.

## `widgets/`: one file per widget

### `Site`

```rust,ignore
// rust/src/ios/widgets/mod.rs
pub(crate) struct Site {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) node: NodeId,
    pub(crate) mtm: MainThreadMarker,
}
```

"Where a view is being created, and where its events go." The three numbers are copied into the
event closures; `mtm` is the proof needed to construct UIKit objects.

### `create`, `update_*`, `wire_*`

The button shows all three parts:

```rust,ignore
// rust/src/ios/widgets/button.rs
pub(crate) fn update_button(button: &UIButton, props: &ButtonProps) {
    let title = NSString::from_str(&props.label);
    button.setTitle_forState(Some(&title), UIControlState::Normal);
    // ... title colour and font
}

pub(crate) fn wire_click(button: &UIButton, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(
        button,
        UIControlEvents::TouchUpInside,
        "krabik.click",
        mtm,
        move |_| schedule(move || dispatch_click(handle, host, id)),
    );
}

pub(crate) fn create(props: &ButtonProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let button = UIButton::buttonWithType(UIButtonType::System, site.mtm);
    update_button(&button, props);
    wire_click(&button, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(button.clone());
    ViewHandle::new(view, Content::Button(button), node)
}
```

- `update_*` applies every property. It is called by `create` and by `update_content`, so
  creation and update cannot drift apart. This is the same guarantee Android gets from
  `configure_view`.
- `wire_*` attaches the event handlers, once, at creation. The handler reads whatever it needs
  from the sender *now* (a field's text, a slider's value) and then schedules the dispatch.
- `create` builds the view, calls both, and returns the `ViewHandle`.

Widgets that are not `UIControl`s use a delegate class instead of `add_action`:
`ScrollDelegate` (`widgets/scroll.rs`) and `PagerDelegate` (`widgets/pager.rs`), both defined
with `objc2::define_class!`.

### `update_content`

```rust,ignore
// rust/src/ios/widgets/mod.rs (trimmed)
match &node.kind {
    NodeKind::Widget(widget) => match (widget, content) {
        (Widget::Text(props), Content::Label(label)) => update_label(label, props),
        (Widget::TextField(props), Content::Field(field)) => update_field(field, props),
        (Widget::Button(props), Content::Button(button)) => update_button(button, props),
        // ... one arm per widget
        (Widget::Image(_), _) => {}
        // The stored content does not belong to this widget.
        (Widget::Text(_) | Widget::TextField(_) | /* every other variant */, _) => {}
    },
    NodeKind::Pager(props) => { /* scroll to the selected page */ }
    NodeKind::Root | NodeKind::Column | NodeKind::Row | NodeKind::Spacer
    | NodeKind::Scroll(_) | NodeKind::Recycler(_) => {}
}
```

The match pairs the widget kind with the stored content. The fall-through arm lists every
variant by name instead of using `_`, so adding a `Widget` variant is a compile error here until
it gets an arm. A mismatch between kind and content means the node changed kind, which the engine
handles by removing and recreating the node, so that arm does nothing.

### Things specific widgets do

- `text_field.rs`: `update_field` compares before it sets, because reconfiguring a text field
  between keystrokes disturbs typing. Keyboard type and return key are read and written with a
  raw `objc_msgSend` (`text_input_traits`, `set_text_input_traits`): UIKit implements those
  `UITextInputTraits` accessors through message forwarding, and `objc2`'s debug-time method
  check rejects the generated bindings. This is the least pleasant code in the directory.
- `check.rs`: iOS has no checkbox or radio button, so both are a custom-type `UIButton` with an
  SF Symbol image (`square`/`checkmark.square.fill`, `circle`/`smallcircle.filled.circle`).
  `radio_group.rs` stacks such buttons in a `UIStackView`.
- `tab_bar.rs`: the in-content tab bar widget is a `UISegmentedControl`. (The host-level bottom
  navigation is a real `UITabBar`, owned by Swift.)
- `scroll.rs`: `scrollViewDidScroll` fires on every frame of a gesture. The delegate stores the
  latest offset in an atomic and schedules at most one dispatch at a time (`ScrollShared`).
- `pager.rs`: a paging `UIScrollView`; the delegate reports the page when dragging or
  deceleration ends.
- `date_picker.rs`: a `UIDatePicker`. Dates are built at local noon so a daylight-saving change
  cannot shift the calendar day.

Some properties that Android applies are not applied on iOS: the date picker ignores text size,
colour and weight, and the segmented control ignores text size and weight. A view can also show
only one corner radius: `apply_style` uses the largest of the four and masks the corners whose
radius is zero.

## `events.rs`: why every event is deferred

`events.rs` is small. `dispatch` turns `(handle, host, node, NodeEvent)` into a call to
`Session::node_event`, inside `guard`. The named functions (`dispatch_click`, `dispatch_text`,
...) build the `NodeEvent`. `dispatch_focus` also asks the host whether the field is still in a
window, to tell a real blur from a row being recycled.

The important part is where these are called from: always inside `schedule`. The reasoning:

```text
 main queue turn N
   Session::node_event            <- lock held
     app.update(msg)
     render_all
       Op::Remove a focused text field
         removeFromSuperview
           UIKit: resignFirstResponder
             UIKit fires EditingDidEnd  --> our UIAction handler runs NOW
                                            schedule(dispatch_focus ...)
                                            returns immediately
   lock released
 main queue turn N+1
   dispatch_focus -> with_session  <- takes the lock, nothing else holds it
```

UIKit fires some control events synchronously in the middle of a mutation that Rust itself is
performing under the lock. Without the deferral, the handler would call `with_session`, block on
the mutex its own thread holds, and hang the app.

Android solves the same problem by trying the lock and dropping the event. That works there
because the events caught are echoes of values Rust wrote a moment ago. On iOS the example above is not
an echo: a field really did lose focus, and Rust needs to hear about it. So iOS delays instead of
dropping. The cost is one main-queue turn of latency on every event, which is well under a frame.

The rule applies beyond widget events. Anything UIKit might call back synchronously while Rust
holds the lock is deferred: the recycler's batch updates (`sync_recycler`), the retry of a row
focus, the cleanup after an exit animation, and on the Swift side the attach of a new host view,
the presentation of a sheet and a render triggered by `layoutSubviews` (chapter 5).

## `motion.rs`: animation

### Frame animation with a display link

When a `Layout` op carries an animation, the view should move from its old frame to the new one
over time. The usual UIKit tool, `UIView.animate`, sets the final frame immediately and animates
only what is drawn. During the animation a tap is hit-tested against the *final* frame, not
against where the view appears. For a button sliding across the screen, taps would miss.

So frame changes are animated by hand:

```rust,ignore
// rust/src/ios/motion.rs (trimmed)
#[unsafe(method(tick:))]
fn tick(&self, link: &CADisplayLink) {
    let ivars = self.ivars();
    let progress = /* (now - start) / duration, clamped to 0..1 */;
    let eased = f64::from(ease_curve(&ivars.curve, progress as f32));
    ivars.frames.apply_rect(lerp_rect(ivars.from, ivars.to, eased));
    if progress >= 1.0 {
        link.invalidate();
    }
}
```

`FrameTicker` is an Objective-C class defined in Rust. A `CADisplayLink` calls its `tick:` once
per screen refresh. Each tick computes the eased progress with `ease_curve` (from
`rust/src/animation.rs`, the reason that module exists) and sets the real frames for that
instant. The model always matches the picture, so hit-testing is exact. The comment at the call
site in `host.rs` says exactly this.

`LayoutFrames` collects every frame one layout op touches, so the same code runs for an immediate
layout and for each tick: the outer view, the inset content or children host, a scroll view's
content host and content size, and the gradient layer. `apply_layout` in `host.rs` builds it,
cancels any link already running for the node, and either starts a `FrameTicker` or calls
`apply_rect` once.

The link retains the ticker, and the ticker owns the views it moves; the `ViewHandle` keeps only
the link, to cancel it.

### Style and enter/exit animation

These do not move frames, so the UIKit and Core Animation tools are fine.

- `run_animation` wraps a `UIViewPropertyAnimator` with timing parameters from
  `timing_parameters`, which maps each `AnimationCurve` to cubic or spring parameters. Enter,
  exit and row transitions use it.
- `animate_style_change` animates a decoration update. It takes a `StyleSnapshot` of the old
  values, lets `apply_style` write the new ones, then animates background and alpha with
  `run_animation` and the layer properties (border, corner radius, gradient colours and
  endpoints, shadow) with `CABasicAnimation`s.
- `timing_function` maps a curve to a `CAMediaTimingFunction`. Core Animation has no spring
  timing function in these bindings, so a spring curve falls back to ease-in-ease-out for layer
  properties. That is a visible inconsistency: a spring animates background as a spring and
  border as an ease.
- `animation_is_enabled` returns false when the user turned on Reduce Motion. All transitions
  then apply immediately.

## `images.rs`

`load_image` sets the content mode and then loads. A bundled asset is loaded synchronously from
the main bundle (`load_asset`). A remote URL goes through `ImageLoader`:

- `cache`: decoded images by URL. It is never evicted; the comment says this is fine for a
  demo-sized number of images, which is an honest limit.
- `in_flight`: URLs being downloaded, so repeated renders do not start a second download.
- `expected`: the URL each image node should currently show.

`start_download` uses `NSURLSession`. Its completion handler runs on a background queue, decodes
the data and schedules `finish_download` on the main queue, which stores the image and sets it on
every view (in the main tree and in bound rows) that still expects that URL. A node that has
moved on to another URL is left alone.

## `style.rs`

`apply_style` writes decoration straight onto the view and its layer: corner radius and masked
corners, clipping, background colour or a `CAGradientLayer`, border, alpha and a shadow for
elevation. Disabled nodes get their DSL-painted colours dimmed, as on Android. A view with both a
corner radius and elevation does not clip, because clipping would cut off its own shadow.

`apply_accessibility` sets the accessibility label, makes the view an accessibility element
(except a radio group and a tab bar, whose items must stay reachable individually), and sets
interaction and `UIControl.enabled`.

## `measure.rs`

`IosMeasure` implements the engine's `Measure` trait. Text is measured with one off-screen
`UILabel` and `sizeThatFits`, with results cached by content, size, weight, width, line limit and
ellipsis. The switch size is the `intrinsicContentSize` of a `UISwitch`, cached. An image's size
is that of the loaded asset.

It carries `unsafe impl Send` and `Sync` for the same reason as the session: the trait object
lives inside state that must be `Send`, and all calls happen on the main thread.

## `rows.rs`: recycled lists

A `List` becomes a `UICollectionView` with a flow layout. As on Android, rows are not part of the
host's main tree; the runtime keeps a row source and rows are built when the collection view asks.

### A data source written in Rust

```rust,ignore
// rust/src/ios/rows.rs (trimmed)
objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikRecyclerDataSource"]
    #[ivars = RecyclerIvars]
    pub(crate) struct RecyclerDataSource;

    unsafe impl UICollectionViewDataSource for RecyclerDataSource {
        #[unsafe(method(collectionView:numberOfItemsInSection:))]
        fn number_of_items_in_section(&self, _cv: &UICollectionView, _section: isize) -> isize { /* count */ }

        #[unsafe(method_id(collectionView:cellForItemAtIndexPath:))]
        fn cell_for_item_at_index_path(&self, collection_view: &UICollectionView, index_path: &NSIndexPath)
            -> Retained<UICollectionViewCell>
        {
            let cell = /* dequeue a reusable cell */;
            let _ = with_state(ivars.handle, ivars.host, |state| {
                state.bind_row(ivars.id, index, &cell, width);
            });
            cell
        }
    }
    // UICollectionViewDelegate: didEndDisplayingCell -> unbind_row
    // UICollectionViewDelegateFlowLayout: sizeForItemAtIndexPath -> row_height
);
```

`define_class!` registers a real Objective-C class at run time. `RecyclerDataSource` is the data
source *and* the delegate of the collection view. Its instance variables (`RecyclerIvars`) are the
session handle, host id, node id, and the list's count, row height and revision. Callbacks
receive `&self`, so the mutable ones are atomics.

Note that `cell_for_item_at_index_path` calls `with_state`, which blocks on the lock. UIKit
therefore must never ask for a cell while Rust holds the lock. That is the reason for the next
section.

### Deferred sync

When a render updates the list's properties, the lock is held. Telling the collection view about
new items right there would make UIKit call the data source synchronously, and the data source
would block on the lock. So `update` only records what changed and schedules one sync:

```rust,ignore
// rust/src/ios/rows.rs (trimmed)
pub(crate) fn update(&self, props: &RecyclerProps) {
    // swap the new count, height and revision into the atomics
    if count_changed { ivars.needs_batch.store(true, Ordering::Release); }
    if height_changed { ivars.needs_layout.store(true, Ordering::Release); }
    if revision_changed { ivars.needs_refresh.store(true, Ordering::Release); }
    if (count_changed || height_changed || revision_changed)
        && !ivars.pending.swap(true, Ordering::AcqRel)
    {
        schedule(move || sync_recycler(handle, host, id));
    }
}
```

`sync_recycler` runs on the next turn, with the lock free. It takes the pending work
(`take_changes`) and applies it:

- a changed count becomes one `performBatchUpdates` that inserts or deletes the tail of index
  paths, without animation;
- a changed row height invalidates the layout;
- a changed revision calls `rebind_visible_rows`, which re-binds every visible cell in place.

There are two counts on purpose. `count` is what the collection view currently believes;
`target_count` is what the app wants. UIKit throws if the data source reports a count that does
not match the batch update being applied, so `count` is only moved to the target at the moment the
batch runs.

Several renders before the sync runs collapse into one sync: the `pending` flag prevents a second
one from being scheduled, and `take_changes` clears it first so an update that arrives during the
batch schedules another.

### Binding a row and the unchanged-row fast path

`bind_row` is the iOS twin of Android's. Rows are keyed by the cell's address (`cell as *const _
as usize`), since UIKit reuses cell objects and there is no holder id.

```rust,ignore
// rust/src/ios/rows.rs (trimmed)
if let Some(state) = self.row_states.get_mut(&key)
    && state.list == list
    && state.index == index
    && state.width == width
    && state.tree == tree
{
    state.tree = tree;
    // ... re-record the row's actions
    return false;
}
```

Same cell, same item, same width, equal tree: nothing to do except take over the new handlers
(the trees compare equal even when their closures differ, because handlers compare by presence).
Since a revision change re-binds every visible row after every message, this path is what makes a
message that does not touch the list cost almost nothing.

Otherwise the row is laid out, diffed against what the cell showed before (or against an empty
tree if the cell is unknown), and the ops are applied into the cell's `contentView` with
`animate = false`. With a row transition, a snapshot view of the old content is laid over the new
and faded out.

`bind_row` returns whether the row's height changed. `rebind_visible_rows` invalidates the layout
only in that case, because the size query lays every visible row out again
(`size_for_item_at_index_path` calls `row_height`, which builds and lays out the row).

Focus works as on Android, with one iOS twist: when a freshly bound row should have focus but its
field is not in a window yet, `try_focus_field` schedules `focus_row_async` to try again on the
next turn.

## Rough edges in this directory

- Correctness depends on UIKit never calling back synchronously while the lock is held. Every
  known case is deferred, but the rule is a convention, not something the types enforce.
- `IosController` retains the Swift controller, and the Swift controller's `deinit` is what
  destroys the session: a cycle (chapter 1).
- The raw `objc_msgSend` calls in `text_field.rs`.
- Padding changes on an existing view, per-corner radii, and some widget styling are not applied
  (noted above).
- The image cache is never evicted.
- The module comment of `mod.rs` calls the Swift side "a thin `UIView`", and the doc comment on
  `IosController` omits the floating action button, drawer and snackbar.

## Where this lives

- `rust/src/ios/entry.rs`: `krabik_ios_create`, `krabik_ios_render`, the chrome entry points.
- `rust/src/ios/mod.rs`: `schedule`, `guard`, `with_session`, `add_action`.
- `rust/src/ios/host.rs`: `IosState::render`, `apply_to`, `create_handle`, `apply_layout`.
- `rust/src/ios/widgets/mod.rs`: `update_content`, `Site`, `create_widget`.
- `rust/src/ios/motion.rs`: `FrameTicker`, `LayoutFrames`, `run_animation`.
- `rust/src/ios/rows.rs`: `RecyclerDataSource`, `sync_recycler`, `bind_row`.

Next: [The iOS shell](05-the-ios-shell.md).
