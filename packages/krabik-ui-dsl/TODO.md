# Krabik UI DSL — Roadmap & Status

Based on the architecture defined in [SPEC.md](SPEC.md), here is the current status of the roadmap.

---

## Done ✅

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
| **M0 — build spike** | `build.rs` skips the Kotlin JNI generator on iOS; the staticlib builds for `aarch64-apple-ios-sim --features ios` and the app renders in the simulator. |
| **M1 — Rust FFI skeleton** | [`ios.rs`](rust/src/ios.rs) behind `#[cfg(target_os = "ios")]`: handle-keyed `Mutex<HashMap<u64, IosState>>` with `create / render / destroy` and typed event entry points mirroring Android. |
| **M2 — measurement** | `IosMeasure` measures `NSString` text with `UIFont` attributes, including size/weight/max-lines. |
| **M3 — UIKit widgets** | `UILabel`, `UIButton`, `UISwitch`, `UITextField`, `UIImageView`, `UIScrollView`, plus gradients (`CAGradientLayer`), borders, radii, opacity, elevation, and accessibility labels. |
| **M4 — root view + parity** | [`RustRootView.swift`](ios/Sources/RustRootView.swift) replaces the placeholder; a UI test drives all ten gallery demos (counter, layouts, typography, alignment, scroll, form, media, styling, controls) and passes on the simulator. |
| **M5 — CI (optional)** | Not added yet; the Android job remains the only mobile CI job. |

Implementation notes:

- Uses `objc2` / `objc2-ui-kit` / `objc2-foundation` / `objc2-quartz-core` rather than UniFFI.
- Actions use `UIControl::addAction_forControlEvents` with `RcBlock` handlers.
- Events are deferred to the next main-queue turn; viewport re-renders from
  `layoutSubviews` are coalesced and deferred the same way, so UIKit can never
  re-enter the state mutex while a handler is rendering.
- `UITextField` keyboard/return-key setters go through raw `objc_msgSend`
  because UIKit implements them via message forwarding.
- Remote URL images are not implemented on iOS yet (bundled assets are).

### 3. Expand Native Widget Set ✅

All core widgets from SPEC.md §7.1 are implemented on Android.

| Rust component | Android renderer | Platform widget |
|---|---|---|
| `Text` | [`update_text()`](rust/src/android.rs) | `TextView` |
| `Button` | [`configure_button()`](rust/src/android.rs) | `Button` |
| `TextField` | [`update_text_field()`](rust/src/android.rs) | Custom `KrabikTextInput` (EditText) |
| `Switch` | [`update_switch()`](rust/src/android.rs) | Custom `KrabikSwitch` (`android.widget.Switch`) |
| `Image` | [`update_image()`](rust/src/android.rs) | `ImageView` (assets + URLs) |
| `ScrollView` | [`create_scroll()`](rust/src/android.rs) | Custom scroll container (vertical + horizontal) |

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
- Controls demo with native switches

### 5. Type-Safe State Management & Event Dispatch ✅

Events are typed in `rust/src/app.rs`:

```rust
pub enum Event<'a> {
    Click { action: &'static str },
    TextChanged { action: &'static str, value: &'a str },
    Submitted { action: &'static str },
    FocusChanged { action: &'static str, focused: bool },
    Toggled { action: &'static str, checked: bool },
}
```

Unidirectional data flow: `view(state) → handle(event) → render()` is the loop.
`App::handle()` returns `bool` to signal whether the UI should re-render.

---

## Remaining ❌

### Verify the Android CI job ❌

The `android` job in [`ci.yml`](../../../.github/workflows/ci.yml) was validated locally only.
Push the branch and confirm it passes. The first run is slow (Rust cross-compile + Gradle
download); later runs are cached. If the runner disagrees on `platforms;android-37.0` or an
action major, adjust the job.

### 4. Optimize JNI Performance ⚠️

In `AndroidState::render`, every node property update issues separate JNI calls (`find_class`, `call_method`, `setX`, `setY`, etc.).

**To do:**
- **Class/Method ID caching**: `env.find_class(TEXT_VIEW_CLASS)` and similar are called dynamically inside helper functions. Caching `jclass` and `jmethodID` handles at startup avoids JNI method table lookups on every layout pass.
- **Batching operations**: Instead of per-node JNI calls, serialize layout updates into batched arrays passed as a single Kotlin call to `RustRootView`.

---

## Future (not yet started)

- Remote URL images on iOS (bundled assets already work)
- iOS CI job (`cargo build --target aarch64-apple-ios-sim --features ios`)
- Virtualized lists (RecyclerView / UICollectionView-based)
- Animations via native platform APIs
- Native intrinsic measurement for switches and images
- Disabled-state styling for custom backgrounds (native text already dims)
- More widgets: Checkbox, RadioButton, Slider, ProgressIndicator, DatePicker, Dialog, BottomSheet, Navigation
- Platform-specific extensions
