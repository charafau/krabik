//! A counter, in the versions the hot reload test swaps between.

use krabik_ui_dsl::app::{App, Component};
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

pub struct Counter {
    count: u32,
    #[cfg(feature = "v3")]
    _history: [u64; 4],
}

#[derive(Clone)]
pub enum Msg {
    Add,
}

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Add => self.count += if cfg!(feature = "v2") { 10 } else { 1 },
        }
        Command::none()
    }

    fn view(&self) -> Element<Msg> {
        let label = if cfg!(feature = "v2") { "Total" } else { "Count" };
        Column::new()
            .child(Text::new(format!("{label}: {}", self.count)))
            .child(Button::new("Add").on_click(Msg::Add))
            .into()
    }
}

impl App for Counter {}

krabik_ui_dsl::hot_reload!(Counter);

/// A new counter, as the raw pointer of its box. The test starts from one
/// made by version one.
#[unsafe(no_mangle)]
pub extern "C" fn fixture_new() -> *mut () {
    Box::into_raw(Box::new(Counter {
        count: 0,
        #[cfg(feature = "v3")]
        _history: [0; 4],
    }))
    .cast()
}
