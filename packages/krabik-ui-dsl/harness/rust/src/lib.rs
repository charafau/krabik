//! The development harness: a gallery of every widget and layout.
//!
//! It is the app the framework itself is developed and UI-tested against, and
//! like any other app it is its own crate that depends on [`krabik_ui_dsl`]
//! and registers itself with [`set_app_factory`].

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::ui::{
    Align, Button, Checkbox, Color, Column, CornerRadii, DatePicker, DatePickerValue, DateValue,
    EdgeInsets, Element, Gradient, Image, ImageScale, ImageSource, ImeAction, Justify,
    KeyboardType, List, ProgressIndicator, RadioGroup, Row, ScrollView, Slider, Spacer, Switch,
    TabBar, Text, TextAlign, TextField,
};

pub const COLOR_MUTED: Color = Color::adaptive(0xFF66_6666, 0xFFA0_A4A8);
pub const COLOR_WHITE: Color = Color::fixed(0xFFFF_FFFF);
pub const COLOR_BLUE: Color = Color::fixed(0xFF15_65C0);
pub const COLOR_TEAL: Color = Color::fixed(0xFF00_897B);
pub const COLOR_AMBER: Color = Color::fixed(0xFFF9_A825);
pub const COLOR_PURPLE: Color = Color::fixed(0xFF6A_1B9A);
pub const COLOR_BORDER: Color = Color::adaptive(0xFFDD_DDDD, 0xFF44_474A);
pub const COLOR_SURFACE: Color = Color::adaptive(0xFFF4_F4F4, 0xFF2A_2B2E);
pub const COLOR_LIGHT_BLUE: Color = Color::adaptive(0xFFE3_F2FD, 0xFF17_324D);
pub const COLOR_LIGHT_TEAL: Color = Color::adaptive(0xFFE0_F2F1, 0xFF14_3835);
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
    Rejected,
    List,
    ListVariable,
    ListRecycled,
    ListRecycledVariable,
    ListRecycledForm,
}

/// Everything that can happen in the gallery.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Opens a demo from the list.
    Open(Demo),
    /// Returns to the list.
    Back,
    CounterAdd,
    CounterReset,
    Name(String),
    Bio(String),
    Email(String),
    Submit,
    BioFocus(bool),
    ClearForm,
    /// A button that exists only to show a style.
    Noop,
    Notifications(bool),
    Wifi(bool),
    Size(usize),
    Volume(f32),
    Stepped(f32),
    Date(DatePickerValue),
    RecycledTap,
    RecycledBump,
    RecycledShrink,
    RecycledReset,
    /// Re-renders, which rebinds the visible rows.
    RecycledRefresh,
    /// The note at this index was edited.
    Note(usize, String),
    /// A control the application never lets the user change.
    Refused,
    /// A slider the application caps at 50.
    Capped(f32),
    /// A field the application strips everything but digits from.
    Digits(String),
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
    /// How often the user tried to change a control that stays as it is.
    pub refused: usize,
    /// A slider value the application keeps at or below 50.
    pub capped: f32,
    /// A field that only keeps digits.
    pub digits: String,
    /// The rows of the list in the rejected-input demo.
    pub locked_rows: Arc<Vec<usize>>,
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
            refused: 0,
            capped: 20.0,
            digits: String::new(),
            locked_rows: Arc::new((0..3).collect()),
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

    fn gallery(&self) -> Element<Msg> {
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
                    .child(Button::new("Counter").on_click(Msg::Open(Demo::Counter)))
                    .child(Button::new("Layouts").on_click(Msg::Open(Demo::Layouts)))
                    .child(Button::new("Typography").on_click(Msg::Open(Demo::Typography)))
                    .child(Button::new("Alignment").on_click(Msg::Open(Demo::Alignment)))
                    .child(Button::new("Scroll").on_click(Msg::Open(Demo::Scroll)))
                    .child(Button::new("Form").on_click(Msg::Open(Demo::Form)))
                    .child(Button::new("Media").on_click(Msg::Open(Demo::Media)))
                    .child(Button::new("Styling").on_click(Msg::Open(Demo::Styling)))
                    .child(Button::new("Controls").on_click(Msg::Open(Demo::Controls)))
                    .child(Button::new("List").on_click(Msg::Open(Demo::List)))
                    .child(Button::new("List (variable)").on_click(Msg::Open(Demo::ListVariable)))
                    .child(Button::new("List (recycled)").on_click(Msg::Open(Demo::ListRecycled)))
                    .child(
                        Button::new("List (recycled, variable)")
                            .on_click(Msg::Open(Demo::ListRecycledVariable)),
                    )
                    .child(
                        Button::new("List (recycled, form)")
                            .on_click(Msg::Open(Demo::ListRecycledForm)),
                    )
                    .child(Button::new("Rejected input").on_click(Msg::Open(Demo::Rejected)))
                    .child(caption(
                        "This list scrolls, so every demo stays reachable on small screens.",
                    )),
            )
            .into()
    }

    fn counter(&self) -> Element<Msg> {
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
                    .child(Button::new("Add one").on_click(Msg::CounterAdd))
                    .child(Button::new("Reset").on_click(Msg::CounterReset)),
            )
            .into()
    }

    fn layouts(&self) -> Element<Msg> {
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

    fn typography(&self) -> Element<Msg> {
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

    fn alignment(&self) -> Element<Msg> {
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

    fn scroll(&self) -> Element<Msg> {
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

    fn form(&self) -> Element<Msg> {
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
                            .on_change(Msg::Name)
                            .on_submit(Msg::Submit)
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
                            .on_change(Msg::Bio)
                            .on_focus(Msg::BioFocus)
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
                            .on_change(Msg::Email)
                            .keyboard(KeyboardType::Email)
                            .ime_action(ImeAction::Send)
                            .on_submit(Msg::Submit)
                            .size(16)
                            .padding_each(EdgeInsets::symmetric(12, 10))
                            .background(COLOR_SURFACE)
                            .corner_radius(10)
                            .border(1, COLOR_BORDER)
                            .width_percent(1.0),
                    )
                    .child(caption(submitted))
                    .child(Button::new("Clear").on_click(Msg::ClearForm)),
            )
            .into()
    }

    fn media(&self) -> Element<Msg> {
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

    fn styling(&self) -> Element<Msg> {
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
                                    .on_click(Msg::Noop)
                                    .background(COLOR_SURFACE)
                                    .corner_radius(22),
                            )
                            .child(Button::new("Faded").on_click(Msg::Noop).opacity(0.45))
                            .child(
                                Button::new("Disabled")
                                    .on_click(Msg::Noop)
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

    fn list(&self) -> Element<Msg> {
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

    fn list_variable(&self) -> Element<Msg> {
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

    fn list_recycled(&self) -> Element<Msg> {
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
                    .child(Button::new("Bump rows").on_click(Msg::RecycledBump))
                    .child(Button::new("Shrink").on_click(Msg::RecycledShrink))
                    .child(Button::new("Reset").on_click(Msg::RecycledReset)),
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
                            .child(Button::new("Tap").on_click(Msg::RecycledTap))
                            .into()
                    }),
            )
            .into()
    }

    fn list_recycled_form(&self) -> Element<Msg> {
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
            .child(Button::new("Refresh").on_click(Msg::RecycledRefresh))
            .child(
                List::new(Arc::clone(&self.recycled_form_rows))
                    .item_height(56.0)
                    .recycled()
                    .key(|index| index.to_string())
                    .row(move |index| {
                        let index = *index;
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 6))
                            .align_items(Align::Center)
                            .child(
                                TextField::new(notes[index].clone())
                                    .placeholder(format!("Note {}", index + 1))
                                    .on_change(move |text| Msg::Note(index, text))
                                    .size(15)
                                    .grow(1.0),
                            )
                            .into()
                    }),
            )
            .into()
    }

    fn list_recycled_variable(&self) -> Element<Msg> {
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
                            .child(Button::new("Tap").on_click(Msg::RecycledTap))
                            .into()
                    }),
            )
            .into()
    }

    /// Controls whose new value the application does not accept. Each one
    /// has to return to what the state says.
    fn rejected(&self) -> Element<Msg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(back_button())
            .child(Text::new("Rejected input").size(26).bold())
            .child(caption(
                "The application overrules these controls; each returns to its state.",
            ))
            .child(Text::new(format!("Refused: {}", self.refused)).size(18))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(12)
                    .align_items(Align::Center)
                    .child(Text::new("Locked").size(18))
                    .child(Spacer::new())
                    .child(
                        Switch::new(false)
                            .on_toggle(|_| Msg::Refused)
                            .accessibility_label("Locked switch"),
                    ),
            )
            .child(
                TabBar::new(["First", "Second"])
                    .selected(0)
                    .on_select(|_| Msg::Refused)
                    .accessibility_label("Locked tabs")
                    .width_percent(1.0),
            )
            .child(Text::new(format!("Capped: {:.0}", self.capped)).size(18))
            .child(
                Slider::new(self.capped)
                    .min(0.0)
                    .max(100.0)
                    .on_change(Msg::Capped)
                    .accessibility_label("Capped")
                    .width_percent(1.0),
            )
            .child(Text::new(format!("Digits: {}", self.digits)).size(18))
            .child(
                TextField::new(self.digits.clone())
                    .placeholder("Digits only")
                    .on_change(Msg::Digits)
                    .accessibility_label("Digits only")
                    .width_percent(1.0),
            )
            .child(
                List::new(Arc::clone(&self.locked_rows))
                    .item_height(56.0)
                    .recycled()
                    .key(|index| index.to_string())
                    .row(|index| {
                        Row::new()
                            .width_percent(1.0)
                            .padding_each(EdgeInsets::symmetric(12, 6))
                            .spacing(12)
                            .align_items(Align::Center)
                            .child(Text::new(format!("Locked row {}", index + 1)).size(15))
                            .child(Spacer::new())
                            .child(
                                Switch::new(false)
                                    .on_toggle(|_| Msg::Refused)
                                    .accessibility_label(format!("Row switch {}", index + 1)),
                            )
                            .into()
                    }),
            )
            .into()
    }

    fn controls(&self) -> Element<Msg> {
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
                                    .on_toggle(Msg::Notifications)
                                    .accessibility_label("Notifications"),
                            ),
                    )
                    .child(caption(state))
                    .child(
                        Checkbox::new("Wi-Fi", self.wifi)
                            .on_toggle(Msg::Wifi)
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
                            .on_select(Msg::Size)
                            .accessibility_label("Size")
                            .width_percent(1.0),
                    )
                    .child(Text::new(format!("Volume: {:.0}", self.volume)).size(18))
                    .child(
                        Slider::new(self.volume)
                            .min(0.0)
                            .max(100.0)
                            .on_change(Msg::Volume)
                            .accessibility_label("Volume")
                            .width_percent(1.0),
                    )
                    .child(Text::new(format!("Stepped: {:.0}", self.stepped)).size(18))
                    .child(
                        Slider::new(self.stepped)
                            .min(0.0)
                            .max(100.0)
                            .step(10.0)
                            .on_change(Msg::Stepped)
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
                            .on_change(Msg::Date)
                            .accessibility_label("Date"),
                    ),
            )
            .into()
    }
}

/// Twenty rows that overflow the vertical scroll viewport.
fn demo_rows() -> Column<Msg> {
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
fn demo_strip() -> ScrollView<Msg> {
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

impl Component for GalleryApp {
    type Msg = Msg;

    fn view(&self) -> Element<Msg> {
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
            Demo::Rejected => self.rejected(),
            Demo::List => self.list(),
            Demo::ListVariable => self.list_variable(),
            Demo::ListRecycled => self.list_recycled(),
            Demo::ListRecycledVariable => self.list_recycled_variable(),
            Demo::ListRecycledForm => self.list_recycled_form(),
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Open(demo) => {
                if demo == Demo::Counter {
                    self.count = 0;
                }
                self.demo = demo;
            }
            Msg::Back => self.demo = Demo::Gallery,
            Msg::CounterAdd => self.count += 1,
            Msg::CounterReset => self.count = 0,
            Msg::Name(name) => {
                self.name = name;
                self.submitted = false;
            }
            Msg::Bio(bio) => self.bio = bio,
            Msg::Email(email) => {
                self.email = email;
                self.submitted = false;
            }
            Msg::Submit => self.submitted = true,
            Msg::BioFocus(focused) => self.bio_focused = focused,
            Msg::ClearForm => {
                self.name.clear();
                self.bio.clear();
                self.email.clear();
                self.submitted = false;
            }
            Msg::Noop | Msg::RecycledRefresh => {}
            Msg::Notifications(on) => self.notifications = on,
            Msg::Wifi(on) => self.wifi = on,
            Msg::Size(index) => self.size_choice = index,
            Msg::Volume(value) => self.volume = value,
            Msg::Stepped(value) => self.stepped = value,
            Msg::Date(value) => {
                if let DatePickerValue::Date(date) = value {
                    self.date = date;
                }
            }
            Msg::RecycledTap => self.recycled_taps += 1,
            Msg::RecycledBump => self.recycled_bumps += 1,
            Msg::RecycledShrink => {
                if self.recycled_items.len() > 200 {
                    self.recycled_items =
                        Arc::new(self.recycled_items.iter().take(200).cloned().collect());
                }
            }
            Msg::RecycledReset => {
                self.recycled_items =
                    Arc::new((1..=1000).map(|index| format!("Row {index}")).collect());
                self.recycled_taps = 0;
                self.recycled_bumps = 0;
            }
            Msg::Note(index, text) => {
                if let Some(note) = Arc::make_mut(&mut self.recycled_notes).get_mut(index) {
                    *note = text;
                }
            }
            // The state the control shows is deliberately left alone.
            Msg::Refused => self.refused += 1,
            Msg::Capped(value) => self.capped = value.min(50.0),
            Msg::Digits(text) => {
                self.digits = text.chars().filter(char::is_ascii_digit).collect();
            }
        }
    }
}

impl App for GalleryApp {
    /// Back returns to the list; on the list the platform leaves the app.
    fn back(&self) -> Option<Msg> {
        (self.demo != Demo::Gallery).then_some(Msg::Back)
    }
}

fn back_button() -> Button<Msg> {
    Button::new("Back").on_click(Msg::Back)
}

fn caption(text: &str) -> Text<Msg> {
    Text::new(text).size(13).color(COLOR_MUTED)
}

fn chip(text: &str, color: Color) -> Text<Msg> {
    Text::new(text)
        .size(14)
        .color(COLOR_WHITE)
        .background(color)
        .padding_each(EdgeInsets::symmetric(10, 4))
}

fn chips(justify: Justify) -> Row<Msg> {
    Row::new()
        .width_percent(1.0)
        .spacing(8)
        .justify_content(justify)
        .child(chip("A", COLOR_BLUE))
        .child(chip("B", COLOR_TEAL))
        .child(chip("C", COLOR_AMBER))
}

impl GalleryApp {
    /// Registers the gallery as the application.
    pub fn install() {
        set_app_factory(Self::new);
    }
}

/// C entry point called by the iOS host before creating the root view.
#[unsafe(no_mangle)]
pub extern "C" fn krabik_ui_dsl_harness_install() {
    GalleryApp::install();
}

/// JNI entry point called by the Android host before creating the root view.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_GalleryBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    GalleryApp::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_gallery() {
        let app = GalleryApp::new();
        assert_eq!(app.demo, Demo::Gallery);
        assert_eq!(app.back(), None);
    }

    #[test]
    fn controls_messages_update_the_state() {
        let mut app = GalleryApp::new();
        app.update(Msg::Wifi(false));
        assert!(!app.wifi);
        app.update(Msg::Notifications(true));
        assert!(app.notifications);
        app.update(Msg::Size(2));
        assert_eq!(app.size_choice, 2);
        app.update(Msg::Volume(75.0));
        assert_eq!(app.volume, 75.0);
        app.update(Msg::Stepped(60.0));
        assert_eq!(app.stepped, 60.0);
        app.update(Msg::Date(DatePickerValue::Date(DateValue::new(2027, 1, 2))));
        assert_eq!(app.date, DateValue::new(2027, 1, 2));
    }

    #[test]
    fn opening_a_demo_and_going_back() {
        let mut app = GalleryApp::new();
        app.update(Msg::Open(Demo::Counter));
        assert_eq!(app.demo, Demo::Counter);
        assert_eq!(app.back(), Some(Msg::Back));

        app.update(Msg::CounterAdd);
        app.update(Msg::CounterAdd);
        assert_eq!(app.count, 2);
        app.update(Msg::CounterReset);
        assert_eq!(app.count, 0);

        app.update(Msg::Back);
        assert_eq!(app.demo, Demo::Gallery);

        // Reopening the counter starts from zero again.
        app.count = 5;
        app.update(Msg::Open(Demo::Counter));
        assert_eq!(app.count, 0);
    }

    #[test]
    fn form_messages_update_the_form_state() {
        let mut app = GalleryApp::new();
        app.update(Msg::Name("Ada".into()));
        app.update(Msg::Bio("Engineer".into()));
        app.update(Msg::Email("ada@example.com".into()));
        assert_eq!(app.name, "Ada");
        assert_eq!(app.bio, "Engineer");
        assert_eq!(app.email, "ada@example.com");

        app.update(Msg::BioFocus(true));
        assert!(app.bio_focused);
        app.update(Msg::Submit);
        assert!(app.submitted);
        // Editing again clears the submitted flag.
        app.update(Msg::Name("Ada L".into()));
        assert!(!app.submitted);

        app.update(Msg::ClearForm);
        assert!(app.name.is_empty() && app.bio.is_empty() && app.email.is_empty());
    }

    #[test]
    fn notes_are_edited_by_index() {
        let mut app = GalleryApp::new();
        app.update(Msg::Note(3, "hello".into()));
        assert_eq!(app.recycled_notes[3], "hello");
        // An index outside the list is ignored.
        app.update(Msg::Note(10_000, "nope".into()));
    }

    #[test]
    fn rejected_input_keeps_the_state() {
        let mut app = GalleryApp::new();
        app.update(Msg::Refused);
        assert_eq!(app.refused, 1);
        app.update(Msg::Capped(80.0));
        assert_eq!(app.capped, 50.0);
        app.update(Msg::Digits("a1b2".into()));
        assert_eq!(app.digits, "12");
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
            Demo::Rejected,
        ] {
            app.demo = demo;
            let element = app.view();
            assert!(!element.children().is_empty());
        }
    }
}
