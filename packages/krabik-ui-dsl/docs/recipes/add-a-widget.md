# Add a widget

This recipe adds a new leaf widget to the framework from the first line of Rust to the last UI
test. It uses one invented example all the way through: a `Stepper`, a minus/plus control that
holds a whole number. After following it you will know every file a widget touches, in which order
to touch them, what the compiler says at each step, and which mistakes the compiler cannot see.

The `Stepper` does not exist in the repository. Every code block that shows it is fenced as
`rust,ignore`, `kotlin` or `swift` and was written by tracing how the existing `Slider` is wired.
Blocks that quote existing code name the file they come from.

Background that this recipe does not repeat: what an `Element`, a `UiNode` and an `Op` are is in
[the engine chapters](../internals/README.md); how Rust reaches Kotlin and UIKit is in
[the platform chapters](../platforms/README.md).

## What a leaf widget is

A leaf widget is one native view with no children of its own. All leaf widgets are variants of one
enum, `Widget`, declared by the `widgets!` table in `rust/src/widgets/mod.rs`. Containers (`Column`,
`Row`, `ScrollView`, `Pager`, lists) are not in that enum; they are separate `ElementKind` and
`NodeKind` variants and are a different, larger job.

The path a widget takes through the code:

```text
 app code                 engine                       platform
 --------                 ------                       --------
 Stepper::new(3)   -->  Widget::Stepper(props)  -->  Op::Create / Op::Update
   (builder from          inside NodeKind::Widget       |
    #[derive(Widget)])    laid out with                 +--> android/widgets/stepper.rs
                          IntrinsicSize                 |      -> KrabikStepper.kt
                                                        +--> ios/widgets/stepper.rs
                                                               -> UIStepper

 Msg::Quantity(4)  <--  Actions::message(event)  <--  NodeEvent::Step(4)
   (your update)          host/events.rs               android/entry.rs / ios/events.rs
```

Going down, the properties travel inside the tree and each platform has one function that creates
the view and one that applies properties. Going up, the native control reports a `NodeEvent`, and
`host/events.rs` turns it into the application's message.

## The design of the example

Decide these before writing code. They determine how many files you touch.

| Question | Answer for `Stepper` | Consequence |
|---|---|---|
| Properties | `value`, `min`, `max`, `step`, `color`, `on_change` | one struct with `#[derive(Widget)]` |
| Does it report an event? | Yes, a new whole number | needs a handler and an event path |
| Does an existing `NodeEvent` fit? | No. `Select(usize)` cannot be negative, `Slider(f32)` snaps floats | a new `NodeEvent::Step(i32)` |
| Size | Fixed, like the spinner | a constant, no new `Measure` method |
| Android view | `android.widget` has no stepper | a small Kotlin class, `KrabikStepper` |
| iOS view | `UIStepper` | no Swift at all, only Rust through `objc2` |

If your widget reports nothing (like `Text` or `Spinner`), skip step 3's event parts, the Android
entry point, the `RustRootView` method and the iOS dispatch function. If an existing `NodeEvent`
fits, reuse it and its entry points: a second on/off control would reuse `NodeEvent::Toggle`,
`nativeOnCheckedChanged` and `dispatch_checked` and add no event code at all.

## The checklist

| # | File | What you add |
|---|---|---|
| 1 | `rust/src/widgets/stepper.rs` (new) | props, derive, `IntrinsicSize`, clamp helper |
| 2 | `rust/src/widgets/mod.rs` | `mod` + `pub use`, one line in `widgets!` |
| 3 | `rust/src/host/events.rs` | `NodeEvent::Step`, `StepperAction`, two match arms |
| 4 | `rust/src/android/mod.rs` | the class-name constant |
| 5 | `rust/src/android/widgets/stepper.rs` (new) | `update_stepper`, `create`, `Handles` |
| 6 | `rust/src/android/widgets/mod.rs` | `mod`, `use`, arms in `configure_view` and `create_widget` |
| 7 | `rust/src/android/handles.rs` | the `Handles` field in `Jni` and its `load` call |
| 8 | `rust/src/android/entry.rs` | `nativeOnStepperChanged` |
| 9 | `shared/android/src/main/java/dev/krabik/ui/KrabikStepper.kt` (new) | the view |
| 10 | `shared/android/src/main/java/dev/krabik/ui/RustRootView.kt` | `onStepperChanged` + `external fun` |
| 11 | `rust/Cargo.toml` | `"UIStepper"` in the `objc2-ui-kit` feature list (cosmetic, see below) |
| 12 | `rust/src/ios/host.rs` | `Content::Stepper` |
| 13 | `rust/src/ios/events.rs` | `dispatch_step` |
| 14 | `rust/src/ios/widgets/stepper.rs` (new) | `update_stepper`, `wire_stepper`, `create` |
| 15 | `rust/src/ios/widgets/mod.rs` | `mod`, `use`, arms in `update_content` and `create_widget` |
| 16 | `rust/src/ios/style.rs` | accessibility exception (only for a composite control) |
| 17 | `rust/src/ui.rs`, `rust/src/engine/runtime.rs` | unit tests |
| 18 | `example/rust/src/lib.rs`, `example/rust/src/pages.rs` | a showcase page |
| 19 | `example/android/.../ShowcaseTest.kt`, `example/ios/UITests/ShowcaseUITests.swift` | UI assertions |

Sixteen framework files (four new, twelve edited) and five test and showcase files.

The three compile commands used below. The Android and iOS renderers are behind
`#[cfg(target_os = ...)]` in `rust/src/lib.rs`, so a plain `cargo check` on your Mac compiles
neither of them:

```sh
cargo check --manifest-path rust/Cargo.toml                                   # shared code
cargo check --manifest-path rust/Cargo.toml --target aarch64-linux-android    # + Android
cargo check --manifest-path rust/Cargo.toml --target aarch64-apple-ios-sim    # + iOS
```

The CI workflow does the iOS one as `cargo clippy ... --target aarch64-apple-ios-sim` for the same
reason; its comment says the `check` job "never compiles" the iOS renderer.

## Step 1: the properties (`rust/src/widgets/stepper.rs`)

Model: `rust/src/widgets/slider.rs`. A widget file holds three things: the properties struct, the
`IntrinsicSize` implementation, and any pure helper both platforms need.

```rust,ignore
//! The `stepper` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::message::Handler;
use crate::style::Color;
use crate::widgets::IntrinsicSize;

/// Platform-independent properties of a stepper.
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = Stepper,
    about = "A native minus/plus control that holds a whole number.",
    new = "Creates a stepper with the given value, the range `0..=100` and a step of 1."
)]
pub struct StepperProps {
    /// The current value.
    #[widget(arg)]
    pub value: i32,
    /// The lowest value.
    #[widget(default = 0, set)]
    pub min: i32,
    /// The highest value.
    #[widget(default = 100, set)]
    pub max: i32,
    /// How much one press changes the value. Always at least 1.
    #[widget(default = 1, set(with = positive_step))]
    pub step: i32,
    /// Reports the new value when the user presses minus or plus.
    pub on_change: Handler<i32>,
    /// Color of the minus and plus symbols.
    #[widget(default = Color::ACCENT, set(into))]
    pub color: Color,
}

fn positive_step(step: i32) -> i32 {
    step.max(1)
}

impl StepperProps {
    /// The range with its bounds in order, whichever way they were given.
    pub(crate) fn range(&self) -> (i32, i32) {
        (self.min.min(self.max), self.max.max(self.min))
    }
}

/// Clamps a stepper value to its range.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn clamp_step(value: i32, min: i32, max: i32) -> i32 {
    value.clamp(min.min(max), max.max(min))
}

/// The size of a stepper: what `UIStepper` reports as its intrinsic size.
const STEPPER_WIDTH: f32 = 94.0;
const STEPPER_HEIGHT: f32 = 32.0;

impl IntrinsicSize for StepperProps {
    /// A stepper is a fixed-size control; an explicit size in the style
    /// still overrides it.
    fn intrinsic_size(&self, max_width: Option<f32>, _measurer: &dyn Measure) -> Size {
        Size::new(
            max_width.map_or(STEPPER_WIDTH, |width| width.min(STEPPER_WIDTH)),
            STEPPER_HEIGHT,
        )
    }
}
```

What the attributes do. The rules are in `rust/macros/src/widget.rs`; the summary on
`derive_widget` in `rust/macros/src/lib.rs` is the short version.

- `#[widget(name = Stepper)]` names the generated builder struct `Stepper<M>` and, unless
  `variant = ...` says otherwise, the `Widget` variant the builder constructs. The derive writes
  `crate::widgets::Widget::Stepper(StepperProps::new(value))`, so the variant in step 2 must have
  exactly this name.
- `#[widget(arg)]` makes `value` the argument of `Stepper::new(value)`.
- `#[widget(default = 0, set)]` gives the field its initial value and generates `.min(value)`.
- `set(with = positive_step)` passes the argument through a function before storing it. The slider
  does the same for its `step` (there together with `from = f32`, because its field is an
  `Option<f32>`).
- `on_change: Handler<i32>` needs no attribute. The derive recognises the type by its name and
  generates `.on_change(|value| message)`. A `Handler<()>` field would instead get
  `.on_click(message)`-style methods that take the message itself.
- `set(into)` on `color` makes the setter accept `impl Into<Color>`, so `0xFF00_897B` works.

The derive also writes `impl WidgetProps for StepperProps`: `map_messages` (calls `.map(wrap)` on
every `Handler` field, which is what makes `Element::map` work) and `resolve_colors` (calls
`.resolve(scheme)` on every `Color` field, which is what makes dark mode work). And it invokes
`style_builder!(Stepper)`, which adds `.padding()`, `.width()`, `.key()`, `.accessibility_label()`
and the rest of the shared methods.

What is hand-written and why:

- `IntrinsicSize`. The design rule is "macros generate boilerplate, not behaviour". How large
  a control wants to be is behaviour. The trait in `rust/src/widgets/mod.rs` says so: "Written by
  hand for every widget."
- `clamp_step` and `range`. The value is clamped in three places (when the event arrives, and when
  each platform configures its view), so the rule lives next to the properties. The slider has
  `snap_slider_value` for the same reason. The `cfg_attr(..., allow(dead_code))` is copied from
  there.

A rough edge to know about: the size is a constant. `94 x 32` is what I expect `UIStepper` to
report; confirm it on a simulator with `intrinsicContentSize()`. The switch does this properly:
`SwitchProps::intrinsic_size` asks `measurer.switch_size()`, and each platform's `Measure`
implementation measures a real control once and caches it (`AndroidMeasure::measure_switch`,
`IosMeasure::switch_size`). Following that route for the stepper means a new method on the
`Measure` trait in `rust/src/engine/measure.rs` with a default, plus an override and a cache in
`rust/src/android/measure.rs` and `rust/src/ios/measure.rs`. It is three more files; do it only if
the constant turns out to be wrong on some OS version.

**Compiler at this point:** nothing new compiles yet, because no `mod stepper;` exists.

## Step 2: the table (`rust/src/widgets/mod.rs`)

Three lines. Two next to their siblings at the top of the file:

```rust,ignore
mod stepper;
// ...
pub use stepper::*;
```

and one in the table:

```rust,ignore
// rust/src/widgets/mod.rs, inside `widgets! { ... }`
    /// A native tab bar.
    TabBar(TabBarProps) = 19,
    /// A native minus/plus control that holds a whole number.
    Stepper(StepperProps) = 21,
```

The `widgets!` macro, defined a few lines above the table, expands each line into a `Widget`
variant and into one arm of three matches: `tag`, `props` and `props_mut`. So the line is all the
"registration" the engine needs: `intrinsic_size`, `map_messages`, `resolve_colors` and
`text_style` on `Widget` all forward through `props()`.

**The tag rule.** The comment above the table reads:

```rust,ignore
// rust/src/widgets/mod.rs
// Tags must never be reused: they are part of a node's identity. The
// containers in `ElementKind` use 1, 2, 3, 8, 9, 11 and 20.
```

The tag is mixed into every node id (`node_id` in `rust/src/engine/tree.rs` hashes the parent id,
the tag, and the key or index). The widgets use 4 to 7, 10 and 12 to 19. The next free number is
21. Why uniqueness matters: if two variants shared a tag, replacing one with the other at the same
position would keep the same node id. The diff in `rust/src/engine/render.rs` decides "replace"
versus "update" by comparing `std::mem::discriminant` of the `NodeKind`, and every widget is the
same `NodeKind::Widget`, so it would emit an `Op::Update` carrying stepper properties for a view
that is a slider. Android would then call `KrabikStepper.configure` on a `KrabikSeekBar`. The
test `every_widget_tag_is_unique` in `rust/src/widgets/mod.rs` fails if a tag is reused, so run
the unit tests after adding the line.

**Re-exports.** There is nothing to do in `rust/src/ui.rs`. It contains
`pub use crate::widgets::*;`, and `widgets/mod.rs` now has `pub use stepper::*;`, so
`krabik_ui_dsl::ui::Stepper` and `krabik_ui_dsl::ui::StepperProps` exist, and `clamp_step` is
reachable inside the crate as `crate::ui::clamp_step` (the platform code imports
`snap_slider_value` the same way). The `view!` macro needs no registration either: it is pure
syntax, so `Stepper(self.quantity, min: 1, on_change: Msg::Quantity)` expands to the builder calls.

**Compiler at this point:** run the first command, `cargo check --manifest-path rust/Cargo.toml`.
You get one error, of this shape:

```text
error[E0004]: non-exhaustive patterns: `&Widget::Stepper(_)` not covered
  --> src/host/events.rs
```

If instead you see `the trait bound StepperProps: WidgetProps is not satisfied` pointing at the
`widgets!` invocation, the derive or the `IntrinsicSize` implementation is missing: `WidgetProps`
has `IntrinsicSize` as a supertrait, and the generated `props()` coerces `&StepperProps` to
`&dyn WidgetProps`.

## Step 3: the event (`rust/src/host/events.rs`)

This file turns "widget `id` did X" into an application message. It has two types. `NodeEvent` is
what a platform reports. `Actions` is what one widget wants to hear about, extracted from its
properties by `Actions::of`.

Add the event variant:

```rust,ignore
// rust/src/host/events.rs, in `enum NodeEvent`
    /// A stepper asked for a value, before clamping to the widget's range.
    Step(i32),
```

Add the action, modelled on `SliderAction`, which carries the handler together with the range the
value must be forced into:

```rust,ignore
// rust/src/host/events.rs
/// A stepper's handler and the range its values are clamped to.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct StepperAction {
    pub on_change: Handler<i32>,
    pub min: i32,
    pub max: i32,
}

// in `struct Actions`
    pub step: Option<StepperAction>,
```

Fill it in `Actions::of`. This is the arm the compiler asked for:

```rust,ignore
// rust/src/host/events.rs, in `Actions::of`
                Widget::Stepper(props) => {
                    actions.step = props.on_change.is_set().then(|| StepperAction {
                        on_change: props.on_change.clone(),
                        min: props.min,
                        max: props.max,
                    });
                }
```

The `is_set().then(...)` matters. `Actions::of` ends with
`(actions != Self::default()).then_some(actions)`: a widget with no handler must produce `None`.
An unconditional `Some(StepperAction { .. })` would make every stepper look like it has actions.

And produce the message in `Actions::message`, which matches on `NodeEvent` exhaustively too, so
the compiler asks for this arm as soon as the variant exists:

```rust,ignore
// rust/src/host/events.rs, in `Actions::message`
            NodeEvent::Step(value) => self.step.as_ref().and_then(|stepper| {
                let value = clamp_step(*value, stepper.min, stepper.max);
                stepper.on_change.message(value)
            }),
```

with `clamp_step` added to the import `use crate::ui::{..., snap_slider_value};` at the top.

Why clamp here and not trust the native control: the same reason the slider snaps here. The
application should never receive a value outside the range it declared, whatever a platform view
reports.

You do not touch `rust/src/host/session.rs`. `Session::node_event` looks up the node's actions
(from the tree, or from the recorded row actions when the widget is inside a recycled list) and
calls `actions.message(&event)`. Its own match on the event has a `_ => {}` arm because only
`Scroll` and `Focus` change engine state.

**Compiler at this point:** the first command passes. The shared code is done; `cargo test` on the
host runs.

## Step 4 to 10: Android

Run the second command now. It reports two errors, both in `rust/src/android/widgets/mod.rs`:
non-exhaustive patterns in `configure_view` (`&NodeKind::Widget(Widget::Stepper(_))` not covered)
and in `create_widget` (`&Widget::Stepper(_)` not covered). Everything below exists to write those
two arms.

### Step 4: the class constant (`rust/src/android/mod.rs`)

```rust,ignore
// rust/src/android/mod.rs, next to SEEK_BAR_CLASS
pub(crate) const STEPPER_CLASS: &JNIStr = framework_class!("KrabikStepper");
```

`framework_class!` prefixes the Kotlin package (`dev/krabik/ui/`), which is spelled in that file
and nowhere else in Rust.

### Step 5: the widget module (`rust/src/android/widgets/stepper.rs`)

Model: `rust/src/android/widgets/slider.rs`. Three items: the function that applies properties,
the function that constructs the view, and the JNI handles the two need.

```rust,ignore
//! The `stepper` widget on Android.

use crate::android::STEPPER_CLASS;
use crate::android::handles::{Method, VOID, class, get as jni_handles, method, widget_ctor};
use crate::android::widgets::{Ctor, Site};
use crate::ui::StepperProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

pub(crate) fn update_stepper(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &StepperProps,
) -> JniResult<()> {
    let (low, high) = props.range();
    jni_handles(env)?.stepper.configure.call_void(
        env,
        view,
        &[
            JValue::Int(props.value.clamp(low, high)),
            JValue::Int(low),
            JValue::Int(high),
            JValue::Int(props.step),
            JValue::Int(props.color.argb() as jint),
        ],
    )
}

/// Creates the native view of a stepper.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = &jni_handles(env)?.stepper;
    site.framework(env, &ids.class, &ids.ctor)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) ctor: Ctor,
    pub(crate) configure: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let stepper = class(env, STEPPER_CLASS)?;
        Ok(Self {
            ctor: widget_ctor(env, &stepper, widget_signature)?,
            configure: method!(env, &stepper, "configure", "(IIIII)V", VOID),
            class: stepper,
        })
    }
}
```

Points worth understanding:

- `create` takes no properties. On Android the view is constructed bare and then configured by
  `configure_view`, the same call an update makes. `AndroidState::create_view` in
  `rust/src/android/host.rs` does `create_widget(...)` and then, for every node kind,
  `configure_view(env, &view, &node.kind, self.density)?`. That shared path is the fix from commit
  `e000272`, when five widgets configured themselves only on creation and ignored later changes.
- `site.framework(...)` calls the constructor every framework widget has:
  `(Context, RustRootView, long nodeId)`. The view keeps the root view and its node id and reports
  its own events through them. `site.stock(...)` is the other option, for an unmodified Android
  class with a `(Context)` constructor that reports nothing (`ProgressBar`).
- `widget_ctor` resolves that constructor from the shared signature that `Jni::load` builds once
  (`"(Landroid/content/Context;{root}J)V"`).
- `"(IIIII)V"` is the JNI signature of `configure(Int, Int, Int, Int, Int): Unit`. It is a string.
  Nothing checks it against the Kotlin source until the app starts. See the last section.
- The colour is already resolved for light or dark when it gets here; `argb()` is all a renderer
  ever calls.

### Step 6: the two arms (`rust/src/android/widgets/mod.rs`)

```rust,ignore
// rust/src/android/widgets/mod.rs
use crate::android::widgets::stepper::update_stepper;
// ...
pub(crate) mod stepper;

// in `configure_view`
        NodeKind::Widget(Widget::Stepper(props)) => update_stepper(env, view, props),

// in `create_widget`
        Widget::Stepper(_) => stepper::create(env, site),
```

**Compiler at this point:** the match errors are gone and a new one appears in
`widgets/stepper.rs`: `no field stepper on type &Jni` (E0609).

### Step 7: register the handles (`rust/src/android/handles.rs`)

Two lines, one in the `Jni` struct under the comment "Handles owned by one widget live in that
widget's module", one in `Jni::load`:

```rust,ignore
// rust/src/android/handles.rs, in `struct Jni`
    pub(crate) stepper: crate::android::widgets::stepper::Handles,

// in `Jni::load`
            stepper: crate::android::widgets::stepper::Handles::load(env, &widget_signature)?,
```

If you add only the struct field, the compiler reports `missing field stepper in initializer of
Jni` (E0063). Why the handles are loaded eagerly, all at once, is explained at the top of that
file: `FindClass` resolves names through the class loader of the calling native method, and
`nativeCreate` is the call that is known to come from the app's own loader.

**Compiler at this point:** the second command passes. Rust can create and configure a view it has
never seen. The event path and the Kotlin class are still missing, and the compiler does not know.

### Step 8: the entry point (`rust/src/android/entry.rs`)

Model: `native_on_slider_changed`.

```rust,ignore
// rust/src/android/entry.rs
/// A stepper asked for a new value.
#[unsafe(export_name = native_name!("nativeOnStepperChanged"))]
pub extern "system" fn native_on_stepper_changed<'caller>(
    mut unenv: EnvUnowned<'caller>,
    _this: JObject<'caller>,
    handle: jlong,
    host: jlong,
    node_id: jlong,
    value: jint,
) {
    run(&mut unenv, "nativeOnStepperChanged", |env| {
        on_node_event(env, handle, host, node_id, NodeEvent::Step(value))
    });
}
```

`native_name!` produces `Java_dev_krabik_ui_RustRootView_nativeOnStepperChanged`, the symbol the
JVM looks up for an `external fun` of that name on `RustRootView`. `run` keeps a Rust panic or JNI
error from reaching the JVM. `on_node_event` takes the session lock with `try_lock` and hands the
event to `Session::node_event`; the reason for `try_lock` is in
[Debug a render](debug-a-render.md#an-android-event-that-never-arrives).

### Step 9: the view (`shared/android/src/main/java/dev/krabik/ui/KrabikStepper.kt`)

Model: `KrabikSeekBar.kt` for the shape (constructor, `configure`), `KrabikRadioGroup.kt` for a
view made of several children.

```kotlin
package dev.krabik.ui

import android.content.Context
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView

/**
 * The native widget used for `Stepper` nodes: a minus half and a plus half.
 *
 * The widget does not change its own value. A press reports the value the
 * user asked for; the value shown changes when Rust applies it through
 * [configure]. So there is no programmatic change that could echo back.
 */
class KrabikStepper(
    context: Context,
    private val root: RustRootView,
    private val nodeId: Long,
) : LinearLayout(context) {

    private var value = 0
    private var minValue = 0
    private var maxValue = 100
    private var stepValue = 1

    private val minus = half("−", "Decrease") { report(value - stepValue) }
    private val plus = half("+", "Increase") { report(value + stepValue) }

    init {
        orientation = LinearLayout.HORIZONTAL
        addView(minus, LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f))
        addView(plus, LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f))
    }

    /** Applies the value, range, step and symbol color from Rust. */
    fun configure(value: Int, min: Int, max: Int, step: Int, color: Int) {
        this.value = value
        minValue = min
        maxValue = max
        stepValue = step
        minus.setTextColor(color)
        plus.setTextColor(color)
        refresh()
    }

    /** A group does not pass its enabled state on to its children by itself. */
    override fun setEnabled(enabled: Boolean) {
        super.setEnabled(enabled)
        refresh()
    }

    private fun report(candidate: Int) {
        val next = candidate.coerceIn(minValue, maxValue)
        if (next != value) {
            root.onStepperChanged(nodeId, next)
        }
    }

    private fun refresh() {
        minus.isEnabled = isEnabled && value > minValue
        plus.isEnabled = isEnabled && value < maxValue
        minus.alpha = if (minus.isEnabled) 1f else 0.4f
        plus.alpha = if (plus.isEnabled) 1f else 0.4f
    }

    private fun half(symbol: String, description: String, onPress: () -> Unit): TextView =
        TextView(context).apply {
            text = symbol
            contentDescription = description
            gravity = Gravity.CENTER
            setTextSize(TypedValue.COMPLEX_UNIT_SP, 20f)
            isClickable = true
            isFocusable = true
            val ripple = TypedValue()
            context.theme.resolveAttribute(
                android.R.attr.selectableItemBackground, ripple, true,
            )
            setBackgroundResource(ripple.resourceId)
            setOnClickListener { onPress() }
        }
}
```

Compare with `KrabikSeekBar` and `KrabikSwitch`. Those wrap a native control that changes itself
when touched, so they need a `silent` flag: `setValueSilently` and `setCheckedSilently` set it
while Rust applies a value, and the listener ignores changes made under it. Without the flag every
render would echo back as a user event. `KrabikStepper` owns its two halves and never changes
`value` on a press, so it has nothing to silence. If you wrap a real stateful Android control,
copy the `silent` pattern.

The frame comes from Rust. `RustRootView.onMeasure` measures each child at exactly the size Rust
calculated, so the two halves share whatever `intrinsic_size` (or the app's `.width()`) decided.
Do not set a size in Kotlin.

### Step 10: the bridge (`shared/android/src/main/java/dev/krabik/ui/RustRootView.kt`)

Two additions, next to their slider twins:

```kotlin
    /** Forwards the value a stepper asked for back to Rust. */
    fun onStepperChanged(nodeId: Long, value: Int) {
        if (handle != 0L) {
            nativeOnStepperChanged(handle, hostId, nodeId, value)
        }
    }

    // further down, with the other declarations
    private external fun nativeOnStepperChanged(handle: Long, host: Long, nodeId: Long, value: Int)
```

The forwarding method adds the session handle and the host id, which the widget does not know.

**Compiler at this point:** Gradle compiles the Kotlin. It checks that `KrabikStepper` calls a
method that exists on `RustRootView`. It does not check that the `external fun` has a Rust
implementation, or that `configure` has the signature the Rust string claims.

## Step 11 to 16: iOS

Run the third command. Two errors, both in `rust/src/ios/widgets/mod.rs`: non-exhaustive patterns
in `update_content` (`(&Widget::Stepper(_), _)` not covered) and in `IosState::create_widget`.

### Step 11: the feature list (`rust/Cargo.toml`)

The `objc2-ui-kit` dependency lists the UIKit classes the renderer uses. Add `"UIStepper"` after
`"UISlider"` to keep the list honest.

Be aware that the list does not gate anything today. The dependency does not set
`default-features = false`, and the default features of `objc2-ui-kit` 0.3 enable every class.
`rust/src/ios/host.rs` already imports `UIStackView`, `UISegmentedControl` and
`UIActivityIndicatorView`, none of which are in the list. So forgetting this step compiles. It
would only start to matter if someone turns the default features off to cut build time.

### Step 12: the typed content (`rust/src/ios/host.rs`)

iOS keeps the concrete UIKit object next to each view so an update does not have to downcast. The
doc comment on `Content` gives the reason: "Keeping the concrete class here avoids downcasting on
every update."

```rust,ignore
// rust/src/ios/host.rs, in `enum Content`
    Stepper(Retained<UIStepper>),
```

plus `UIStepper` in the `use objc2_ui_kit::{...}` list of that file. The two methods on `Content`
(`inset_handle`, `children_host`) have `_ =>` arms and need no change: a stepper is neither inset
in a wrapper nor a host for children.

### Step 13: the dispatch function (`rust/src/ios/events.rs`)

```rust,ignore
// rust/src/ios/events.rs
pub(crate) fn dispatch_step(handle: u64, host: u64, id: NodeId, value: i32) {
    dispatch(handle, host, id, NodeEvent::Step(value));
}
```

`dispatch` locks the session and calls `Session::node_event`, the same function the Android entry
point reaches. There is no C entry point and no change to `shared/ios/KrabikIOS.h`: on iOS a
widget's event handler is a Rust closure attached to the control, so the event never crosses into
Swift.

### Step 14: the widget module (`rust/src/ios/widgets/stepper.rs`)

Model: `rust/src/ios/widgets/slider.rs`.

```rust,ignore
//! The `stepper` widget on iOS.

use crate::ios::events::dispatch_step;
use crate::ios::widgets::{Content, RenderNode, Site};
use crate::ios::{ViewHandle, cast_view};
use crate::ios::{add_action, color, schedule};
use crate::tree::NodeId;
use crate::ui::StepperProps;
use objc2::MainThreadMarker;
use objc2_ui_kit::{UIControlEvents, UIStepper};

pub(crate) fn update_stepper(stepper: &UIStepper, props: &StepperProps) {
    let (low, high) = props.range();
    // Minimum first: UIKit raises the maximum to match a larger minimum,
    // but rejects a maximum below the current minimum.
    stepper.setMinimumValue(f64::from(low));
    stepper.setMaximumValue(f64::from(high));
    stepper.setStepValue(f64::from(props.step));
    let value = f64::from(props.value.clamp(low, high));
    if (stepper.value() - value).abs() > f64::EPSILON {
        stepper.setValue(value);
    }
    let tint = color(props.color);
    // SAFETY: the color is not nil and is retained by the view.
    unsafe { stepper.setTintColor(Some(&tint)) };
}

pub(crate) fn wire_stepper(
    stepper: &UIStepper,
    handle: u64,
    host: u64,
    id: NodeId,
    mtm: MainThreadMarker,
) {
    add_action(
        stepper,
        UIControlEvents::ValueChanged,
        "krabik.stepper",
        mtm,
        move |action| {
            let value = action
                .sender()
                .and_then(|sender| sender.downcast::<UIStepper>().ok())
                .map(|stepper| stepper.value())
                .unwrap_or(0.0);
            schedule(move || dispatch_step(handle, host, id, value.round() as i32));
        },
    );
}

/// Creates the native view of a stepper.
pub(crate) fn create(node: &RenderNode, site: &Site) -> ViewHandle {
    let stepper = UIStepper::new(site.mtm);
    wire_stepper(&stepper, site.handle, site.host, site.node, site.mtm);
    let view = cast_view(stepper.clone());
    ViewHandle::new(view, Content::Stepper(stepper), node)
}
```

The differences from Android are structural, not cosmetic:

- `create` only builds the view and wires its events. `IosState::create_handle` in
  `rust/src/ios/host.rs` calls `update_content` right after it, the same function a later change
  goes through, so the properties are applied in one place. What you must not forget is the arm
  for your widget in `update_content` (`rust/src/ios/widgets/mod.rs`); without it the control
  keeps UIKit's defaults.
- `wire_stepper` attaches the event closure once, at creation. The closure captures the session
  handle, host and node id from the `Site`.
- `schedule(...)` defers the dispatch to the next main-queue turn. The comment on `schedule` in
  `rust/src/ios/mod.rs` explains why: UIKit can deliver an action synchronously while Rust is
  rebuilding the tree, and the state lock must not be re-entered.
- Setting `value` from code does not send `ValueChanged`, so there is no echo to suppress.
- `setStepValue` must be positive, which is why `positive_step` exists in step 1.

### Step 15: the two arms (`rust/src/ios/widgets/mod.rs`)

```rust,ignore
// rust/src/ios/widgets/mod.rs
use crate::ios::widgets::stepper::update_stepper;
// ...
pub(crate) mod stepper;

// in `update_content`, with the other pairs
            (Widget::Stepper(props), Content::Stepper(stepper)) => update_stepper(stepper, props),

// in `update_content`, in the "does not belong to this widget" arm
                | Widget::Spinner(_)
                | Widget::Stepper(_),
                _,
            ) => {}

// in `IosState::create_widget`
            Widget::Stepper(props) => stepper::create(props, node, site),
```

`update_content` matches on the pair `(widget, content)`. The first new arm handles the pair that
creation produced. The second lists every widget with any other content and does nothing; its
comment says a mismatch "means the node changed kind, which the engine handles by recreating the
view". You need both. Read the warning about this arm in the last section.

### Step 16: accessibility of a composite control (`rust/src/ios/style.rs`)

Skip this step for a control that is one accessibility element (a slider, a switch). `UIStepper`
is two buttons inside one view, and `apply_accessibility` does this when the node has an
accessibility label:

```rust,ignore
// rust/src/ios/style.rs, in `apply_accessibility`
        // A radio group must stay a plain container so its options remain
        // individually reachable by accessibility services.
        view.setIsAccessibilityElement(
            !matches!(
                node.kind,
                NodeKind::Widget(Widget::RadioGroup(_)) | NodeKind::Widget(Widget::TabBar(_))
            ),
            mtm,
        );
```

A stepper with `.accessibility_label("Quantity")` would become one opaque element, and neither
VoiceOver nor XCUITest could reach its minus and plus halves. Add
`| NodeKind::Widget(Widget::Stepper(_))` to that `matches!`. Nothing forces you to: it is a
`matches!`, not an exhaustive `match`.

**Compiler at this point:** the third command passes.

## What the compiler points at when you add the enum variant

These are all the exhaustive matches on `Widget` in the crate (no `_ =>` arm), found by searching
for `Widget::` across `rust/src`. Adding `Stepper(StepperProps) = 21` breaks each hand-written one.

| Where | Function | Target that compiles it | What it is for |
|---|---|---|---|
| `rust/src/widgets/mod.rs` | `Widget::tag` | all | generated by `widgets!`; nothing to do |
| `rust/src/widgets/mod.rs` | `Widget::props`, `props_mut` | all | generated; requires `StepperProps: WidgetProps` |
| `rust/src/host/events.rs` | `Actions::of` | all | which events the widget reports |
| `rust/src/android/widgets/mod.rs` | `configure_view` | Android | apply properties, on create and on update |
| `rust/src/android/widgets/mod.rs` | `create_widget` | Android | construct the view |
| `rust/src/ios/widgets/mod.rs` | `update_content` | iOS | apply properties on update |
| `rust/src/ios/widgets/mod.rs` | `IosState::create_widget` | iOS | construct, configure and wire the view |

Five hand-written sites, in three files, across three compilation targets. One more exhaustive
match appears only if you add a `NodeEvent` variant: `Actions::message` in
`rust/src/host/events.rs`.

Places that mention specific widgets but are *not* exhaustive, so they stay silent:

| Where | What it special-cases | Do you need it? |
|---|---|---|
| `rust/src/ios/style.rs`, `apply_accessibility` | `RadioGroup`, `TabBar` stay containers | yes, for a composite control (step 16) |
| `rust/src/android/style.rs`, `apply_style` | `ripple` only for `Button` | if the widget should show a press ripple on its background |
| `rust/src/engine/layout.rs`, `taffy_style` | `Text` may shrink below its content width | if the widget wraps its content |
| `rust/src/engine/layout.rs`, `content_size` | `Image` scales from its natural size | if the size depends on the node's style |
| `rust/src/ios/style.rs`, `apply_style` | `fills_frame` for a filling `Image` | no |
| `rust/src/ios/host.rs`, `Op::Update` | `Image` updates need the state for downloads | only if an update needs `IosState` itself |

## Step 17: unit tests

A builder test in the `tests` module of `rust/src/ui.rs`, next to `slider_step_is_optional`:

```rust,ignore
    #[test]
    fn stepper_keeps_its_range_and_a_positive_step() {
        let element: Element = Stepper::new(3).min(1).max(9).step(0).into();
        let ElementKind::Widget(Widget::Stepper(props)) = &element.kind else {
            panic!("expected stepper");
        };
        assert_eq!((props.value, props.min, props.max), (3, 1, 9));
        assert_eq!(props.step, 1);
        assert!(!props.on_change.is_set());

        assert_eq!(clamp_step(12, 1, 9), 9);
        // Bounds given the wrong way round still clamp.
        assert_eq!(clamp_step(0, 9, 1), 1);
    }
```

An event test in the `tests` module of `rust/src/engine/runtime.rs`, next to
`switches_report_toggles`. It uses that module's `actions` and `viewport` helpers and needs
`NodeEvent` imported from `crate::host`:

```rust,ignore
    #[test]
    fn steppers_report_clamped_values() {
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let element: Element<i32> = Column::new()
            .child(Stepper::new(3).max(5).on_change(|value| value))
            .into();
        runtime.update(&element, viewport());

        let stepper = runtime.tree().children[0].children[0].id;
        let message = actions(&runtime, stepper)
            .message(&NodeEvent::Step(8))
            .expect("the stepper reports changes");
        assert_eq!(message.downcast_ref::<i32>(), Some(&5));

        // Without a handler the widget has no actions at all.
        let silent: Element = Column::new().child(Stepper::new(3)).into();
        runtime.update(&silent, viewport());
        assert_eq!(actions(&runtime, stepper), Actions::default());
    }
```

The second half guards the `is_set().then(...)` rule from step 3. How these helpers work is in
[Write tests](write-tests.md).

## Step 18: a showcase page (`example/rust`)

The UI tests drive the example app, so the widget needs a page there. The example keeps one page
per widget. The sites, all modelled on the slider page:

In `example/rust/src/lib.rs`: a message variant `Quantity(i32)` next to `Volume(f32)`; a field
`quantity: i32` on `ShowcaseApp`; its initial value (`quantity: 2`) in `ShowcaseApp::new`; and the
arm `Msg::Quantity(value) => self.quantity = value` in `update`. The compiler asks for the
initial value (E0063) and the `update` arm (E0004) once the variant and the field exist.

In `example/rust/src/pages.rs`: `Stepper` in the `use krabik_ui_dsl::ui::{...}` list, then four
string tables and the page itself:

```rust,ignore
// screen_key
        "stepper" => "stepper",
// title
        "stepper" => "Stepper",
// screen
        "stepper" => stepper_page(app),
// home, in the "Values" group
                ("Stepper", "page:stepper"),

fn stepper_page(app: &ShowcaseApp) -> Element<Msg> {
    page(
        "Stepper",
        "A stepper changes a whole number one step at a time, inside its range.",
        body![
            Text::new(format!("Quantity: {}", app.quantity))
                .size(18)
                .color(COLOR_TEXT),
            Stepper::new(app.quantity)
                .min(1)
                .max(5)
                .on_change(Msg::Quantity)
                .accessibility_label("Quantity"),
            caption("The range is 1 to 5; the buttons stop at its ends."),
        ],
    )
}
```

None of those four tables is checked. They match on strings and all have a fallback:
`screen_key` returns `None` (and `page_button` then turns the catalog entry into
`Msg::OpenDialog`, so a typo makes the button open the demo dialog), `title` falls back to
`"Krabik UI DSL"`, `screen` returns `None`. Test the page by opening it.

## Step 19: UI assertions

Android, in `catalogsNewControls` in
`example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/ShowcaseTest.kt`, using the
helpers from `UiHelpers.kt`. It needs `isDescendantOfA` from `ViewMatchers` and `allOf` from
Hamcrest:

```kotlin
            openPage("Stepper")
            onView(withText("Quantity: 2")).check(matches(isDisplayed()))
            val increase = allOf(
                withContentDescription("Increase"),
                isDescendantOfA(withContentDescription("Quantity")),
            )
            onView(increase).perform(click())
            onView(withText("Quantity: 3")).check(matches(isDisplayed()))
            // The range stops at 5.
            repeat(4) { onView(increase).perform(click()) }
            onView(withText("Quantity: 5")).check(matches(isDisplayed()))
            onView(increase).check(matches(not(isEnabled())))
            tapBack()
```

The outer content description is the node's accessibility label, which `apply_accessibility` in
`rust/src/android/style.rs` sets on the widget's view. The inner one is set by `KrabikStepper`.

iOS, in `testNewWidgets` in `example/ios/UITests/ShowcaseUITests.swift`:

```swift
        open("Stepper")
        XCTAssertTrue(app.staticTexts["Quantity: 2"].waitForExistence(timeout: 5))
        let increment = app.buttons["Increment"]
        XCTAssertTrue(increment.waitForExistence(timeout: 5))
        increment.tap()
        XCTAssertTrue(app.staticTexts["Quantity: 3"].waitForExistence(timeout: 5))
        backToCatalog()
```

`"Increment"` is the label I expect UIKit to give the plus half in English; check it in the
accessibility inspector before relying on it. This assertion is also what tells you step 16 was
needed: without it the query finds no button.

## Verify

In this order, cheapest first:

1. `cargo test --manifest-path rust/Cargo.toml --workspace`: the two unit tests, and everything
   else still passes.
2. `cargo test --manifest-path example/rust/Cargo.toml`: the example's own tests.
3. The three `cargo check` commands from the top of this recipe, or let the app builds do it.
4. Android: `cd example/android && ./gradlew :app:installDebug -PkrabikAbis=arm64-v8a`, open the
   page, press both halves, rotate, switch to dark mode (the symbols must change colour, which
   proves `resolve_colors`).
5. iOS: run the example scheme in Xcode, same checks.
6. The two UI test methods; [Write tests](write-tests.md#running-one-class-or-one-method) shows
   how to run only those.
7. Put a stepper inside a `List::recycled()` row once. Row widgets go through the same
   `apply_op`/`apply_to` code, but their handlers are found through `row_actions`, and that is a
   path the showcase page above does not exercise.

## What the compiler will not tell you

This is the list to reread before calling the widget done. Each item compiles cleanly when
forgotten.

**A unique tag.** The compiler does not check that `21` is unused: the macro emits one match arm
per variant returning a number, and two arms returning the same number are valid Rust. The
consequence is described in step 2. A unit test does check it (`every_widget_tag_is_unique` in
`rust/src/widgets/mod.rs`), so this one is caught by `cargo test`, not by `cargo check`.

**The "does nothing" arms.** Two exhaustive matches have an arm that lists widgets and does
nothing: `Widget::Text(_) | Widget::Progress(_) | Widget::Spinner(_) | Widget::Image(_) => {}` in
`Actions::of`, and the content-mismatch arm in iOS `update_content`. Adding your variant to either
list satisfies the compiler completely. In the first case the widget never reports an event. In
the second, if you forget the real `(Widget::Stepper(props), Content::Stepper(stepper))` arm, iOS
creates the widget correctly and then ignores every later change. That is precisely the bug the
exhaustive match was introduced to prevent on Android, and on iOS it is one careless line away.

**Android and iOS are not compiled on the host.** `cargo check` and `cargo test` on a Mac or in
the `check` CI job see only step 1 to 3. A widget can pass every host test with no platform code
at all.

**iOS creation and update share one path, like Android's.** `create_handle` calls
`update_content` after every `create`, so a widget's properties are applied by one function. The
arm in `update_content` is still yours to add.

**Every JNI name and signature is a string.** `"configure"`, `"(IIIII)V"`, `"KrabikStepper"`, the
constructor signature. A mismatch with the Kotlin source is found at run time, and the failure is
far from the cause: `Jni::load` resolves all handles for all widgets inside `nativeCreate`. If one
lookup fails, `run` in `rust/src/android/mod.rs` logs and returns the default value, which for
`nativeCreate` is the handle `0`. `RustRootView` refuses to continue with `0` and throws, so the
app stops at launch with a message pointing at the log, not only the new widget. Run
`adb logcat -s krabik`: the failed lookup is named there ("JNI lookup failed for method
setValueSilently (F)V"), next to the Java exception (`NoSuchMethodError` or
`ClassNotFoundException`).

**The `external fun` and its Rust symbol.** If `nativeOnStepperChanged` exists in Kotlin but the
Rust function is missing or misnamed, the app starts and renders. It throws
`UnsatisfiedLinkError` the first time the user presses the control. If the argument lists differ
(say `Int` in Kotlin, `jfloat` in Rust), nothing fails; the value is garbage.

**Minification.** Rust finds Kotlin classes and methods by name. The apps set
`isMinifyEnabled = false` and the library ships no keep rules, so an app that turns R8 on would
break every one of these lookups.

**The derive matches types by their written name.** In `rust/macros/src/widget.rs`, a field is a
colour when the last segment of its type is `Color`, directly or inside an `Option` or a `Vec`.
A type alias for `Color` is not recognised and is not resolved for dark mode, silently. A handler
is recognised by the last path segment being `Handler`, so a type alias for a handler is not
mapped by `Element::map`, silently. Spell the types the way the existing widgets do.

**A field with no attribute.** A plain field that is not a `Handler` is initialised with
`Default::default()` and gets no setter. Forgetting `set` gives a property nobody can change.

**The iOS accessibility exception** (step 16) and the other `matches!` sites in the second table
above.

**The Cargo feature list** (step 11) is not enforced.

**The example's string tables** (step 18).

**A control that changes itself has to be set back when the application disagrees.**
`UIStepper` changes its own value when pressed, then Rust is told, the application updates, and
the next render applies the value the application now holds. If the application does *not* change
its state (it ignores the message, or clamps the value back to what it was), the rebuilt tree
equals the old one. The session covers this: after a value event (`Toggle`, `Text`, `Slider`,
`Select`, `Picker`) it asks the runtime to update that node on the next render even if nothing
about it changed. What the widget owes in return is an update function that compares against the
value the native view shows *now*. A function that compares against a copy it stored at the last
render will see no difference and leave the control wrong.

## Where this lives

- `rust/src/widgets/slider.rs`, `rust/src/widgets/mod.rs`: the model for step 1 and the table
- `rust/macros/src/widget.rs`: what `#[derive(Widget)]` generates, and its tests
- `rust/src/host/events.rs`: `NodeEvent`, `Actions::of`, `Actions::message`
- `rust/src/android/widgets/mod.rs`: `configure_view`, `create_widget`, `Site`
- `rust/src/android/handles.rs`: `Jni`, `Jni::load`, `widget_ctor`
- `rust/src/android/entry.rs`: `on_node_event` and the `nativeOn...` entry points
- `shared/android/src/main/java/dev/krabik/ui/KrabikSeekBar.kt`, `RustRootView.kt`
- `rust/src/ios/widgets/mod.rs`: `update_content`, `IosState::create_widget`, `Site`
- `rust/src/ios/host.rs`: `Content`, `ViewHandle`, `create_handle`
- `rust/src/ios/events.rs`, `rust/src/ios/mod.rs` (`add_action`, `schedule`)

Next: [Add a chrome slot](add-a-chrome-slot.md).
