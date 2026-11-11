# Architecture

## 1. Overview

The framework aims to provide a way to build native mobile applications using Rust while retaining native platform UI components.

The core idea is to keep application logic, UI descriptions, state management and layout calculations in Rust, while delegating actual widget creation and platform-specific behaviour to the native operating systems.

The framework should not attempt to implement its own graphics renderer. Instead, it should use native platform widgets:

- Android: Android Views.
- iOS: UIKit Views.

The architecture is inspired by React Native, but with Rust as the primary language and runtime.

### Goals

- Rust-first application development.
- Native platform widgets rather than custom-drawn UI.
- Shared UI architecture between Android and iOS.
- Platform-independent layout calculations.
- Declarative UI descriptions.
- Incremental updates instead of recreating the entire view hierarchy.
- Minimal platform-specific code exposed to application developers.

### Non-goals

- Replacing native platform UI frameworks.
- Implementing a custom graphics engine.
- Recreating Flutter's rendering pipeline.
- Depending on Jetpack Compose or SwiftUI as the primary rendering layer.
- Providing complete feature parity with native UI frameworks initially.

---

## 2. High-level architecture

The framework is divided into several independent layers.

```text
┌──────────────────────────────────────────┐
│              Application                 │
│                                          │
│       Rust application code              │
├──────────────────────────────────────────┤
│              Public UI API               │
│                                          │
│       Column / Row / Text / Button        │
├──────────────────────────────────────────┤
│               UI Runtime                 │
│                                          │
│    UI Tree / State / Events / Diffing     │
├──────────────────────────────────────────┤
│              Layout Engine               │
│                                          │
│                   Taffy                  │
├──────────────────────────────────────────┤
│            Platform Renderer             │
│                                          │
│       Android          iOS               │
│       Renderer         Renderer          │
├──────────────────────────────────────────┤
│            Native Platform               │
│                                          │
│       Android Views    UIKit Views       │
└──────────────────────────────────────────┘
```

The Rust runtime should remain independent of the underlying platform.

Platform-specific implementations are responsible only for translating framework operations into native platform operations.

---

## 3. UI description

Applications describe their UI using Rust types and components.

The public API should be declarative and independent of the underlying layout engine.

For example:

```rust
Column::new()
    .spacing(16)
    .padding(16)
    .child(
        Text::new("Hello world")
    )
    .child(
        Row::new()
            .spacing(8)
            .child(Button::new("Cancel"))
            .child(Button::new("Save"))
    )
```

The public API should not expose Android-specific concepts such as `LinearLayout`, `FrameLayout` or `ConstraintLayout`.

Similarly, it should not expose Taffy's internal node identifiers or style structures.

The UI description is converted into an internal representation owned by the runtime.

### UI node categories

There are two primary types of nodes.

#### Layout nodes

Layout nodes describe positioning and structure. They do not necessarily correspond to native platform views.

Examples:

- Column
- Row
- Stack
- Padding
- Center
- Spacer

These nodes are represented in Rust and processed by Taffy.

#### Native nodes

Native nodes represent actual platform widgets.

Examples:

- Text
- Button
- Image
- TextField
- Switch
- ScrollView

These nodes are rendered using the corresponding native platform components.

This separation is important because not every layout operation requires an additional native view.

---

## 4. UI runtime

The UI runtime owns the application UI tree and coordinates updates.

Its responsibilities include:

- Maintaining the current UI tree.
- Managing stable node identifiers.
- Tracking state changes.
- Reconciling changes between UI descriptions.
- Coordinating layout calculations.
- Sending rendering operations to the platform renderer.
- Dispatching native events back to Rust.

### Internal representation

The runtime should use an intermediate UI representation rather than directly rendering public API objects.

Conceptually:

```rust
struct UiNode {
    id: NodeId,
    kind: NodeKind,
    style: LayoutStyle,
    properties: NodeProperties,
    children: Vec<UiNode>,
}
```

The actual implementation may use an arena or another indexed storage structure rather than recursively allocating every node.

Node identifiers must remain stable while a node represents the same logical element.

Stable identifiers are important for:

- Preserving native widget instances.
- Updating properties.
- Maintaining focus.
- Preserving scroll positions.
- Dispatching events.
- Avoiding unnecessary view recreation.

### Reconciliation

When the application updates its UI description, the runtime compares the new representation with the previous one.

It should produce a minimal set of operations:

- Create node.
- Remove node.
- Update properties.
- Update layout.
- Reorder children.

The runtime should avoid recreating native widgets when only their properties change.

For example, changing a Text node's content should update the existing native TextView rather than destroy and recreate it.

---

## 5. Layout engine

Taffy is responsible for calculating layout.

It provides a platform-independent layout implementation written in Rust.

The runtime translates its layout nodes and styles into Taffy nodes.

### Responsibilities

Taffy handles:

- Flexbox layout.
- Grid layout.
- Dimensions.
- Minimum and maximum dimensions.
- Padding.
- Margins.
- Alignment.
- Gaps.
- Positioning.

The initial implementation should focus on Flexbox.

### Layout process

```text
UI Tree
   |
   v
Convert nodes to Taffy nodes
   |
   v
Calculate available dimensions
   |
   v
Measure content-dependent nodes
   |
   v
Run Taffy layout
   |
   v
Read calculated layouts
   |
   v
Send layout updates to renderer
```

### Measurement

Taffy does not render content and cannot independently determine the intrinsic size of native platform widgets.

Some components require platform-specific measurement.

Text is the most important example.

The layout engine should support a measurement interface:

```rust
trait Measure {
    fn measure(
        &self,
        constraints: Constraints,
    ) -> MeasuredSize;
}
```

The Android implementation should use native text measurement APIs.

The iOS implementation should use UIKit text measurement.

Measurement should be cached where possible to avoid unnecessary platform calls.

The measurement interface must support constraints because text wrapping and intrinsic sizing depend on available width and height.

---

## 6. Platform renderer

The platform renderer translates the Rust UI tree and calculated layout into native widgets.

Each platform has its own implementation.

```text
                 Rust Runtime
                      |
                      v
              Platform Renderer API
                      |
             ┌────────┴────────┐
             |                 |
             v                 v
         Android             iOS
         Renderer          Renderer
             |                 |
             v                 v
       Android Views        UIKit Views
```

The renderer should expose a small set of operations:

```rust
trait Renderer {
    fn create_node(&mut self, node: &RenderNode);
    fn update_node(&mut self, node: &RenderNode);
    fn remove_node(&mut self, id: NodeId);

    fn update_layout(
        &mut self,
        id: NodeId,
        layout: LayoutResult,
    );

    fn handle_event(&mut self, event: PlatformEvent);
}
```

This is a conceptual interface. The actual implementation can use batched operations to reduce FFI overhead.

### Layout nodes versus native views

Layout-only nodes should not automatically create native platform widgets.

For example:

```text
Column
 ├── Text
 └── Row
      ├── Button
      └── Button
```

The Android implementation could produce:

```text
RustRootView : ViewGroup
 ├── TextView
 ├── Button
 └── Button
```

The Column and Row are represented only in the Rust UI tree and Taffy layout tree.

Their calculated positions are applied directly to the native widgets.

This avoids unnecessary native view nesting.

---

## 7. Android implementation

Android uses a custom ViewGroup as the root container.

```kotlin
class RustRootView(
    context: Context
) : ViewGroup(context) {

    override fun onMeasure(
        widthMeasureSpec: Int,
        heightMeasureSpec: Int
    ) {
        // Determine available dimensions.
        // Measure native children where necessary.
        // Coordinate with Rust layout.
    }

    override fun onLayout(
        changed: Boolean,
        left: Int,
        top: Int,
        right: Int,
        bottom: Int
    ) {
        // Apply calculated positions
        // to native children.
    }
}
```

The root ViewGroup is responsible for integrating the Rust layout engine with Android's view lifecycle.

### Native widgets

The initial widget mapping:

| Rust component | Android implementation        |
| -------------- | ----------------------------- |
| Text           | TextView                      |
| Button         | Native button implementation  |
| Image          | ImageView                     |
| TextField      | EditText                      |
| Switch         | SwitchCompat or native Switch |
| ScrollView     | ScrollView                    |
| Root           | Custom ViewGroup              |

The exact widget implementation should be replaceable without changing the public Rust API.

### JNI communication

Rust communicates with Android through JNI.

The Kotlin layer should remain relatively thin.

Its responsibilities include:

- Creating native widgets.
- Updating widget properties.
- Managing native widget instances.
- Applying layout coordinates.
- Forwarding user interactions.
- Providing native measurement information.

Avoid sending individual JNI calls for every property update.

Prefer batching operations where practical.

For example:

```text
Rust
  |
  | RenderBatch
  v
Kotlin Bridge
  |
  ├── Create TextView
  ├── Update Button
  ├── Update TextView
  └── Apply layout changes
```

All Android View mutations must happen on the main thread.

---

## 8. iOS implementation

The iOS renderer should follow the same architecture.

Instead of Android Views, it creates UIKit views.

Initial mapping:

| Rust component | iOS implementation |
| -------------- | ------------------ |
| Text           | UILabel            |
| Button         | UIButton           |
| Image          | UIImageView        |
| TextField      | UITextField        |
| Switch         | UISwitch           |
| ScrollView     | UIScrollView       |
| Root           | Custom UIView      |

The Rust UI tree, runtime and layout engine should not depend on UIKit.

Only the iOS renderer and measurement implementation should contain UIKit-specific code.

This allows the same Rust UI description to be rendered on both platforms.

---

## 9. Scrolling and native containers

Scrolling requires special treatment because it involves native platform behaviour.

The initial implementation should use native scroll containers.

For Android, this means ScrollView or an appropriate RecyclerView-based implementation later.

For iOS, this means UIScrollView or a collection-view-based implementation later.

A scrollable component should have a clearly defined viewport and content area.

Taffy should calculate the content layout, while the native container handles scrolling, gestures and platform-specific behaviour.

Virtualized lists should be considered a separate feature rather than implemented as a large collection of ordinary native views.

---

## 10. Events and state

User interactions originate from native widgets and are forwarded to Rust.

For example:

```text
User taps Button
       |
       v
Android OnClickListener
       |
       v
JNI callback
       |
       v
Rust event dispatcher
       |
       v
Application event handler
       |
       v
State update
       |
       v
UI reconciliation
       |
       v
Native view update
```

The Rust runtime should own application-facing event handling.

Native widgets should not contain application business logic.

The same event model should work on iOS.

---

## 11. Threading model

The initial implementation should use a simple threading model.

### Rust runtime thread

Responsible for:

- Application logic.
- UI descriptions.
- State updates.
- Reconciliation.
- Taffy layout calculations.

### Platform UI thread

Responsible for:

- Native widget creation.
- Native widget updates.
- Native measurement.
- User interaction callbacks.
- Platform lifecycle integration.

Native widget operations must always execute on the platform UI thread.

The initial implementation can perform UI reconciliation and layout synchronously, provided native operations are dispatched safely to the UI thread.

More advanced scheduling and asynchronous layout can be considered later.

---

## 12. Project structure

A possible workspace structure:

```text
framework/
├── crates/
│   ├── ui/
│   │   └── Public UI API
│   │
│   ├── runtime/
│   │   └── UI tree, state and reconciliation
│   │
│   ├── layout/
│   │   └── Taffy integration and measurement
│   │
│   ├── renderer/
│   │   └── Platform renderer interfaces
│   │
│   └── core/
│       └── Shared types and primitives
│
├── platforms/
│   ├── android/
│   │   ├── rust/
│   │   └── kotlin/
│   │
│   └── ios/
│       ├── rust/
│       └── swift/
│
└── examples/
    ├── hello-world/
    └── basic-layout/
```

This structure is illustrative. The initial proof of concept does not need all these crates.

---

## 13. Initial proof of concept

The first milestone should validate the most important architectural assumption:

**Can Rust create, position and update native Android Views through JNI?**

The proof of concept should be intentionally small.

### Scope

1. Create an Android Activity.
2. Install a custom RustRootView as its content view.
3. Initialize the Rust runtime.
4. Create a simple Rust UI tree.
5. Use Taffy to calculate layout.
6. Create native TextView and Button instances.
7. Apply calculated coordinates to the native widgets.
8. Update the TextView content from Rust.
9. Forward Button clicks back to Rust.

### Example UI

```text
┌──────────────────────────────┐
│                              │
│         Hello Rust!          │
│                              │
│       ┌──────────────┐       │
│       │    Click     │       │
│       └──────────────┘       │
│                              │
└──────────────────────────────┘
```

### Acceptance criteria

- Rust can initialize the UI runtime.
- Rust can describe a simple UI tree.
- Taffy can calculate the layout.
- Android creates actual native widgets.
- Widgets appear at the expected positions.
- Rust can update widget properties.
- Android interactions can trigger Rust callbacks.
- The application does not recreate widgets unnecessarily.

The first milestone does not need to support iOS, complex scrolling, animations or a complete widget library.

---

## 14. Future considerations

The architecture should leave room for additional capabilities.

### Widget library

More native components can be introduced over time:

- DatePicker
- Dialog
- BottomSheet
- Navigation components

### Virtualized lists

Large collections should eventually use native virtualization mechanisms.

### Accessibility

Native widgets should retain their platform accessibility behaviour wherever possible.

Layout-only nodes may need to contribute accessibility grouping and semantics.

### Animations

Animations should initially use native platform animation APIs where appropriate, while keeping their descriptions and lifecycle coordinated by Rust.

### Platform-specific extensions

The framework should allow platform-specific functionality without compromising the shared architecture.

Applications should be able to access native APIs when necessary.

---

## 15. Architectural decisions

| Decision                 | Choice               |
| ------------------------ | -------------------- |
| Primary language         | Rust                 |
| UI description           | Rust declarative API |
| UI runtime               | Custom Rust runtime  |
| Layout engine            | Taffy                |
| Android rendering        | Native Android Views |
| Android root             | Custom ViewGroup     |
| iOS rendering            | UIKit                |
| iOS root                 | Custom UIView        |
| Android interoperability | JNI                  |
| iOS interoperability     | FFI                  |
| Layout ownership         | Rust                 |
| Widget lifecycle         | Platform renderer    |
| State ownership          | Rust                 |

## Conclusion

The framework should use Rust as the owner of the UI runtime, layout system and application logic.

Taffy provides platform-independent layout calculations, while native platform renderers translate the resulting UI into Android Views and UIKit components.

This approach provides a shared foundation for Android and iOS without depending on Flutter, Jetpack Compose or SwiftUI as the primary UI implementation.

The first goal is not to build a complete cross-platform framework. It is to prove that Rust can own the UI tree and layout while native platform widgets handle actual rendering and interaction.
