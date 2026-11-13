# 6. The Android and iOS halves

Most of the framework is platform-independent Rust. This chapter covers the
part that is not: how Rust drives native views, and what the small Kotlin and
Swift layers do.

## Who does what

| | Rust | Kotlin / Swift |
|---|---|---|
| Decides what is on screen | yes | no |
| Creates and updates widgets | yes, through JNI / UIKit calls | Android only: a few widget subclasses |
| Lays out | yes (Taffy) | no; views are placed at the frames Rust computed |
| Hosts the root view, reports its size | – | yes |
| Owns navigation, toolbar, tab bar, alert, sheet | decides | carries it out |

The native layer never keeps application state and never decides what to
show. When the user does something, it tells Rust and waits.

## Android

### The Kotlin library: `shared/android`

An Android library module (`:krabik-ui`, package `dev.krabik.ui`) that every
app includes.

- **`KrabikActivity`**: the activity an app subclasses. It builds the fixed
  frame (toolbar, content, bottom navigation), draws edge to edge, and
  forwards system back to Rust while the app handles it.
- **`RustRootView`**: one `ViewGroup` per host. It reports its size to Rust
  (`nativeRender`), receives events from widgets (`onClick`,
  `onCheckedChanged`, …) and passes them to Rust through `native…` methods.
- **`KrabikHostController`**: owns the screen stack (fragments), the toolbar,
  the bottom bar, the alert and the sheet. Rust calls `push`, `pop`,
  `setToolbar`, `showDialog`, `presentSheet` and so on.
- **`Krabik…` widgets**: thin subclasses (`KrabikSwitch`, `KrabikTabBar`,
  `KrabikPager`, …) for widgets that need a listener or behaviour the stock
  view lacks. Each one reports user input to `RustRootView` and applies state
  from Rust *silently*, so setting a value does not echo back as an event.

### The Rust side: `rust/src/android/`

| File | Contents |
|---|---|
| `mod.rs` | The framework's Kotlin package (in one place), class names, small JNI helpers. |
| `handles.rs` | Every JNI class, method and constructor id, looked up once. |
| `host.rs` | `AndroidState`: applies engine operations to views. |
| `widgets/` | One file per widget: how its properties reach the native view. |
| `rows.rs` | Recycled lists: binding rows into `RecyclerView` holders. |
| `style.rs` | Padding, decoration, accessibility, frames. |
| `measure.rs` | Text measurement with an off-screen `TextView`. |
| `controller.rs` | The `Platform` pieces: calls into `KrabikHostController`. |
| `entry.rs` | The `native…` methods of `RustRootView`: decode arguments, call the session. |

Rust creates views itself (`new TextView(context)` through JNI) and adds them
to the `RustRootView`. Frames for a whole render are sent in one batched call.

### The app's native library

An app crate depends on `krabik-ui-dsl` and builds one `.so` that contains
the framework and the app. `MainActivity` loads it and installs the app:

```kotlin
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() = System.loadLibrary("my_app")
    override fun installApp() = MyAppBridge.install()
}

object MyAppBridge {
    external fun install()
}
```

Gradle builds the Rust crate as part of the normal Android build
(`shared/android/rust.gradle.kts`).

## iOS

### The Swift host: `shared/ios`

Three Swift files and one C header, compiled into every app.

- **`KrabikHostController`**: a view controller holding a
  `UINavigationController`. It carries out what Rust decides: `krabikPush`,
  `krabikPop`, `krabikSetToolbar`, `krabikSetBottomNavigation`,
  `krabikShowDialog`, `krabikPresentSheet`.
- **`RustHostView`**: one `UIView` per host; reports its size to Rust.
- **`ContentView`**: the SwiftUI wrapper around the controller.
- **`KrabikIOS.h`**: the C functions Swift calls (`krabik_ios_create`,
  `krabik_ios_render`, `krabik_ios_on_back`, …).

There are no widget classes in Swift at all.

### The Rust side: `rust/src/ios/`

Rust creates and configures UIKit views directly with `objc2`
(`UILabel::new`, `UISwitch::new`, …), and attaches event handlers as
`UIAction` blocks. The layout mirrors Android's, plus:

| File | Contents |
|---|---|
| `events.rs` | Widget events, delivered to the session on the next main-queue turn. |
| `images.rs` | Bundled and remote images. |
| `motion.rs` | Frame and decoration animations. |

### The app's native library

The app crate builds a static library that Xcode links. A build phase runs
`cargo build`, so the app never links a stale library. The app's Swift is
only its entry point:

```swift
@main
struct MyApp: App {
    init() { my_app_install() }
    var body: some Scene { WindowGroup { ContentView() } }
}
```

## The shared contract

Both platforms implement the same two traits from `rust/src/host/platform.rs`
and nothing else of the session:

```rust
trait Host<P> {
    fn render(&mut self, cx, element: &RawElement) -> Result<(), P::Error>;
    // ... plus access to the host's engine state
}

trait Controller<P> {
    fn push(&self, cx, host: u64, screen: &Screen) -> ...;
    fn pop(&self, cx, count: usize, transition: ScreenTransition) -> ...;
    fn set_chrome(&self, cx, screen: Option<&Screen>) -> ...;
    fn show_dialog(&self, cx, dialog: &Dialog) -> ...;
    fn present_sheet(&self, cx, host: u64) -> ...;
    // ...
}
```

A third platform would implement these two traits and the entry points that
feed events into the session.

Next: [adding a widget](07-adding-a-widget.md).
