# Architecture: Rust-to-Android Native UI

## 1. Purpose

This document describes a proof of concept that creates, configures, navigates, and updates native
Android UI entirely from Rust through JNI.

The app is a widget browser: the main screen is a list of buttons, one per Android widget, and each
button opens a demo screen built from the real framework widget. The purpose is to validate direct
Rust ownership of UI construction and interaction logic without introducing Compose, XML layouts, a
cross-platform abstraction layer, or a custom renderer.

## 2. Design principles

- **Rust owns UI logic:** Screen structure, navigation, widget configuration, and interaction state
  live in Rust.
- **Native widgets:** Every control is an instance of an Android framework class, not a
  custom-rendered equivalent.
- **Thin platform layer:** Kotlin/Java is limited to application startup, the JNI boundary, and the
  listener adapter required by the Android framework.
- **Direct JNI:** Use the Rust `jni` crate rather than UniFFI or another abstraction layer.
- **Declarative tree:** Screens are plain Rust values; one renderer translates them into native
  views.
- **Minimal implementation:** Avoid general framework machinery that the PoC does not need.

## 3. High-level architecture

```text
┌──────────────────────────────────────────────┐
│                MainActivity (Kotlin)         │
│                                              │
│  - Load libkrabik_ui_example.so           │
│  - Create AndroidBridge                      │
│  - Forward create/destroy lifecycle          │
│  - Register OnBackInvokedCallback (API 33+)  │
└──────────────────────┬───────────────────────┘
                       │ JNI
                       ▼
┌──────────────────────────────────────────────┐
│            AndroidBridge (Java)              │
│                                              │
│  - nativeCreateUi(activity, listener)        │
│  - onClick(view) → nativeOnClick(view)       │
│  - back() → nativeBack()                     │
│  - nativeDestroyUi()                         │
└──────────────────────┬───────────────────────┘
                       │ JNI
                       ▼
┌──────────────────────────────────────────────┐
│         Rust (example/rust/src/android/)     │
│                                              │
│  - WidgetKind / Action / Screen model        │
│  - Declarative Element tree                  │
│  - Renderer: create and configure views      │
│  - UiState: activity, listener, density,     │
│    current screen, button counter            │
│  - Tag-based click dispatch and navigation   │
│  - Back handling and screen rebuilding       │
└──────────────────────┬───────────────────────┘
                       │ JNI calls
                       ▼
┌──────────────────────────────────────────────┐
│           Android framework APIs             │
│                                              │
│  Activity.setContentView                     │
│  ScrollView / LinearLayout / RadioGroup      │
│  TextView / Button / EditText / ImageView    │
│  CheckBox / RadioButton / Switch / Toggle…   │
│  ProgressBar / SeekBar / RatingBar           │
│  Spinner / AutoCompleteTextView / …          │
│  ArrayAdapter / SystemClock / Log            │
└──────────────────────────────────────────────┘
```

Android remains responsible for drawing, measuring, layout, input dispatch, and the widget
lifecycle. Rust decides what the screens contain and what a click means.

## 4. Component responsibilities

### 4.1 MainActivity

`MainActivity` is the process entry point. It:

- Creates `AndroidBridge` and calls `createUi()`.
- Registers an `OnBackInvokedCallback` on API 33+. Apps targeting API 35+ no longer receive
  `onBackPressed()`, so this is required for the system back gesture to reach Rust.
- Keeps `onBackPressed()` as the fallback for API < 33.
- Calls `destroyUi()` when the activity is destroyed.

It does not know which screen is showing, does not create views, and does not own state.

### 4.2 AndroidBridge

`AndroidBridge` is the only Java class that crosses the JNI boundary. It implements
`View.OnClickListener` and forwards the clicked `View` to Rust, which reads the view's tag to
decide what to do. It also exposes `back()` returning a boolean, and `destroyUi()` for cleanup.

The bridge deliberately contains no navigation or widget logic.

### 4.3 Rust UI model

The Rust side is split into a reusable library crate (`krabik-ui-android`, in
`packages/krabik-ui-android`)
and the demo app crate (`krabik-ui-example`, in `example/rust`).

The **`krabik-ui-android` library crate** contains the Android-facing bindings:

1. **Declarative tree** (`ui`) — `Element` is either a `Scroll`, a `Linear` container, a
   `RadioGroup`, or a leaf widget. `Placed` adds width, height, and margin hints used by the parent
   layout.
2. **Components** (`ui::components`) — small plain-data structs (`Text`, `Button`, `Spinner`,
   `Seek`, …) describing each widget's initial configuration.
3. **Views** (`views`) — creates framework classes, layout parameters, and applies widget setters.
4. **Renderer** (`render`) — walks the tree, resolves dp sizes against the display density,
   constructs the matching Android class, applies properties, and attaches the view with the
   correct `LayoutParams`.
5. **JNI helpers** (`env`, `error`, `constants`) — fallible calls, error reporting, and framework
   names.

The library is application-agnostic. `Button::action(tag)` stores a plain string; the caller's
click listener decides what that tag means. The library has no screens, navigation, or Java-specific
JNI entry points.

The **`krabik-ui-example` app crate** contains:

1. **Widget catalogue** (`widget`) — `WidgetKind` enumerates all 29 demos and provides a title,
   description, stable key, and demo content. `Action` maps tags to navigation and the tap counter.
2. **Screens** (`screens`) — the main list and the per-widget demo trees, built from
   `krabik-ui-android` components.
3. **State and navigation** (`mod.rs`) — the current screen, retained activity/listener references,
   click dispatch, and back handling.
4. **JNI entry points** (`ffi`) — the four `extern "system"` functions called by the Java bridge.

The tree itself contains no JNI calls, so screen definitions can be inspected independently of the
renderer. Adding a widget demo means adding a `WidgetKind` variant, a component variant, and a
renderer arm in `krabik-ui-android`.

### 4.4 Rust UI state

`UiState` is kept in a `static Mutex<Option<UiState>>` for the lifetime of the activity:

```rust
struct UiState {
    activity: Global<JObject<'static>>,
    listener: Global<JObject<'static>>,
    density: f32,
    screen: Screen,
    button_taps: u32,
    button_status: Option<Global<JObject<'static>>>,
}
```

- The activity and listener are JNI global references so later clicks can rebuild the content
  view.
- `screen` records whether the list or a specific demo is showing.
- `button_taps` is Rust-owned interaction state used by the Button demo.
- `button_status` retains the `TextView` that displays the tap count.

`nativeDestroyUi` drops the state and releases all global references before the activity goes away.

## 5. Screen model and navigation

```rust
enum Screen {
    Main,
    Demo(WidgetKind),
}
```

`show_screen` builds the tree for the requested screen, renders it, and calls
`Activity.setContentView`. Navigation is therefore a full rebuild of the native hierarchy from the
Rust tree; there is no Android-side screen manager.

Every button in the main list carries a tag derived from `Action`: `WidgetKind::key()` (for
example `"textview"`), `"back"`, or `"tap"`. The demo header's Back button is tagged `"back"`, and
the Button demo's action button is tagged `"tap"`. `nativeOnClick` reads the tag with `View.getTag()`
and dispatches:

```text
"back"            → Screen::Main
"tap"             → increment counter, update retained TextView
<widget key>      → Screen::Demo(WidgetKind)
```

The same mapping is expressed by `Action`, which parses a tag back into the action it represents.

The system back path is separate: `nativeBack` returns `false` on the main list (the activity
finishes normally) and rebuilds the main list when a demo is open.

## 6. Widget coverage

Each demo creates the real framework class:

| Widget | Notable configuration |
| --- | --- |
| TextView | text, size, colour, gravity, padding, background |
| Button | text, size, all-caps, enabled, click listener |
| EditText | hint, text, input type (including password) |
| ImageView / ImageButton | framework drawable resource |
| CheckBox / Switch / ToggleButton / CheckedTextView | checked state, labels |
| RadioButton | hosted in a real `RadioGroup`; checked state applied after attach |
| ProgressBar | horizontal determinate and indeterminate styles |
| SeekBar | initial progress |
| RatingBar | rating, star count, step size |
| Spinner | `ArrayAdapter` with framework spinner layouts |
| AutoCompleteTextView | `ArrayAdapter` of suggestions |
| MultiAutoCompleteTextView | `CommaTokenizer` plus suggestions |
| NumberPicker | min/max/value |
| DatePicker / TimePicker / CalendarView / AnalogClock | framework defaults |
| Chronometer | started with `SystemClock.elapsedRealtime()` as its base |
| TextClock | 32sp clock following the device 12/24-hour setting |
| SearchView | query hint, iconified by default off |
| Toolbar | title, subtitle, navigation icon, menu items added via `getMenu()` |
| AppCompat Toolbar | `androidx.appcompat.widget.Toolbar` with the same title, subtitle, icon, and menu |
| LinearLayout | nested horizontal and vertical containers with padding, background, and margins |
| RecyclerView | `LinearLayoutManager`/`GridLayoutManager` plus an app-provided item adapter |
| ViewPager2 | orientation, paging limits, and the same app-provided item adapter |

## 7. JNI boundary and error handling

Rust uses the `jni` crate to find classes, invoke constructors, call instance and static methods,
and pass objects between Android APIs. Calls are explicit and grouped by concern (text, padding,
background, layout params, adapters, and so on).

At each boundary Rust:

- Checks for pending Java exceptions and converts them into descriptive errors.
- Logs failures and caught panics to Logcat under the `RustUi` tag.
- Returns safely to the JVM; a Rust panic never unwinds across JNI.

Click handling runs on Android's main thread, so updating a retained `TextView` directly through
JNI is safe. If background Rust work is added later, UI mutations must be posted back to the main
thread.

## 8. Lifecycle and back handling

- Global references are created in `nativeCreateUi` and released in `nativeDestroyUi`.
- A configuration change recreates the activity, the bridge, and the Rust state; the app returns to
  the main list. Preserving navigation state across recreation is out of scope.
- On API 33+, `OnBackInvokedCallback` calls `bridge.back()`. If Rust returns `false` (main list),
  the activity finishes.
- On older API levels, `onBackPressed()` performs the same check.

## 9. Build and packaging

```text
Rust source
    │
    ▼
cargo builds the selected Android targets
    │
    ▼
libkrabik_ui_example.so staged in example/android/app/build/rustJniLibs
    │
    ▼
Gradle packages the libraries into the APK
    │
    ▼
Android launches MainActivity
    │
    ▼
AndroidBridge loads libkrabik_ui_example.so and calls nativeCreateUi
```

`example/rust/build-android.sh` supports building all ABIs or a single ABI via `RUST_ANDROID_ABIS`
(for example `RUST_ANDROID_ABIS="arm64-v8a"`).

## 10. Testing strategy

### Build and startup

- `cargo check --workspace` at the repository root and
  `cargo check --manifest-path packages/krabik-ui-android/example/rust/Cargo.toml`.
- Build `libkrabik_ui_example.so` for the device ABI.
- Build and install the APK.
- Confirm the native library loads and the main list renders.

### Navigation

- Tap every widget button and confirm its demo renders the expected framework widget.
- Return with both the in-app Back button and the system back gesture.
- Confirm the main list is restored each time.

### Interaction

- Tap the Button demo repeatedly and verify the Rust-owned counter increments once per tap.
- Toggle checkboxes, switches, and radio buttons; move seek/rating controls.
- Confirm Spinner and autocomplete adapters show their suggestions.

### Diagnostics

```bash
adb logcat -s RustUi AndroidRuntime
```

Failures from `nativeCreateUi`, `nativeOnClick`, `nativeBack`, and `nativeDestroyUi` are logged
under `RustUi`. No `AndroidRuntime` crash should appear during the smoke test.

## 11. Architectural decisions

| Decision | Choice | Rationale |
| --- | --- | --- |
| FFI | Direct JNI | Tests direct access to Android APIs from Rust |
| UI toolkit | Android framework views | Validates native widget creation |
| Screen structure | Rust `Element` tree | Keeps UI description out of Kotlin/Java |
| Navigation | Rust `Screen` state + full rebuild | Simple, deterministic, no Android screen manager |
| Click dispatch | View tag read by Rust | Avoids one native method per action |
| UI state | Rust | Validates Rust ownership of interaction logic |
| Back handling | Platform callback on 33+, legacy fallback | Works with predictive back |
| Layout engine | None | Android `ViewGroup`s handle layout |
| Cross-platform abstraction | None yet | Avoids premature framework design |

## 12. Success criteria

The architecture is validated when Rust can:

1. Build the list and every widget demo as native Android views.
2. Receive clicks from framework widgets and decide what they mean.
3. Navigate between the list and demos without Kotlin/Java involvement.
4. Handle the system back gesture and rebuild the previous screen.
5. Own and update interaction state, such as the Button demo counter.

All of this happens without XML layouts, Compose, a WebView, or UI logic in the platform layer.
