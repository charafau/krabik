//! A widget showcase built with the UI DSL.
//!
//! The home screen lists every widget; each entry opens a native navigation
//! screen that demonstrates that widget in isolation. The application publishes
//! its route stack through [`App::navigation`] and builds each screen through
//! [`App::screen`]; the platform supplies the native navigation controller.
//!
//! The app is a normal Rust crate that depends on [`krabik_ui_dsl`], implements
//! [`App`], and registers itself through [`set_app_factory`]. The iOS shell in
//! `../ios` links it and the Android shell in `../android` loads its cdylib.

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::navigation::{Navigation, Screen, ScreenTransition, Toolbar};
use krabik_ui_dsl::presentation::{BottomSheet, Dialog};
use krabik_ui_dsl::ui::{DatePickerValue, DateValue, Element, Icon, TimeValue};

mod checklist;
mod pages;

use checklist::Checklist;

/// Everything that can happen in the showcase.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// Opens the screen with this key; `home` returns to the catalog.
    Open(&'static str),
    /// The system or toolbar back action.
    Back,
    /// Something happened on the checklist screen, a component of its own.
    Checklist(checklist::Msg),

    Name(String),
    SubmitName,
    Tap,
    ResetTaps,
    Progress(f32),
    RecycledTap,
    RecycledShuffle,
    ToggleAnimations,
    ToggleDecoration,
    AddCard,
    RemoveCard,
    RunCurves,
    /// A button that exists only to show a style.
    Noop,
    /// A native toolbar item was tapped.
    Toolbar(&'static str),
    OpenDialog,
    CloseDialog,
    OpenSheet,
    CloseSheet,

    Notifications(bool),
    Wifi(bool),
    Bluetooth(bool),
    Radio(usize),
    Size(usize),
    Date(DatePickerValue),
    InlineDate(DatePickerValue),
    RangeDate(DatePickerValue),
    Time(DatePickerValue),
    DateTime(DatePickerValue),
    Tab(usize),
    Page(usize),
    TabsView(usize),
    BottomDest(usize),
    Volume(f32),
    Stepped(f32),
}

/// The state of the showcase application.
pub struct ShowcaseApp {
    /// The native navigation stack, starting at `home`.
    stack: Vec<&'static str>,
    /// The checklist demo, which has its own state.
    checklist: Checklist,
    name: String,
    submitted: String,
    taps: i64,
    notifications: bool,
    wifi: bool,
    bluetooth: bool,
    radio_choice: usize,
    size_choice: usize,
    animations: bool,
    decoration_alt: bool,
    cards: usize,
    curves_alt: bool,
    pager_page: usize,
    tabs_view: usize,
    bottom_dest: usize,
    toolbar_taps: usize,
    toolbar_last: String,
    date: DateValue,
    inline_date: DateValue,
    time: TimeValue,
    date_time_date: DateValue,
    date_time: TimeValue,
    range_date: DateValue,
    tab: usize,
    volume: f32,
    stepped: f32,
    progress: f32,
    list_items: Arc<Vec<String>>,
    recycled_items: Arc<Vec<String>>,
    recycled_taps: usize,
    recycled_alt: bool,
    dialog_open: bool,
    sheet_open: bool,
}

impl ShowcaseApp {
    /// Creates the showcase on its home page.
    pub fn new() -> Self {
        Self {
            stack: vec!["home"],
            checklist: Checklist::new(),
            name: String::new(),
            submitted: String::new(),
            taps: 0,
            notifications: false,
            wifi: true,
            bluetooth: false,
            radio_choice: 1,
            size_choice: 1,
            animations: true,
            decoration_alt: true,
            cards: 2,
            curves_alt: false,
            pager_page: 0,
            tabs_view: 0,
            bottom_dest: 0,
            toolbar_taps: 0,
            toolbar_last: String::new(),
            date: DateValue::new(2026, 11, 12),
            inline_date: DateValue::new(2026, 11, 12),
            time: TimeValue::new(10, 30),
            date_time_date: DateValue::new(2026, 11, 12),
            date_time: TimeValue::new(10, 30),
            range_date: DateValue::new(2026, 6, 15),
            tab: 0,
            volume: 40.0,
            stepped: 30.0,
            progress: 0.4,
            list_items: Arc::new(
                (1..=200)
                    .map(|index| format!("Windowed row {index}"))
                    .collect(),
            ),
            recycled_items: Arc::new(
                (1..=1000)
                    .map(|index| format!("Recycled row {index}"))
                    .collect(),
            ),
            recycled_taps: 0,
            recycled_alt: false,
            dialog_open: false,
            sheet_open: false,
        }
    }

    /// Registers the showcase as the global application factory.
    ///
    /// The host shell calls this once at startup, before creating the root
    /// view. Without it the platform views fall back to the built-in gallery.
    pub fn install() {
        set_app_factory(Self::new);
    }
}

impl Default for ShowcaseApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for ShowcaseApp {
    type Msg = Msg;

    /// Not shown: with navigation, the first screen is the root.
    fn view(&self) -> Element<Msg> {
        pages::home()
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Open("home") => self.stack.truncate(1),
            Msg::Open(key) => {
                // Only real screens are pushed, and never twice in a row.
                if self.stack.last() != Some(&key) && pages::screen(self, key).is_some() {
                    self.stack.push(key);
                }
            }
            // Back closes whatever is on top: the dialog, the sheet, a screen.
            Msg::Back => {
                if self.dialog_open {
                    self.dialog_open = false;
                } else if self.sheet_open {
                    self.sheet_open = false;
                } else if self.stack.len() > 1 {
                    self.stack.pop();
                }
            }
            Msg::Checklist(msg) => self.checklist.update(msg),

            Msg::Name(name) => {
                self.name = name;
                self.submitted.clear();
            }
            Msg::SubmitName => self.submitted = self.name.clone(),
            Msg::Tap => self.taps += 1,
            Msg::ResetTaps => self.taps = 0,
            Msg::Progress(value) => self.progress = value,
            Msg::RecycledTap => self.recycled_taps += 1,
            Msg::RecycledShuffle => self.recycled_alt = !self.recycled_alt,
            Msg::ToggleAnimations => self.animations = !self.animations,
            Msg::ToggleDecoration => self.decoration_alt = !self.decoration_alt,
            Msg::AddCard => self.cards = (self.cards + 1).min(4),
            Msg::RemoveCard => self.cards = self.cards.saturating_sub(1),
            Msg::RunCurves => self.curves_alt = !self.curves_alt,
            Msg::Noop => {}
            Msg::Toolbar(item) => {
                self.toolbar_taps += 1;
                self.toolbar_last = item.to_string();
            }
            Msg::OpenDialog => self.dialog_open = true,
            Msg::CloseDialog => self.dialog_open = false,
            Msg::OpenSheet => self.sheet_open = true,
            Msg::CloseSheet => self.sheet_open = false,

            Msg::Notifications(on) => self.notifications = on,
            Msg::Wifi(on) => self.wifi = on,
            Msg::Bluetooth(on) => self.bluetooth = on,
            Msg::Radio(index) => self.radio_choice = index.min(2),
            Msg::Size(index) => self.size_choice = index.min(2),
            Msg::Date(value) => self.date = value.date().unwrap_or(self.date),
            Msg::InlineDate(value) => self.inline_date = value.date().unwrap_or(self.inline_date),
            Msg::RangeDate(value) => self.range_date = value.date().unwrap_or(self.range_date),
            Msg::Time(value) => self.time = value.time().unwrap_or(self.time),
            Msg::DateTime(value) => {
                self.date_time_date = value.date().unwrap_or(self.date_time_date);
                self.date_time = value.time().unwrap_or(self.date_time);
            }
            Msg::Tab(index) => self.tab = index.min(2),
            Msg::Page(index) => self.pager_page = index.min(2),
            Msg::TabsView(index) => self.tabs_view = index.min(2),
            Msg::BottomDest(index) => self.bottom_dest = index.min(2),
            Msg::Volume(value) => self.volume = value,
            Msg::Stepped(value) => self.stepped = value,
        }
    }
}

impl App for ShowcaseApp {
    fn navigation(&self) -> Option<Navigation<Msg>> {
        Some(Navigation::new(
            self.stack
                .iter()
                .map(|key| {
                    let transition = match *key {
                        "transition-fade" => ScreenTransition::Fade,
                        "transition-none" => ScreenTransition::None,
                        _ => ScreenTransition::Slide,
                    };
                    let mut screen = Screen::new(*key, pages::title(key)).transition(transition);
                    if *key == "bottom-navigation" {
                        screen = screen.bottom_navigation(pages::bottom_navigation(self));
                    }
                    if *key == "toolbar" {
                        screen = screen.toolbar(
                            Toolbar::new()
                                .subtitle("Host-level native chrome")
                                .back_title("Back")
                                .action("Refresh", Msg::Toolbar("Refresh"))
                                .action_with_icon(
                                    "Save",
                                    Icon::new().drawable("ic_save").sf("square.and.arrow.down"),
                                    Msg::Toolbar("Save"),
                                ),
                        );
                    }
                    screen
                })
                .collect(),
        ))
    }

    fn screen(&self, key: &str) -> Option<Element<Msg>> {
        pages::screen(self, key)
    }

    fn dialog(&self) -> Option<Dialog<Msg>> {
        self.dialog_open.then(pages::dialog)
    }

    fn sheet(&self) -> Option<BottomSheet<Msg>> {
        self.sheet_open.then(pages::sheet)
    }

    /// Back is ours while something can be closed; on the catalog with
    /// nothing open, the platform leaves the app.
    fn back(&self) -> Option<Msg> {
        (self.dialog_open || self.sheet_open || self.stack.len() > 1).then_some(Msg::Back)
    }
}

/// C entry point called by the iOS host before creating the root view.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "C" fn krabik_ui_dsl_example_install() {
    ShowcaseApp::install();
}

/// JNI entry point called by the Android host before creating the root view.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_ExampleBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    ShowcaseApp::install();
}

#[cfg(test)]
mod tests {
    use super::*;
    use krabik_ui_dsl::widgets::Widget;

    fn app() -> ShowcaseApp {
        ShowcaseApp::new()
    }

    #[test]
    fn starts_on_the_widget_catalog() {
        let app = app();
        assert_eq!(app.stack, vec!["home"]);
        assert_eq!(app.navigation().unwrap().screens.len(), 1);
        assert!(app.screen("home").is_some());
        assert_eq!(app.size_choice, 1);
        assert_eq!(app.volume, 40.0);
    }

    #[test]
    fn page_actions_push_and_back_pops() {
        let mut app = app();
        app.update(Msg::Open("radio-group"));
        assert_eq!(app.stack, vec!["home", "radio-group"]);
        assert!(app.screen("radio-group").is_some());
        app.update(Msg::Open("radio-group"));
        app.update(app.back().expect("back is handled"));
        assert_eq!(app.stack, vec!["home"]);
        app.update(Msg::Open("checklist"));
        assert!(app.screen("checklist").is_some());
    }

    #[test]
    fn dialog_and_sheet_open_and_close() {
        let mut app = app();
        assert!(app.dialog().is_none());
        assert!(app.sheet().is_none());
        app.update(Msg::OpenDialog);
        let dialog = app.dialog().expect("dialog");
        assert_eq!(dialog.title, "Delete task?");
        assert!(dialog.is_cancelable());
        app.update(Msg::CloseDialog);
        assert!(app.dialog().is_none());
        app.update(Msg::OpenSheet);
        assert!(app.sheet().expect("sheet").on_dismiss.is_set());
        app.update(app.back().expect("back is handled"));
        assert!(app.sheet().is_none());
    }

    #[test]
    fn the_bottom_navigation_screen_declares_its_bar() {
        let mut app = app();
        app.update(Msg::Open("bottom-navigation"));
        let screens = app.navigation().expect("navigation").screens;
        assert!(screens[0].bottom_navigation.is_none());
        let bar = screens[1].bottom_navigation.as_ref().expect("bar");
        assert_eq!(bar.items.len(), 3);
        assert!(bar.on_select.is_set());
    }

    #[test]
    fn unknown_pages_are_ignored_and_back_at_home_is_unhandled() {
        let mut app = app();
        app.update(Msg::Open("unknown"));
        assert_eq!(app.back(), None);
    }

    #[test]
    fn radio_group_updates_the_selection() {
        let mut app = app();
        app.update(Msg::Size(2));
        assert_eq!(app.size_choice, 2);
        app.update(Msg::Size(2));
        app.update(Msg::Size(9));
    }

    #[test]
    fn stepped_slider_keeps_the_snapped_value_from_the_dsl() {
        let mut app = app();
        app.update(Msg::Stepped(70.0));
        assert_eq!(app.stepped, 70.0);
    }

    #[test]
    fn date_pickers_update_their_dates() {
        let mut app = app();
        app.update(Msg::Date(DatePickerValue::Date(DateValue::new(2027, 1, 2))));
        assert_eq!(app.date, DateValue::new(2027, 1, 2));
        app.update(Msg::InlineDate(DatePickerValue::Date(DateValue::new(
            2027, 3, 4,
        ))));
        assert_eq!(app.inline_date, DateValue::new(2027, 3, 4));
    }

    #[test]
    fn time_and_range_pickers_update_their_values() {
        let mut app = app();
        app.update(Msg::Time(DatePickerValue::Time(TimeValue::new(9, 45))));
        assert_eq!(app.time, TimeValue::new(9, 45));
        app.update(Msg::DateTime(DatePickerValue::DateTime {
            date: DateValue::new(2027, 2, 3),
            time: TimeValue::new(18, 15),
        }));
        assert_eq!(app.date_time_date, DateValue::new(2027, 2, 3));
        assert_eq!(app.date_time, TimeValue::new(18, 15));
        app.update(Msg::RangeDate(DatePickerValue::Date(DateValue::new(
            2026, 8, 1,
        ))));
        assert_eq!(app.range_date, DateValue::new(2026, 8, 1));
    }

    #[test]
    fn tabs_page_contains_both_tab_bars() {
        let element = pages::screen(&app(), "tabs").expect("tabs page");
        let tree = krabik_ui_dsl::tree::build(&element);
        let bars: Vec<_> = krabik_ui_dsl::tree::preorder(&tree)
            .into_iter()
            .filter(|node| {
                matches!(
                    node.kind,
                    krabik_ui_dsl::tree::NodeKind::Widget(Widget::TabBar(_))
                )
            })
            .collect();
        assert_eq!(bars.len(), 2);
        for bar in bars {
            let krabik_ui_dsl::tree::NodeKind::Widget(Widget::TabBar(props)) = &bar.kind else {
                continue;
            };
            assert!(!props.tabs.is_empty());
        }
    }

    #[test]
    fn tab_bar_updates_the_selected_tab() {
        let mut app = app();
        app.update(Msg::Tab(2));
        assert_eq!(app.tab, 2);
        app.update(Msg::Tab(2));
        app.update(Msg::Tab(9));
    }

    #[test]
    fn animations_toggle() {
        let mut app = app();
        assert!(app.animations);
        app.update(Msg::ToggleAnimations);
        assert!(!app.animations);
        app.update(Msg::ToggleAnimations);
        assert!(app.animations);

        assert!(app.decoration_alt);
        app.update(Msg::ToggleDecoration);
        assert!(!app.decoration_alt);

        assert!(!app.curves_alt);
        app.update(Msg::RunCurves);
        assert!(app.curves_alt);

        assert_eq!(app.cards, 2);
        app.update(Msg::AddCard);
        app.update(Msg::AddCard);
        assert_eq!(app.cards, 4);
        app.update(Msg::AddCard);
        app.update(Msg::RemoveCard);
        assert_eq!(app.cards, 3);
    }

    #[test]
    fn animation_pages_are_registered() {
        let app = app();
        for key in [
            "animations",
            "animation-decoration",
            "animation-enter-exit",
            "animation-curves",
            "transitions",
            "transition-slide",
            "transition-fade",
            "transition-none",
            "bottom-sheet",
            "pager",
            "tabs-view",
            "bottom-navigation",
            "toolbar",
        ] {
            assert!(pages::screen(&app, key).is_some(), "missing page {key}");
            assert!(pages::screen_key(&format!("page:{key}")).is_some());
        }
    }

    #[test]
    fn container_widgets_update_their_state() {
        let mut app = app();
        app.update(Msg::Page(2));
        assert_eq!(app.pager_page, 2);
        app.update(Msg::TabsView(1));
        assert_eq!(app.tabs_view, 1);
        app.update(Msg::BottomDest(2));
        assert_eq!(app.bottom_dest, 2);
        app.update(Msg::Toolbar("Refresh"));
        assert_eq!(app.toolbar_taps, 1);
        assert_eq!(app.toolbar_last, "Refresh");
        app.update(Msg::Toolbar("Save"));
        assert_eq!(app.toolbar_taps, 2);
        assert_eq!(app.toolbar_last, "Save");
    }

    #[test]
    fn tabs_view_selection_updates_the_existing_tree() {
        use krabik_ui_dsl::measure::ApproximateMeasure;
        use krabik_ui_dsl::render::Op;
        use krabik_ui_dsl::runtime::Runtime;
        use krabik_ui_dsl::tree::NodeKind;

        let mut app = app();
        app.stack = vec!["home", "tabs-view"];
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        let viewport = krabik_ui_dsl::geometry::Size::new(390.0, 800.0);
        runtime.update(
            &pages::screen(&app, "tabs-view").expect("tabs page"),
            viewport,
        );
        app.update(Msg::TabsView(1));
        let update = runtime.update(
            &pages::screen(&app, "tabs-view").expect("tabs page"),
            viewport,
        );
        let pager = krabik_ui_dsl::tree::preorder(runtime.tree())
            .into_iter()
            .find(|node| matches!(node.kind, NodeKind::Pager(_)))
            .expect("pager");
        let NodeKind::Pager(props) = &pager.kind else {
            unreachable!()
        };
        assert_eq!(props.selected, 1);
        // The pager is updated in place: nothing is torn down and rebuilt.
        assert!(update.ops.iter().any(|op| matches!(
            op,
            Op::Update(node) if matches!(&node.kind, NodeKind::Pager(p) if p.selected == 1)
        )));
        assert!(
            !update
                .ops
                .iter()
                .any(|op| matches!(op, Op::Create(_) | Op::Remove { .. }))
        );
        assert!(update.ops.iter().any(|op| matches!(
            op,
            Op::Update(node) if matches!(&node.kind, NodeKind::Widget(Widget::TabBar(p)) if p.selected == 1)
        )));
    }

    #[test]
    fn tabs_view_selection_reaches_the_pager() {
        let mut app = app();
        app.stack = vec!["home", "tabs-view"];
        app.update(Msg::TabsView(1));
        let element = pages::screen(&app, "tabs-view").expect("tabs page");
        let tree = krabik_ui_dsl::tree::build(&element);
        let pager = krabik_ui_dsl::tree::preorder(&tree)
            .into_iter()
            .find(|node| matches!(node.kind, krabik_ui_dsl::tree::NodeKind::Pager(_)))
            .expect("pager");
        let krabik_ui_dsl::tree::NodeKind::Pager(props) = &pager.kind else {
            unreachable!()
        };
        assert_eq!(props.pages, 3);
        assert_eq!(props.selected, 1);
    }

    #[test]
    fn toolbar_screen_exposes_native_actions() {
        let mut app = app();
        app.stack = vec!["home", "toolbar"];
        let nav = app.navigation().expect("navigation");
        let toolbar = nav
            .screens
            .last()
            .and_then(|screen| screen.toolbar.as_ref())
            .expect("toolbar");
        assert_eq!(
            toolbar.subtitle.as_deref(),
            Some("Host-level native chrome")
        );
        assert_eq!(toolbar.actions.len(), 2);
    }

    #[test]
    fn styling_page_contains_a_disabled_control() {
        let element = pages::screen(&app(), "styling").expect("styling page");
        let tree = krabik_ui_dsl::tree::build(&element);
        assert!(
            krabik_ui_dsl::tree::preorder(&tree)
                .into_iter()
                .any(|node| node.style.enabled == Some(false))
        );
    }

    #[test]
    fn demo_screens_choose_their_native_transition() {
        let mut app = app();
        app.stack = vec!["home", "transition-fade"];
        let nav = app.navigation().expect("navigation");
        assert_eq!(
            nav.screens.last().unwrap().transition,
            ScreenTransition::Fade
        );

        app.stack = vec!["home", "transition-none"];
        let nav = app.navigation().expect("navigation");
        assert_eq!(
            nav.screens.last().unwrap().transition,
            ScreenTransition::None
        );

        app.stack = vec!["home", "text"];
        let nav = app.navigation().expect("navigation");
        assert_eq!(
            nav.screens.last().unwrap().transition,
            ScreenTransition::Slide
        );
    }

    #[test]
    fn controls_update_their_state() {
        let mut app = app();
        app.update(Msg::Tap);
        assert_eq!(app.taps, 1);
        app.update(Msg::ResetTaps);
        assert_eq!(app.taps, 0);
        app.update(Msg::Wifi(false));
        assert!(!app.wifi);
        app.update(Msg::Radio(2));
        assert_eq!(app.radio_choice, 2);
        app.update(Msg::Progress(1.0));
        assert_eq!(app.progress, 1.0);
    }

    #[test]
    fn text_field_tracks_typing_and_submission() {
        let mut app = app();
        app.update(Msg::Name("Ada".to_string()));
        assert_eq!(app.name, "Ada");
        app.update(Msg::SubmitName);
        assert_eq!(app.submitted, "Ada");
    }

    #[test]
    fn checklist_screen_delegates_its_events() {
        let mut app = app();
        app.update(Msg::Open("checklist"));
        assert_eq!(app.stack.last(), Some(&"checklist"));
        app.update(Msg::Checklist(checklist::Msg::SetTask(1, true)));
        assert!(app.checklist.summary().contains("2 of 3 tasks done"));
    }
}
