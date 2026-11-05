Analyzed SPEC.md:L1-250
Analyzed SPEC.md:L251-500
Analyzed SPEC.md:L501-709
Ran find files in ios directory
Analyzed KrabikUiDslApp.swift:L1-10
Analyzed ContentView.swift:L1-9
Analyzed Cargo.toml:L1-25
Analyzed lib.rs:L1-32
Ran find uniffi-bindgen files

Based on a review of the architecture, layout engine, and platform implementations against [SPEC.md](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/SPEC.md), here is a prioritized roadmap of suggestions to improve this codebase:

---

### 1. Decouple Application Code from the Platform Renderer (High Priority)

Currently, the demo gallery and screen logic (`gallery()`, `counter()`, `Demo` enum, click handling) are hardcoded inside [`rust/src/android.rs`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/android.rs#L296-L507).

* **The Problem**: A user cannot write their own app screens without editing the Android JNI renderer code. The core promise of the framework—writing UI once in Rust and running on any platform—is blocked by this coupling.
* **Suggestion**:
  - Define an **`App`** or **`Component`** trait in pure Rust:
    ```rust
    pub trait App: Send + 'static {
        type Message;
        fn view(&self) -> Element;
        fn update(&mut self, message: Self::Message);
    }
    ```
  - Move the gallery demo out of [`android.rs`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/android.rs) into an `examples/gallery` module or a dedicated `App` implementation.
  - Make `AndroidState` (and any future iOS state) generic over `App` or hold a dynamic `Box<dyn App>`.

---

### 2. Implement the iOS Renderer for Cross-Platform Parity

The iOS app currently only has placeholder SwiftUI files ([`ContentView.swift`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/ios/Sources/ContentView.swift)) calling `getGreeting()`.

* **Suggestion**:
  - Implement **`ios.rs`** mirroring [`android.rs`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/android.rs):
    - Implement [`Measure`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/measure.rs) using iOS text measurement (`NSAttributedString.boundingRect` or an offscreen `UILabel`).
    - Expose C FFI / UniFFI functions to create, layout, update, and remove `UIView` / `UILabel` / `UIButton` instances.
  - Create a custom **`RustRootView: UIView`** in Swift that manages native `UIView` children based on Taffy frames and dispatches touch events back to Rust.

---

### 3. Expand the Native Widget Set

Currently only [`Text`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/ui.rs#L494) and [`Button`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/ui.rs#L511) exist as native widgets. According to [SPEC.md](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/SPEC.md#L388-L399), the core mobile primitives needed next are:

1. **`ScrollView` / scroll container**:
   - Right now, any UI larger than the viewport overflows and clips because there is no scrolling.
   - Taffy can compute the inner content frame while Android `ScrollView` / iOS `UIScrollView` handles scrolling and viewport gestures.
2. **`TextField` / Input**:
   - Android `EditText`, iOS `UITextField` with two-way text synchronization and `on_change` events.
3. **`Image`**:
   - Android `ImageView`, iOS `UIImageView` for local assets and URLs.
4. **Enhanced Styling**:
   - Corner radius, borders, opacity, and custom button background colors.

---

### 4. Optimize FFI / JNI Performance (Operation Batching & Caching)

In [`AndroidState::render`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/android.rs#L509-L558), every single node property update, layout change, and view creation issues separate individual JNI calls (`find_class`, `call_method`, `setX`, `setY`, etc.).

* **Class and Method ID Caching**:
  - `env.find_class(TEXT_VIEW_CLASS)` and similar are called dynamically inside helper functions during rendering. Caching `jclass` and `jmethodID` handles at startup avoids JNI method table lookups on every layout pass.
* **Batching Operations**:
  - Instead of calling JNI per-node across the boundary, serialize or pass batched arrays of layout updates (e.g. `[node_id, x, y, width, height, ...]`) into a single Kotlin call on [`RustRootView`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/android/app/src/main/java/com/example/krabik_ui_dsl/RustRootView.kt).

---

### 5. Type-Safe State Management & Event Dispatch

Currently, event handling uses untyped string slices like `"open:counter"` and `"counter:add"` in [`Button::action`](file:///Users/charafau/Projects/rust/mobile/krabik/packages/krabik-ui-dsl/rust/src/ui.rs#L521):

* **Typed Actions / Messages**:
  - Support typed messages (e.g. `enum Message { Increment, Reset, Navigate(Screen) }`) or closures (`.on_click(|| ...)`).
* **State Container**:
  - Introduce an Elm-like / Redux-like unidirectional data flow architecture (`view(state) -> Message -> update(&mut state, message) -> reconcile`), ensuring UI always strictly reflects pure application state.

---

### Where would you like to start?

1. **Decouple App from Android**: Refactor the demo screens out of `android.rs` into a generic `App` architecture so you can write custom screens cleanly.
2. **Add Missing Widgets**: Implement `ScrollView`, `Image`, or `TextField`.
3. **Build the iOS Bridge**: Implement the UIKit equivalent of `RustRootView` and Swift/Rust FFI.
4. **JNI Batching & Caching**: Optimize the bridge performance.