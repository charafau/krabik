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
use krabik_ui_dsl::app::{App, Event, set_app_factory};
use krabik_ui_dsl::navigation::{Navigation, Screen};
use krabik_ui_dsl::presentation::{Presentation, PresentationKind};
use krabik_ui_dsl::ui::{DateValue, Element, TimeValue};

mod checklist;
mod pages;

use checklist::Checklist;

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
            dialog_open: false,
            sheet_open: false,
        }
    }

    /// Registers the showcase as the global application factory.
    ///
    /// The host shell calls this once at startup, before creating the root
    /// view. Without it the platform views fall back to the built-in gallery.
    pub fn install() {
        set_app_factory(|| Box::new(Self::new()));
    }
}

impl Default for ShowcaseApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for ShowcaseApp {
    fn view(&self) -> Element {
        pages::home()
    }

    fn navigation(&self) -> Option<Navigation> {
        Some(Navigation::new(
            self.stack
                .iter()
                .map(|key| Screen::new(*key, pages::title(key)))
                .collect(),
        ))
    }

    fn screen(&self, key: &str) -> Option<Element> {
        pages::screen(self, key)
    }

    fn presentation(&self) -> Option<Presentation> {
        if self.dialog_open {
            Some(Presentation::new(
                "dialog",
                PresentationKind::Dialog,
                "dialog:close",
            ))
        } else if self.sheet_open {
            Some(Presentation::new(
                "sheet",
                PresentationKind::Sheet,
                "sheet:close",
            ))
        } else {
            None
        }
    }

    fn presentation_view(&self) -> Option<Element> {
        if self.dialog_open {
            Some(pages::dialog_view(self))
        } else if self.sheet_open {
            Some(pages::sheet_view(self))
        } else {
            None
        }
    }

    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::Click { action } if action.starts_with("page:") => {
                let Some(key) = pages::screen_key(action) else {
                    return false;
                };
                if key == "home" {
                    if self.stack.len() > 1 {
                        self.stack.truncate(1);
                        true
                    } else {
                        false
                    }
                } else if self.stack.last() == Some(&key) {
                    false
                } else {
                    self.stack.push(key);
                    true
                }
            }
            Event::Back => {
                if self.dialog_open {
                    self.dialog_open = false;
                    true
                } else if self.sheet_open {
                    self.sheet_open = false;
                    true
                } else if self.stack.len() > 1 {
                    self.stack.pop();
                    true
                } else {
                    false
                }
            }
            // The checklist screen owns every remaining event on its page.
            _ if self.stack.last() == Some(&"checklist") => self.checklist.handle(event),
            Event::TextChanged {
                action: "demo:name",
                value,
            } => {
                self.name = value.to_string();
                self.submitted.clear();
                true
            }
            Event::Submitted {
                action: "demo:name:submit",
            } => {
                self.submitted = self.name.clone();
                true
            }
            Event::Click { action } => match action {
                "demo:tap" => {
                    self.taps += 1;
                    true
                }
                "demo:reset" => {
                    self.taps = 0;
                    true
                }
                "demo:progress:25" => {
                    self.progress = 0.25;
                    true
                }
                "demo:progress:50" => {
                    self.progress = 0.5;
                    true
                }
                "demo:progress:100" => {
                    self.progress = 1.0;
                    true
                }
                "demo:recycled-tap" => {
                    self.recycled_taps += 1;
                    true
                }
                "demo:animate" => {
                    self.animations = !self.animations;
                    true
                }
                "demo:decoration" => {
                    self.decoration_alt = !self.decoration_alt;
                    true
                }
                "demo:add-card" => {
                    if self.cards >= 4 {
                        return false;
                    }
                    self.cards += 1;
                    true
                }
                "demo:remove-card" => {
                    if self.cards == 0 {
                        return false;
                    }
                    self.cards -= 1;
                    true
                }
                "demo:curves" => {
                    self.curves_alt = !self.curves_alt;
                    true
                }
                "dialog:open" => {
                    self.dialog_open = true;
                    true
                }
                "dialog:close" => {
                    let changed = self.dialog_open;
                    self.dialog_open = false;
                    changed
                }
                "sheet:open" => {
                    self.sheet_open = true;
                    true
                }
                "sheet:close" => {
                    let changed = self.sheet_open;
                    self.sheet_open = false;
                    changed
                }
                _ => false,
            },
            Event::Toggled { action, checked } => match action {
                "demo:notifications" => {
                    self.notifications = checked;
                    true
                }
                "demo:wifi" => {
                    self.wifi = checked;
                    true
                }
                "demo:bluetooth" => {
                    self.bluetooth = checked;
                    true
                }
                "demo:radio:0" if checked => {
                    self.radio_choice = 0;
                    true
                }
                "demo:radio:1" if checked => {
                    self.radio_choice = 1;
                    true
                }
                "demo:radio:2" if checked => {
                    self.radio_choice = 2;
                    true
                }
                _ => false,
            },
            Event::Selected {
                action: "demo:size",
                index,
            } => {
                if index > 2 || self.size_choice == index {
                    return false;
                }
                self.size_choice = index;
                true
            }
            Event::DateChanged {
                action: "demo:date",
                date,
            } => {
                self.date = date;
                true
            }
            Event::DateChanged {
                action: "demo:date-inline",
                date,
            } => {
                self.inline_date = date;
                true
            }
            Event::DateChanged {
                action: "demo:date-range",
                date,
            } => {
                self.range_date = date;
                true
            }
            Event::TimeChanged {
                action: "demo:time",
                time,
            } => {
                self.time = time;
                true
            }
            Event::DateTimeChanged {
                action: "demo:date-time",
                date,
                time,
            } => {
                self.date_time_date = date;
                self.date_time = time;
                true
            }
            Event::Selected {
                action: "demo:tab",
                index,
            } => {
                if index > 2 || self.tab == index {
                    return false;
                }
                self.tab = index;
                true
            }
            Event::ValueChanged {
                action: "demo:volume",
                value,
            } => {
                self.volume = value;
                true
            }
            Event::ValueChanged {
                action: "demo:stepped",
                value,
            } => {
                self.stepped = value;
                true
            }
            _ => false,
        }
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

    fn app() -> ShowcaseApp {
        ShowcaseApp::new()
    }

    fn click(action: &'static str) -> Event<'static> {
        Event::Click { action }
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
        assert!(app.handle(click("page:radio-group")));
        assert_eq!(app.stack, vec!["home", "radio-group"]);
        assert!(app.screen("radio-group").is_some());
        assert!(!app.handle(click("page:radio-group")));
        assert!(app.handle(Event::Back));
        assert_eq!(app.stack, vec!["home"]);
        assert!(app.handle(click("page:checklist")));
        assert!(app.screen("checklist").is_some());
    }

    #[test]
    fn presentations_are_exclusive_and_dismissible() {
        let mut app = app();
        assert!(app.presentation().is_none());
        assert!(app.handle(click("dialog:open")));
        assert_eq!(app.presentation().unwrap().kind, PresentationKind::Dialog);
        assert!(app.presentation_view().is_some());
        assert!(app.handle(click("dialog:close")));
        assert!(app.presentation().is_none());
        assert!(app.handle(click("sheet:open")));
        assert_eq!(app.presentation().unwrap().kind, PresentationKind::Sheet);
        assert!(app.handle(Event::Back));
        assert!(app.presentation().is_none());
    }

    #[test]
    fn unknown_pages_are_ignored_and_back_at_home_is_unhandled() {
        let mut app = app();
        assert!(!app.handle(click("page:unknown")));
        assert!(!app.handle(Event::Back));
    }

    #[test]
    fn radio_group_updates_the_selection() {
        let mut app = app();
        assert!(app.handle(Event::Selected {
            action: "demo:size",
            index: 2,
        }));
        assert_eq!(app.size_choice, 2);
        assert!(!app.handle(Event::Selected {
            action: "demo:size",
            index: 2,
        }));
        assert!(!app.handle(Event::Selected {
            action: "demo:size",
            index: 9,
        }));
    }

    #[test]
    fn stepped_slider_keeps_the_snapped_value_from_the_dsl() {
        let mut app = app();
        assert!(app.handle(Event::ValueChanged {
            action: "demo:stepped",
            value: 70.0,
        }));
        assert_eq!(app.stepped, 70.0);
    }

    #[test]
    fn date_pickers_update_their_dates() {
        let mut app = app();
        assert!(app.handle(Event::DateChanged {
            action: "demo:date",
            date: DateValue::new(2027, 1, 2),
        }));
        assert_eq!(app.date, DateValue::new(2027, 1, 2));
        assert!(app.handle(Event::DateChanged {
            action: "demo:date-inline",
            date: DateValue::new(2027, 3, 4),
        }));
        assert_eq!(app.inline_date, DateValue::new(2027, 3, 4));
    }

    #[test]
    fn time_and_range_pickers_update_their_values() {
        let mut app = app();
        assert!(app.handle(Event::TimeChanged {
            action: "demo:time",
            time: TimeValue::new(9, 45),
        }));
        assert_eq!(app.time, TimeValue::new(9, 45));
        assert!(app.handle(Event::DateTimeChanged {
            action: "demo:date-time",
            date: DateValue::new(2027, 2, 3),
            time: TimeValue::new(18, 15),
        }));
        assert_eq!(app.date_time_date, DateValue::new(2027, 2, 3));
        assert_eq!(app.date_time, TimeValue::new(18, 15));
        assert!(app.handle(Event::DateChanged {
            action: "demo:date-range",
            date: DateValue::new(2026, 8, 1),
        }));
        assert_eq!(app.range_date, DateValue::new(2026, 8, 1));
    }

    #[test]
    fn tabs_page_contains_both_tab_bars() {
        let element = pages::screen(&app(), "tabs").expect("tabs page");
        let tree = krabik_ui_dsl::tree::build(&element);
        let bars: Vec<_> = krabik_ui_dsl::tree::preorder(&tree)
            .into_iter()
            .filter(|node| matches!(node.kind, krabik_ui_dsl::tree::NodeKind::TabBar(_)))
            .collect();
        assert_eq!(bars.len(), 2);
        for bar in bars {
            let krabik_ui_dsl::tree::NodeKind::TabBar(props) = &bar.kind else {
                continue;
            };
            assert!(!props.tabs.is_empty());
        }
    }

    #[test]
    fn tab_bar_updates_the_selected_tab() {
        let mut app = app();
        assert!(app.handle(Event::Selected {
            action: "demo:tab",
            index: 2,
        }));
        assert_eq!(app.tab, 2);
        assert!(!app.handle(Event::Selected {
            action: "demo:tab",
            index: 2,
        }));
        assert!(!app.handle(Event::Selected {
            action: "demo:tab",
            index: 9,
        }));
    }

    #[test]
    fn animations_toggle() {
        let mut app = app();
        assert!(app.animations);
        assert!(app.handle(click("demo:animate")));
        assert!(!app.animations);
        assert!(app.handle(click("demo:animate")));
        assert!(app.animations);

        assert!(app.decoration_alt);
        assert!(app.handle(click("demo:decoration")));
        assert!(!app.decoration_alt);

        assert!(!app.curves_alt);
        assert!(app.handle(click("demo:curves")));
        assert!(app.curves_alt);

        assert_eq!(app.cards, 2);
        assert!(app.handle(click("demo:add-card")));
        assert!(app.handle(click("demo:add-card")));
        assert_eq!(app.cards, 4);
        assert!(!app.handle(click("demo:add-card")));
        assert!(app.handle(click("demo:remove-card")));
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
        ] {
            assert!(pages::screen(&app, key).is_some(), "missing page {key}");
            assert!(pages::screen_key(&format!("page:{key}")).is_some());
        }
    }

    #[test]
    fn controls_update_their_state() {
        let mut app = app();
        assert!(app.handle(click("demo:tap")));
        assert_eq!(app.taps, 1);
        assert!(app.handle(click("demo:reset")));
        assert_eq!(app.taps, 0);
        assert!(app.handle(Event::Toggled {
            action: "demo:wifi",
            checked: false,
        }));
        assert!(!app.wifi);
        assert!(app.handle(Event::Toggled {
            action: "demo:radio:2",
            checked: true,
        }));
        assert_eq!(app.radio_choice, 2);
        assert!(app.handle(click("demo:progress:100")));
        assert_eq!(app.progress, 1.0);
    }

    #[test]
    fn text_field_tracks_typing_and_submission() {
        let mut app = app();
        assert!(app.handle(Event::TextChanged {
            action: "demo:name",
            value: "Ada",
        }));
        assert_eq!(app.name, "Ada");
        assert!(app.handle(Event::Submitted {
            action: "demo:name:submit",
        }));
        assert_eq!(app.submitted, "Ada");
    }

    #[test]
    fn checklist_screen_delegates_its_events() {
        let mut app = app();
        assert!(app.handle(click("page:checklist")));
        assert_eq!(app.stack.last(), Some(&"checklist"));
        assert!(app.handle(Event::Toggled {
            action: "task:1",
            checked: true,
        }));
        assert!(app.checklist.summary().contains("2 of 3 tasks done"));
    }
}
