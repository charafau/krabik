//! A standalone todo application built with the UI DSL.
//!
//! The showcase example uses the whole current widget set: a virtualized
//! recycled list with checkbox rows, radio-button filters, a priority slider, a
//! progress indicator and a sort switch. It is a normal Rust crate that depends
//! on [`krabik_ui_dsl`], implements [`App`], and registers itself through
//! [`set_app_factory`]. The iOS shell in `../ios` links it and the Android
//! shell in `../android` loads its cdylib.

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Event, set_app_factory};
use krabik_ui_dsl::ui::{
    Align, Button, Checkbox, Column, EdgeInsets, Element, List, ProgressIndicator, RadioButton,
    Row, Slider, Spacer, Switch, Text, TextField,
};

const COLOR_TEXT: u32 = 0xFF20_2125;
const COLOR_MUTED: u32 = 0xFF66_6666;
const COLOR_SURFACE: u32 = 0xFFF4_F4F4;
const COLOR_DONE: u32 = 0xFFE6_F4EA;
const COLOR_ACCENT: u32 = 0xFF00_897B;

/// Every recycled row has the same height, so the list can stay virtualized.
const ROW_HEIGHT: f32 = 60.0;

/// Which todos the list shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Filter {
    #[default]
    All,
    Active,
    Done,
}

/// One todo item owned by the application.
#[derive(Clone, Debug, PartialEq)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
    priority: u8,
}

/// The snapshot of one todo passed to the recycled list.
#[derive(Clone, Debug)]
struct TodoRow {
    id: u64,
    title: String,
    done: bool,
    priority: u8,
}

/// The state of the example application.
pub struct TodoApp {
    todos: Vec<Todo>,
    next_id: u64,
    draft: String,
    priority: f32,
    filter: Filter,
    sort_by_priority: bool,
}

impl TodoApp {
    /// Creates the example application with a few starter tasks.
    pub fn new() -> Self {
        Self {
            todos: vec![
                Todo {
                    id: 1,
                    title: "Review the UI DSL".to_string(),
                    done: true,
                    priority: 3,
                },
                Todo {
                    id: 2,
                    title: "Wire the native renderer".to_string(),
                    done: false,
                    priority: 4,
                },
                Todo {
                    id: 3,
                    title: "Ship the todo app".to_string(),
                    done: false,
                    priority: 5,
                },
                Todo {
                    id: 4,
                    title: "Write the example README".to_string(),
                    done: false,
                    priority: 2,
                },
            ],
            next_id: 5,
            draft: String::new(),
            priority: 3.0,
            filter: Filter::All,
            sort_by_priority: false,
        }
    }

    /// Registers the example as the global application factory.
    ///
    /// The host shell calls this once at startup, before creating the root
    /// view. Without it the platform views fall back to the built-in gallery.
    pub fn install() {
        set_app_factory(|| Box::new(Self::new()));
    }

    fn summary(&self) -> String {
        let done = self.todos.iter().filter(|todo| todo.done).count();
        format!("{done} of {} done", self.todos.len())
    }

    fn progress(&self) -> f32 {
        if self.todos.is_empty() {
            return 0.0;
        }
        let done = self.todos.iter().filter(|todo| todo.done).count();
        done as f32 / self.todos.len() as f32
    }

    fn visible_rows(&self) -> Arc<Vec<TodoRow>> {
        let mut rows: Vec<TodoRow> = self
            .todos
            .iter()
            .filter(|todo| match self.filter {
                Filter::All => true,
                Filter::Active => !todo.done,
                Filter::Done => todo.done,
            })
            .map(|todo| TodoRow {
                id: todo.id,
                title: todo.title.clone(),
                done: todo.done,
                priority: todo.priority,
            })
            .collect();

        if self.sort_by_priority {
            rows.sort_by(|left, right| {
                right
                    .priority
                    .cmp(&left.priority)
                    .then(left.id.cmp(&right.id))
            });
        }
        Arc::new(rows)
    }

    fn add_todo(&mut self) -> bool {
        let title = self.draft.trim().to_string();
        if title.is_empty() {
            return false;
        }
        let todo = Todo {
            id: self.next_id,
            title,
            done: false,
            priority: self.priority.round().clamp(1.0, 5.0) as u8,
        };
        self.next_id += 1;
        self.todos.push(todo);
        self.draft.clear();
        true
    }

    fn set_done(&mut self, id: u64, done: bool) -> bool {
        let Some(todo) = self.todos.iter_mut().find(|todo| todo.id == id) else {
            return false;
        };
        if todo.done == done {
            return false;
        }
        todo.done = done;
        true
    }

    fn delete_todo(&mut self, id: u64) -> bool {
        let before = self.todos.len();
        self.todos.retain(|todo| todo.id != id);
        self.todos.len() != before
    }

    fn clear_completed(&mut self) -> bool {
        let before = self.todos.len();
        self.todos.retain(|todo| !todo.done);
        self.todos.len() != before
    }

    fn set_filter(&mut self, filter: Filter) -> bool {
        if self.filter == filter {
            return false;
        }
        self.filter = filter;
        true
    }
}

impl Default for TodoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for TodoApp {
    fn view(&self) -> Element {
        let rows = self.visible_rows();
        let done = self.todos.iter().filter(|todo| todo.done).count();

        let mut column = Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .height_percent(1.0)
            .align_items(Align::Start)
            .child(Text::new("Todo").size(30).bold().color(COLOR_TEXT))
            .child(caption(&self.summary()))
            .child(
                ProgressIndicator::new(self.progress())
                    .color(COLOR_ACCENT)
                    .width_percent(1.0),
            )
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(RadioButton::new("All", self.filter == Filter::All).action("filter:all"))
                    .child(
                        RadioButton::new("Active", self.filter == Filter::Active)
                            .action("filter:active"),
                    )
                    .child(
                        RadioButton::new("Done", self.filter == Filter::Done).action("filter:done"),
                    ),
            )
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(
                        TextField::new(self.draft.clone())
                            .placeholder("What needs doing?")
                            .action("todo:draft")
                            .submit_action("todo:add")
                            .grow(1.0),
                    )
                    .child(Button::new("Add").action("todo:add")),
            )
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(
                        Text::new(format!("Priority: {:.0}", self.priority))
                            .size(14)
                            .color(COLOR_MUTED),
                    )
                    .child(
                        Slider::new(self.priority)
                            .min(1.0)
                            .max(5.0)
                            .action("todo:priority")
                            .grow(1.0),
                    ),
            )
            .child(
                Row::new()
                    .width_percent(1.0)
                    .align_items(Align::Center)
                    .child(Text::new("Sort by priority").size(14).color(COLOR_MUTED))
                    .child(Spacer::new())
                    .child(
                        Switch::new(self.sort_by_priority)
                            .action("todo:sort")
                            .accessibility_label("Sort by priority"),
                    ),
            )
            .child(
                Row::new()
                    .width_percent(1.0)
                    .align_items(Align::Center)
                    .child(Spacer::new())
                    .child(
                        Button::new("Clear completed")
                            .action("todo:clear-done")
                            .enabled(done > 0),
                    ),
            );

        if rows.is_empty() {
            column = column.child(caption("Nothing here yet."));
        }

        column
            .child(
                List::new(rows)
                    .item_height(ROW_HEIGHT)
                    .buffer(3)
                    .recycled()
                    .key(|row| row.id.to_string())
                    .row(todo_row),
            )
            .into()
    }

    fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::TextChanged {
                action: "todo:draft",
                value,
            } => {
                self.draft = value.to_string();
                true
            }
            Event::Submitted { action: "todo:add" } => self.add_todo(),
            Event::Click { action } => {
                if let Some(id) = action
                    .strip_prefix("todo:delete:")
                    .and_then(|id| id.parse::<u64>().ok())
                {
                    return self.delete_todo(id);
                }
                match action {
                    "todo:add" => self.add_todo(),
                    "todo:clear-done" => self.clear_completed(),
                    _ => false,
                }
            }
            Event::Toggled { action, checked } => {
                if let Some(id) = action
                    .strip_prefix("todo:toggle:")
                    .and_then(|id| id.parse::<u64>().ok())
                {
                    return self.set_done(id, checked);
                }
                match action {
                    "filter:all" if checked => self.set_filter(Filter::All),
                    "filter:active" if checked => self.set_filter(Filter::Active),
                    "filter:done" if checked => self.set_filter(Filter::Done),
                    "todo:sort" => {
                        self.sort_by_priority = checked;
                        true
                    }
                    _ => false,
                }
            }
            Event::ValueChanged {
                action: "todo:priority",
                value,
            } => {
                let snapped = value.round().clamp(1.0, 5.0);
                if (snapped - self.priority).abs() > f32::EPSILON {
                    self.priority = snapped;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// One recycled row: a checkbox, a priority chip and a delete button.
fn todo_row(row: &TodoRow) -> Element {
    Row::new()
        .width_percent(1.0)
        .height(ROW_HEIGHT)
        .padding_each(EdgeInsets::symmetric(10, 6))
        .spacing(8)
        .align_items(Align::Center)
        .background(if row.done { COLOR_DONE } else { COLOR_SURFACE })
        .corner_radius(10)
        .child(
            Checkbox::new(row.title.clone(), row.done)
                .action(format!("todo:toggle:{}", row.id))
                .size(15)
                .grow(1.0),
        )
        .child(
            Text::new(format!("P{}", row.priority))
                .size(12)
                .color(COLOR_ACCENT),
        )
        .child(
            Button::new("Delete")
                .action(format!("todo:delete:{}", row.id))
                .accessibility_label(format!("Delete {}", row.title)),
        )
        .into()
}

/// A muted secondary line of copy.
fn caption(content: &str) -> Text {
    Text::new(content).size(14).color(COLOR_MUTED)
}

/// C entry point called by the iOS host before creating the root view.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "C" fn krabik_ui_dsl_todo_install() {
    TodoApp::install();
}

/// JNI entry point called by the Android host before creating the root view.
///
/// The Android shell loads this crate's `cdylib`, which statically includes
/// the DSL renderer, so this one library provides the `RustRootView` JNI
/// entry points as well as this factory install.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_TodoBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    TodoApp::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> TodoApp {
        TodoApp::new()
    }

    #[test]
    fn starts_with_starter_tasks() {
        let app = app();
        assert_eq!(app.todos.len(), 4);
        assert_eq!(app.summary(), "1 of 4 done");
        assert_eq!(app.filter, Filter::All);
    }

    #[test]
    fn adding_trims_and_ignores_empty_drafts() {
        let mut app = app();
        assert!(!app.add_todo());

        app.draft = "  Buy milk  ".to_string();
        app.priority = 4.6;
        assert!(app.add_todo());

        let todo = app.todos.last().expect("todo");
        assert_eq!(todo.title, "Buy milk");
        assert_eq!(todo.priority, 5);
        assert!(app.draft.is_empty());
    }

    #[test]
    fn toggling_updates_the_summary() {
        let mut app = app();
        assert!(app.set_done(2, true));
        assert_eq!(app.summary(), "2 of 4 done");
        assert!(!app.set_done(2, true));
        assert!(app.set_done(2, false));
        assert_eq!(app.summary(), "1 of 4 done");
    }

    #[test]
    fn filters_select_visible_rows() {
        let mut app = app();

        assert!(app.set_filter(Filter::Active));
        assert_eq!(app.visible_rows().len(), 3);
        assert!(app.visible_rows().iter().all(|row| !row.done));

        assert!(app.set_filter(Filter::Done));
        assert_eq!(app.visible_rows().len(), 1);
        assert!(app.visible_rows().iter().all(|row| row.done));

        assert!(!app.set_filter(Filter::Done));
    }

    #[test]
    fn sorting_puts_high_priorities_first() {
        let mut app = app();
        app.sort_by_priority = true;
        let rows = app.visible_rows();
        let priorities: Vec<u8> = rows.iter().map(|row| row.priority).collect();
        assert_eq!(priorities, vec![5, 4, 3, 2]);
    }

    #[test]
    fn deleting_and_clearing_remove_rows() {
        let mut app = app();
        assert!(app.delete_todo(2));
        assert_eq!(app.todos.len(), 3);

        assert!(app.clear_completed());
        assert!(app.todos.iter().all(|todo| !todo.done));
        assert!(!app.clear_completed());
    }

    #[test]
    fn priority_values_are_snapped_to_the_slider_range() {
        let mut app = app();
        let event = Event::ValueChanged {
            action: "todo:priority",
            value: 9.4,
        };
        assert!(app.handle(event));
        assert_eq!(app.priority, 5.0);

        let event = Event::ValueChanged {
            action: "todo:priority",
            value: 0.2,
        };
        assert!(app.handle(event));
        assert_eq!(app.priority, 1.0);
    }
}
