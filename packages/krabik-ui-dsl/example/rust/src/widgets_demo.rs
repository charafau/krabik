//! The pages for the layout additions and the widgets that are assembled
//! from others: their state, their messages and their views.

use std::sync::Arc;

use krabik_ui_dsl::ui::{
    Align, Autocomplete, Avatar, Badge, Button, Card, Chip, Color, Column, Container, Divider,
    Dropdown, Element, ExpansionTile, Grid, Icon, IconButton, Image, Inset, Justify, List,
    ListTile, Menu, MenuButton, NativeView, NumberStepper, RangeSlider, Row, SearchBar,
    SegmentedControl, Stack, Stepper, Switch, Text, TextAlign, WebView, Wrap,
};

use crate::pages::{COLOR_ACCENT, COLOR_SURFACE, COLOR_TEXT, caption};

const LANGUAGES: [&str; 6] = ["Rust", "Kotlin", "Swift", "Dart", "TypeScript", "Go"];
const COUNTRIES: [&str; 4] = ["Poland", "Japan", "Norway", "Portugal"];
const RANGES: [&str; 3] = ["Day", "Week", "Month"];
/// A page that needs no network.
const WEB_PAGE: &str = "data:text/html,<body style='font-family:sans-serif;padding:16px'><h2>Hello from a web view</h2><p>This page is a data URL.</p></body>";
const VIDEO_URL: &str = "https://interactive-examples.mdn.mozilla.net/media/cc0-videos/flower.mp4";
const CITIES: [&str; 8] = [
    "Kraków", "Kyoto", "Tokyo", "Oslo", "Osaka", "Lisbon", "London", "Toronto",
];

#[derive(Clone, Debug, PartialEq)]
pub enum WidgetsMsg {
    Tapped,
    Held,
    Language(usize),
    RemoveTag(String),
    ResetTags,
    Wifi(bool),
    OpenWifi,
    Unread,
    ClearUnread,
    Share,
    Toggle,
    Step(usize),
    Quantity(f32),
    Query(String),
    Search,
    City(String),
    Sort(&'static str),
    Action(&'static str),
    Country(usize),
    Range(usize),
    Price((f32, f32)),
    Stars(String),
    Loaded(String),
    Video(String),
    ShowVideo,
    DismissRow(u32),
    MoveRow(usize, usize),
    ResetRows,
}

/// One row of the list gestures page: a section header or an entry.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    id: u32,
    label: String,
    header: bool,
}

fn entries() -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut id = 0;
    for section in ["Fruit", "Vegetables", "Grains", "Spices"] {
        id += 1;
        entries.push(Entry {
            id,
            label: section.to_owned(),
            header: true,
        });
        for number in 1..=7 {
            id += 1;
            entries.push(Entry {
                id,
                label: format!("{section} {number}"),
                header: false,
            });
        }
    }
    entries
}

/// The state behind those pages.
pub(crate) struct WidgetsDemo {
    taps: u32,
    holds: u32,
    languages: [bool; 6],
    tags: Vec<String>,
    wifi: bool,
    wifi_opened: u32,
    unread: usize,
    shares: u32,
    expanded: bool,
    step: usize,
    quantity: f32,
    query: String,
    searched: String,
    city: String,
    sort: &'static str,
    action: &'static str,
    country: Option<usize>,
    range: usize,
    price: (f32, f32),
    stars: u32,
    pages_loaded: u32,
    video: Option<&'static str>,
    entries: Arc<Vec<Entry>>,
    tiles: Arc<Vec<u32>>,
}

fn tags() -> Vec<String> {
    ["draft", "urgent", "work"].map(str::to_owned).to_vec()
}

impl WidgetsDemo {
    pub(crate) fn new() -> Self {
        Self {
            taps: 0,
            holds: 0,
            languages: [true, false, false, false, false, false],
            tags: tags(),
            wifi: true,
            wifi_opened: 0,
            unread: 3,
            shares: 0,
            expanded: false,
            step: 0,
            quantity: 2.0,
            query: String::new(),
            searched: String::new(),
            city: String::new(),
            sort: "Name",
            action: "none",
            country: None,
            range: 0,
            price: (20.0, 80.0),
            stars: 2,
            pages_loaded: 0,
            video: None,
            entries: Arc::new(entries()),
            tiles: Arc::new((1..=60).collect()),
        }
    }

    pub(crate) fn update(&mut self, message: WidgetsMsg) {
        match message {
            WidgetsMsg::Tapped => self.taps += 1,
            WidgetsMsg::Held => self.holds += 1,
            WidgetsMsg::Language(index) => self.languages[index] = !self.languages[index],
            WidgetsMsg::RemoveTag(tag) => self.tags.retain(|other| *other != tag),
            WidgetsMsg::ResetTags => self.tags = tags(),
            WidgetsMsg::Wifi(on) => self.wifi = on,
            WidgetsMsg::OpenWifi => self.wifi_opened += 1,
            WidgetsMsg::Unread => self.unread += 1,
            WidgetsMsg::ClearUnread => self.unread = 0,
            WidgetsMsg::Share => self.shares += 1,
            WidgetsMsg::Toggle => self.expanded = !self.expanded,
            WidgetsMsg::Step(index) => self.step = index.min(2),
            WidgetsMsg::Quantity(value) => self.quantity = value,
            WidgetsMsg::Query(text) => self.query = text,
            WidgetsMsg::Search => self.searched = self.query.clone(),
            WidgetsMsg::City(text) => self.city = text,
            WidgetsMsg::Sort(by) => self.sort = by,
            WidgetsMsg::Action(action) => self.action = action,
            WidgetsMsg::Country(index) => self.country = Some(index),
            WidgetsMsg::Range(index) => self.range = index,
            WidgetsMsg::Price(price) => self.price = price,
            WidgetsMsg::Stars(stars) => self.stars = stars.parse().unwrap_or(self.stars),
            WidgetsMsg::Loaded(_) => self.pages_loaded += 1,
            WidgetsMsg::Video(state) => {
                self.video = Some(if state == "playing" {
                    "playing"
                } else {
                    "failed"
                });
            }
            WidgetsMsg::ShowVideo => self.video = Some("loading"),
            // A header stays: the list shows it again.
            WidgetsMsg::DismissRow(id) => {
                let mut entries = (*self.entries).clone();
                entries.retain(|entry| entry.header || entry.id != id);
                self.entries = Arc::new(entries);
            }
            WidgetsMsg::MoveRow(from, to) => {
                let mut entries = (*self.entries).clone();
                if from < entries.len() && to < entries.len() {
                    let entry = entries.remove(from);
                    entries.insert(to, entry);
                    self.entries = Arc::new(entries);
                }
            }
            WidgetsMsg::ResetRows => self.entries = Arc::new(entries()),
        }
    }

    /// The content of a page, or `None` for a key this module does not own.
    /// The grid page fills the screen; the others go into the scrolling
    /// frame every page shares.
    pub(crate) fn body(&self, key: &str) -> Option<Vec<Element<WidgetsMsg>>> {
        Some(match key {
            "layout-extras" => self.layout_extras(),
            "pieces" => self.pieces(),
            "composed-inputs" => self.composed_inputs(),
            "menus" => self.menus(),
            "native-views" => self.native_views(),
            _ => return None,
        })
    }

    fn layout_extras(&self) -> Vec<Element<WidgetsMsg>> {
        let heading = |text: &str| Text::new(text).size(18).bold().color(COLOR_TEXT);
        let box_of = |color: u32| {
            Container::new()
                .width(48)
                .height(32)
                .background(color)
                .corner_radius(6)
        };
        let spread = |justify| {
            Row::new()
                .width_percent(1.0)
                .padding(8)
                .background(COLOR_SURFACE)
                .corner_radius(8)
                .justify_content(justify)
                .child(box_of(0xFF15_65C0))
                .child(box_of(0xFFF9_A825))
                .child(box_of(0xFF6A_1B9A))
        };
        let mut wrap = Wrap::new().spacing(8).run_spacing(8).width_percent(1.0);
        for (index, language) in LANGUAGES.into_iter().enumerate() {
            wrap = wrap.child(
                Chip::new(language)
                    .selected(self.languages[index])
                    .on_tap(WidgetsMsg::Language(index))
                    .accessibility_label(format!("Language {language}")),
            );
        }
        let picked = LANGUAGES
            .into_iter()
            .zip(self.languages)
            .filter_map(|(language, on)| on.then_some(language))
            .collect::<Vec<_>>()
            .join(", ");
        vec![
            heading("Wrap").into(),
            wrap.into(),
            caption(&format!("Picked: {picked}")).into(),
            heading("Positioned").into(),
            Stack::new()
                .width(220)
                .height(110)
                .background(COLOR_SURFACE)
                .corner_radius(12)
                .accessibility_label("Positioned stack")
                .child(
                    Text::new("NEW")
                        .size(12)
                        .bold()
                        .color(Color::fixed(0xFFFF_FFFF))
                        .padding(4)
                        .background(0xFFD3_2F2F)
                        .corner_radius(6)
                        .positioned(Inset::new().top(8).right(8)),
                )
                .child(
                    Text::new("Held at the bottom edge")
                        .size(13)
                        .text_align(TextAlign::Center)
                        .padding(6)
                        .background(0x3321_96F3)
                        .positioned(Inset::new().left(0).right(0).bottom(0)),
                )
                .into(),
            heading("Minimum and maximum").into(),
            Button::outlined("At least 220 wide")
                .min_width(220)
                .on_click(WidgetsMsg::Tapped)
                .into(),
            Text::new("This text may be at most 160 points wide, so it wraps early.")
                .max_width(160)
                .background(COLOR_SURFACE)
                .accessibility_label("Narrow text")
                .into(),
            heading("Space around, space evenly").into(),
            spread(Justify::SpaceAround)
                .accessibility_label("Space around")
                .into(),
            spread(Justify::SpaceEvenly)
                .accessibility_label("Space evenly")
                .into(),
            heading("Icons").into(),
            Row::new()
                .spacing(16)
                .align_items(Align::Center)
                .child(Image::icon(Icon::add()).accessibility_label("Add icon"))
                .child(Image::icon(Icon::search()).tint(COLOR_ACCENT))
                .child(Image::icon(Icon::check()).tint(0xFF2E_7D32).size(32))
                .child(Image::icon(Icon::close()).tint(0xFFD3_2F2F).size(16))
                .child(Image::icon(Icon::chevron_right()))
                .child(Image::icon(Icon::more()))
                .child(
                    Image::icon(Icon::new().shared("no-such-icon"))
                        .accessibility_label("Missing icon"),
                )
                .into(),
            caption("The last icon names nothing, so it shows nothing.").into(),
            heading("Long press").into(),
            Container::new()
                .padding(16)
                .background(COLOR_SURFACE)
                .corner_radius(12)
                .on_tap(WidgetsMsg::Tapped)
                .on_long_press(WidgetsMsg::Held)
                .accessibility_label("Press target")
                .child(Text::new("Tap me, or press and hold"))
                .into(),
            Text::new(format!("Taps: {} · Holds: {}", self.taps, self.holds))
                .color(COLOR_ACCENT)
                .into(),
        ]
    }

    fn pieces(&self) -> Vec<Element<WidgetsMsg>> {
        let mut tags = Wrap::new().spacing(8).run_spacing(8).width_percent(1.0);
        for tag in &self.tags {
            tags = tags.child(
                Chip::new(tag.clone())
                    .on_delete(WidgetsMsg::RemoveTag(tag.clone()))
                    .key(tag.clone()),
            );
        }
        vec![
            Card::new()
                .width_percent(1.0)
                .spacing(4)
                .child(Text::new("A card").bold())
                .child(caption("A raised surface around related content."))
                .into(),
            Column::new()
                .width_percent(1.0)
                .child(
                    ListTile::new("Wi-Fi")
                        .subtitle(if self.wifi { "Home network" } else { "Off" })
                        .leading(Avatar::initials("W").radius(18))
                        .trailing(
                            Switch::new(self.wifi)
                                .on_toggle(WidgetsMsg::Wifi)
                                .accessibility_label("Wi-Fi switch"),
                        )
                        .on_tap(WidgetsMsg::OpenWifi),
                )
                .child(Divider::new().indent(16))
                .child(
                    ListTile::new("About")
                        .trailing(Image::icon(Icon::chevron_right()).tint(Color::SECONDARY_TEXT))
                        .on_tap(WidgetsMsg::OpenWifi),
                )
                .into(),
            Text::new(format!("Tile taps: {}", self.wifi_opened))
                .color(COLOR_ACCENT)
                .into(),
            Row::new()
                .spacing(24)
                .align_items(Align::Center)
                .child(Avatar::initials("RW"))
                .child(
                    Avatar::initials("K")
                        .radius(28)
                        .background(COLOR_ACCENT)
                        .color(0xFFFF_FFFF),
                )
                .child(
                    Badge::new(
                        Image::icon(Icon::search())
                            .size(28)
                            .accessibility_label("Unread icon"),
                    )
                    .count(self.unread),
                )
                .child(Badge::new(Avatar::initials("D").radius(18)).dot(self.unread > 0))
                .into(),
            Row::new()
                .spacing(12)
                .child(Button::outlined("Add unread").on_click(WidgetsMsg::Unread))
                .child(Button::new("Clear").on_click(WidgetsMsg::ClearUnread))
                .into(),
            Text::new(format!("Unread: {}", self.unread))
                .color(COLOR_ACCENT)
                .into(),
            tags.into(),
            Button::new("Reset tags")
                .on_click(WidgetsMsg::ResetTags)
                .into(),
            Row::new()
                .spacing(12)
                .align_items(Align::Center)
                .child(IconButton::new(Icon::add(), "Share item").on_tap(WidgetsMsg::Share))
                .child(
                    IconButton::new(Icon::search(), "Find")
                        .text("Find")
                        .tint(COLOR_ACCENT)
                        .on_tap(WidgetsMsg::Share),
                )
                .child(Button::filled("Filled").on_click(WidgetsMsg::Share))
                .into(),
            Text::new(format!("Button taps: {}", self.shares))
                .color(COLOR_ACCENT)
                .into(),
        ]
    }

    fn composed_inputs(&self) -> Vec<Element<WidgetsMsg>> {
        let next =
            |label: &str, step: usize| Button::filled(label).on_click(WidgetsMsg::Step(step));
        vec![
            ExpansionTile::new("Advanced")
                .subtitle("Tap to show more")
                .expanded(self.expanded)
                .on_toggle(WidgetsMsg::Toggle)
                .width_percent(1.0)
                .background(COLOR_SURFACE)
                .corner_radius(12)
                .child(Text::new("Hidden until expanded"))
                .child(caption("The content fades in."))
                .into(),
            Row::new()
                .spacing(16)
                .align_items(Align::Center)
                .child(Text::new("Quantity"))
                .child(
                    NumberStepper::new(self.quantity)
                        .min(1)
                        .max(5)
                        .on_change(WidgetsMsg::Quantity),
                )
                .into(),
            Text::new(format!("Quantity: {:.0}", self.quantity))
                .color(COLOR_ACCENT)
                .into(),
            SearchBar::new(self.query.clone())
                .placeholder("Search notes")
                .on_change(WidgetsMsg::Query)
                .on_submit(WidgetsMsg::Search)
                .width_percent(1.0)
                .into(),
            Text::new(format!(
                "Query: \"{}\" · searched: \"{}\"",
                self.query, self.searched
            ))
            .color(COLOR_ACCENT)
            .into(),
            Autocomplete::new(self.city.clone(), CITIES)
                .placeholder("City")
                .on_change(WidgetsMsg::City)
                .width_percent(1.0)
                .into(),
            Text::new(format!("City: {}", self.city))
                .color(COLOR_ACCENT)
                .into(),
            Stepper::new()
                .current(self.step)
                .on_step(WidgetsMsg::Step)
                .width_percent(1.0)
                .step(
                    "Address",
                    Column::new()
                        .spacing(8)
                        .align_items(Align::Start)
                        .child(caption("Where should it go?"))
                        .child(next("To payment", 1)),
                )
                .step(
                    "Payment",
                    Column::new()
                        .spacing(8)
                        .align_items(Align::Start)
                        .child(caption("How do you pay?"))
                        .child(next("To confirmation", 2)),
                )
                .step("Confirm", caption("Everything is in place."))
                .into(),
        ]
    }

    fn menus(&self) -> Vec<Element<WidgetsMsg>> {
        let actions = || {
            Menu::new()
                .item("Rename", WidgetsMsg::Action("Rename"))
                .item_with_icon("Find", Icon::search(), WidgetsMsg::Action("Find"))
                .destructive("Delete", WidgetsMsg::Action("Delete"))
        };
        vec![
            Row::new()
                .spacing(12)
                .align_items(Align::Center)
                .child(
                    MenuButton::new(
                        format!("Sort: {}", self.sort),
                        Menu::new()
                            .item("Name", WidgetsMsg::Sort("Name"))
                            .item("Date", WidgetsMsg::Sort("Date"))
                            .item("Size", WidgetsMsg::Sort("Size")),
                    )
                    .background(COLOR_SURFACE)
                    .accessibility_label("Sort menu"),
                )
                .child(MenuButton::icon(Icon::more(), "More actions", actions()))
                .child(
                    IconButton::new(Icon::search(), "Find notes")
                        .on_tap(WidgetsMsg::Action("Find"))
                        .tooltip("Search every note"),
                )
                .into(),
            Text::new(format!("Sorted by {}", self.sort))
                .color(COLOR_ACCENT)
                .into(),
            Container::new()
                .padding(16)
                .background(COLOR_SURFACE)
                .corner_radius(12)
                .on_tap(WidgetsMsg::Action("Opened"))
                .context_menu(actions())
                .accessibility_label("Context target")
                .child(Text::new("Tap to open, hold for a menu"))
                .into(),
            Text::new(format!("Last action: {}", self.action))
                .color(COLOR_ACCENT)
                .into(),
            Dropdown::new(COUNTRIES)
                .selected(self.country)
                .placeholder("Country")
                .on_select(WidgetsMsg::Country)
                .width(220)
                .accessibility_label("Country dropdown")
                .into(),
            Text::new(format!(
                "Country: {}",
                self.country.map_or("none", |index| COUNTRIES[index])
            ))
            .color(COLOR_ACCENT)
            .into(),
            RangeSlider::new(self.price.0, self.price.1)
                .max(100)
                .step(10.0)
                .on_change(WidgetsMsg::Price)
                .accessibility_label("Price range")
                .width_percent(1.0)
                .into(),
            Text::new(format!("Price: {:.0} to {:.0}", self.price.0, self.price.1))
                .color(COLOR_ACCENT)
                .into(),
            Text::new("Hold this text to copy it")
                .selectable()
                .accessibility_label("Copyable text")
                .into(),
            SegmentedControl::new(RANGES)
                .selected(self.range)
                .on_select(WidgetsMsg::Range)
                .width_percent(1.0)
                .into(),
            Text::new(format!("Range: {}", RANGES[self.range]))
                .color(COLOR_ACCENT)
                .into(),
        ]
    }

    fn native_views(&self) -> Vec<Element<WidgetsMsg>> {
        let mut body: Vec<Element<WidgetsMsg>> = vec![
            caption("A star rating the example app makes in Kotlin and in Swift:").into(),
            NativeView::new("rating")
                .prop("stars", self.stars.to_string())
                .on_event(WidgetsMsg::Stars)
                .width(240)
                .height(56)
                .into(),
            Text::new(format!("Stars: {}", self.stars))
                .color(COLOR_ACCENT)
                .into(),
            Button::outlined("One star")
                .on_click(WidgetsMsg::Stars("1".into()))
                .into(),
            caption("The platform's web view, showing a page the app wrote:").into(),
            WebView::new(WEB_PAGE)
                .on_navigate(WidgetsMsg::Loaded)
                .width_percent(1.0)
                .height(160)
                .corner_radius(12)
                .into(),
            Text::new(format!("Pages loaded: {}", self.pages_loaded))
                .color(COLOR_ACCENT)
                .into(),
            caption("A name nothing is registered under shows an empty view:").into(),
            NativeView::new("no-such-view")
                .width_percent(1.0)
                .height(24)
                .background(COLOR_SURFACE)
                .accessibility_label("Missing view")
                .into(),
        ];
        match self.video {
            // The video comes from the network, so it is only asked for.
            None => body.push(
                Button::outlined("Show a video")
                    .on_click(WidgetsMsg::ShowVideo)
                    .into(),
            ),
            Some(state) => {
                body.push(
                    NativeView::new("video")
                        .prop("url", VIDEO_URL)
                        .on_event(WidgetsMsg::Video)
                        .width_percent(1.0)
                        .height(190)
                        .corner_radius(12)
                        .into(),
                );
                body.push(
                    Text::new(format!("Video: {state}"))
                        .color(COLOR_ACCENT)
                        .into(),
                );
            }
        }
        body
    }

    /// The list gestures page: swipe a row away, drag it elsewhere, and
    /// section headers that stay at the top.
    pub(crate) fn list_gestures(&self) -> Element<WidgetsMsg> {
        let first = self
            .entries
            .iter()
            .filter(|entry| !entry.header)
            .take(3)
            .map(|entry| entry.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(Text::new("List gestures").size(28).bold().color(COLOR_TEXT))
            .child(caption(
                "Swipe a row away, hold and drag it elsewhere. Headers stay at the top.",
            ))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(12)
                    .align_items(Align::Center)
                    .child(
                        Text::new(format!("{} rows · {first}", self.entries.len()))
                            .size(14)
                            .color(COLOR_ACCENT)
                            .grow(1),
                    )
                    .child(Button::new("Reset").on_click(WidgetsMsg::ResetRows)),
            )
            .child(
                List::new(self.entries.clone())
                    .item_height(52)
                    .recycled()
                    .key(|entry| entry.id.to_string())
                    .sticky_header(|entry| entry.header)
                    .on_dismiss(|entry| WidgetsMsg::DismissRow(entry.id))
                    .on_reorder(WidgetsMsg::MoveRow)
                    .row(|entry| {
                        let row = Row::new()
                            .width_percent(1.0)
                            .height(52)
                            .padding_each(krabik_ui_dsl::ui::EdgeInsets::symmetric(16, 0))
                            .align_items(Align::Center);
                        if entry.header {
                            row.background(COLOR_ACCENT)
                                .child(
                                    Text::new(entry.label.clone())
                                        .bold()
                                        .color(Color::fixed(0xFFFF_FFFF)),
                                )
                                .into()
                        } else {
                            row.background(COLOR_SURFACE)
                                .child(Text::new(entry.label.clone()))
                                .into()
                        }
                    }),
            )
            .into()
    }

    /// The grid page: three columns of numbered tiles in a recycled list.
    pub(crate) fn grid(&self) -> Element<WidgetsMsg> {
        Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(Text::new("Grid").size(28).bold().color(COLOR_TEXT))
            .child(caption("A list with three equally wide items on a line."))
            .child(
                Grid::new(self.tiles.clone())
                    .columns(3)
                    .spacing(8)
                    .cell_height(72)
                    .recycled()
                    .key(|tile| tile.to_string())
                    .cell(|tile| {
                        Container::new()
                            .height(72)
                            .background(COLOR_SURFACE)
                            .corner_radius(10)
                            .align_items(Align::Center)
                            .justify_content(Justify::Center)
                            .accessibility_label(format!("Tile {tile}"))
                            .child(Text::new(format!("{tile}")).size(18).bold())
                            .into()
                    }),
            )
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_builds_and_the_state_follows_the_messages() {
        let mut demo = WidgetsDemo::new();
        for key in [
            "layout-extras",
            "pieces",
            "composed-inputs",
            "menus",
            "native-views",
        ] {
            assert!(!demo.body(key).expect("a page").is_empty());
        }
        assert!(demo.body("slider").is_none());
        let _ = demo.grid();

        demo.update(WidgetsMsg::Language(2));
        assert!(demo.languages[2]);
        demo.update(WidgetsMsg::RemoveTag("urgent".into()));
        assert_eq!(demo.tags, ["draft", "work"]);
        demo.update(WidgetsMsg::Query("tea".into()));
        demo.update(WidgetsMsg::Search);
        assert_eq!(demo.searched, "tea");
        demo.update(WidgetsMsg::Step(7));
        assert_eq!(demo.step, 2);
        // The pages still build in every state.
        demo.update(WidgetsMsg::Toggle);
        demo.update(WidgetsMsg::City("to".into()));
        assert!(demo.body("composed-inputs").is_some());
    }
}
