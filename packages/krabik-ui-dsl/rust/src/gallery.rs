//! The gallery demo application.
//!
//! Demonstrates the UI components, flexbox layouts, typography, and interactive
//! state updates in a platform-independent way.

use crate::app::{App, Event};
use crate::ui::{
    Align, Button, Column, EdgeInsets, Element, Image, ImageScale, ImageSource, Justify, Row,
    ScrollView, Spacer, Text, TextField,
};

pub const COLOR_MUTED: u32 = 0xFF66_6666;
pub const COLOR_WHITE: u32 = 0xFFFF_FFFF;
pub const COLOR_BLUE: u32 = 0xFF15_65C0;
pub const COLOR_TEAL: u32 = 0xFF00_897B;
pub const COLOR_AMBER: u32 = 0xFFF9_A825;
pub const COLOR_PURPLE: u32 = 0xFF6A_1B9A;
pub const COLOR_BORDER: u32 = 0xFFDD_DDDD;
pub const COLOR_SURFACE: u32 = 0xFFF4_F4F4;

/// The example screens reachable from the gallery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Demo {
    #[default]
    Gallery,
    Counter,
    Layouts,
    Typography,
    Alignment,
    Scroll,
    Form,
    Media,
    Styling,
}

/// The state of the gallery demo application.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GalleryApp {
    pub demo: Demo,
    pub count: i64,
    pub name: String,
}

impl GalleryApp {
    /// Creates a new gallery demo application.
    pub fn new() -> Self {
        Self::default()
    }

    fn gallery(&self) -> Element {
        ScrollView::new()
            .width_percent(1.0)
            .height_percent(1.0)
            .child(
                Column::new()
                    .padding(24)
                    .spacing(12)
                    .width_percent(1.0)
                    .align_items(Align::Start)
                    .child(Text::new("Native UI DSL").size(30).bold())
                    .child(caption(
                        "Every screen is described in Rust and drawn with real Android views.",
                    ))
                    .child(Spacer::new().grow(0.0).height(8))
                    .child(Button::new("Counter").action("open:counter"))
                    .child(Button::new("Layouts").action("open:layouts"))
                    .child(Button::new("Typography").action("open:typography"))
                    .child(Button::new("Alignment").action("open:alignment"))
                    .child(Button::new("Scroll").action("open:scroll"))
                    .child(Button::new("Form").action("open:form"))
                    .child(Button::new("Media").action("open:media"))
                    .child(Button::new("Styling").action("open:styling"))
                    .child(caption(
                        "This list scrolls, so every demo stays reachable on small screens.",
                    )),
            )
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

    fn scroll(&self) -> Element {
        ScrollView::new()
            .width_percent(1.0)
            .height_percent(1.0)
            .child(
                Column::new()
                    .padding(24)
                    .spacing(14)
                    .width_percent(1.0)
                    .align_items(Align::Start)
                    .child(back_button())
                    .child(Text::new("ScrollView").size(26).bold())
                    .child(caption(
                        "Content taller than the viewport scrolls natively, with momentum.",
                    ))
                    .child(demo_rows())
                    .child(caption(
                        "The same component scrolls horizontally along the other axis.",
                    ))
                    .child(demo_strip()),
            )
            .into()
    }

    fn form(&self) -> Element {
        let greeting = if self.name.is_empty() {
            "Type your name below.".to_string()
        } else {
            format!("Hello, {}!", self.name)
        };

        Column::new()
            .padding(24)
            .spacing(14)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Form").size(26).bold())
            .child(caption(
                "Each edit reaches Rust as a typed TextChanged event.",
            ))
            .child(
                TextField::new(self.name.clone())
                    .placeholder("Your name")
                    .action("form:name")
                    .size(18)
                    .padding_each(EdgeInsets::symmetric(12, 10))
                    .background(COLOR_SURFACE)
                    .corner_radius(10)
                    .border(1, COLOR_BORDER)
                    .width_percent(1.0),
            )
            .child(Text::new(greeting).size(20).color(COLOR_BLUE))
            .child(Button::new("Clear").action("form:clear"))
            .into()
    }

    fn media(&self) -> Element {
        ScrollView::new()
            .width_percent(1.0)
            .height_percent(1.0)
            .child(
                Column::new()
                    .padding(24)
                    .spacing(14)
                    .width_percent(1.0)
                    .align_items(Align::Start)
                    .child(back_button())
                    .child(Text::new("Media").size(26).bold())
                    .child(caption(
                        "Bundled assets load instantly; remote images use Coil.",
                    ))
                    .child(
                        Image::new(ImageSource::asset("krabik.webp"))
                            .width(96)
                            .height(96)
                            .corner_radius(18),
                    )
                    .child(caption("Asset image, 96x96 with rounded corners"))
                    .child(
                        Image::new(ImageSource::url("https://picsum.photos/600/300"))
                            .width_percent(1.0)
                            .height(180)
                            .corner_radius(16)
                            .scale(ImageScale::Fill),
                    )
                    .child(caption("Remote image, cropped to fill its frame")),
            )
            .into()
    }

    fn styling(&self) -> Element {
        ScrollView::new()
            .width_percent(1.0)
            .height_percent(1.0)
            .child(
                Column::new()
                    .padding(24)
                    .spacing(14)
                    .width_percent(1.0)
                    .align_items(Align::Start)
                    .child(back_button())
                    .child(Text::new("Styling").size(26).bold())
                    .child(caption(
                        "Decorations apply to layout containers as well as widgets.",
                    ))
                    .child(
                        Column::new()
                            .width_percent(1.0)
                            .padding(16)
                            .spacing(8)
                            .background(COLOR_SURFACE)
                            .corner_radius(16)
                            .border(1, COLOR_BORDER)
                            .child(Text::new("Card container").size(18).bold())
                            .child(caption(
                                "Background, radius, border and padding in one place.",
                            )),
                    )
                    .child(
                        Row::new()
                            .width_percent(1.0)
                            .spacing(10)
                            .align_items(Align::Center)
                            .child(chip("Rounded", COLOR_BLUE).corner_radius(999))
                            .child(chip("Faded", COLOR_TEAL).opacity(0.4))
                            .child(
                                chip("Outline", COLOR_AMBER)
                                    .color(COLOR_AMBER)
                                    .background(COLOR_WHITE)
                                    .border(2, COLOR_AMBER)
                                    .corner_radius(8),
                            ),
                    )
                    .child(
                        Row::new()
                            .width_percent(1.0)
                            .spacing(10)
                            .align_items(Align::Center)
                            .child(
                                Button::new("Ripple")
                                    .action("styling:noop")
                                    .background(COLOR_SURFACE)
                                    .corner_radius(22),
                            )
                            .child(Button::new("Faded").action("styling:noop").opacity(0.45)),
                    )
                    .child(caption(
                        "Buttons keep their native ripple under the custom shape.",
                    )),
            )
            .into()
    }
}

/// Twenty rows that overflow the vertical scroll viewport.
fn demo_rows() -> Column {
    let mut column = Column::new().spacing(8).width_percent(1.0);
    for index in 1..=20 {
        let surface = if index % 2 == 0 {
            COLOR_SURFACE
        } else {
            COLOR_WHITE
        };
        column = column.child(
            Row::new()
                .width_percent(1.0)
                .padding_each(EdgeInsets::symmetric(12, 10))
                .background(surface)
                .corner_radius(8)
                .child(Text::new(format!("Row {index}")).size(15)),
        );
    }
    column
}

/// A horizontal strip of chips that overflows its width.
fn demo_strip() -> ScrollView {
    let mut strip = ScrollView::horizontal()
        .width_percent(1.0)
        .height(52)
        .align_items(Align::Center);
    for (label, color) in [
        ("One", COLOR_BLUE),
        ("Two", COLOR_TEAL),
        ("Three", COLOR_AMBER),
        ("Four", COLOR_PURPLE),
        ("Five", COLOR_BLUE),
        ("Six", COLOR_TEAL),
        ("Seven", COLOR_AMBER),
        ("Eight", COLOR_PURPLE),
    ] {
        strip = strip.child(chip(label, color).margin_each(EdgeInsets::symmetric(4, 0)));
    }
    strip
}

impl App for GalleryApp {
    fn view(&self) -> Element {
        match self.demo {
            Demo::Gallery => self.gallery(),
            Demo::Counter => self.counter(),
            Demo::Layouts => self.layouts(),
            Demo::Typography => self.typography(),
            Demo::Alignment => self.alignment(),
            Demo::Scroll => self.scroll(),
            Demo::Form => self.form(),
            Demo::Media => self.media(),
            Demo::Styling => self.styling(),
        }
    }

    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::TextChanged {
                action: "form:name",
                value,
            } => self.name = value.to_string(),
            Event::Click { action } => match action {
                "back" => self.demo = Demo::Gallery,
                "open:counter" => {
                    self.demo = Demo::Counter;
                    self.count = 0;
                }
                "open:layouts" => self.demo = Demo::Layouts,
                "open:typography" => self.demo = Demo::Typography,
                "open:alignment" => self.demo = Demo::Alignment,
                "open:scroll" => self.demo = Demo::Scroll,
                "open:form" => self.demo = Demo::Form,
                "open:media" => self.demo = Demo::Media,
                "open:styling" => self.demo = Demo::Styling,
                "counter:add" => self.count += 1,
                "counter:reset" => self.count = 0,
                "form:clear" => self.name.clear(),
                _ => return false,
            },
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

    fn click(action: &'static str) -> Event<'static> {
        Event::Click { action }
    }

    #[test]
    fn navigation_and_counter_actions() {
        let mut app = GalleryApp::new();

        assert!(app.handle(click("open:counter")));
        assert_eq!(app.demo, Demo::Counter);
        assert_eq!(app.count, 0);

        assert!(app.handle(click("counter:add")));
        assert_eq!(app.count, 1);

        assert!(app.handle(click("counter:add")));
        assert_eq!(app.count, 2);

        assert!(app.handle(click("counter:reset")));
        assert_eq!(app.count, 0);

        assert!(app.handle(click("back")));
        assert_eq!(app.demo, Demo::Gallery);

        assert!(app.handle(click("open:layouts")));
        assert_eq!(app.demo, Demo::Layouts);

        assert!(app.handle(click("open:typography")));
        assert_eq!(app.demo, Demo::Typography);

        assert!(app.handle(click("open:alignment")));
        assert_eq!(app.demo, Demo::Alignment);

        assert!(app.handle(click("open:scroll")));
        assert_eq!(app.demo, Demo::Scroll);

        assert!(app.handle(click("open:form")));
        assert_eq!(app.demo, Demo::Form);

        assert!(app.handle(click("open:media")));
        assert_eq!(app.demo, Demo::Media);

        assert!(app.handle(click("open:styling")));
        assert_eq!(app.demo, Demo::Styling);

        assert!(!app.handle(click("unknown_action")));
    }

    #[test]
    fn text_changes_update_the_form_state() {
        let mut app = GalleryApp::new();

        assert!(app.handle(Event::TextChanged {
            action: "form:name",
            value: "Ada",
        }));
        assert_eq!(app.name, "Ada");

        assert!(app.handle(click("form:clear")));
        assert!(app.name.is_empty());

        assert!(!app.handle(Event::TextChanged {
            action: "unknown",
            value: "ignored",
        }));
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
            Demo::Scroll,
            Demo::Form,
            Demo::Media,
            Demo::Styling,
        ] {
            app.demo = demo;
            let element = app.view();
            assert!(!element.children().is_empty());
        }
    }
}
