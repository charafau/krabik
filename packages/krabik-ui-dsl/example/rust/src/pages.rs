//! One page per widget for the showcase app.

use krabik_ui_dsl::presentation::{BottomSheet, Dialog};
use krabik_ui_dsl::ui::{
    Align, AnimationCurve, BottomNavItem, BottomNavigation, Button, Checkbox, Column, DatePicker,
    DatePickerMode, DateValue, EdgeInsets, Element, Gradient, Icon, Image, ImageScale, ImageSource,
    Justify, List, Pager, ProgressIndicator, RadioButton, RadioGroup, Row, ScrollView, Slider,
    Spacer, Switch, TabBar, Tabs, TabsView, Text, TextAlign, TextField, TimeValue,
};

use crate::ShowcaseApp;

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

/// Maps a `page:*` action to a stable screen key.
pub(crate) fn screen_key(action: &str) -> Option<&'static str> {
    Some(match action.strip_prefix("page:")? {
        "home" => "home",
        "text" => "text",
        "button" => "button",
        "text-field" => "text-field",
        "switch" => "switch",
        "checkbox" => "checkbox",
        "radio-buttons" => "radio-buttons",
        "radio-group" => "radio-group",
        "date-picker" => "date-picker",
        "tabs" => "tabs",
        "animations" => "animations",
        "animation-decoration" => "animation-decoration",
        "animation-enter-exit" => "animation-enter-exit",
        "animation-curves" => "animation-curves",
        "transitions" => "transitions",
        "transition-slide" => "transition-slide",
        "transition-fade" => "transition-fade",
        "transition-none" => "transition-none",
        "bottom-sheet" => "bottom-sheet",
        "pager" => "pager",
        "tabs-view" => "tabs-view",
        "bottom-navigation" => "bottom-navigation",
        "toolbar" => "toolbar",
        "slider" => "slider",
        "progress" => "progress",
        "spinner" => "spinner",
        "image" => "image",
        "scroll" => "scroll",
        "list" => "list",
        "recycled-list" => "recycled-list",
        "layouts" => "layouts",
        "styling" => "styling",
        "checklist" => "checklist",
        _ => return None,
    })
}

/// The native navigation-bar title for one screen.
pub(crate) fn title(key: &str) -> &'static str {
    match key {
        "home" => "Widgets",
        "text" => "Text",
        "button" => "Button",
        "text-field" => "Text field",
        "switch" => "Switch",
        "checkbox" => "Checkbox",
        "radio-buttons" => "Radio buttons",
        "radio-group" => "Radio group",
        "date-picker" => "Date picker",
        "tabs" => "Tabs",
        "animations" => "Frames & opacity",
        "animation-decoration" => "Decoration",
        "animation-enter-exit" => "Enter & exit",
        "animation-curves" => "Curves & springs",
        "transitions" => "Page transitions",
        "transition-slide" => "Slide",
        "transition-fade" => "Fade",
        "transition-none" => "Instant",
        "bottom-sheet" => "Bottom sheet",
        "pager" => "View pager",
        "tabs-view" => "Tabs view",
        "bottom-navigation" => "Bottom navigation",
        "toolbar" => "Toolbar",
        "slider" => "Slider",
        "progress" => "Progress",
        "spinner" => "Spinner",
        "image" => "Image",
        "scroll" => "Scroll view",
        "list" => "Windowed list",
        "recycled-list" => "Recycled list",
        "layouts" => "Layouts",
        "styling" => "Styling",
        "checklist" => "Checklist",
        _ => "Krabik UI DSL",
    }
}

/// Builds the content for one navigation screen.
pub(crate) fn screen(app: &ShowcaseApp, key: &str) -> Option<Element> {
    Some(match key {
        "home" => home(),
        "text" => text_page(),
        "button" => button_page(app),
        "text-field" => text_field_page(app),
        "switch" => switch_page(app),
        "checkbox" => checkbox_page(app),
        "radio-buttons" => radio_buttons_page(app),
        "radio-group" => radio_group_page(app),
        "date-picker" => date_picker_page(app),
        "tabs" => tabs_page(app),
        "animations" => animations_page(app),
        "animation-decoration" => animation_decoration_page(app),
        "animation-enter-exit" => animation_enter_exit_page(app),
        "animation-curves" => animation_curves_page(app),
        "transitions" => page_transitions_page(),
        "transition-slide" => transition_demo_page("transition-slide"),
        "transition-fade" => transition_demo_page("transition-fade"),
        "transition-none" => transition_demo_page("transition-none"),
        "bottom-sheet" => bottom_sheet_page(),
        "pager" => pager_page(app),
        "tabs-view" => tabs_view_page(app),
        "bottom-navigation" => bottom_navigation_page(app),
        "toolbar" => toolbar_page(app),
        "slider" => slider_page(app),
        "progress" => progress_page(app),
        "spinner" => spinner_page(),
        "image" => image_page(),
        "scroll" => scroll_page(),
        "list" => list_page(app),
        "recycled-list" => recycled_list_page(app),
        "layouts" => layouts_page(),
        "styling" => styling_page(),
        "checklist" => app.checklist.view(),
        _ => return None,
    })
}

fn page_button(label: &'static str, action: &'static str) -> Button {
    Button::new(label).action(action).width_percent(1.0)
}

pub(crate) fn home() -> Element {
    let catalog: [(&str, &[(&str, &str)]); 8] = [
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
            "Motion",
            &[
                ("Frames & opacity", "page:animations"),
                ("Decoration", "page:animation-decoration"),
                ("Enter & exit", "page:animation-enter-exit"),
                ("Curves & springs", "page:animation-curves"),
                ("Page transitions", "page:transitions"),
            ],
        ),
        (
            "Containers",
            &[
                ("View pager", "page:pager"),
                ("Tabs view", "page:tabs-view"),
                ("Bottom navigation", "page:bottom-navigation"),
                ("Toolbar", "page:toolbar"),
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
                ("Tabs", "page:tabs"),
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
            &[
                ("Styling", "page:styling"),
                ("Checklist", "page:checklist"),
                ("Dialog", "dialog:open"),
                ("Bottom sheet", "page:bottom-sheet"),
            ],
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

pub(crate) fn dialog_view(_app: &ShowcaseApp) -> Element {
    Dialog::new()
        .child(Text::new("Delete task?").size(20).bold().color(COLOR_TEXT))
        .child(
            Text::new("This cannot be undone.")
                .size(15)
                .color(COLOR_MUTED),
        )
        .child(
            Row::new()
                .spacing(8)
                .justify_content(Justify::End)
                .child(Button::new("Cancel").action("dialog:close"))
                .child(Button::new("Delete").action("dialog:close")),
        )
        .into()
}

pub(crate) fn sheet_view(_app: &ShowcaseApp) -> Element {
    BottomSheet::new()
        .child(Text::new("Filters").size(20).bold().color(COLOR_TEXT))
        .child(
            Text::new("The native sheet provides the drag handle and dismissal.")
                .size(15)
                .color(COLOR_MUTED),
        )
        .child(Button::new("Close").action("sheet:close"))
        .into()
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
            Text::new(format!("Time: {}", app.time))
                .size(18)
                .color(COLOR_TEXT),
            DatePicker::time(app.time)
                .time_range(TimeValue::new(8, 0), TimeValue::new(18, 0))
                .action("demo:time")
                .accessibility_label("Time"),
            caption("Time pickers use the same widget; values are bounded to 08:00-18:00."),
            Text::new(format!(
                "Date and time: {} {}",
                app.date_time_date, app.date_time
            ))
            .size(18)
            .color(COLOR_TEXT),
            DatePicker::date_time(app.date_time_date, app.date_time)
                .action("demo:date-time")
                .accessibility_label("Date and time"),
            caption("Date-and-time pickers report both components together."),
            Text::new(format!("Bounded: {}", app.range_date))
                .size(18)
                .color(COLOR_TEXT),
            DatePicker::new(app.range_date)
                .range(DateValue::new(2026, 1, 1), DateValue::new(2026, 12, 31))
                .action("demo:date-range")
                .accessibility_label("Bounded date"),
            caption("Dates outside 2026 are clamped to the nearest bound."),
        ],
    )
}

fn tabs_page(app: &ShowcaseApp) -> Element {
    let content = match app.tab {
        0 => "Overview: the first tab is selected by default.",
        1 => "Details: the app owns the selected index.",
        _ => "Settings: tapping a tab reports Event::Selected.",
    };
    page(
        "Tabs",
        "A tab bar reports the tapped index; the app owns the selected content.",
        vec![
            Tabs::new(["Overview", "Details", "Settings"])
                .selected(app.tab)
                .action("demo:tab")
                .child(Text::new(content).size(18).color(COLOR_TEXT).padding(16))
                .into(),
            caption("Tabs composes a TabBar above application-owned content.").into(),
            TabBar::new(["Day", "Week", "Month"])
                .selected(app.tab.min(2))
                .action("demo:tab")
                .accessibility_label("Period")
                .into(),
        ],
    )
}

fn animations_page(app: &ShowcaseApp) -> Element {
    let open = app.animations;
    let extra: Element = if open {
        Text::new("Entered with a fade and scale.")
            .color(COLOR_ACCENT)
            .animate_enter(220)
            .enter_curve(AnimationCurve::spring(0.7, 0.3))
            .animate_exit(160)
            .exit_curve(AnimationCurve::EaseIn)
            .into()
    } else {
        Spacer::new().height(0).into()
    };
    page(
        "Frames & opacity",
        "Frame size, position and opacity follow state changes with a spring curve.",
        body![
            Button::new(if open { "Hide panel" } else { "Show panel" }).action("demo:animate"),
            Column::new()
                .animate(320)
                .animation_curve(AnimationCurve::spring(0.72, 0.35))
                .background(if open { COLOR_DONE } else { COLOR_SURFACE })
                .border(
                    if open { 2 } else { 1 },
                    if open { COLOR_ACCENT } else { COLOR_BORDER }
                )
                .corner_radius(if open { 18 } else { 8 })
                .padding(16)
                .spacing(8)
                .width_percent(1.0)
                .child(
                    Text::new(if open {
                        "Panel is open."
                    } else {
                        "Panel is closed."
                    })
                    .bold()
                    .color(COLOR_TEXT),
                )
                .child(extra),
            Text::new("This caption follows a custom cubic Bezier curve.")
                .color(COLOR_MUTED)
                .animate(320)
                .animation_curve(AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0)),
            Text::new("Opacity and decoration share the panel's spring.")
                .size(18)
                .color(COLOR_ACCENT)
                .opacity(if open { 1.0 } else { 0.25 })
                .animate(320)
                .animation_curve(AnimationCurve::spring(0.72, 0.35)),
        ],
    )
}

fn animation_decoration_page(app: &ShowcaseApp) -> Element {
    let alt = app.decoration_alt;
    page(
        "Decoration",
        "Fill, gradient, border, corners and elevation interpolate with one transition.",
        body![
            Button::new(if alt { "Cool palette" } else { "Warm palette" })
                .action("demo:decoration"),
            Column::new()
                .animate(360)
                .animation_curve(AnimationCurve::cubic_bezier(0.2, 0.8, 0.2, 1.0))
                .background(if alt { 0xFFFF_F3E0 } else { 0xFFE3_F2FD })
                .border(
                    if alt { 1 } else { 3 },
                    if alt { COLOR_AMBER } else { COLOR_BLUE },
                )
                .corner_radius(if alt { 10 } else { 30 })
                .elevation(if alt { 2 } else { 12 })
                .padding(20)
                .spacing(8)
                .width_percent(1.0)
                .child(
                    Text::new(if alt { "Warm card" } else { "Cool card" })
                        .bold()
                        .color(COLOR_TEXT),
                )
                .child(caption(
                    "Background, border, corners and elevation animate together."
                )),
            Column::new()
                .animate(360)
                .animation_curve(AnimationCurve::cubic_bezier(0.2, 0.8, 0.2, 1.0))
                .gradient(if alt {
                    Gradient::vertical(0xFFF9_A825, 0xFFE6_5100)
                } else {
                    Gradient::horizontal(0xFF15_65C0, 0xFF6A_1B9A)
                })
                .corner_radius(if alt { 8 } else { 24 })
                .height(90)
                .width_percent(1.0)
                .padding(16)
                .justify_content(Justify::Center)
                .child(
                    Text::new(if alt {
                        "Warm gradient"
                    } else {
                        "Cool gradient"
                    })
                    .bold()
                    .color(0xFFFF_FFFF),
                ),
            caption("The gradient stops animate; changing orientation snaps."),
        ],
    )
}

fn animation_enter_exit_page(app: &ShowcaseApp) -> Element {
    let mut cards = Column::new()
        .animate(320)
        .animation_curve(AnimationCurve::spring(0.75, 0.3))
        .spacing(10)
        .width_percent(1.0);
    for index in 0..app.cards {
        cards = cards.child(
            Row::new()
                .key(format!("card-{index}"))
                .animate(280)
                .animate_enter(220)
                .enter_curve(AnimationCurve::spring(0.7, 0.3))
                .animate_exit(160)
                .exit_curve(AnimationCurve::EaseIn)
                .background(if index % 2 == 0 {
                    COLOR_SURFACE
                } else {
                    COLOR_DONE
                })
                .corner_radius(12)
                .padding(14)
                .width_percent(1.0)
                .child(
                    Text::new(format!("Card {}", index + 1))
                        .bold()
                        .color(COLOR_TEXT),
                )
                .child(Spacer::new())
                .child(Text::new("fades and scales").color(COLOR_MUTED)),
        );
    }
    page(
        "Enter & exit",
        "Created nodes fade and scale in; removed nodes animate out before detaching.",
        body![
            Row::new()
                .spacing(10)
                .child(
                    Button::new("Add card")
                        .action("demo:add-card")
                        .enabled(app.cards < 4),
                )
                .child(
                    Button::new("Remove card")
                        .action("demo:remove-card")
                        .enabled(app.cards > 0),
                ),
            Text::new(format!("{} cards", app.cards))
                .size(18)
                .color(COLOR_TEXT),
            cards,
            Text::new("This caption moves as cards are added or removed.")
                .color(COLOR_MUTED)
                .animate(320),
        ],
    )
}

fn animation_curves_page(app: &ShowcaseApp) -> Element {
    let wide = app.curves_alt;
    let bar = |label: &'static str, color: u32, curve: AnimationCurve| -> Element {
        Column::new()
            .spacing(6)
            .width_percent(1.0)
            .child(Text::new(label).color(COLOR_TEXT))
            .child(
                Spacer::new()
                    .width(if wide { 280 } else { 56 })
                    .height(22)
                    .background(color)
                    .corner_radius(11)
                    .animate(520)
                    .animation_curve(curve),
            )
            .into()
    };
    page(
        "Curves & springs",
        "Four bars run the same width change with different timing curves.",
        body![
            Button::new("Run curves").action("demo:curves"),
            Text::new(if wide {
                "Target: wide"
            } else {
                "Target: narrow"
            })
            .size(18)
            .color(COLOR_TEXT),
            bar("Linear", COLOR_BLUE, AnimationCurve::Linear),
            bar(
                "Cubic Bezier",
                COLOR_ACCENT,
                AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0),
            ),
            bar("Spring", COLOR_PURPLE, AnimationCurve::spring(0.55, 0.45)),
            bar("Ease out", COLOR_AMBER, AnimationCurve::EaseOut),
            caption("Run it again to compare how each curve settles."),
        ],
    )
}

fn page_transitions_page() -> Element {
    page(
        "Page transitions",
        "Each screen chooses the native transition used when it is pushed.",
        body![
            Button::new("Push with slide")
                .action("page:transition-slide")
                .width_percent(1.0),
            Button::new("Push with fade")
                .action("page:transition-fade")
                .width_percent(1.0),
            Button::new("Push with no animation")
                .action("page:transition-none")
                .width_percent(1.0),
            caption("Go back after each push to compare another style."),
            caption("Pop reverses the same native transition."),
        ],
    )
}

fn transition_demo_page(key: &'static str) -> Element {
    let (title, detail) = match key {
        "transition-fade" => ("Fade", "Pushed with a cross-fade transition."),
        "transition-none" => ("Instant", "Pushed with no animation."),
        _ => ("Slide", "Pushed with the native slide transition."),
    };
    page(
        title,
        "This page demonstrates one native navigation transition.",
        body![
            Text::new(detail).size(18).color(COLOR_TEXT),
            caption("Use the native Back button to return and try another style."),
        ],
    )
}

fn bottom_sheet_page() -> Element {
    page(
        "Bottom sheet",
        "A Rust button asks the session to present a platform-native bottom sheet.",
        body![
            Button::new("Open bottom sheet")
                .action("sheet:open")
                .width_percent(1.0),
            caption(
                "Android uses a Material BottomSheetDialogFragment; iOS uses                  UISheetPresentationController with medium and large detents.",
            ),
            caption("Close it with the sheet's button or drag it down."),
        ],
    )
}

fn pager_page(app: &ShowcaseApp) -> Element {
    let selected = app.pager_page.min(2);
    let card = |title: &'static str, body: &'static str, color: u32| -> Element {
        Column::new()
            .background(color)
            .padding(20)
            .spacing(8)
            .child(Text::new(title).size(22).bold().color(0xFFFF_FFFF))
            .child(Text::new(body).size(15).color(0xFFFF_FFFF))
            .into()
    };
    page(
        "View pager",
        "Swipe horizontally or let the app change the selected page.",
        body![
            Text::new(format!("Page {} of 3", selected + 1))
                .size(18)
                .color(COLOR_TEXT),
            Pager::new()
                .selected(selected)
                .action("demo:pager")
                .height(320)
                .accessibility_label("View pager")
                .child(card("One", "The first page.", 0xFF15_65C0))
                .child(card("Two", "The second page.", 0xFF00_897B))
                .child(card("Three", "The third page.", 0xFF6A_1B9A)),
            caption("Swiping reports the settled page through Event::Selected."),
        ],
    )
}

fn tabs_view_page(app: &ShowcaseApp) -> Element {
    let selected = app.tabs_view.min(2);
    let page_body = |title: &'static str, detail: &'static str, color: u32| -> Element {
        Column::new()
            .background(color)
            .padding(20)
            .spacing(8)
            .child(Text::new(title).size(22).bold().color(0xFFFF_FFFF))
            .child(Text::new(detail).size(15).color(0xFFFF_FFFF))
            .into()
    };
    page(
        "Tabs view",
        "A tab strip and pager share one action and one selected index.",
        body![
            Text::new(format!(
                "Selected: {}",
                ["Overview", "Details", "Settings"][selected]
            ))
            .size(18)
            .color(COLOR_TEXT),
            TabsView::new(["Overview", "Details", "Settings"])
                .selected(selected)
                .action("demo:tabs-view")
                .height(360)
                .child(page_body(
                    "Overview page",
                    "The first tab page.",
                    0xFF15_65C0,
                ))
                .child(page_body(
                    "Details page",
                    "Details page content.",
                    0xFF00_897B,
                ))
                .child(page_body(
                    "Settings page",
                    "Settings page content.",
                    0xFF6A_1B9A,
                )),
            caption("Tapping a tab and swiping the pager both dispatch Event::Selected."),
        ],
    )
}

fn bottom_navigation_page(app: &ShowcaseApp) -> Element {
    let selected = app.bottom_dest.min(2);
    let content = match selected {
        0 => "Home destination",
        1 => "Search destination",
        _ => "Settings destination",
    };
    Column::new()
        .padding(24)
        .spacing(14)
        .width_percent(1.0)
        .height_percent(1.0)
        .child(
            Text::new("Bottom navigation")
                .size(28)
                .bold()
                .color(COLOR_TEXT),
        )
        .child(caption(
            "The bar is a widget; the application owns the content above it.",
        ))
        .child(Text::new(content).size(20).color(COLOR_ACCENT).grow(1.0))
        .child(
            BottomNavigation::new([
                BottomNavItem::new("Home", Icon::new().drawable("ic_home").sf("house.fill")),
                BottomNavItem::new(
                    "Search",
                    Icon::new().drawable("ic_search").sf("magnifyingglass"),
                ),
                BottomNavItem::new(
                    "Settings",
                    Icon::new().drawable("ic_settings").sf("gearshape.fill"),
                ),
            ])
            .selected(selected)
            .action("demo:bottom-nav"),
        )
        .into()
}

fn toolbar_page(app: &ShowcaseApp) -> Element {
    page(
        "Toolbar",
        "The native toolbar is configured by the Screen, not by page content.",
        body![
            Text::new(format!("Toolbar taps: {}", app.toolbar_taps))
                .size(18)
                .color(COLOR_TEXT),
            Text::new(if app.toolbar_last.is_empty() {
                "No toolbar action yet.".to_string()
            } else {
                format!("Last action: {}", app.toolbar_last)
            })
            .size(18)
            .color(COLOR_ACCENT),
            caption(
                "Tap Refresh or Save in the native bar. The subtitle and actions come from the Screen description.",
            ),
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
