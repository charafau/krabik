//! The gallery demo application.
//!
//! Demonstrates the UI components, flexbox layouts, typography, and interactive
//! state updates in a platform-independent way.

use crate::app::App;
use crate::ui::{
    Align, Button, Column, EdgeInsets, Element, FontWeight, Justify, Row, Spacer, Text,
};

pub const COLOR_MUTED: u32 = 0xFF66_6666;
pub const COLOR_WHITE: u32 = 0xFFFF_FFFF;
pub const COLOR_BLUE: u32 = 0xFF15_65C0;
pub const COLOR_TEAL: u32 = 0xFF00_897B;
pub const COLOR_AMBER: u32 = 0xFFF9_A825;
pub const COLOR_PURPLE: u32 = 0xFF6A_1B9A;

/// The example screens reachable from the gallery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Demo {
    #[default]
    Gallery,
    Counter,
    Layouts,
    Typography,
    Alignment,
}

/// The state of the gallery demo application.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalleryApp {
    pub demo: Demo,
    pub count: i64,
}

impl GalleryApp {
    /// Creates a new gallery demo application.
    pub fn new() -> Self {
        Self::default()
    }

    fn gallery(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(Text::new("Native UI DSL").size(30).bold())
            .child(caption(
                "Every screen is described in Rust and drawn with real Android views.",
            ))
            .child(Spacer::new())
            .child(Button::new("Counter").action("open:counter"))
            .child(Button::new("Layouts").action("open:layouts"))
            .child(Button::new("Typography").action("open:typography"))
            .child(Button::new("Alignment").action("open:alignment"))
            .into()
    }

    fn counter(&self) -> Element {
        let label = match self.count {
            0 => "Not tapped yet".to_string(),
            1 => "Tapped once".to_string(),
            count => format!("Tapped {count} times"),
        };

        Column::new()
            .padding(24)
            .spacing(14)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Counter").size(26).bold())
            .child(Text::new(label).size(36).bold().color(COLOR_BLUE))
            .child(caption(
                "The label is updated in place on the existing native TextView.",
            ))
            .child(
                Row::new()
                    .spacing(8)
                    .child(Button::new("Add one").action("counter:add"))
                    .child(Button::new("Reset").action("counter:reset")),
            )
            .into()
    }

    fn layouts(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(14)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Layouts").size(26).bold())
            .child(caption("A spacer pushes its siblings to the edges"))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .child(chip("Start", COLOR_BLUE))
                    .child(Spacer::new())
                    .child(chip("End", COLOR_TEAL)),
            )
            .child(caption("A fixed width next to a growing one"))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .child(chip("120dp", COLOR_BLUE).width(120))
                    .child(chip("grow", COLOR_TEAL).grow(1.0)),
            )
            .child(caption("Nested rows and columns"))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .child(
                        Column::new()
                            .spacing(6)
                            .child(chip("A", COLOR_BLUE))
                            .child(chip("B", COLOR_TEAL)),
                    )
                    .child(
                        Column::new()
                            .spacing(6)
                            .child(chip("C", COLOR_AMBER))
                            .child(chip("D", COLOR_PURPLE)),
                    ),
            )
            .into()
    }

    fn typography(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(10)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Typography").size(26).bold())
            .child(Text::new("Display 34 / bold").size(34).bold())
            .child(
                Text::new("Heading 24 / bold")
                    .size(24)
                    .bold()
                    .color(COLOR_BLUE),
            )
            .child(Text::new("Title 20").size(20))
            .child(Text::new("Body 16 - The quick brown fox jumps over the lazy dog.").size(16))
            .child(Text::new("Caption 12 / muted").size(12).color(COLOR_MUTED))
            .child(Text::new("Accent 16 / teal").size(16).color(COLOR_TEAL))
            .into()
    }

    fn alignment(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(10)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Alignment").size(26).bold())
            .child(caption("justify_content on a full-width row"))
            .child(caption("Start"))
            .child(chips(Justify::Start))
            .child(caption("Center"))
            .child(chips(Justify::Center))
            .child(caption("End"))
            .child(chips(Justify::End))
            .child(caption("SpaceBetween"))
            .child(chips(Justify::SpaceBetween))
            .child(caption("align_items=Center with different text sizes"))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(chip("small", COLOR_BLUE).size(12))
                    .child(chip("medium", COLOR_TEAL).size(18))
                    .child(chip("large", COLOR_AMBER).size(24)),
            )
            .into()
    }
}

impl App for GalleryApp {
    fn view(&self) -> Element {
        match self.demo {
            Demo::Gallery => self.gallery(),
            Demo::Counter => self.counter(),
            Demo::Layouts => self.layouts(),
            Demo::Typography => self.typography(),
            Demo::Alignment => self.alignment(),
        }
    }

    fn handle(&mut self, action: &str) -> bool {
        match action {
            "back" => self.demo = Demo::Gallery,
            "open:counter" => {
                self.demo = Demo::Counter;
                self.count = 0;
            }
            "open:layouts" => self.demo = Demo::Layouts,
            "open:typography" => self.demo = Demo::Typography,
            "open:alignment" => self.demo = Demo::Alignment,
            "counter:add" => self.count += 1,
            "counter:reset" => self.count = 0,
            _ => return false,
        }
        true
    }
}

fn back_button() -> Button {
    Button::new("Back").action("back")
}

fn caption(text: &str) -> Text {
    Text::new(text).size(13).color(COLOR_MUTED)
}

fn chip(text: &str, color: u32) -> Text {
    Text::new(text)
        .size(14)
        .color(COLOR_WHITE)
        .background(color)
        .padding_each(EdgeInsets::symmetric(10, 4))
}

fn chips(justify: Justify) -> Row {
    Row::new()
        .width_percent(1.0)
        .spacing(8)
        .justify_content(justify)
        .child(chip("A", COLOR_BLUE))
        .child(chip("B", COLOR_TEAL))
        .child(chip("C", COLOR_AMBER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_gallery() {
        let app = GalleryApp::new();
        assert_eq!(app.demo, Demo::Gallery);
        assert_eq!(app.count, 0);
    }

    #[test]
    fn navigation_and_counter_actions() {
        let mut app = GalleryApp::new();

        assert!(app.handle("open:counter"));
        assert_eq!(app.demo, Demo::Counter);
        assert_eq!(app.count, 0);

        assert!(app.handle("counter:add"));
        assert_eq!(app.count, 1);

        assert!(app.handle("counter:add"));
        assert_eq!(app.count, 2);

        assert!(app.handle("counter:reset"));
        assert_eq!(app.count, 0);

        assert!(app.handle("back"));
        assert_eq!(app.demo, Demo::Gallery);

        assert!(app.handle("open:layouts"));
        assert_eq!(app.demo, Demo::Layouts);

        assert!(app.handle("open:typography"));
        assert_eq!(app.demo, Demo::Typography);

        assert!(app.handle("open:alignment"));
        assert_eq!(app.demo, Demo::Alignment);

        assert!(!app.handle("unknown_action"));
    }

    #[test]
    fn view_produces_elements_for_all_screens() {
        let mut app = GalleryApp::new();
        for demo in [
            Demo::Gallery,
            Demo::Counter,
            Demo::Layouts,
            Demo::Typography,
            Demo::Alignment,
        ] {
            app.demo = demo;
            let element = app.view();
            assert!(!element.children().is_empty());
        }
    }
}

