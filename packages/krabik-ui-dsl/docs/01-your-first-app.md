# 1. Your first app

This chapter builds a small counter and explains the three ideas every app
is made of: **state**, **messages** and a **view**.

## The shape of an app

An app is a type that implements `Component`:

```rust
use krabik_ui_dsl::app::{App, Component};
use krabik_ui_dsl::ui::{Button, Column, Element, Text, TextField};

/// The state: everything the screen is drawn from.
pub struct Counter {
    count: i64,
    name: String,
}

/// The messages: everything that can happen.
#[derive(Clone)]
pub enum Msg {
    Increment,
    Reset,
    Name(String),
}

impl Component for Counter {
    type Msg = Msg;

    /// Applies one message to the state.
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Increment => self.count += 1,
            Msg::Reset => self.count = 0,
            Msg::Name(name) => self.name = name,
        }
    }

    /// Describes the screen for the current state.
    fn view(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .child(Text::new(format!("Hello {}: {}", self.name, self.count)).size(22))
            .child(TextField::new(&self.name).placeholder("Your name").on_change(Msg::Name))
            .child(Button::new("+1").on_click(Msg::Increment))
            .child(Button::new("Reset").on_click(Msg::Reset))
            .into()
    }
}

/// Nothing extra: no navigation, no dialogs.
impl App for Counter {}
```

That is the whole loop:

1. The framework calls `view()` and shows the result with native widgets.
2. The user does something. The widget that was touched carries a message,
   for example `Msg::Increment`.
3. The framework calls `update(msg)`.
4. Back to step 1. Only what changed is applied to the native views.

You never touch a native view, and you never write "when this button is
pressed, change that label". You change the state; the view follows.

## Messages are ordinary values

A widget is told what to send in one of two ways.

A **fixed message**, for events with no data:

```rust
Button::new("Reset").on_click(Msg::Reset)
TextField::new(&self.name).on_submit(Msg::Save)
```

A **function that builds the message** from the event's data. An enum variant
that takes that data is already such a function:

```rust
TextField::new(&self.name).on_change(Msg::Name)        // Fn(String) -> Msg
Switch::new(self.dark).on_toggle(Msg::DarkMode)        // Fn(bool) -> Msg
Slider::new(self.volume).on_change(Msg::Volume)        // Fn(f32) -> Msg
RadioGroup::new(["S", "M", "L"]).on_select(Msg::Size)  // Fn(usize) -> Msg
```

A closure works too, which is how a row in a list knows which item it is:

```rust
Checkbox::new(&todo.title, todo.done)
    .on_toggle(move |done| Msg::SetDone(todo_id, done))
```

Because a message is a value of your own enum, the compiler checks every
event: a misspelled action or a forgotten case in `update` does not compile.

The message type needs `Clone + Send + Sync + 'static`; `#[derive(Clone)]` on
an enum of plain data is enough.

## Registering the app

The framework needs to know which app to create. Each app crate exports an
install function that the native shells call once at startup:

```rust
use krabik_ui_dsl::set_app_factory;

impl Counter {
    pub fn install() {
        // Called again for every new activity or scene, so each gets its
        // own fresh state.
        set_app_factory(|| Counter { count: 0, name: String::new() });
    }
}

/// Called by the iOS shell.
#[unsafe(no_mangle)]
pub extern "C" fn counter_install() {
    Counter::install();
}

/// Called by the Android shell (`object CounterBridge { external fun install() }`).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_counter_CounterBridge_install<'caller>(
    _env: jni::EnvUnowned<'caller>,
    _this: jni::objects::JObject<'caller>,
) {
    Counter::install();
}
```

Chapter 7 shows the shells that call these.

## Testing without a device

State and messages are plain Rust, so the logic is tested with ordinary unit
tests, no emulator needed:

```rust
#[test]
fn incrementing_and_resetting() {
    let mut app = Counter { count: 0, name: String::new() };
    app.update(Msg::Increment);
    app.update(Msg::Increment);
    assert_eq!(app.count, 2);
    app.update(Msg::Reset);
    assert_eq!(app.count, 0);
}
```

`view()` can be checked too: it returns a description, not native views. The
example apps do this, for instance to assert that a page contains two tab
bars (`example/rust/src/lib.rs`).

Next: [widgets, layout and styling](02-widgets-and-layout.md).
