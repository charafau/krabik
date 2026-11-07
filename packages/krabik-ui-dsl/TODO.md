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

### 3. Expand Native Widget Set ✅

All core widgets from SPEC.md §7.1 are implemented on Android.

| Rust component | Android renderer | Platform widget |
|---|---|---|
| `Text` | [`update_text()`](rust/src/android.rs) | `TextView` |
| `Button` | [`configure_button()`](rust/src/android.rs) | `Button` |
| `TextField` | [`update_text_field()`](rust/src/android.rs) | Custom `KrabikTextInput` (EditText) |
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

### 5. Type-Safe State Management & Event Dispatch ✅

Events are typed in `rust/src/app.rs`:

```rust
pub enum Event<'a> {
    Click { action: &'static str },
    TextChanged { action: &'static str, value: &'a str },
    Submitted { action: &'static str },
    FocusChanged { action: &'static str, focused: bool },
}
```

Unidirectional data flow: `view(state) → handle(event) → render()` is the loop.
`App::handle()` returns `bool` to signal whether the UI should re-render.

---

## Remaining ❌

### 2. Implement the iOS Renderer ❌ *(next priority)*

The iOS app currently has only placeholder SwiftUI files ([`ContentView.swift`](ios/Sources/ContentView.swift)) calling `getGreeting()`.

**To do:**
- Implement `ios.rs` mirroring `android.rs`:
  - `Measure` using UIKit text measurement (`NSAttributedString.boundingRect` or offscreen `UILabel`).
  - Expose C FFI / UniFFI functions to create, layout, update, and remove `UIView` / `UILabel` / `UIButton` instances.
- Create a custom `RustRootView: UIView` in Swift that manages native `UIView` children based on Taffy frames and dispatches touch events back to Rust.

### 4. Optimize JNI Performance ⚠️

In `AndroidState::render`, every node property update issues separate JNI calls (`find_class`, `call_method`, `setX`, `setY`, etc.).

**To do:**
- **Class/Method ID caching**: `env.find_class(TEXT_VIEW_CLASS)` and similar are called dynamically inside helper functions. Caching `jclass` and `jmethodID` handles at startup avoids JNI method table lookups on every layout pass.
- **Batching operations**: Instead of per-node JNI calls, serialize layout updates into batched arrays passed as a single Kotlin call to `RustRootView`.

---

## Future (not yet started)

- Virtualized lists (RecyclerView / UICollectionView-based)
- Animations via native platform APIs
- More widgets: Checkbox, RadioButton, Slider, ProgressIndicator, DatePicker, Dialog, BottomSheet, Navigation
- Platform-specific extensions
