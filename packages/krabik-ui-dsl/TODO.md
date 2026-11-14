# Krabik UI DSL — Roadmap & Status

How the framework works is described in the [documentation](docs/README.md). This file tracks
status.

---

## Done ✅

> The entries below are a log, oldest first. Sections 1 to 5 were written before the
> restructure (section 6): file names such as `rust/src/android.rs`, string actions and
> `Event::…` in them refer to code that has since been reorganized. The commit-by-commit account
> of sections 6 and 7 is in [docs/design/history.md](docs/design/history.md).

### 1. Decouple Application Code from Platform Renderer ✅

Application logic is fully decoupled from the renderer.

- `rust/src/app.rs` defines the `App` trait:
  ```rust
  pub trait App: Send + 'static {
      fn view(&self) -> Element;
      fn handle(&mut self, event: Event<'_>) -> bool;
  }
  ```
- `AndroidState` uses `Box<dyn App>`.
- `set_app_factory()` lets users register their own app.
- Demo gallery lives in [`rust/src/gallery.rs`](rust/src/gallery.rs), not hardcoded in `android.rs`.

### 2. Implement the iOS Renderer ✅

The iOS app drives UIKit directly from Rust through a raw C ABI, mirroring the
Android renderer's JNI surface. The Swift side is a single `RustRootView` that
forwards its viewport and owns no UI logic.

| Milestone | Result |
|---|---|
| **M0 — build spike** | The staticlib builds for `aarch64-apple-ios-sim` and the app renders in the simulator. |
| **M1 — Rust FFI skeleton** | [`ios.rs`](rust/src/ios.rs) behind `#[cfg(target_os = "ios")]`: handle-keyed `Mutex<HashMap<u64, IosState>>` with `create / render / destroy` and typed event entry points mirroring Android. |
| **M2 — measurement** | `IosMeasure` measures `NSString` text with `UIFont` attributes, including size/weight/max-lines. |
| **M3 — UIKit widgets** | `UILabel`, `UIButton`, `UISwitch`, `UITextField`, `UIImageView`, `UIScrollView`, `UICollectionView` (recycled lists), plus gradients (`CAGradientLayer`), borders, radii, opacity, elevation, and accessibility labels. |
| **M4 — root view + parity** | [`RustRootView.swift`](ios/Sources/RustRootView.swift) replaces the placeholder; a UI test drives all ten gallery demos (counter, layouts, typography, alignment, scroll, form, media, styling, controls) and passes on the simulator. |
| **M5 — CI** | The `ios` job in [`ci.yml`](../../../.github/workflows/ci.yml) lints and builds the `aarch64-apple-ios-sim` static library. Like the Android job, it was validated locally only so far. |

Implementation notes:

- Uses `objc2` / `objc2-ui-kit` / `objc2-foundation` / `objc2-quartz-core` rather than UniFFI.
- Actions use `UIControl::addAction_forControlEvents` with `RcBlock` handlers.
- Events are deferred to the next main-queue turn; viewport re-renders from
  `layoutSubviews` are coalesced and deferred the same way, so UIKit can never
  re-enter the state mutex while a handler is rendering.
- `UITextField` keyboard/return-key setters go through raw `objc_msgSend`
  because UIKit implements them via message forwarding; the trait getters are
  read the same way (objc2's debug-time method verification rejects them).
- Field updates skip redundant placeholder/keyboard/secure-entry setters, so a
  re-render between keystrokes no longer pokes the text input system.
- Remote URL images download through `NSURLSession`: one download per URL, an
  in-memory cache with in-flight de-duplication, decoded off the main thread
  and applied on the main queue only while the view still expects that URL.
- Recycled lists are a `UICollectionView` whose data source and flow-layout
  delegate are Rust classes. Variable row heights come from the runtime's
  measurement pass; count/height changes schedule a `reloadData` and
  revision-only changes diff the visible cells in place. Both run on the next
  main-queue turn, outside the render lock, because UIKit may query the data
  source synchronously.

### 2b. Standalone example app ✅

[`example/`](example/) documents the consumption pattern: a standalone Rust
crate (`krabik-ui-dsl-example`) that path-depends on the DSL, implements its
own checklist app, and registers it through `set_app_factory` — plus a platform
shell for each renderer:

- **iOS** — an Xcode project whose Swift host calls the C function
  `krabik_ui_dsl_example_install` at startup and hosts the root view.
- **Android** — a Gradle project whose `MainActivity` calls the JNI entry point
  `ExampleBridge.install`, exported by the example `cdylib`. The crate builds
  as `staticlib` + `cdylib`, and the Gradle task cross-compiles it for the
  selected ABIs and copies the `.so` into `jniLibs`.

Both Android shells compile the Kotlin view layer from
[`shared/android/src/main/java`](shared/android/src/main/java) through a shared
Gradle source set, so the widgets and `KrabikActivity` exist once. Each
`MainActivity` only loads its own native library (`krabik_ui_dsl` for the
harness, `krabik_ui_dsl_example` for the example) and installs an app factory
when it has one.

Per-note delete buttons are built with dynamic actions
(`format!("note:delete:{index}")`) and covered by an XCUITest and an Espresso
test. The example crate builds a `cdylib` alongside its `staticlib`, and the
DSL crate exposes `rlib`/`cdylib`/`staticlib` so consumers can link and load it.

Android event entry points try-lock the state map and drop the event when a
render holds it. All event dispatch and `nativeRender` run on the main thread,
so contention can only mean a widget listener fired synchronously during a
render and re-entered the entry point; blocking there would deadlock the UI.

### 2c. Todo example apps ✅

Two standalone todo apps sit beside the checklist example and show how a real
application is built on the DSL:

- [`example-todo/`](example-todo/) (`krabik-ui-dsl-todo`) is the widget
  showcase: a recycled list of checkbox rows with per-row delete, radio-button
  filters (`All` / `Active` / `Done`), a priority slider, a progress indicator,
  a sort switch, a text field with add/submit, and clear-completed.
- [`example-todo-minimal/`](example-todo-minimal/)
  (`krabik-ui-dsl-todo-minimal`) keeps only the core loop: add, toggle, delete
  and clear completed, all in a recycled list.

Each app has its own `rust/` crate, Xcode project and Gradle project, so it can
be built and installed independently of the others:

| App | Android application id | iOS bundle id | Native library |
|---|---|---|---|
| Showcase | `com.example.krabik_ui_dsl_todo` | `com.example.krabik-ui-dsl-todo` | `krabik_ui_dsl_todo` |
| Minimal | `com.example.krabik_ui_dsl_todo_minimal` | `com.example.krabik-ui-dsl-todo-minimal` | `krabik_ui_dsl_todo_minimal` |

Both crates export a C install function (`krabik_ui_dsl_todo_install`,
`krabik_ui_dsl_todo_minimal_install`) and a JNI install entry point
(`TodoBridge.install`, `MinimalTodoBridge.install`). Rust unit tests cover the
state transitions; XCUITest and Espresso suites drive the same add/complete/
filter/slider/clear flow through the native widgets.

The recycled list disables `RecyclerView.ItemAnimator`: every render revision
calls `notifyItemRangeChanged`, and the default animator rebinds holders often
enough to clear the per-row action map and flicker focused fields.

### 2d. Widget showcase ✅

[`example/`](example/) is now a one-page-per-widget catalog instead of only a
checklist. The home page lists every widget, each entry opens a page that
demonstrates it in isolation, and the checklist is preserved as a page.

Navigation is application state: a `Page` enum and `page:*` actions. No
navigation primitive was added to the DSL yet; this keeps the showcase
independent of that future design.

The catalog covers `Text`, `Button`, `TextField`, `Switch`, `Checkbox`,
`RadioButton`, the new `RadioGroup`, continuous and stepped `Slider`, `Progress`,
the new indeterminate `Spinner`, `Image`, `ScrollView`, windowed and recycled
`List`, layouts and styling. `krabik.webp` is bundled in both shells for the
image page.

### 2e. Date picker ✅

`DatePicker` uses platform-independent `DateValue`, `TimeValue` and
`DatePickerValue` types. One widget covers three value kinds:

- **Date** — `DatePicker::new(date)` reports `Event::DateChanged { action, date }`.
- **Time** — `DatePicker::time(time)` reports `Event::TimeChanged { action, time }`.
- **Date and time** — `DatePicker::date_time(date, time)` reports
  `Event::DateTimeChanged { action, date, time }`.

Two presentation modes are available:

- **Compact** — a field; Android opens the native date/time dialog, iOS uses a
  compact `UIDatePicker`.
- **Inline** — an always-visible picker (`DatePicker`/`TimePicker` on Android,
  inline `UIDatePicker` on iOS).

`.range(min, max)` and `.time_range(min, max)` set inclusive bounds. Android
applies them to the native pickers and clamps values; iOS maps dates to
`minimumDate`/`maximumDate` and the runtime clamps every reported value, so a
date-and-time value also respects the time bound on the first and last day.

Dates are converted through calendar components at local noon so daylight
saving cannot shift the selected day; time-only values use the reference date
2000-01-01. The controls demo and the widget showcase exercise every kind.

### 2f. Native presentation and navigation ✅

The runtime now supports multiple native hosts per application session:

- **Session/host core** — one `App` instance, one `RustRootView`/host per main
  screen, navigation screen and modal. Events carry a host id; each host owns
  its runtime, scroll/focus and recycled-row state.
- **Dialog** — `App::dialog` returns a structured `Dialog` (title, message,
  buttons with default/cancel/destructive roles) shown as the system alert:
  `UIAlertController` on iOS, a Material alert dialog on Android. Buttons
  dispatch `Event::Click`.
- **BottomSheet** — `App::sheet` returns a `BottomSheet` with Rust content. The
  platform supplies the surface and sizes the sheet to its content: a Material
  `BottomSheetDialogFragment` on Android, `UISheetPresentationController` on
  iOS. Swiping it away dispatches the sheet's dismiss action.
- **Navigation** — the app implements `navigation()` and `screen(key)`; Android
  mirrors the stack with AndroidX fragments and a toolbar, iOS with
  `UINavigationController`. Rust remains the source of truth for the stack:
  Android does not use the fragment back stack, and a covered screen keeps its
  view, so popping reveals it as the user left it.
- **Back** — Android's system/toolbar back dispatches `Event::Back` through the
  AndroidX `OnBackPressedDispatcher`; the app pops a screen, closes a modal or
  returns to the catalog before the Activity exits. iOS navigation uses the
  native bar's Back item.
- **Transitions** — each `Screen` carries a `ScreenTransition::Slide`, `Fade`
  or `None`. Android uses shared slide/fade fragment animation resources, with
  the entering screen above on push and the leaving screen above on pop; iOS
  keeps the native navigation push for slides and uses a native `CATransition`
  cross-fade otherwise. The Motion section has a Page transitions demo, and
  the Bottom sheet catalog entry is now a page with an Open button.

The widget showcase itself is now a native navigation stack: every widget page
is a screen, Dialog is the system alert and Bottom sheet is a modal host. Android instrumented
tests and iOS XCUITests cover push/pop, system back, dialog dismissal and sheet
dismissal.

### 2g. Tabs ✅

`TabBar` is a native tab selector: Android uses a horizontal custom
`KrabikTabBar` built from weighted `TextView`s, iOS uses
`UISegmentedControl`. Tapping a tab reports
`Event::Selected { action, index }`; the application owns the index and passes
it back with `.selected(index)`.

`Tabs` is a plain composition of a `TabBar` above one application-owned child.
It does not create a native tab controller or per-tab navigation stacks; the
app decides which content to build for the current index. The widget showcase
has a Tabs page covering both the composed container and a standalone bar.

### 2h. Implicit animations ✅

`Style` can opt into an implicit transition with `.animate(duration_ms)`,
`.animation(Animation)`, or `.animation_curve(AnimationCurve)`. Entry and exit
use `.animate_enter(ms)` / `.animate_exit(ms)` with the same curve builders.
Animation metadata travels with layout operations and removals from the
renderer diff; nodes without an animation behave exactly as before.

- **Frames and opacity** — Android moves views with a `ValueAnimator`, iOS
  uses `UIViewPropertyAnimator`.
- **Decoration** — background, gradient stops, border color/width and corner
  radii interpolate. Android also animates elevation; iOS animates the backing
  layer with `CABasicAnimation`.
- **Curves** — linear, ease-in, ease-out, ease-in-out, custom cubic Bezier and
  damped springs. iOS uses native timing parameters; Android maps springs to a
  damped-spring interpolator.
- **Enter/exit** — created nodes fade and scale in; removed nodes animate out
  before the platform detaches them. A recreated node cancels a pending exit.
- **Snapping** — viewport changes, recycled-row rebinds and zero durations
  apply immediately, so startup and rotation do not animate.
- **Hit-testing** — iOS drives frame animations from a `CADisplayLink` that
  updates the model frames on every tick, so touches hit the view where it
  actually is; Android already hit-tests against the live translation.
- **Accessibility** — Android respects the system animator duration scale and
  iOS skips animations when Reduce Motion is enabled.
- **Coverage** — the Motion section has four pages: Frames & opacity,
  Decoration, Enter & exit, and Curves & springs. They exercise spring frames,
  a cubic caption, fill/border/radius/elevation changes, fade/scale
  creation/removal and four timing curves; Rust renderer/frame-batch tests and
  both UI suites cover them.

### 2i. Pager, tabs view, bottom navigation and toolbar ✅

Four container/navigation widgets round out the native set:

- **Pager** — a horizontal, swipeable page container. Android uses a custom
  `KrabikPager` with `OverScroller` and velocity tracking; iOS uses a paging
  `UIScrollView`. Pages are ordinary Rust children laid out side by side.
  Swiping reports `Event::Selected`; programmatic selection scrolls the
  native pager in place.
- **TabsView** — a `TabBar` above a `Pager`; tab taps and page swipes share one
  action and one selected index.
- **BottomNavigation** — host-level chrome configured by
  `Screen::bottom_navigation`, like the toolbar: a Material
  `BottomNavigationView` on Android and a system `UITabBar` on iOS, each at
  its platform height and pinned to the bottom edge. Items use custom `Icon`
  values with optional platform-specific names (`drawable` / `sf`) and a
  shared fallback.
- **Toolbar** — host-level chrome configured by `Screen::toolbar`: subtitle,
  back title, hidden flag, and text actions with optional icons. Android uses
  the framework `Toolbar`; iOS uses the native navigation item. Toolbar
  actions dispatch `Event::Click`.

The showcase has a Containers section with View pager, Tabs view, Bottom
navigation and Toolbar pages, and platform tests cover swipes, tab sync,
destination changes and toolbar actions.

Per-tab navigation stacks and collapsing/large toolbars remain future work.

### 2j. Rendering & UX wins ✅

- **Disabled decoration** — `.enabled(false)` dims the fill, gradient stops,
  border and shadow the DSL paints (alpha × 0.4) while native text and controls
  keep their own disabled appearance. The rule lives in
  [`dim_color()`](rust/src/ui.rs), so Android and iOS agree.
- **Intrinsic sizing** — `Measure` gained `switch_size()` and `image_size()`.
  Android measures a framework switch and reads asset headers with
  `BitmapFactory`; iOS uses `UISwitch.intrinsicContentSize()` and the bundled
  `UIImage`. A fixed width or height derives the other image axis from the
  natural aspect ratio. Remote images keep the explicit-size fallback.
- **Motion polish** — iOS animates shadow opacity/radius/offset and gradient
  start/end points with Core Animation.
- **Exact hit-testing** — iOS frame animations run on a `CADisplayLink` that
  writes the interpolated frame into the model layer every tick. Touches
  therefore hit the view where it actually is, and the old interaction lock is
  gone; a departing view simply stops accepting touches.
- **Recycled-row cross-fade** — `List::row_transition(ms)` captures the previous
  holder content into an overlay and fades it out after the new widgets are in
  place. The holder is never withheld from reuse.

The showcase covers a disabled styled control, an intrinsic-size asset image
and a shuffled recycled list with row transitions; both UI suites assert them.

### 3b. Virtualized lists ✅

[`List`](rust/src/ui.rs) (`List::new(items).item_height(..).buffer(..).key(..).row(..)`)
expands during tree build into a native scroll container whose content is
`[top spacer, visible rows, bottom spacer]`. Only the rows around the viewport
become elements and native views; the spacers give the scroll extent the full
list height, so rows keep their absolute content position and native scrolling
needs no repositioning when the window slides. `List::recycled()` uses a native
recycler (`RecyclerView` on Android, `UICollectionView` on iOS) instead of this
materialization; the rest of the section describes the windowed path.

- Rows are keyed, so their node identifiers stay stable across window changes.
- The runtime owns the scroll offset per list and rebuilds only when the
  visible index range moves; sub-row scrolls just update the stored offset.
- Rows may have different heights. A list without `.item_height()` materializes
  the window from an estimated height, measures the rows it built after layout,
  caches the heights by item key, and rebuilds (at most three passes) until the
  prefix sums and the window agree. The estimate is configuration, not derived
  state, so a scroll clamp cannot feed back into another rebuild.
- A list whose item keys change while keeping its node id (for example after a
  screen swap at the same tree position) discards its cached heights and scroll
  offset instead of applying them to the new items.
- `List::revision(u64)` lets the runtime skip the per-build key pass. When the
  revision, item count, width and estimate match, it reuses the cached prefix
  sums and calls the key builder only for the materialized rows. Measured
  heights patch the cached prefix in place, so the convergence passes stay
  O(window). Without a revision the list stays correct but is fully re-keyed on
  every build, which caps it at hundreds of rows.
- The runtime tracks native focus and pins the focused node's row into the
  window, so scrolling does not tear down a focused field. Clearing focus, or
  losing the node, releases the row. The pinned range stays contiguous, so a
  programmatic jump while a field is focused materializes the rows in between.
- `List::recycled()` swaps the windowed scroll container for a native recycler:
  the adapter asks Rust to build a row into the holder/cell it is about to show
  (`nativeBindRow` on Android, the collection-view data source on iOS), and a
  side map keeps the row's button, switch and text actions working even though
  the row is not part of the main tree. The holder's previous tree is cached and
  diffed on rebind, so a state change updates widgets in place instead of
  recreating the row. Rows may have a fixed height (`.item_height(..)`) or any
  height; without one, Rust measures the row and the holder wraps it.
- An app render bumps a revision stamped into every recycled list; the adapter
  rebinds its attached rows, which is how application state changes reach a row
  the recycler already owns. Unchanged widgets are not touched, so focus and
  text survive the rebind. iOS diffs the visible cells in place for a
  revision-only change, applies count changes as insert/delete batches, and
  invalidates the layout when row heights change.
- Android's `onViewRecycled` and iOS's `didEndDisplayingCell` report the
  holder/cell back to Rust, which drops the cached row tree and its actions; a
  count change uses ranged insert/remove notifications on Android, and a rebind
  of the visible range is a single `notifyItemRangeChanged`.
- Focus inside a recycled row is tracked by item, not by view. A blur caused by
  the row leaving the hierarchy parks that focus instead of clearing it; when
  the row is bound again the field regains focus (and the keyboard). A real
  user blur clears it. Android defers focus reports out of RecyclerView layout
  so a focus change cannot notify the adapter mid-layout.
- Both recyclers apply their updates outside the render lock: batch updates and
  their data-source callbacks can run synchronously, and those callbacks
  acquire the same state lock that produced the update.
- Android reports offsets from `KrabikScrollView` (coalesced to once per frame,
  pixels converted to points in `nativeOnScroll`).
- iOS reports offsets from a `UIScrollViewDelegate` implemented in Rust; the
  latest value is coalesced into one main-queue update, matching the other
  deferred events.
- Gallery demos: 1000 fixed rows (44-point, four-row buffer), 300
  variable-height rows, a 1000-row recycled list with a per-row button and
  bump/shrink controls, a 300-row variable-height recycled list, and a
  200-row recycled form whose focused field survives a rebind and a round trip
  through the recycler.

Covered by Rust tests (window maths, spacer geometry, row-id stability,
rebuild throttling, variable-height convergence, focus pinning, row-root
identity, revision stamping, revision cache reuse and prefix patching) and
instrumented tests that scroll the windowed demo and assert the materialized
row count stays bounded, plus the recycled demos (bounded attached rows,
late-row scrolling, row action dispatch, state changes reaching visible rows,
shrinking the list, variable-height holders, focus retention and real-blur
clearing) on both Android and iOS.

A focused field drops the keyboard while its row is off-screen and regains it
when the row returns; the runtime does not keep an off-screen cell attached.

### 3. Expand Native Widget Set ✅

All core widgets are implemented on Android and iOS.

| Rust component | Android renderer | Platform widget |
|---|---|---|
| `Text` | [`update_text()`](rust/src/android.rs) | `TextView` |
| `Button` | [`configure_button()`](rust/src/android.rs) | `Button` |
| `TextField` | [`update_text_field()`](rust/src/android.rs) | Custom `KrabikTextInput` (EditText) |
| `Switch` | [`update_switch()`](rust/src/android.rs) | Custom `KrabikSwitch` (`android.widget.Switch`) |
| `Checkbox` | [`update_check()`](rust/src/android.rs) | Custom `KrabikCheckBox` (`CheckBox`) |
| `RadioButton` | [`update_check()`](rust/src/android.rs) | Custom `KrabikRadioButton` (`RadioButton`) |
| `RadioGroup` | [`update_radio_group()`](rust/src/android.rs) | Custom `KrabikRadioGroup` (`RadioGroup`) |
| `Slider` | [`update_slider()`](rust/src/android.rs) | Custom `KrabikSeekBar` (`SeekBar`) |
| `ProgressIndicator` | [`update_progress()`](rust/src/android.rs) | `ProgressBar` (horizontal) |
| `ProgressIndicator::indeterminate()` | [`update_spinner()`](rust/src/android.rs) | Circular `ProgressBar` |
| `DatePicker` (date/time/date-time) | [`update_date_picker()`](rust/src/android.rs) | Custom `KrabikDatePicker` (`DatePickerDialog`, `TimePickerDialog`, inline `DatePicker`/`TimePicker`) |
| `TabBar` | [`update_tab_bar()`](rust/src/android.rs) | Custom `KrabikTabBar` (horizontal `LinearLayout`) |
| `Pager` | [`update_pager()`](rust/src/android.rs) | Custom `KrabikPager` (`FrameLayout` + `OverScroller`) |
| `TabsView` | [`TabsView`](rust/src/ui.rs) | `TabBar` + `Pager` composition |
| Host bottom navigation | [`sync_chrome()`](rust/src/android.rs) | Material `BottomNavigationView` |
| Host toolbar | [`sync_chrome()`](rust/src/android.rs) | Framework `Toolbar` with menu/custom action views |
| `Image` | [`update_image()`](rust/src/android.rs) | `ImageView` (assets + URLs) |
| `ScrollView` | [`create_scroll()`](rust/src/android.rs) | Custom scroll container (vertical + horizontal) |

iOS renders the controls as `UIButton` with SF Symbol images (checkbox and
radio), `UISlider`, `UIProgressView`, and `UIActivityIndicatorView` for the
spinner. Checkbox and radio buttons dispatch the same `Toggled` event as a
switch; a slider dispatches `ValueChanged`; a radio group dispatches
`Selected { index }`; a tab bar dispatches the same event from
`UISegmentedControl` on iOS. Standalone radio buttons remain independent
controls, while `RadioGroup` owns mutual exclusion (`UIStackView` on iOS, native
`RadioGroup` on Android).

Sliders accept an optional `step`; the runtime quantizes values centrally, so
both platforms report the same multiples to the application.

Full `Style` system (applied via [`apply_style()`](rust/src/android.rs)):
- Background color & linear gradients
- Corner radius & per-corner radii
- Border (width + color)
- Opacity
- Elevation (shadow)
- Enabled / disabled
- Accessibility label

Additional capabilities:
- Text alignment, max lines and ellipsis
- Multiline, secure and keyboard-configured inputs
- Submit and focus events for inputs
- Aspect-ratio sizing
- Stepped sliders and indeterminate spinners
- Controls demo with switches, checks, a radio group, continuous and stepped sliders, a progress bar and a spinner, covered by `ControlsTest` on Android and `testControls` on iOS

### 5. Type-Safe State Management & Event Dispatch ✅

Events are typed in `rust/src/app.rs`:

```rust
pub enum Event<'a> {
    Click { action: &'a str },
    TextChanged { action: &'a str, value: &'a str },
    Submitted { action: &'a str },
    FocusChanged { action: &'a str, focused: bool },
    Toggled { action: &'a str, checked: bool },
    ValueChanged { action: &'a str, value: f32 },
    Selected { action: &'a str, index: usize },
}
```

Unidirectional data flow: `view(state) → handle(event) → render()` is the loop.
`App::handle()` returns `bool` to signal whether the UI should re-render.

Actions are `Cow<'static, str>` in the props, so builders accept both
`&'static str` and runtime-built `String`s — e.g.
`Button::new("Delete").action(format!("note:delete:{index}"))` for a row in a
list. Events borrow the action for the duration of the `handle` call, so the
trait signature is unchanged and static actions stay allocation-free. The
[`example/`](example/) checklist uses this for per-note delete buttons.

Element keys are `Cow<'static, str>`, so a component can be keyed with a
runtime string (for example an item id in a list) as well as a static one.
Keys must be unique among siblings; a debug build asserts it, because two
siblings with the same key would hash to the same node identifier.

### 6. Restructure: typed messages, shared session, one module per widget ✅

- **Typed messages and components.** `Component { type Msg; update; view }` replaces string
  actions and `App::handle`. Widgets carry the message to send (`on_click`, `on_change`, …),
  `Element::map` composes child components, and `App::back` returns the message for the system
  back action.
- **One list of widgets.** `enum Widget` is shared by the description and the node tree and
  matched exhaustively everywhere, including both platform renderers. `widgets/<name>.rs` holds
  a widget's properties, builder and intrinsic size.
- **One session.** `host/` holds the session and the event resolver shared by Android and iOS
  behind a small `Platform` trait, unit-tested with a fake platform.
- **Renderers split into modules.** `android/` and `ios/` replace two files of 2,600 and 3,600
  lines.
- **Shells exist once.** `shared/android` is an Android library (`dev.krabik.ui`), `shared/ios`
  the Swift host; each app is `android/` + `ios/` + `rust/`. The gallery is the `harness/` app
  and the library ships no application.
- **Android.** Updates reach every widget (RadioGroup, TabBar, Pager, DatePicker and Spinner
  used to ignore changes after creation); rotation resizes instead of recreating; back is
  predictive at the root; the app draws edge to edge.
- **Native chrome.** System alert, content-sized bottom sheet, host-level bottom navigation.

### 7. Follow-ups: cheap renders, macros, dark theme, Scaffold ✅

- **Unchanged renders are cheap.** A rebuilt tree equal to the current one skips layout and
  emits nothing; a recycled row does the same against the tree it showed last.
- **Each widget owns its platform code.** `create` sits next to `update` in every
  `android/widgets` and `ios/widgets` module, with the widget's cached JNI handles; the Kotlin
  host controller is called through cached method ids.
- **Proc macros.** `#[derive(Widget)]` generates a widget's builder, `widgets!` declares the
  `Widget` enum from one table, and `view!` lets an app write a tree declaratively.
- **Dark theme.** `Color` carries a light and a dark value, resolved by the engine. Android
  themes are DayNight and the application survives the activity recreation a theme switch
  causes; iOS repaints on an appearance change.
- **Scaffold.** `App::screen` returns a `Scaffold`: app bar, body, bottom navigation, floating
  action button, navigation drawer and snackbar. `Navigation` is a list of routes.
- **Tests and builds.** A rotation test, appearance tests on both platforms, shared Android test
  helpers, iOS device builds, and CI jobs for the UI suites.

---

## Remaining ❌

### Run the CI workflow for the first time ❌

[`ci.yml`](../../.github/workflows/ci.yml) has jobs for the Rust checks, the Android and iOS
builds, a device-target build, and the Espresso and XCUITest suites on an emulator and a
simulator. None of it has run on a runner: every command was only run locally in the same form.
Push the branch to GitHub and expect to adjust SDK package names, action versions or simulator
names on the first run. The UI suites sleep in places and may need longer timeouts on a slower
machine.

### Run on a physical iPhone ❌

The iOS projects build and link for a device (`aarch64-apple-ios`), but that was only checked
without signing. Put `DEVELOPMENT_TEAM = <team id>` in an app's `ios/Local.xcconfig` and run it.

### 4. Optimize JNI Performance ⚠️

**Class/method ID caching ✅**

Every property update used to resolve its class and method by name
(`FindClass` + `GetMethodID`) before the call, so a single frame paid hundreds
of JNI table lookups. [`android/handles.rs`](rust/src/android/handles.rs) now
resolves every class, method, field and constructor once, in `nativeCreate`
(where the app's class loader is current), and holds global references to the
declaring classes so the IDs stay valid for the process lifetime. The render
paths call through small typed wrappers that keep the previous exception
checks.

Measured on the `Pixel_10` emulator (API 37.1, arm64) with a temporary timer
around `AndroidState::render`, 60 taps of the Counter demo's "Add one" button:

| | median | mean | min |
|---|---|---|---|
| before | 546 µs | 1009 µs | 447 µs |
| cached handles | 396 µs | 770 µs | 339 µs |

**One call per frame update ✅**

A layout update used to cost four crossings per node: `setX`, `setY`, a new
`FrameLayout.LayoutParams` allocation and `setLayoutParams`. `RustRootView`
now exposes `applyFrame(view, x, y, width, height)`; the Kotlin side mutates
the existing params, reassigns them only when the size changed, and updates
the translation. A pure move no longer allocates or schedules a layout pass.

Measured on the `Pixel_10` emulator (API 37.1, arm64) with a temporary timer
around the operation loop, forcing a full re-layout by alternating the
viewport width (about 12 layout operations per frame):

| | median | mean | p90 |
|---|---|---|---|
| four calls per node | 372 µs | 568 µs | 1008 µs |
| `applyFrame` | 348 µs | 428 µs | 706 µs |

**Batched operations ✅**

`RustRootView.applyLayouts(ids, frames)` applies a whole frame of layout
updates with one crossing: two primitive arrays plus a registry of node ids to
views. Rust registers a view when it creates it and unregisters it when it
removes it; Kotlin keeps weak references, so a missed unregister can never keep
a detached view alive. The per-node `applyFrame` call stays for reparenting,
which moves exactly one view.

Applying operations is only ~0.35 ms of a ~2.6 ms full render on the demo
screens, so the end-to-end gain is small there. The batch API is about the
shape of the cost: it no longer grows with the number of nodes, which is what
matters for large lists.


---

## Future backlog (not yet started)

### Rendering & UX

- Continuous focus for recycled rows: today the field regains focus when its
  row is bound again. Keeping the focused holder attached off-screen would need
  a custom Android `LayoutManager`; iOS has no native collection-view
  equivalent and would need a separate container.
- Keep a row's scroll position and focus through a screen swap at the same
  tree position where possible.

### Widgets

- Native tab containers: per-tab navigation stacks and retention, beyond the
  current `TabBar`/`Tabs` composition.
- DatePicker seconds, timezone pickers and non-clamping disabled dates.

### Lists & scaling

- Automatic item fingerprint, so `List::revision` becomes optional for large
  lists; unversioned lists are currently fully re-keyed on every build.
- Incremental or estimated prefix sums (for example a Fenwick tree) if
  tens of thousands of mixed-height rows become a target.
- RecyclerView: payload-based partial row updates instead of a full
  `notifyItemRangeChanged` of the visible range on every render.

### Platform-specific extensions

- Expose platform-only capabilities without leaking them into the shared DSL
  (for example Android ripple/press states, iOS SF Symbols or blur effects).

## Known rough edges

Found while the documentation was written by reading every file. None is a crash; each is a place
where behaviour is surprising or one platform does less than the other. They were read from the
source, not all reproduced on a device.

### Behaviour

- **Native controls can drift from state.** If `update` ignores or reverts a value, the tree is
  unchanged, nothing is sent, and a switch, slider or bottom navigation bar keeps what the user
  did. Host chrome that can be dismissed natively (drawer, snackbar, dialog, sheet) has a
  "remember what the platform shows and re-sync" rule; widgets and the bottom bar do not.
- **Route keys must be unique in a stack.** A screen's host is found by its key, so the same key
  twice makes one pop drop both. Documented on `Route`; not enforced.
- **Nothing pops unless the app implements `App::back`.** The app bar's back arrow goes through
  the same path as the system back action. iOS swipe-back is disabled.
- **Chrome needs navigation.** Without `navigation()`, there is no app bar, floating button,
  drawer or snackbar; a single-screen app needs a one-route stack to get them.
- **Style on an undecorated container can be dropped.** A plain `Column`, `Row` or `Spacer` has
  no native view, so `enabled`, `accessibility_label` and animations set only on it reach
  nothing, and `enabled(false)` is not inherited by children.
- **An identical snackbar is not shown again**, because descriptions are compared. Put something
  that changes in the message.
- **A windowed list measures against the host's viewport**, not its own frame, so a list filling
  half the screen materializes about twice the rows it needs.
- **`reset_scroll` only clears Rust's stored offset**; the native scroll view is not told.

### One platform does less

- iOS: `AppBar::back_title` has no visible effect (the back button is a custom chevron); the
  date picker ignores text size, colour and weight; the tab bar ignores size and weight; only
  the largest of four corner radii is used; a sheet's height is fixed when it is presented; the
  alert has no `cancelable` option.
- Android: the alert shows at most three buttons and drops a second non-cancel, non-first one
  beyond the neutral slot silently.

### Safety nets that are missing

- An exhaustive `match` can still be satisfied by adding a variant to a do-nothing arm. On iOS a
  widget's `create` calls its update function only by convention; Android funnels both through
  `configure_view`.
- `#[derive(Widget)]` recognises `Color`, `Handler`, `Option`, `Vec` and `String` by the name as
  written. `crate::style::Color` or `Option<Color>` would be skipped for dark mode.
- Chrome event codes are kept in step by hand in four places (Rust, Kotlin, Swift, the C header),
  and a new `Chrome` field that one platform's controller never sends still compiles.
- A recycled list is only a recycler on Android and iOS, so no host test covers row binding.
- One wrong JNI name or signature makes `nativeCreate` fail and nothing renders; the Rust-side
  error is printed with `eprintln!`, which may not reach logcat.
- Duplicate sibling keys are only asserted in debug builds.

### Build

- `rust.gradle.kts` does not list `rust/macros/src` among its inputs, so a change only to the
  macro crate may not rebuild the Android library.
- Debug and release Rust builds write to the same `app/src/main/jniLibs`, and the default ABI is
  the host's only: pass `-PkrabikAbis=...` for a release.
- `jni` is an unconditional dependency, so it is compiled for iOS and the host as well.
- The slide and fade durations in `res/anim/krabik_*.xml` must match `TRANSITION_MS` in
  `KrabikHostController` by hand.
- No R8 keep rules: classes and methods are looked up by name, which only matters if an app turns
  minification on.

## Nice-to-haves

- Let a view read the current appearance (light or dark), for cases adaptive colours do not
  cover, such as choosing a different image.
- A drawer menu button on Android and iOS only appears on a root screen; a pushed screen keeps
  its back button and opens the drawer by a swipe (Android) or from the app's own button.
- Creating a widget's native view is still one exhaustive `match` per platform, in
  `android/widgets/mod.rs` and `ios/widgets/mod.rs`.
- A screen's description is rebuilt after every message. Unchanged output is cheap to apply, but
  building it still allocates; very large non-list screens would benefit from memoising subtrees.
- `List::revision` is manual. Losing the cached prefix by accident is safe but slow; a debug
  warning when a very large list has no revision would catch it.
- Benchmark fixtures for large lists (10k+ rows) so prefix/key regressions show up in CI rather
  than in a demo.
- Document the recycled-row focus rules (restore on rebind, real blur clears) in the guide, not
  only the gallery demo and the API docs.
