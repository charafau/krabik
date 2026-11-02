# PRD: Rust-to-Android Native UI Proof of Concept

> Historical document. This is the original proposal for the PoC that became the
> `krabik-ui-android` library and the example widget browser. Paths and commands below may refer to
> the pre-restructure layout; see the root `README.md` for current usage.

## 1. Overview

**Working title:** Rust Native UI — Android PoC  
**Platform:** Android  
**Language:** Rust + Kotlin  
**FFI:** JNI using the Rust `jni` crate

### Objective

Prove that Rust can directly create, configure, and display native Android UI widgets by calling Android framework APIs through JNI.

The PoC should establish the foundation for a future cross-platform UI framework where UI and application logic live in Rust, while platform-specific code provides access to native widgets.

## 2. Goals

The PoC must demonstrate that Rust can:

1. Receive an Android `Activity` reference from Kotlin.
2. Create native Android `View` objects through JNI.
3. Configure their properties from Rust.
4. Assemble a native view hierarchy.
5. Display that hierarchy using `Activity.setContentView()`.
6. Handle a button click in Rust.
7. Update a native `TextView` from Rust in response to the click.

## 3. Non-goals

The PoC will not implement:

- A declarative UI API or widget framework.
- A virtual UI tree or reconciliation algorithm.
- Taffy or any other layout engine.
- Cross-platform abstractions or an iOS backend.
- State management, reactive streams, or Kotlin `StateFlow`.
- Navigation, styling systems, themes, or animations.
- A custom renderer or custom-drawn widgets.
- Production-grade lifecycle management.
- A general-purpose JNI abstraction layer.

These concerns should be considered only after the basic FFI approach works.

## 4. User story

As a Rust application developer, I want to construct a simple Android interface from Rust so that I can verify that a future Rust-first UI framework can use native Android widgets without implementing its UI logic in Kotlin.

## 5. Minimal UI

The application displays a vertically arranged native Android layout:

- A `TextView` displaying `Hello from Rust!`
- A `Button` displaying `Click me`
- A `TextView` displaying `Clicked 0 times`

When the user taps the button, Rust increments a counter and updates the second `TextView`.

All three widgets must be real Android framework views, not a WebView, a Flutter view, or a custom-drawn canvas.

## 6. Architecture

### Kotlin responsibilities

Kotlin is a thin platform entry point. It must:

- Provide the Android `Activity` to Rust.
- Load the compiled Rust native library.
- Declare the JNI entry point.
- Avoid constructing or updating the PoC's UI widgets itself.

### Rust responsibilities

Rust must:

- Receive the `Activity` reference.
- Create `LinearLayout`, `TextView`, and `Button` instances through JNI.
- Configure text and orientation.
- Add child views to the layout.
- Call `setContentView()` on the `Activity`.
- Maintain the click counter.
- Update the counter `TextView` when the button is tapped.

### Communication

```text
Kotlin Activity
      |
      | Calls nativeCreateUi(activity)
      v
Rust via JNI
      |
      | Creates Android objects
      v
Native Android View hierarchy
      |
      | Button click listener
      v
Rust callback
      |
      | Updates TextView
      v
Android UI updated
```

The initial UI creation must run on Android's main thread. Button callbacks and subsequent UI mutations must also run on the main thread.

## 7. Technical approach

- Use the existing Rust mobile tooling or a minimal Android/Rust project.
- Use the `jni` crate for direct JNI access.
- Compile Rust into an Android-compatible shared library (`.so`).
- Package the library in the Android application's native library directory for the target ABI.
- Load the library from Kotlin using `System.loadLibrary()`.
- Pass the current `Activity` into the native method.
- Use JNI to instantiate and configure Android framework classes.

For click handling, Rust must be able to register a Java-compatible listener that calls back into Rust. The implementation may use a small Kotlin listener adapter if necessary, but the click counter and UI-update logic must remain in Rust.

The PoC must not introduce UniFFI or another FFI framework unless direct JNI proves impractical.

## 8. Implementation milestones

### Milestone 1 — Native library integration

- Build a Rust shared library for Android.
- Package it into the application.
- Load it successfully from Kotlin.
- Call a native Rust method from the `Activity`.

**Acceptance criteria:** The app launches and successfully enters Rust through JNI.

### Milestone 2 — Create native views from Rust

- Instantiate a `LinearLayout`.
- Instantiate and configure two `TextView` instances and one `Button`.
- Add the widgets to the layout.
- Set the layout as the activity's content view.

**Acceptance criteria:** The expected UI appears, and all widgets are native Android views created and configured from Rust.

### Milestone 3 — Handle button clicks in Rust

- Register a click listener.
- Invoke Rust code when the button is tapped.
- Increment a Rust-owned counter.
- Update the counter `TextView`.

**Acceptance criteria:** Every tap increments the displayed count correctly without crashing or blocking the UI.

### Milestone 4 — Verify repeatability

- Test repeated button clicks.
- Check Android logs for JNI errors or uncaught exceptions.
- Verify the app remains responsive.

**Acceptance criteria:** The UI remains functional across repeated interactions and normal activity recreation during manual testing.

## 9. Constraints and implementation notes

- **Threading:** Android view creation and mutation must happen on the main thread.
- **JNI references:** Use local references only for short-lived JNI operations. Keep long-lived objects reachable through appropriate global references or Java-side ownership.
- **Callback lifetime:** Ensure listener objects and native state remain valid for as long as callbacks can occur.
- **Memory safety:** Do not retain local JNI references beyond their valid scope.
- **Exceptions:** Check for Java exceptions after JNI calls that may fail and surface useful diagnostics during development.
- **Lifecycle:** Do not assume a Rust-held reference to an `Activity` remains valid after recreation. The initial PoC should avoid unnecessary global ownership of the activity.
- **Scope:** Prefer straightforward JNI calls over designing a reusable abstraction prematurely.

## 10. Definition of done

The PoC is complete when:

- [ ] The Android app builds and launches.
- [ ] Kotlin successfully invokes Rust through JNI.
- [ ] Rust creates and configures the native view hierarchy.
- [ ] The UI is displayed using `Activity.setContentView()`.
- [ ] A native button click invokes Rust logic.
- [ ] Rust updates the displayed counter.
- [ ] No Kotlin UI-building logic is required, apart from any minimal listener adapter needed to bridge callbacks.
- [ ] The project includes concise build and run instructions.

## 11. Expected outcome

A working Android application that proves Rust can construct and manipulate a native Android UI through JNI.

The result is a technical validation, not yet a UI framework. If successful, the next step will be to introduce a minimal Rust `Element` representation and translate it into native Android views, keeping the working JNI implementation as the platform backend.
