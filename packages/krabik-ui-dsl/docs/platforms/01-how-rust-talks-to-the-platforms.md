# How Rust talks to the platforms

This chapter puts the two bridges side by side: JNI on Android, and a C interface plus the
`objc2` crates on iOS. It covers who calls whom, on which thread, who owns which object, and how
two numbers (a session handle and a host id) identify everything. At the end there is a table of
every entry point in both directions, so you can find the Rust function behind any native call and
the native method behind any Rust call.

## Four words from the engine

The engine is described in [`../internals/`](../internals/). Four of its terms are used on every
page of this part:

- An **`Op`** (`rust/src/engine/render.rs`) is one instruction for a renderer: `Create`, `Update`,
  `Reparent`, `Remove` or `Layout`. The engine diffs two trees and returns a list of them.
- A **host** is one native root view that Rust renders into, together with its engine state (a
  `Runtime`). The main root is a host; so is every navigation screen and the bottom sheet.
- A **session** (`Session<P>` in `rust/src/host/session.rs`) is one running application plus all
  of its hosts. It turns a native event into a message, calls the app's `update`, and re-renders.
- A **controller** is the native object that owns the chrome: the screen stack, app bar, bottom
  navigation, floating action button, drawer, snackbar, alert and sheet. Rust tells it what to
  show; it reports what the user did.

A platform plugs into the session by implementing three traits from `rust/src/host/platform.rs`:

```rust,ignore
// rust/src/host/platform.rs
pub(crate) trait Platform: Sized {
    /// Whatever a native call needs for the duration of one entry point: the
    /// JNI environment on Android, the main-thread marker on iOS.
    type Cx<'a>;
    type Error;
    type Host: Host<Self>;
    type Controller: Controller<Self>;
}
```

Android fills these in with `Env`, `jni::errors::Error`, `AndroidState` and `AndroidController`
(`rust/src/android/controller.rs`). iOS uses `MainThreadMarker`, `Infallible`, `IosState` and
`IosController` (`rust/src/ios/controller.rs`). The difference in `Error` is real: every JNI call
can fail with a pending Java exception, while a UIKit call either works or raises an Objective-C
exception, which is handled separately (see `guard` in chapter 4).

## The shape both platforms share

```text
            native side                         Rust side
  +---------------------------+      +----------------------------+
  | activity / view controller|      | static map: handle ->      |
  |   root view  (host 0)     |<---->|   Session                  |
  |   screen views (host 1..) |      |     app                    |
  |   sheet view   (host n)   |      |     hosts: id -> Host      |
  |   controller (chrome)     |<---->|     controller             |
  +---------------------------+      +----------------------------+
        ^            |                     |            ^
        |            +--- entry points --->+            |
        +----------- calls on native objects -----------+
```

Both platforms keep their sessions in a process-wide map behind a mutex:

```rust,ignore
// rust/src/android/entry.rs
pub(crate) static STATES: OnceLock<Mutex<HashMap<jlong, AndroidSession>>> = OnceLock::new();
pub(crate) static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

// rust/src/ios/entry.rs
pub(crate) static STATES: OnceLock<Mutex<HashMap<u64, IosSession>>> = OnceLock::new();
pub(crate) static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
```

The **session handle** is the key into that map. Native code receives it from the "create" entry
point and passes it back on every later call. It is a plain number, never a pointer, so a stale
handle cannot crash: the lookup fails and the entry point returns. Zero means "no session".

The **host id** selects one host inside a session. Host 0 is the main root (`MAIN_HOST` in
`rust/src/host/session.rs`). The session hands out the other ids itself, in `sync_navigation` and
`sync_sheet`, when it asks the controller to push a screen or present the sheet. The controller
creates a native view for that id, and the view then announces itself with the "attach host" entry
point. Until then the id sits in the session's `pending` map.

A **node id** (`NodeId`) identifies one widget inside a host. Events on a widget carry all three
numbers: session handle, host id, node id.

### Threads

Everything runs on the platform's main (UI) thread. The mutex around the map is not there to
arbitrate between threads; it exists because a `static` must be `Sync`, and because it makes
re-entrancy visible. Re-entrancy is the real problem on both platforms: Rust changes a native
widget while holding the lock, the widget synchronously fires a listener, and the listener calls
back into Rust, which wants the lock again. A `std::sync::Mutex` is not re-entrant, so that second
lock attempt on the same thread would block forever. The two platforms solve this differently:

- Android **tries** the lock in event entry points and drops the event when it is taken
  (`lock_states_for_event` in `rust/src/android/entry.rs`; chapter 2 explains why that is safe).
- iOS **defers** every widget event to the next main-queue turn (`schedule` in
  `rust/src/ios/mod.rs`; chapter 4), and the Swift side defers the calls that UIKit could make
  while Rust holds the lock.

There is one exception to "main thread only": the completion handler of an iOS image download
runs on a background queue. It does nothing but decode the image and hop back to the main queue
(`start_download` in `rust/src/ios/images.rs`).

## Android: JNI

JNI (the Java Native Interface) lets JVM code call functions in a native library and lets native
code call JVM methods. The crate `jni` (version 0.22, declared in `rust/Cargo.toml`) wraps it.

### Kotlin calling Rust

A Kotlin method marked `external` has no body. When it is first called, the JVM looks in the
loaded native libraries for a symbol with a mangled name: `Java_`, the package with dots replaced
by underscores, the class, the method. All of the framework's `external fun`s live in one class,
`RustRootView`:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt
private external fun nativeRender(handle: Long, host: Long, widthPx: Int, heightPx: Int)
private external fun nativeOnClick(handle: Long, host: Long, nodeId: Long)
```

On the Rust side the symbol name is built by a small macro, so the Kotlin package is spelled in
one place:

```rust,ignore
// rust/src/android/mod.rs
macro_rules! native_name {
    ($method:literal) => {
        concat!("Java_dev_krabik_ui_RustRootView_", $method)
    };
}

// rust/src/android/entry.rs
#[unsafe(export_name = native_name!("nativeOnClick"))]
pub extern "system" fn native_on_click<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
) {
    run(&mut unenv, "nativeOnClick", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Click)
    });
}
```

`export_name` sets the symbol the linker emits; the Rust function keeps an ordinary snake-case
name. Every JNI function receives the environment first and the receiver (`this`, or the class for
a static method) second, then the declared parameters. `extern "system"` is the calling convention
JNI specifies.

The body always goes through `run` (`rust/src/android/mod.rs`). It converts the raw environment
into a usable `Env`, runs the closure, and makes sure neither a Rust panic nor a pending Java
exception escapes into the JVM in a broken state: on an error it prints and clears the Java
exception, reports the Rust error through `crate::log::error` (logcat, tag `krabik`), and returns
`T::default()`.

Why do all native methods hang off `RustRootView`, even ones about the toolbar or the dialog? One
class means one mangled prefix and one place to look. The controller reports chrome events by
calling ordinary Kotlin methods on the main `RustRootView` (`main.onChromeEvent(...)`), which then
call the `external fun`.

### Rust calling Kotlin

In the other direction Rust needs a class, a method id and a receiver. Class names are built by a
second macro, again so the package is written once:

```rust,ignore
// rust/src/android/mod.rs
macro_rules! framework_class {
    ($name:literal) => {
        jni::jni_str!("dev/krabik/ui/", $name)
    };
}

pub(crate) const RUST_ROOT_VIEW_CLASS: &JNIStr = framework_class!("RustRootView");
pub(crate) const SWITCH_CLASS: &JNIStr = framework_class!("KrabikSwitch");
```

A third spelling, `ROOT_VIEW_DESCRIPTOR` (`"Ldev/krabik/ui/RustRootView;"`), is used inside
method signatures. Renaming the Kotlin package means changing these three items and the Kotlin
sources, nothing else in Rust.

Looking a class or a method up by name on every call is slow, so every id is resolved once and
cached in one struct, `Jni` (`rust/src/android/handles.rs`). Chapter 2 covers it. A call then
looks like this:

```rust,ignore
// rust/src/android/widgets/switch.rs
jni_handles(env)?.switch.set_checked_silently.call_void(
    env,
    view,
    &[JValue::Bool(props.checked)],
)
```

Rust calls three kinds of JVM code: stock Android classes (`View.setPadding`,
`TextView.setText`), the framework's own widget classes (`KrabikSwitch.setCheckedSilently`), and
static helpers on Kotlin `object`s (`KrabikStyle.apply`, `KrabikRecyclers.create`). The helpers
exist where one Kotlin call replaces many JNI calls, or where the work needs a Kotlin class (an
adapter, an animator, a listener).

### Ownership on Android

- A Java object passed into an entry point is a **local reference**: valid until the entry point
  returns. Anything Rust keeps must become a **global reference** (`env.new_global_ref`). Every
  `ViewHandle` holds global references to its view; `AndroidState` holds global references to the
  root view and the context; `AndroidController` wraps a global reference to the Kotlin
  controller. Dropping the Rust value releases the reference.
- The JVM owns the views. A global reference keeps a view alive but does not keep it attached.
  Rust removes views explicitly (`Op::Remove`, and `removeAllViews` in `nativeDestroy`).
- Rust owns the application, the trees and all state. Kotlin keeps only the session handle, the
  host id, and for each framework widget its node id.

## iOS: a C interface and objc2

iOS uses two different mechanisms, one per direction.

### Swift calling Rust

Swift cannot call Rust directly, but both can speak C. The header `shared/ios/KrabikIOS.h`
declares the functions; each app's bridging header includes it, which makes them visible to Swift
as ordinary global functions:

```c
// shared/ios/KrabikIOS.h
uint64_t krabik_ios_create(void *root, double width, double height, bool dark,
                           const char *locale);
void krabik_ios_render(uint64_t handle, uint64_t host, double width, double height);
```

Rust defines them with `#[unsafe(no_mangle)]` and `extern "C"`:

```rust,ignore
// rust/src/ios/entry.rs
#[unsafe(no_mangle)]
pub unsafe extern "C" fn krabik_ios_render(handle: u64, host: u64, width: f64, height: f64) {
    let Some(mut mtm) = MainThreadMarker::new() else {
        return;
    };
    guard("render", || {
        with_session(handle, |session| {
            if let Some(state) = session.host_mut(host) {
                state.viewport = Size::new(width.max(0.0) as f32, height.max(0.0) as f32);
            }
            infallible(session.render_all(&mut mtm));
        });
    });
}
```

The header and the Rust file are kept in step by hand. Nothing generates one from the other, and
a mismatch in a parameter type would not be caught by either compiler. The list is short (fourteen
functions), which is why this is tolerable.

`MainThreadMarker::new()` returns `None` off the main thread, so most entry points refuse to run
there. A `MainThreadMarker` is a zero-sized proof value that `objc2` requires before it will
create a UIKit object; it is this platform's `Cx`.

Views cross the boundary as untyped pointers. Swift passes
`Unmanaged.passUnretained(view).toOpaque()`, and Rust retains it:

```rust,ignore
// rust/src/ios/entry.rs
let Some(root) = (unsafe { Retained::retain(root.cast::<UIView>()) }) else {
    return 0;
};
```

### Rust calling UIKit directly

Unlike Android, there is no Swift class per widget. The crates `objc2`, `objc2-ui-kit`,
`objc2-foundation` and `objc2-quartz-core` expose UIKit's Objective-C classes as Rust types, so
Rust creates a `UILabel`, sets its text and adds it to a superview with ordinary method calls:

```rust,ignore
// rust/src/ios/widgets/text.rs
let label = UILabel::new(site.mtm);
label.setText(Some(&NSString::from_str(&props.content)));
```

Where UIKit wants a delegate or a target object, Rust defines an Objective-C class with
`objc2::define_class!` (`ScrollDelegate`, `PagerDelegate`, `RecyclerDataSource`, `FrameTicker`).
Where it wants a closure, Rust passes a block from the `block2` crate.

This is why the iOS shell is much smaller than the Android one: Swift only owns the things UIKit
expresses as view controllers (navigation, alert, sheet) and the chrome views.

### Rust calling Swift

The controller is a Swift class, `KrabikHostController`. Rust holds it as a plain `NSObject` and
sends it Objective-C messages by selector name with `msg_send!`:

```rust,ignore
// rust/src/ios/controller.rs
pub(crate) fn controller_push(controller: &NSObject, host: u64, transition: i32) {
    // SAFETY: the controller is a live Swift object with this @objc method.
    let _: () = unsafe {
        objc2::msg_send![
            controller,
            krabikPush: host as i64,
            transition: transition as i64
        ]
    };
}
```

```swift
// shared/ios/KrabikHostController.swift
@objc func krabikPush(_ host: Int64, transition: Int) {
```

The selector is `krabikPush:transition:`. Again nothing checks the two sides against each other
at compile time: a typo in the selector or a wrong argument type is a run-time failure. The
`krabik` prefix keeps these methods from colliding with anything UIKit defines.

### Ownership on iOS

- `Retained<T>` is a strong reference (it retains on creation and releases on drop). Every
  `ViewHandle` holds one to its view.
- `IosState.root` is a **weak** reference to the Swift host view. The comment in
  `rust/src/ios/host.rs` gives the reason: a strong one would keep the view alive, its `deinit`
  would never run, and `deinit` is where Swift tells Rust to drop the host.
- UIKit holds delegates and data sources weakly, so the `ViewHandle` owns them (the `_delegate`
  and `data_source` fields of `Content`).
- `IosController` holds a strong reference to the Swift controller, and the Swift controller's
  `deinit` destroys the session. That is a reference cycle through the Rust map: the controller is
  only released if something else destroys the session first. In the current apps the controller
  lives as long as the process, so this does not show, but it is worth knowing before embedding
  the controller in a screen that can go away.

## Every entry point

### Android: Kotlin calls Rust

All are `external fun`s of `RustRootView`, implemented in `rust/src/android/entry.rs`. "Lock"
says how the entry point takes the state map: `lock` blocks, `try` gives up when a render holds it.

| Kotlin `external fun` | Lock | What it does |
| --- | --- | --- |
| `nativeCreate(context, density, night, locale, retainedApp)` | lock | Loads the JNI handles, creates the session and its main host, returns the handle. |
| `nativeRender(handle, host, w, h)` | lock | Sets the host's viewport and renders every host. |
| `nativeContentHeight(handle, host)` | lock | Height in pixels of what the host rendered last, for the sheet. |
| `nativeAttachHost(handle, host)` | lock | Adopts a screen or sheet view the controller created, then renders. |
| `nativeSetController(handle, controller)` | lock | Registers the Kotlin controller, then renders (which pushes the first screens). |
| `nativeDestroyHost(handle, host)` | try | Drops one host. |
| `nativeDestroy(handle, retain)` | lock | Removes all views and the session; with `retain` keeps the app and returns its id. |
| `nativeDropRetainedApp(id)` (static) | none | Drops an app that was retained but never picked up. |
| `nativeOnClick(handle, host, node)` | try | A stock button was pressed. |
| `nativeOnTextChanged(..., text)` | try | A text field's content changed. |
| `nativeOnSubmitted(...)` | try | A text field was submitted from the keyboard. |
| `nativeOnFocusChanged(..., focused, attached)` | try | A text field gained or lost focus. |
| `nativeRestoreRowFocus(..., node, holder)` | try | A recycled row came back; refocus its field if it had the logical focus. |
| `nativeOnCheckedChanged(..., checked)` | try | A switch, checkbox or radio button was toggled. |
| `nativeOnSliderChanged(..., value)` | try | A slider moved. |
| `nativeOnPickerChanged(..., kind, y, m, d, h, min)` | try | A date or time picker chose a value. |
| `nativeOnRadioGroupSelected(..., index)` | try | A radio group selected an option. |
| `nativeOnTabSelected(..., index)` | try | A tab bar widget selected a tab. |
| `nativeOnPageSelected(..., index)` | try | A pager settled on a page. |
| `nativeOnScroll(..., offset)` | try | A scroll container moved. |
| `nativeBindRow(..., node, index, holder, host, w)` | lock | Builds one recycled row into a holder; returns its height. |
| `nativeUnbindRow(..., node, holder)` | try | A holder was recycled. |
| `nativeOnToolbarAction(handle, index)` | try | An app bar action was tapped. |
| `nativeOnBottomNavigationSelected(handle, index)` | try | A bottom navigation item was picked. |
| `nativeOnChromeEvent(handle, kind, index)` | try | Floating action button, drawer or snackbar event. |
| `nativeOnDialogAction(handle, button)` | try | The alert closed, by a button or (negative) by the platform. |
| `nativeOnModalDismiss(handle)` | try | The user dismissed the bottom sheet. |
| `nativeOnBack(handle)` | try | System back; returns whether the app consumed it. |

Each app adds one more, on its own bridge class: `install()`, which registers the app factory.

### Android: Rust calls Kotlin

| Receiver | Methods | Called from |
| --- | --- | --- |
| `RustRootView` | `applyFrame`, `applyLayouts`, `registerView`, `unregisterView`, `startEnter`, `startExit`, `ensureKeyboardShown` | `android/host.rs`, `android/style.rs`, `android/entry.rs` |
| `KrabikHostController` | `push`, `pop`, `setRootVisible`, `setToolbar`, `setBottomNavigation`, `setFab`, `setDrawer`, `showSnackbar`, `dismissSnackbar`, `setBackEnabled`, `showDialog`, `dismissDialog`, `presentSheet`, `dismissSheet` | `android/controller.rs` |
| `KrabikStyle` (static) | `apply` | `android/style.rs` |
| `KrabikViews` (static) | `createScroll`, `createProgress`, `updateProgress`, `updateSpinner` | `android/host.rs`, `android/widgets/progress.rs` |
| `KrabikRecyclers` (static) | `create`, `update` | `android/rows.rs` |
| `KrabikRows` (static) | `snapshot`, `fadeOut` | `android/rows.rs` |
| `KrabikImages` (static) | `load`, `size` | `android/widgets/image.rs`, `android/measure.rs` |
| Framework widgets | `setTextSilently`, `setCheckedSilently`, `setValueSilently`, `configure`, `setDateRange`, `setTimeRange`, `setStyle` | `android/widgets/*.rs` |
| Stock Android classes | `View`, `ViewGroup`, `TextView` setters, constructors, `measure` | everywhere, through `handles.rs` |

### iOS: Swift calls Rust

All are declared in `shared/ios/KrabikIOS.h` and defined in `rust/src/ios/entry.rs`.

| C function | What it does |
| --- | --- |
| `krabik_ios_create(root, width, height, dark, locale)` | Creates the session and its main host, renders once, returns the handle. |
| `krabik_ios_set_locale(handle, locale)` | The user's language changed while the app runs; tell the application and re-render. |
| `krabik_ios_set_color_scheme(handle, dark)` | The device switched appearance; re-render with the other colours. |
| `krabik_ios_render(handle, host, width, height)` | Sets the host's viewport and renders every host. |
| `krabik_ios_attach_host(handle, host, root)` | Adopts a screen or sheet view, then renders. |
| `krabik_ios_destroy_host(handle, host)` | Drops one host; called from the host view's `deinit`. |
| `krabik_ios_set_controller(handle, controller)` | Registers the Swift controller, then renders. |
| `krabik_ios_destroy(handle)` | Removes all views and the session. |
| `krabik_ios_content_height(handle, host)` | Height in points of what the host rendered last. |
| `krabik_ios_on_back(handle)` | The custom back button was tapped; returns whether the app consumed it. |
| `krabik_ios_on_toolbar_action(handle, index)` | A navigation bar action was tapped. |
| `krabik_ios_on_bottom_navigation_selected(handle, index)` | A tab was picked. |
| `krabik_ios_on_chrome_event(handle, kind, index)` | Floating action button, drawer or snackbar event. |
| `krabik_ios_on_dialog_action(handle, button)` | The alert closed. |
| `krabik_ios_on_modal_dismiss(handle)` | The user dismissed the sheet. |

There is no C function per widget event. Widget events never pass through Swift: Rust attaches a
`UIAction` or a delegate to the control itself and handles the callback in Rust
(`rust/src/ios/events.rs`). Each app adds one C function of its own, for example
`krabik_ui_dsl_example_install`.

### iOS: Rust calls Swift

All are `@objc` methods of `KrabikHostController`, sent from `rust/src/ios/controller.rs`.

| Selector | What it does |
| --- | --- |
| `krabikPush:transition:` | Pushes a screen for a host id. |
| `krabikPop:transition:` | Pops a number of screens in one step. |
| `krabikSetRootVisible:` | Shows the main root or the screen stack. |
| `krabikSetToolbar:subtitle:backTitle:hidden:actionLabels:actionIcons:` | Applies the top screen's app bar. |
| `krabikSetBottomNavigation:icons:selected:` | Applies the tab bar; no labels hides it. |
| `krabikSetFab:icon:visible:` | Applies the floating action button. |
| `krabikSetDrawer:icons:selected:present:open:` | Applies the drawer. |
| `krabikShowSnackbar:action:` / `krabikDismissSnackbar` | Shows or removes the snackbar. |
| `krabikShowDialog:message:labels:roles:` / `krabikDismissDialog` | Shows or closes the alert. |
| `krabikPresentSheet:` / `krabikDismissSheet` | Presents or dismisses the sheet. |

`Controller::set_back_enabled` has no selector: iOS has no system back action, so the Rust
implementation is empty.

### Numbers that both sides must agree on

A few small integers are defined twice, once in Rust and once natively, with a comment on each
side pointing at the other:

- Chrome events, `ChromeEvent::from_code` in `rust/src/host/platform.rs`: 0 floating action
  button, 1 drawer item picked, 2 drawer opened, 3 drawer closed, 4 snackbar action, 5 snackbar
  dismissed. Kotlin: `EVENT_*` in `KrabikHostController`. Swift: `event*` in
  `KrabikHostController`.
- Screen transitions, `ScreenTransition::code` in `rust/src/navigation.rs`: 0 slide, 1 fade,
  2 none.
- Dialog button roles: 0 default, 1 cancel, 2 destructive.
- Picker kinds: 0 date, 1 time, 2 date and time.
- Animation curves, packed as five integers by `AnimationCurve::packed` and decoded by
  `KrabikStyle.interpolator` (Android only; iOS evaluates curves in Rust or hands them to UIKit).

## Where this lives

- `rust/src/host/platform.rs`: `Platform`, `Host`, `Controller`, `ChromeEvent`.
- `rust/src/android/mod.rs`: `native_name!`, `framework_class!`, `jcall!`, `run`, class names.
- `rust/src/android/entry.rs`: every JNI entry point and the state map.
- `shared/android/src/main/java/dev/krabik/ui/RustRootView.kt`: every `external fun`.
- `shared/ios/KrabikIOS.h` and `rust/src/ios/entry.rs`: the C interface, both halves.
- `rust/src/ios/controller.rs`: every `msg_send!` to the Swift controller.

Next: [The Android renderer](02-the-android-renderer.md).
