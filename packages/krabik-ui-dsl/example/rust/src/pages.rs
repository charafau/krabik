//! One page per widget for the showcase app.

use krabik_ui_dsl::ui::{
    Align, Button, Checkbox, Column, DatePicker, DatePickerMode, EdgeInsets, Element, Gradient,
    Image, ImageScale, ImageSource, Justify, List, ProgressIndicator, RadioButton, RadioGroup, Row,
    ScrollView, Slider, Spacer, Switch, Text, TextAlign, TextField,
};

use crate::{Page, ShowcaseApp};

pub(crate) const COLOR_TEXT: u32 = 0xFF20_2125;
pub(crate) const COLOR_MUTED: u32 = 0xFF66_6666;
pub(crate) const COLOR_SURFACE: u32 = 0xFFF4_F4F4;
pub(crate) const COLOR_DONE: u32 = 0xFFE6_F4EA;
pub(crate) const COLOR_ACCENT: u32 = 0xFF00_897B;
const COLOR_BLUE: u32 = 0xFF15_65C0;
const COLOR_AMBER: u32 = 0xFFF9_A825;
const COLOR_PURPLE: u32 = 0xFF6A_1B9A;
const COLOR_BORDER: u32 = 0xFFDD_DDDD;

/// A muted secondary line of copy.
pub(crate) fn caption(text: &str) -> Text {
    Text::new(text).size(14).color(COLOR_MUTED)
}

/// The control that returns from a widget page to the catalog.
pub(crate) fn back_button() -> Button {
    Button::new("Back").action("page:home")
}

/// Wraps page content in the standard scrolling frame.
macro_rules! body {
    ($($item:expr),* $(,)?) => {
        vec![$($item.into()),*]
    };
}

fn page(title: &str, subtitle: &str, body: Vec<Element>) -> Element {
    let mut column = Column::new()
        .padding(24)
        .spacing(14)
        .width_percent(1.0)
        .align_items(Align::Start)
        .child(back_button())
        .child(Text::new(title).size(28).bold().color(COLOR_TEXT))
        .child(caption(subtitle));

    for child in body {
        column = column.child(child);
    }

    ScrollView::new()
        .width_percent(1.0)
        .height_percent(1.0)
        .child(column)
        .into()
}

/// Builds the page for the current [`Page`].
pub(crate) fn view(app: &ShowcaseApp) -> Element {
    match app.page {
        Page::Home => home(),
        Page::Text => text_page(),
        Page::Button => button_page(app),
        Page::TextField => text_field_page(app),
        Page::Switch => switch_page(app),
        Page::Checkbox => checkbox_page(app),
        Page::RadioButtons => radio_buttons_page(app),
        Page::RadioGroup => radio_group_page(app),
        Page::DatePicker => date_picker_page(app),
        Page::Slider => slider_page(app),
        Page::Progress => progress_page(app),
        Page::Spinner => spinner_page(),
        Page::Image => image_page(),
        Page::Scroll => scroll_page(),
        Page::List => list_page(app),
        Page::RecycledList => recycled_list_page(app),
        Page::Layouts => layouts_page(),
        Page::Styling => styling_page(),
        Page::Checklist => app.checklist.view(),
    }
}

fn page_button(label: &'static str, action: &'static str) -> Button {
    Button::new(label).action(action).width_percent(1.0)
}

fn home() -> Element {
    let catalog: [(&str, &[(&str, &str)]); 6] = [
        (
            "Text & layout",
            &[
                ("Text", "page:text"),
                ("Button", "page:button"),
                ("Text field", "page:text-field"),
                ("Layouts", "page:layouts"),
            ],
        ),
        (
            "Inputs",
            &[
                ("Switch", "page:switch"),
                ("Checkbox", "page:checkbox"),
                ("Radio buttons", "page:radio-buttons"),
                ("Radio group", "page:radio-group"),
                ("Date picker", "page:date-picker"),
            ],
        ),
        (
            "Values",
            &[
                ("Slider", "page:slider"),
                ("Progress", "page:progress"),
                ("Spinner", "page:spinner"),
            ],
        ),
        (
            "Media & scrolling",
            &[("Image", "page:image"), ("Scroll view", "page:scroll")],
        ),
        (
            "Lists",
            &[
                ("Windowed list", "page:list"),
                ("Recycled list", "page:recycled-list"),
            ],
        ),
        (
            "Extras",
            &[("Styling", "page:styling"), ("Checklist", "page:checklist")],
        ),
    ];

    let mut column = Column::new()
        .padding(24)
        .spacing(12)
        .width_percent(1.0)
        .align_items(Align::Start)
        .child(Text::new("Krabik UI DSL").size(30).bold().color(COLOR_TEXT))
        .child(caption(
            "Every screen here is a Rust tree drawn with real UIKit and Android widgets.",
        ))
        .child(
            Text::new("Open a widget to see it in isolation.")
                .size(16)
                .color(COLOR_TEXT),
        );

    for (section, pages) in catalog {
        column = column
            .child(Spacer::new().height(6))
            .child(Text::new(section).size(18).bold().color(COLOR_TEXT));
        for &(label, action) in pages {
            column = column.child(page_button(label, action));
        }
    }

    ScrollView::new()
        .width_percent(1.0)
        .height_percent(1.0)
        .child(column)
        .into()
}

fn text_page() -> Element {
    page(
        "Text",
        "Sizes, weights, colors, alignment and truncation.",
        body![
            Text::new("Display 34 / bold")
                .size(34)
                .bold()
                .color(COLOR_TEXT),
            Text::new("Heading 24 / bold")
                .size(24)
                .bold()
                .color(COLOR_BLUE),
            Text::new("Title 20").size(20).color(COLOR_TEXT),
            Text::new("Body 16 — the quick brown fox jumps over the lazy dog.")
                .size(16)
                .color(COLOR_TEXT),
            Text::new("Caption 12 / muted").size(12).color(COLOR_MUTED),
            Text::new("Accent 16 / teal").size(16).color(COLOR_ACCENT),
            Text::new("Centered text")
                .size(16)
                .text_align(TextAlign::Center)
                .width_percent(1.0),
            Text::new(
                "This long paragraph is limited to two lines and truncated with an ellipsis when \
                 it does not fit.",
            )
            .size(16)
            .max_lines(2)
            .ellipsis()
            .width_percent(1.0),
        ],
    )
}

fn button_page(app: &ShowcaseApp) -> Element {
    page(
        "Button",
        "Clicks dispatch typed events; disabled buttons ignore input.",
        body![
            Text::new(format!("Taps: {}", app.taps))
                .size(20)
                .color(COLOR_TEXT),
            Button::new("Add one").action("demo:tap"),
            Button::new("Reset").action("demo:reset"),
            Button::new("Disabled").enabled(false),
        ],
    )
}

fn text_field_page(app: &ShowcaseApp) -> Element {
    let submitted = if app.submitted.is_empty() {
        "Press return to submit.".to_string()
    } else {
        format!("Submitted: {}", app.submitted)
    };
    page(
        "Text field",
        "Every keystroke updates the controlled value; return submits it.",
        body![
            TextField::new(app.name.clone())
                .placeholder("Type your name")
                .action("demo:name")
                .submit_action("demo:name:submit")
                .accessibility_label("Name")
                .width_percent(1.0),
            Text::new(format!("Value: {}", app.name))
                .size(16)
                .color(COLOR_TEXT),
            caption(&submitted),
        ],
    )
}

fn switch_page(app: &ShowcaseApp) -> Element {
    page(
        "Switch",
        "Booleans are simple: the app stores the state and the switch reflects it.",
        body![
            Row::new()
                .width_percent(1.0)
                .align_items(Align::Center)
                .child(Text::new("Notifications").size(18).color(COLOR_TEXT))
                .child(Spacer::new())
                .child(
                    Switch::new(app.notifications)
                        .action("demo:notifications")
                        .accessibility_label("Notifications"),
                ),
            if app.notifications {
                Text::new("Notifications are on.")
                    .size(16)
                    .color(COLOR_ACCENT)
            } else {
                Text::new("Notifications are off.")
                    .size(16)
                    .color(COLOR_MUTED)
            },
        ],
    )
}

fn checkbox_page(app: &ShowcaseApp) -> Element {
    page(
        "Checkbox",
        "Independent checkboxes, each with its own toggle action.",
        body![
            Checkbox::new("Wi-Fi", app.wifi)
                .action("demo:wifi")
                .accessibility_label("Wi-Fi"),
            caption(if app.wifi {
                "Wi-Fi is on."
            } else {
                "Wi-Fi is off."
            }),
            Checkbox::new("Bluetooth", app.bluetooth)
                .action("demo:bluetooth")
                .accessibility_label("Bluetooth"),
            caption(if app.bluetooth {
                "Bluetooth is on."
            } else {
                "Bluetooth is off."
            }),
        ],
    )
}

fn radio_buttons_page(app: &ShowcaseApp) -> Element {
    page(
        "Radio buttons",
        "These are independent controls: the application owns the mutual exclusion.",
        body![
            RadioButton::new("Small", app.radio_choice == 0).action("demo:radio:0"),
            RadioButton::new("Medium", app.radio_choice == 1).action("demo:radio:1"),
            RadioButton::new("Large", app.radio_choice == 2).action("demo:radio:2"),
            caption(&format!(
                "Selected: {}",
                ["Small", "Medium", "Large"][app.radio_choice]
            )),
        ],
    )
}

fn radio_group_page(app: &ShowcaseApp) -> Element {
    page(
        "Radio group",
        "The group owns the selection and reports the chosen index with one event.",
        body![
            RadioGroup::new(["Small", "Medium", "Large"])
                .selected(app.size_choice)
                .action("demo:size")
                .accessibility_label("Size")
                .width_percent(1.0),
            caption(&format!(
                "Selected: {}",
                ["Small", "Medium", "Large"][app.size_choice]
            )),
        ],
    )
}

fn date_picker_page(app: &ShowcaseApp) -> Element {
    page(
        "Date picker",
        "Compact fields open the platform picker; inline calendars stay visible.",
        body![
            Text::new(format!("Compact: {}", app.date))
                .size(18)
                .color(COLOR_TEXT),
            DatePicker::new(app.date)
                .action("demo:date")
                .accessibility_label("Compact date"),
            caption("Tap the field to open the native date picker."),
            Text::new(format!("Inline: {}", app.inline_date))
                .size(18)
                .color(COLOR_TEXT),
            DatePicker::new(app.inline_date)
                .mode(DatePickerMode::Inline)
                .action("demo:date-inline")
                .accessibility_label("Inline date"),
            caption("The inline calendar reports every selected day."),
        ],
    )
}

fn slider_page(app: &ShowcaseApp) -> Element {
    page(
        "Slider",
        "Continuous sliders report every change; stepped sliders quantize the value.",
        body![
            Text::new(format!("Volume: {:.0}", app.volume))
                .size(18)
                .color(COLOR_TEXT),
            Slider::new(app.volume)
                .min(0.0)
                .max(100.0)
                .action("demo:volume")
                .accessibility_label("Volume")
                .width_percent(1.0),
            Text::new(format!("Stepped: {:.0}", app.stepped))
                .size(18)
                .color(COLOR_TEXT),
            Slider::new(app.stepped)
                .min(0.0)
                .max(100.0)
                .step(10.0)
                .action("demo:stepped")
                .accessibility_label("Stepped volume")
                .width_percent(1.0),
            caption("The stepped slider only reports multiples of ten."),
        ],
    )
}

fn progress_page(app: &ShowcaseApp) -> Element {
    page(
        "Progress",
        "A determinate bar driven by application state.",
        body![
            Text::new(format!("{:.0}%", app.progress * 100.0))
                .size(20)
                .color(COLOR_TEXT),
            ProgressIndicator::new(app.progress)
                .color(COLOR_ACCENT)
                .accessibility_label("Progress bar")
                .width_percent(1.0),
            Row::new()
                .spacing(8)
                .child(Button::new("Set 25%").action("demo:progress:25"))
                .child(Button::new("Set 50%").action("demo:progress:50"))
                .child(Button::new("Set 100%").action("demo:progress:100")),
        ],
    )
}

fn spinner_page() -> Element {
    page(
        "Spinner",
        "An indeterminate indicator for work whose end is not known.",
        body![
            ProgressIndicator::indeterminate()
                .color(COLOR_ACCENT)
                .accessibility_label("Loading spinner")
                .width(32)
                .height(32),
            caption("The spinner animates until the node is removed."),
        ],
    )
}

fn image_page() -> Element {
    page(
        "Image",
        "Bundled assets load immediately; remote images load from a URL.",
        body![
            Image::new(ImageSource::asset("krabik.webp"))
                .width(96)
                .height(96)
                .corner_radius(18)
                .accessibility_label("Krabik app icon"),
            caption("Bundled asset, 96×96 with rounded corners."),
            Image::new(ImageSource::url("https://picsum.photos/600/300"))
                .width_percent(1.0)
                .aspect_ratio(2.0)
                .corner_radius(16)
                .scale(ImageScale::Fill)
                .accessibility_label("Remote landscape photo"),
            caption("Remote image, 2:1 aspect ratio cropped to fill."),
        ],
    )
}

fn scroll_page() -> Element {
    let mut vertical = Column::new().spacing(8).width_percent(1.0);
    for index in 1..=20 {
        vertical = vertical.child(
            Row::new()
                .width_percent(1.0)
                .padding_each(EdgeInsets::symmetric(12, 10))
                .background(if index % 2 == 0 {
                    COLOR_SURFACE
                } else {
                    0xFFFF_FFFF
                })
                .corner_radius(8)
                .child(Text::new(format!("Row {index}")).size(15)),
        );
    }

    let mut strip = ScrollView::horizontal()
        .width_percent(1.0)
        .height(52)
        .align_items(Align::Center);
    for (label, color) in [
        ("One", COLOR_BLUE),
        ("Two", COLOR_ACCENT),
        ("Three", COLOR_AMBER),
        ("Four", COLOR_PURPLE),
        ("Five", COLOR_BLUE),
        ("Six", COLOR_ACCENT),
        ("Seven", COLOR_AMBER),
        ("Eight", COLOR_PURPLE),
    ] {
        strip = strip.child(
            Text::new(label)
                .size(14)
                .color(0xFFFF_FFFF)
                .background(color)
                .padding_each(EdgeInsets::symmetric(10, 4))
                .margin_each(EdgeInsets::symmetric(4, 0)),
        );
    }

    page(
        "Scroll view",
        "Vertical and horizontal containers clip their content and scroll natively.",
        body![
            ScrollView::new()
                .width_percent(1.0)
                .height(220)
                .child(vertical),
            caption("Vertical scroll view, 20 rows."),
            strip,
            caption("Horizontal scroll view, eight chips."),
        ],
    )
}

fn list_page(app: &ShowcaseApp) -> Element {
    let items = app.list_items.clone();
    let mut column = Column::new()
        .padding(24)
        .spacing(12)
        .width_percent(1.0)
        .height_percent(1.0)
        .align_items(Align::Start)
        .child(back_button())
        .child(Text::new("Windowed list").size(28).bold().color(COLOR_TEXT))
        .child(caption(
            "Only the rows inside the window exist as native widgets.",
        ))
        .child(
            List::new(items)
                .item_height(44)
                .buffer(3)
                .key(|row| row.clone())
                .row(|row| {
                    Row::new()
                        .width_percent(1.0)
                        .padding_each(EdgeInsets::symmetric(12, 8))
                        .background(COLOR_SURFACE)
                        .corner_radius(8)
                        .child(Text::new(row.clone()).size(15))
                        .into()
                }),
        );
    column = column.child(caption("Scroll to keep discovering rows."));
    column.into()
}

fn recycled_list_page(app: &ShowcaseApp) -> Element {
    let items = app.recycled_items.clone();
    Column::new()
        .padding(24)
        .spacing(12)
        .width_percent(1.0)
        .height_percent(1.0)
        .align_items(Align::Start)
        .child(back_button())
        .child(Text::new("Recycled list").size(28).bold().color(COLOR_TEXT))
        .child(caption(
            "A native recycler builds and reuses rows on demand.",
        ))
        .child(
            Text::new(format!("Row taps: {}", app.recycled_taps))
                .size(16)
                .color(COLOR_ACCENT),
        )
        .child(
            List::new(items)
                .item_height(56)
                .buffer(3)
                .recycled()
                .key(|row| row.clone())
                .row(|row| {
                    Row::new()
                        .width_percent(1.0)
                        .height(56)
                        .padding_each(EdgeInsets::symmetric(10, 6))
                        .spacing(8)
                        .align_items(Align::Center)
                        .background(COLOR_SURFACE)
                        .corner_radius(8)
                        .child(Text::new(row.clone()).size(15).grow(1.0))
                        .child(
                            Button::new("Tap")
                                .action("demo:recycled-tap")
                                .accessibility_label(format!("Tap {row}")),
                        )
                        .into()
                }),
        )
        .into()
}

fn layouts_page() -> Element {
    page(
        "Layouts",
        "Rows, columns and spacers compose with flexbox.",
        body![
            caption("A spacer pushes its siblings to the edges."),
            Row::new()
                .width_percent(1.0)
                .spacing(8)
                .child(chip("Start", COLOR_BLUE))
                .child(Spacer::new())
                .child(chip("End", COLOR_ACCENT)),
            caption("A fixed width next to a growing one."),
            Row::new()
                .width_percent(1.0)
                .spacing(8)
                .child(chip("120dp", COLOR_BLUE).width(120))
                .child(chip("grow", COLOR_ACCENT).grow(1.0)),
            caption("Distribution: space between."),
            Row::new()
                .width_percent(1.0)
                .spacing(8)
                .justify_content(Justify::SpaceBetween)
                .child(chip("A", COLOR_BLUE))
                .child(chip("B", COLOR_AMBER))
                .child(chip("C", COLOR_PURPLE)),
        ],
    )
}

fn styling_page() -> Element {
    page(
        "Styling",
        "Backgrounds, gradients, borders, radii, opacity and elevation.",
        body![
            Text::new("Background & radius")
                .size(16)
                .color(0xFFFF_FFFF)
                .background(COLOR_BLUE)
                .padding_each(EdgeInsets::symmetric(14, 10))
                .corner_radius(12),
            Text::new("Gradient")
                .size(16)
                .color(0xFFFF_FFFF)
                .gradient(Gradient::horizontal(COLOR_ACCENT, COLOR_BLUE))
                .padding_each(EdgeInsets::symmetric(14, 10))
                .corner_radius(12),
            Text::new("Border")
                .size(16)
                .color(COLOR_TEXT)
                .background(0xFFFF_FFFF)
                .border(2, COLOR_BORDER)
                .padding_each(EdgeInsets::symmetric(14, 10))
                .corner_radius(12),
            Text::new("Opacity 60%")
                .size(16)
                .color(COLOR_TEXT)
                .background(COLOR_SURFACE)
                .opacity(0.6)
                .padding_each(EdgeInsets::symmetric(14, 10))
                .corner_radius(12),
            Text::new("Elevation")
                .size(16)
                .color(COLOR_TEXT)
                .background(0xFFFF_FFFF)
                .elevation(4)
                .padding_each(EdgeInsets::symmetric(14, 10))
                .corner_radius(12),
        ],
    )
}

fn chip(text: &str, color: u32) -> Text {
    Text::new(text)
        .size(14)
        .color(0xFFFF_FFFF)
        .background(color)
        .padding_each(EdgeInsets::symmetric(10, 4))
        .corner_radius(6)
}
