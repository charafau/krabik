//! A widget showcase built with the UI DSL.
//!
//! The home screen lists every widget; each entry opens a page that
//! demonstrates that widget in isolation. Navigation is application state (a
//! [`Page`] enum) so the example stays independent of any future navigation
//! primitive.
//!
//! The app is a normal Rust crate that depends on [`krabik_ui_dsl`], implements
//! [`App`], and registers itself through [`set_app_factory`]. The iOS shell in
//! `../ios` links it and the Android shell in `../android` loads its cdylib.

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Event, set_app_factory};
use krabik_ui_dsl::ui::{DateValue, Element};

mod checklist;
mod pages;

use checklist::Checklist;

/// A page in the widget catalog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Page {
    /// The list of every widget.
    #[default]
    Home,
    Text,
    Button,
    TextField,
    Switch,
    Checkbox,
    RadioButtons,
    RadioGroup,
    DatePicker,
    Slider,
    Progress,
    Spinner,
    Image,
    Scroll,
    List,
    RecycledList,
    Layouts,
    Styling,
    Checklist,
}

impl Page {
    /// Maps a `page:*` action to a page.
    fn from_action(action: &str) -> Option<Self> {
        Some(match action.strip_prefix("page:")? {
            "home" => Self::Home,
            "text" => Self::Text,
            "button" => Self::Button,
            "text-field" => Self::TextField,
            "switch" => Self::Switch,
            "checkbox" => Self::Checkbox,
            "radio-buttons" => Self::RadioButtons,
            "radio-group" => Self::RadioGroup,
            "date-picker" => Self::DatePicker,
            "slider" => Self::Slider,
            "progress" => Self::Progress,
            "spinner" => Self::Spinner,
            "image" => Self::Image,
            "scroll" => Self::Scroll,
            "list" => Self::List,
            "recycled-list" => Self::RecycledList,
            "layouts" => Self::Layouts,
            "styling" => Self::Styling,
            "checklist" => Self::Checklist,
            _ => return None,
        })
    }
}

/// The state of the showcase application.
pub struct ShowcaseApp {
    /// The page currently shown.
    pub page: Page,
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
    date: DateValue,
    inline_date: DateValue,
    volume: f32,
    stepped: f32,
    progress: f32,
    list_items: Arc<Vec<String>>,
    recycled_items: Arc<Vec<String>>,
    recycled_taps: usize,
}

impl ShowcaseApp {
    /// Creates the showcase on its home page.
    pub fn new() -> Self {
        Self {
            page: Page::Home,
            checklist: Checklist::new(),
            name: String::new(),
            submitted: String::new(),
            taps: 0,
            notifications: false,
            wifi: true,
            bluetooth: false,
            radio_choice: 1,
            size_choice: 1,
            date: DateValue::new(2026, 11, 12),
            inline_date: DateValue::new(2026, 11, 12),
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
        pages::view(self)
    }

    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::Click { action } if action.starts_with("page:") => {
                let Some(page) = Page::from_action(action) else {
                    return false;
                };
                if self.page == page {
                    return false;
                }
                self.page = page;
                true
            }
            Event::Back => {
                if self.page == Page::Home {
                    false
                } else {
                    self.page = Page::Home;
                    true
                }
            }
            // The checklist page owns every remaining event on its page.
            _ if self.page == Page::Checklist => self.checklist.handle(event),
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
        assert_eq!(app.page, Page::Home);
        assert_eq!(app.size_choice, 1);
        assert_eq!(app.volume, 40.0);
    }

    #[test]
    fn page_actions_navigate_and_back() {
        let mut app = app();
        assert!(app.handle(click("page:radio-group")));
        assert_eq!(app.page, Page::RadioGroup);
        assert!(!app.handle(click("page:radio-group")));
        assert!(app.handle(click("page:home")));
        assert_eq!(app.page, Page::Home);
        assert!(!app.handle(click("page:unknown")));
    }

    #[test]
    fn back_returns_to_the_catalog() {
        let mut app = app();
        assert!(!app.handle(Event::Back));
        assert!(app.handle(click("page:radio-group")));
        assert!(app.handle(Event::Back));
        assert_eq!(app.page, Page::Home);
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
    fn checklist_page_delegates_its_events() {
        let mut app = app();
        assert!(app.handle(click("page:checklist")));
        assert_eq!(app.page, Page::Checklist);
        assert!(app.handle(Event::Toggled {
            action: "task:1",
            checked: true,
        }));
        assert!(app.checklist.summary().contains("2 of 3 tasks done"));
    }
}
