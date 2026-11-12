//! The gallery demo application.
//!
//! Demonstrates the UI components, flexbox layouts, typography, and interactive
//! state updates in a platform-independent way.

use std::sync::Arc;

use crate::app::{App, Event};
use crate::ui::{
    Align, Button, Checkbox, Column, CornerRadii, DatePicker, DateValue, EdgeInsets, Element,
    Gradient, Image, ImageScale, ImageSource, ImeAction, Justify, KeyboardType, List,
    ProgressIndicator, RadioGroup, Row, ScrollView, Slider, Spacer, Switch, Text, TextAlign,
    TextField,
};

pub const COLOR_MUTED: u32 = 0xFF66_6666;
pub const COLOR_WHITE: u32 = 0xFFFF_FFFF;
pub const COLOR_BLUE: u32 = 0xFF15_65C0;
pub const COLOR_TEAL: u32 = 0xFF00_897B;
pub const COLOR_AMBER: u32 = 0xFFF9_A825;
pub const COLOR_PURPLE: u32 = 0xFF6A_1B9A;
pub const COLOR_BORDER: u32 = 0xFFDD_DDDD;
pub const COLOR_SURFACE: u32 = 0xFFF4_F4F4;
pub const COLOR_LIGHT_BLUE: u32 = 0xFFE3_F2FD;
pub const COLOR_LIGHT_TEAL: u32 = 0xFFE0_F2F1;
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
    Controls,
    List,
    ListVariable,
    ListRecycled,
    ListRecycledVariable,
    ListRecycledForm,
}

/// The state of the gallery demo application.
#[derive(Clone, Debug, PartialEq)]
pub struct GalleryApp {
    pub demo: Demo,
    pub count: i64,
    pub name: String,
    pub bio: String,
    pub email: String,
    pub bio_focused: bool,
    pub submitted: bool,
    pub notifications: bool,
    /// Checkbox state in the controls demo.
    pub wifi: bool,
    /// Selected radio button in the controls demo.
    pub size_choice: usize,
    /// Slider value in the controls demo.
    pub volume: f32,
    /// Stepped slider value in the controls demo.
    pub stepped: f32,
    /// Date selected in the controls demo.
    pub date: DateValue,
    /// The thousand rows of the virtualized list demo, shared across rebuilds.
    pub list_items: Arc<Vec<String>>,
    /// Rows of mixed height for the measured-list demo.
    pub variable_items: Arc<Vec<String>>,
    /// Rows of the recycled-list demo, so the test can shrink the list.
    pub recycled_items: Arc<Vec<String>>,
    /// How many buttons inside recycled rows were pressed.
    pub recycled_taps: usize,
    /// A value shown inside every recycled row, to prove rebinds pick up state.
    pub recycled_bumps: usize,
    /// Notes edited by the recycled form demo.
    pub recycled_notes: Arc<Vec<String>>,
    /// Stable indices of the form rows.
    pub recycled_form_rows: Arc<Vec<usize>>,
}

impl Default for GalleryApp {
    fn default() -> Self {
        Self {
            demo: Demo::default(),
            count: 0,
            name: String::new(),
            bio: String::new(),
            email: String::new(),
            bio_focused: false,
            submitted: false,
            notifications: false,
            wifi: true,
            size_choice: 1,
            volume: 40.0,
            stepped: 30.0,
            date: DateValue::new(2026, 11, 12),
            list_items: Arc::new((1..=1000).map(|index| format!("Row {index}")).collect()),
            recycled_taps: 0,
            recycled_bumps: 0,
            recycled_items: Arc::new((1..=1000).map(|index| format!("Row {index}")).collect()),
            recycled_notes: Arc::new(vec![String::new(); 200]),
            recycled_form_rows: Arc::new((0..200).collect()),
            variable_items: Arc::new(
                (1..=300)
                    .map(|index| {
                        format!("Row {index} - {}", "wrapping text ".repeat(1 + index % 4))
                    })
                    .collect(),
            ),
        }
    }
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
                        "Every screen is described in Rust and drawn with real native views.",
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
                    .child(Button::new("Controls").action("open:controls"))
                    .child(Button::new("List").action("open:list"))
                    .child(Button::new("List (variable)").action("open:list-variable"))
                    .child(Button::new("List (recycled)").action("open:list-recycled"))
                    .child(
                        Button::new("List (recycled, variable)")
                            .action("open:list-recycled-variable"),
                    )
                    .child(Button::new("List (recycled, form)").action("open:list-recycled-form"))
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
                "The label is updated in place on the existing native label.",
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
            .child(
                Text::new("Centered body text")
                    .size(16)
                    .text_align(TextAlign::Center)
                    .width_percent(1.0),
            )
            .child(
                Text::new("Trailing body text")
                    .size(16)
                    .text_align(TextAlign::End)
                    .width_percent(1.0),
            )
            .child(caption("Two lines maximum, truncated with an ellipsis:"))
            .child(
                Text::new(
                    "This paragraph is long enough to need more lines than the limit, \
                     so it is clipped and the last visible line ends with an ellipsis.",
                )
                .size(14)
                .max_lines(2)
                .ellipsis()
                .width_percent(1.0),
            )
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
        let focus = if self.bio_focused {
            "The bio field is focused."
        } else {
            "The bio field is not focused."
        };
        let submitted = if self.submitted {
            "Submitted from the keyboard."
        } else {
            "Press the keyboard action key on the name field to submit."
        };

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
                    .child(Text::new("Form").size(26).bold())
                    .child(caption(
                        "Edits, submits and focus changes all reach Rust as typed events.",
                    ))
                    .child(
                        TextField::new(self.name.clone())
                            .placeholder("Your name")
                            .action("form:name")
                            .submit_action("form:submit")
                            .ime_action(ImeAction::Next)
                            .size(18)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .background(COLOR_SURFACE)
                            .corner_radius(10)
                            .border(1, COLOR_BORDER)
                            .width_percent(1.0),
                    )
                    .child(Text::new(greeting).size(20).color(COLOR_BLUE))
                    .child(
                        TextField::new(self.bio.clone())
                            .placeholder("Tell us about yourself")
                            .action("form:bio")
                            .focus_action("form:bio-focus")
                            .multiline(true)
                            .size(16)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .background(COLOR_SURFACE)
                            .corner_radius(10)
                            .border(1, COLOR_BORDER)
                            .height(96)
                            .width_percent(1.0),
                    )
                    .child(caption(focus))
                    .child(
                        TextField::new(self.email.clone())
                            .placeholder("you@example.com")
                            .action("form:email")
                            .keyboard(KeyboardType::Email)
                            .ime_action(ImeAction::Send)
                            .submit_action("form:submit")
                            .size(16)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .background(COLOR_SURFACE)
                            .corner_radius(10)
                            .border(1, COLOR_BORDER)
                            .width_percent(1.0),
                    )
                    .child(caption(submitted))
                    .child(Button::new("Clear").action("form:clear")),
            )
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
                        "Bundled assets load instantly; remote images load from a URL.",
                    ))
                    .child(
                        Image::new(ImageSource::asset("krabik.webp"))
                            .width(96)
                            .height(96)
                            .corner_radius(18)
                            .accessibility_label("Krabik app icon"),
                    )
                    .child(caption("Asset image, 96x96 with rounded corners"))
                    .child(
                        Image::new(ImageSource::url("https://picsum.photos/600/300"))
                            .width_percent(1.0)
                            .aspect_ratio(2.0)
                            .corner_radius(16)
                            .scale(ImageScale::Fill)
                            .accessibility_label("Remote landscape photo"),
                    )
                    .child(caption("Remote image, 2:1 aspect ratio cropped to fill")),
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
                        Column::new()
                            .width_percent(1.0)
                            .padding(16)
                            .spacing(8)
                            .gradient(Gradient::horizontal(COLOR_LIGHT_BLUE, COLOR_LIGHT_TEAL))
                            .corner_radii(CornerRadii::new(24.0, 6.0, 24.0, 6.0))
                            .elevation(6)
                            .child(Text::new("Gradient card").size(18).bold())
                            .child(caption(
                                "Two-color gradient, per-corner radii and native elevation.",
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
                            .child(Button::new("Faded").action("styling:noop").opacity(0.45))
                            .child(
                                Button::new("Disabled")
                                    .action("styling:noop")
                                    .enabled(false)
                                    .background(COLOR_SURFACE)
                                    .corner_radius(22),
                            ),
                    )
                    .child(caption(
                        "Buttons keep their native interaction and honour enabled state.",
                    )),
            )
            .into()
    }

    fn list(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("List").size(26).bold())
            .child(caption(
                "1000 rows, but only the ones around the viewport exist as native views.",
            ))
            .child(
                List::new(Arc::clone(&self.list_items))
                    .item_height(44.0)
                    .buffer(4)
                    .key(|item| item.clone())
                    .row(|item| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .child(Text::new(item.clone()).size(15))
                            .into()
                    }),
            )
            .into()
    }

    fn list_variable(&self) -> Element {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("List (variable)").size(26).bold())
            .child(caption(
                "Rows are measured on first sight; scrolling past them uses the cached heights.",
            ))
            .child(
                List::new(Arc::clone(&self.variable_items))
                    .revision(1)
                    .buffer(4)
                    .key(|item| item.clone())
                    .row(|item| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .child(Text::new(item.clone()).size(15).width_percent(1.0))
                            .into()
                    }),
            )
            .into()
    }

    fn list_recycled(&self) -> Element {
        let bumps = self.recycled_bumps;
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("List (recycled)").size(26).bold())
            .child(caption(
                "A native recycler owns scrolling; Rust diffs each row on bind.",
            ))
            .child(Text::new(format!("Taps: {}", self.recycled_taps)).size(15))
            .child(Text::new(format!("Rows: {}", self.recycled_items.len())).size(15))
            .child(
                Row::new()
                    .spacing(8)
                    .child(Button::new("Bump rows").action("recycled:bump"))
                    .child(Button::new("Shrink").action("recycled:shrink"))
                    .child(Button::new("Reset").action("recycled:reset")),
            )
            .child(
                List::new(Arc::clone(&self.recycled_items))
                    .item_height(56.0)
                    .recycled()
                    .key(|item| item.clone())
                    .row(move |item| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 6))
                            .align_items(Align::Center)
                            .child(Text::new(format!("{item} · v{}", bumps)).size(15).grow(1.0))
                            .child(Button::new("Tap").action("recycled:tap"))
                            .into()
                    }),
            )
            .into()
    }

    fn list_recycled_form(&self) -> Element {
        let notes = Arc::clone(&self.recycled_notes);
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("List (recycled, form)").size(26).bold())
            .child(caption(
                "A focused field returns with its value after its row is recycled.",
            ))
            .child(Button::new("Refresh").action("recycled:refresh"))
            .child(
                List::new(Arc::clone(&self.recycled_form_rows))
                    .item_height(56.0)
                    .recycled()
                    .key(|index| index.to_string())
                    .row(move |index| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 6))
                            .align_items(Align::Center)
                            .child(
                                TextField::new(notes[*index].clone())
                                    .placeholder(format!("Note {}", index + 1))
                                    .action(format!("recycled:note:{index}"))
                                    .size(15)
                                    .grow(1.0),
                            )
                            .into()
                    }),
            )
            .into()
    }

    fn list_recycled_variable(&self) -> Element {
        let bumps = self.recycled_bumps;
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("List (recycled, variable)").size(26).bold())
            .child(caption(
                "Each holder takes the height Rust measured for its row.",
            ))
            .child(Text::new(format!("Taps: {}", self.recycled_taps)).size(15))
            .child(
                List::new(Arc::clone(&self.variable_items))
                    .recycled()
                    .key(|item| item.clone())
                    .row(move |item| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .align_items(Align::Center)
                            .child(Text::new(format!("{item} · v{}", bumps)).size(15).grow(1.0))
                            .child(Button::new("Tap").action("recycled:tap"))
                            .into()
                    }),
            )
            .into()
    }

    fn controls(&self) -> Element {
        let state = if self.notifications {
            "Notifications are on."
        } else {
            "Notifications are off."
        };
        let size = ["Small", "Medium", "Large"]
            .get(self.size_choice)
            .copied()
            .unwrap_or("Medium");

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
                    .child(Text::new("Controls").size(26).bold())
                    .child(caption(
                        "Switches, checks, radio buttons and sliders report typed events.",
                    ))
                    .child(
                        Row::new()
                            .width_percent(1.0)
                            .spacing(12)
                            .align_items(Align::Center)
                            .child(Text::new("Notifications").size(18))
                            .child(Spacer::new())
                            .child(
                                Switch::new(self.notifications)
                                    .action("controls:notifications")
                                    .accessibility_label("Notifications"),
                            ),
                    )
                    .child(caption(state))
                    .child(
                        Checkbox::new("Wi-Fi", self.wifi)
                            .action("controls:wifi")
                            .accessibility_label("Wi-Fi"),
                    )
                    .child(caption(if self.wifi {
                        "Wi-Fi is on."
                    } else {
                        "Wi-Fi is off."
                    }))
                    .child(Text::new(format!("Size: {size}")).size(18))
                    .child(
                        RadioGroup::new(["Small", "Medium", "Large"])
                            .selected(self.size_choice)
                            .action("controls:size")
                            .accessibility_label("Size")
                            .width_percent(1.0),
                    )
                    .child(Text::new(format!("Volume: {:.0}", self.volume)).size(18))
                    .child(
                        Slider::new(self.volume)
                            .min(0.0)
                            .max(100.0)
                            .action("controls:volume")
                            .accessibility_label("Volume")
                            .width_percent(1.0),
                    )
                    .child(Text::new(format!("Stepped: {:.0}", self.stepped)).size(18))
                    .child(
                        Slider::new(self.stepped)
                            .min(0.0)
                            .max(100.0)
                            .step(10.0)
                            .action("controls:stepped")
                            .accessibility_label("Stepped")
                            .width_percent(1.0),
                    )
                    .child(Text::new("Progress").size(18))
                    .child(
                        ProgressIndicator::new(self.volume / 100.0)
                            .color(COLOR_TEAL)
                            .accessibility_label("Progress bar")
                            .width_percent(1.0),
                    )
                    .child(Text::new("Loading").size(18))
                    .child(
                        ProgressIndicator::indeterminate()
                            .color(COLOR_TEAL)
                            .accessibility_label("Loading spinner")
                            .width(24)
                            .height(24),
                    )
                    .child(Text::new(format!("Date: {}", self.date)).size(18))
                    .child(
                        DatePicker::new(self.date)
                            .action("controls:date")
                            .accessibility_label("Date"),
                    ),
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
            Demo::Controls => self.controls(),
            Demo::List => self.list(),
            Demo::ListVariable => self.list_variable(),
            Demo::ListRecycled => self.list_recycled(),
            Demo::ListRecycledVariable => self.list_recycled_variable(),
            Demo::ListRecycledForm => self.list_recycled_form(),
        }
    }

    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::TextChanged {
                action: "form:name",
                value,
            } => {
                self.name = value.to_string();
                self.submitted = false;
            }
            Event::TextChanged {
                action: "form:bio",
                value,
            } => self.bio = value.to_string(),
            Event::TextChanged {
                action: "form:email",
                value,
            } => {
                self.email = value.to_string();
                self.submitted = false;
            }
            Event::TextChanged { action, value } => {
                let Some(index) = action
                    .strip_prefix("recycled:note:")
                    .and_then(|index| index.parse::<usize>().ok())
                else {
                    return false;
                };
                if index >= self.recycled_notes.len() {
                    return false;
                }
                Arc::make_mut(&mut self.recycled_notes)[index] = value.to_string();
                // The native field already shows the text; rebinding the whole
                // list on every keystroke would only disturb typing.
                return false;
            }
            Event::Submitted {
                action: "form:submit",
            } => self.submitted = true,
            Event::FocusChanged {
                action: "form:bio-focus",
                focused,
            } => self.bio_focused = focused,
            Event::Back => {
                if self.demo == Demo::Gallery {
                    return false;
                }
                self.demo = Demo::Gallery;
            }
            Event::Toggled {
                action: "controls:notifications",
                checked,
            } => self.notifications = checked,
            Event::Toggled {
                action: "controls:wifi",
                checked,
            } => self.wifi = checked,
            Event::Selected {
                action: "controls:size",
                index,
            } => self.size_choice = index,
            Event::ValueChanged {
                action: "controls:volume",
                value,
            } => self.volume = value,
            Event::ValueChanged {
                action: "controls:stepped",
                value,
            } => self.stepped = value,
            Event::DateChanged {
                action: "controls:date",
                date,
            } => self.date = date,
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
                "open:controls" => self.demo = Demo::Controls,
                "open:list" => self.demo = Demo::List,
                "open:list-variable" => self.demo = Demo::ListVariable,
                "open:list-recycled" => self.demo = Demo::ListRecycled,
                "open:list-recycled-variable" => self.demo = Demo::ListRecycledVariable,
                "open:list-recycled-form" => self.demo = Demo::ListRecycledForm,
                "recycled:refresh" => {}
                "recycled:tap" => self.recycled_taps += 1,
                "recycled:bump" => self.recycled_bumps += 1,
                "recycled:shrink" => {
                    if self.recycled_items.len() > 200 {
                        self.recycled_items =
                            Arc::new(self.recycled_items.iter().take(200).cloned().collect());
                    }
                }
                "recycled:reset" => {
                    self.recycled_items =
                        Arc::new((1..=1000).map(|index| format!("Row {index}")).collect());
                    self.recycled_taps = 0;
                    self.recycled_bumps = 0;
                }
                "counter:add" => self.count += 1,
                "counter:reset" => self.count = 0,
                "form:clear" => {
                    self.name.clear();
                    self.bio.clear();
                    self.email.clear();
                    self.submitted = false;
                }
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
    fn controls_dispatch_typed_events() {
        let mut app = GalleryApp::new();
        assert!(app.wifi);
        assert_eq!(app.size_choice, 1);
        assert_eq!(app.volume, 40.0);

        assert!(app.handle(Event::Toggled {
            action: "controls:wifi",
            checked: false,
        }));
        assert!(!app.wifi);

        assert!(app.handle(Event::Selected {
            action: "controls:size",
            index: 2,
        }));
        assert_eq!(app.size_choice, 2);

        assert!(app.handle(Event::ValueChanged {
            action: "controls:volume",
            value: 75.0,
        }));
        assert_eq!(app.volume, 75.0);

        assert!(app.handle(Event::ValueChanged {
            action: "controls:stepped",
            value: 70.0,
        }));
        assert_eq!(app.stepped, 70.0);

        assert!(app.handle(Event::DateChanged {
            action: "controls:date",
            date: DateValue::new(2027, 1, 2),
        }));
        assert_eq!(app.date, DateValue::new(2027, 1, 2));
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

        assert!(!app.handle(Event::Back));
        assert!(app.handle(click("open:counter")));
        assert!(app.handle(Event::Back));
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

        assert!(app.handle(click("open:controls")));
        assert_eq!(app.demo, Demo::Controls);

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

        assert!(app.handle(Event::TextChanged {
            action: "form:bio",
            value: "Loves layout engines.",
        }));
        assert_eq!(app.bio, "Loves layout engines.");

        assert!(app.handle(Event::TextChanged {
            action: "form:email",
            value: "ada@example.com",
        }));
        assert_eq!(app.email, "ada@example.com");

        assert!(app.handle(click("form:clear")));
        assert!(app.name.is_empty());
        assert!(app.bio.is_empty());
        assert!(app.email.is_empty());
        assert!(!app.submitted);

        assert!(!app.handle(Event::TextChanged {
            action: "unknown",
            value: "ignored",
        }));
    }

    #[test]
    fn submitting_and_focusing_update_the_form_state() {
        let mut app = GalleryApp::new();
        assert!(!app.submitted);
        assert!(!app.bio_focused);

        assert!(app.handle(Event::FocusChanged {
            action: "form:bio-focus",
            focused: true,
        }));
        assert!(app.bio_focused);

        assert!(app.handle(Event::FocusChanged {
            action: "form:bio-focus",
            focused: false,
        }));
        assert!(!app.bio_focused);

        assert!(app.handle(Event::Submitted {
            action: "form:submit",
        }));
        assert!(app.submitted);

        // Editing a submitted field clears the confirmation.
        assert!(app.handle(Event::TextChanged {
            action: "form:name",
            value: "Grace",
        }));
        assert!(!app.submitted);

        assert!(!app.handle(Event::Submitted { action: "unknown" }));
        assert!(!app.handle(Event::FocusChanged {
            action: "unknown",
            focused: true,
        }));
    }

    #[test]
    fn toggling_a_switch_updates_the_controls_state() {
        let mut app = GalleryApp::new();
        assert!(!app.notifications);

        assert!(app.handle(Event::Toggled {
            action: "controls:notifications",
            checked: true,
        }));
        assert!(app.notifications);

        assert!(app.handle(Event::Toggled {
            action: "controls:notifications",
            checked: false,
        }));
        assert!(!app.notifications);

        assert!(!app.handle(Event::Toggled {
            action: "unknown",
            checked: true,
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
            Demo::Controls,
        ] {
            app.demo = demo;
            let element = app.view();
            assert!(!element.children().is_empty());
        }
    }
}
