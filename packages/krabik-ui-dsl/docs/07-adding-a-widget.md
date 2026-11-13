# 7. Adding a widget

A widget is one variant of `enum Widget` plus one file in each of three
places. Every place that must treat widgets differently is an exhaustive
`match`, so once the variant exists the compiler lists what is still missing.

This chapter follows `Switch`, the smallest widget with an event, from top to
bottom. To add your own, do the same steps with your names.

## Checklist

| Step | File | What you add |
|---|---|---|
| 1 | `rust/src/widgets/switch.rs` | Properties, builder, intrinsic size |
| 2 | `rust/src/widgets/mod.rs` | The `Widget` variant and its arms |
| 3 | `rust/src/host/events.rs` | Which events it reports |
| 4 | `shared/android/.../KrabikSwitch.kt` | Android: the native view (if a stock view is not enough) |
| 5 | `rust/src/android/` | Android: create and configure it |
| 6 | `rust/src/ios/` | iOS: create, configure and wire it |
| 7 | a demo page and tests | Proof that it works |

## 1. Properties and builder

`rust/src/widgets/switch.rs` holds everything about the widget that is not
platform-specific.

The **properties** are plain data, compared with `==` to detect changes:

```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchProps {
    pub checked: bool,
    /// Reported with the new state when the switch is toggled.
    pub on_toggle: Handler<bool>,
}
```

A `Handler<A>` is "what to tell the app when an event with payload `A`
happens". Two handlers compare equal when both are set or both unset, so a
new closure on every `view()` does not count as a change.

The **builder** is what apps write. It is generic over the app's message
type `M` and wraps an untyped element:

```rust
pub struct Switch<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Switch<M> {
    pub fn new(checked: bool) -> Self {
        Self(
            RawElement::leaf(ElementKind::Widget(Widget::Switch(SwitchProps::new(checked)))),
            PhantomData,
        )
    }

    pub fn on_toggle(mut self, message: impl Fn(bool) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        if let ElementKind::Widget(Widget::Switch(props)) = &mut self.0.kind {
            props.on_toggle = Handler::new(message);
        }
        self
    }
}

impl<M> From<Switch<M>> for Element<M> {
    fn from(component: Switch<M>) -> Self {
        Element::from_raw(component.0)
    }
}

// Adds padding, width, background, key and the other shared methods.
style_builder!(Switch);
```

The **intrinsic size** is the size the widget wants when the style gives
none:

```rust
impl SwitchProps {
    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        let size = measurer.switch_size();
        Size::new(max_width.map_or(size.width, |w| w.min(size.width)), size.height)
    }
}
```

## 2. The `Widget` variant

In `rust/src/widgets/mod.rs`, declare the module, add the variant, and fill
the three matches the compiler now complains about:

```rust
mod switch;
pub use switch::*;

pub enum Widget {
    // ...
    Switch(SwitchProps),
}

impl Widget {
    pub(crate) fn tag(&self) -> u64 {
        // A number that is never reused; it is part of a node's identity.
        Self::Switch(_) => 10,
    }

    pub(crate) fn map_messages(&mut self, wrap: &Wrap) {
        Self::Switch(props) => props.on_toggle.map(wrap),
    }

    pub(crate) fn intrinsic_size(&self, max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        Self::Switch(props) => props.intrinsic_size(max_width, measurer),
    }
}
```

`map_messages` is what lets `Element::map` reach the handler when the widget
sits inside a child component.

## 3. Events

`rust/src/host/events.rs` says which of the widget's handlers answers which
native event:

```rust
Widget::Switch(props) => actions.toggle = props.on_toggle.clone(),
```

The native events a platform can report are the variants of `NodeEvent`
(`Click`, `Text`, `Toggle`, `Select`, `Slider`, …). Reuse one when it fits;
add a variant only for a new kind of interaction.

At this point the widget works in unit tests: it lays out, diffs and resolves
events on your development machine.

## 4. Android: the native view

If a stock Android view does everything needed, skip this step. A switch
needs a listener that reports toggles and a way to set its state without
reporting it, so there is a small subclass in
`shared/android/src/main/java/dev/krabik/ui/KrabikSwitch.kt`:

```kotlin
class KrabikSwitch(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : Switch(context) {

    private var silent = false

    init {
        setOnCheckedChangeListener { _, isChecked ->
            if (!silent) root.onCheckedChanged(nodeId, isChecked)
        }
    }

    /** Applies the state from Rust without reporting it back. */
    fun setCheckedSilently(checked: Boolean) {
        if (isChecked == checked) return
        silent = true
        try { isChecked = checked } finally { silent = false }
    }
}
```

Two rules for every such class: the constructor takes
`(Context, RustRootView, Long)`, and anything Rust sets must not fire the
listener.

`root.onCheckedChanged` already exists in `RustRootView`. A new kind of event
needs a method there, a matching `external fun native…`, and an entry point
in `rust/src/android/entry.rs` that turns it into a `NodeEvent`.

## 5. Android: Rust

Four small additions in `rust/src/android/`:

```rust
// mod.rs: the class
pub(crate) const SWITCH_CLASS: &JNIStr = framework_class!("KrabikSwitch");

// handles.rs: look up the class, its constructor and its setter once
switch_ctor: widget_ctor(env, &switch, &widget_signature)?,
set_checked_silently: method!(env, &switch, "setCheckedSilently", "(Z)V", VOID),

// host.rs, in create_view: construct it
NodeKind::Widget(Widget::Switch(_)) => {
    ids.switch_ctor.construct(env, &ids.switch, &child_context)?
}

// widgets/switch.rs: apply the properties
pub(crate) fn update_switch(env: &mut Env<'_>, view: &JObject<'_>, props: &SwitchProps) -> JniResult<()> {
    jni_handles(env)?
        .set_checked_silently
        .call_void(env, view, &[JValue::Bool(props.checked)])
}

// widgets/mod.rs, in configure_view: call it
NodeKind::Widget(Widget::Switch(props)) => update_switch(env, view, props),
```

`configure_view` runs both when the view is created and whenever the node
changes, so there is one place that applies properties.

## 6. iOS: Rust

No Swift is needed. In `rust/src/ios/`:

```rust
// host.rs: what the view handle keeps for this widget
enum Content {
    // ...
    Switch(Retained<UISwitch>),
}

// host.rs, in create_handle: create, set the initial state, wire the event
NodeKind::Widget(Widget::Switch(props)) => {
    let switch = UISwitch::new(mtm);
    switch.setOn(props.checked);
    wire_switch(&switch, self.handle, self.host, node.id, mtm);
    let view = cast_view(switch.clone());
    ViewHandle::new(view, Content::Switch(switch), node)
}

// widgets/switch.rs: report user toggles
pub(crate) fn wire_switch(switch: &UISwitch, handle: u64, host: u64, id: NodeId, mtm: MainThreadMarker) {
    add_action(switch, UIControlEvents::ValueChanged, "krabik.switch", mtm, move |action| {
        let checked = /* read isOn from the sender */;
        schedule(move || dispatch_checked(handle, host, id, checked));
    });
}

// widgets/mod.rs, in update_content: apply changes
(Widget::Switch(props), Content::Switch(switch)) => switch.setOn(props.checked),
```

If the UIKit class is new to the project, enable its feature for
`objc2-ui-kit` in `rust/Cargo.toml`.

## 7. Prove it

- A unit test next to the widget for its properties and size.
- A page in the harness gallery (`harness/rust/src/lib.rs`) or the showcase
  (`example/rust/src/pages.rs`).
- A UI test that drives it on each platform
  (`harness/android/app/src/androidTest`, `harness/ios/UITests`).

## Where proc macros will fit

Steps 1 to 3 have the same shape for every widget: properties, a builder
method per property, a handler per event, three match arms. That uniform
shape is deliberate. A future `#[derive(Widget)]` on the properties struct
can generate the builder, the `From` impl and the `Widget` arms, leaving only
the intrinsic size and the two platform files to write by hand.

Next: [project layout, building and testing](08-project-layout.md).
