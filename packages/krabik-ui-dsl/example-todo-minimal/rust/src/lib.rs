//! The minimal todo example: a standalone app built with the UI DSL.
//!
//! It keeps only the core todo flow — add, complete, delete and clear — and
//! renders the list with a `List::recycled()` on both platforms. The fuller
//! widget showcase lives in the sibling `example-todo` app.

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Event, set_app_factory};
use krabik_ui_dsl::ui::{
    Align, Button, Checkbox, Column, EdgeInsets, Element, List, Row, Spacer, Text, TextField,
};

const COLOR_TEXT: u32 = 0xFF20_2125;
const COLOR_MUTED: u32 = 0xFF66_6666;
const COLOR_SURFACE: u32 = 0xFFF4_F4F4;
const COLOR_DONE: u32 = 0xFFE6_F4EA;

const ROW_HEIGHT: f32 = 56.0;

#[derive(Clone, Debug, PartialEq)]
struct Todo {
    id: u64,
    title: String,
    done: bool,
}

#[derive(Clone, Debug)]
struct TodoRow {
    id: u64,
    title: String,
    done: bool,
}

/// The state of the minimal todo application.
pub struct MinimalTodoApp {
    todos: Vec<Todo>,
    next_id: u64,
    draft: String,
}

impl MinimalTodoApp {
    /// Creates the app with a few starter tasks.
    pub fn new() -> Self {
        Self {
            todos: vec![
                Todo {
                    id: 1,
                    title: "Review the UI DSL".to_string(),
                    done: true,
                },
                Todo {
                    id: 2,
                    title: "Wire the native renderer".to_string(),
                    done: false,
                },
                Todo {
                    id: 3,
                    title: "Ship the todo app".to_string(),
                    done: false,
                },
                Todo {
                    id: 4,
                    title: "Write the example README".to_string(),
                    done: false,
                },
            ],
            next_id: 5,
            draft: String::new(),
        }
    }

    /// Registers the app as the global application factory.
    pub fn install() {
        set_app_factory(|| Box::new(Self::new()));
    }

    fn summary(&self) -> String {
        let done = self.todos.iter().filter(|todo| todo.done).count();
        format!("{done} of {} done", self.todos.len())
    }

    fn visible_rows(&self) -> Arc<Vec<TodoRow>> {
        Arc::new(
            self.todos
                .iter()
                .map(|todo| TodoRow {
                    id: todo.id,
                    title: todo.title.clone(),
                    done: todo.done,
                })
                .collect(),
        )
    }

    fn add_todo(&mut self) -> bool {
        let title = self.draft.trim().to_string();
        if title.is_empty() {
            return false;
        }
        self.todos.push(Todo {
            id: self.next_id,
            title,
            done: false,
        });
        self.next_id += 1;
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
}

impl Default for MinimalTodoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for MinimalTodoApp {
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
                    .buffer(2)
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
                let Some(id) = action
                    .strip_prefix("todo:toggle:")
                    .and_then(|id| id.parse::<u64>().ok())
                else {
                    return false;
                };
                self.set_done(id, checked)
            }
            _ => false,
        }
    }
}

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
            Button::new("Delete")
                .action(format!("todo:delete:{}", row.id))
                .accessibility_label(format!("Delete {}", row.title)),
        )
        .into()
}

fn caption(content: &str) -> Text {
    Text::new(content).size(14).color(COLOR_MUTED)
}

/// C entry point called by the iOS host before creating the root view.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "C" fn krabik_ui_dsl_todo_minimal_install() {
    MinimalTodoApp::install();
}

/// JNI entry point called by the Android host before creating the root view.
///
/// # Safety
///
/// Must be called before any root view is created, and only once.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_MinimalTodoBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    MinimalTodoApp::install();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> MinimalTodoApp {
        MinimalTodoApp::new()
    }

    #[test]
    fn starts_with_starter_tasks() {
        let app = app();
        assert_eq!(app.todos.len(), 4);
        assert_eq!(app.summary(), "1 of 4 done");
    }

    #[test]
    fn adding_trims_and_ignores_empty_drafts() {
        let mut app = app();
        assert!(!app.add_todo());
        app.draft = "  Buy milk  ".to_string();
        assert!(app.add_todo());
        assert_eq!(app.todos.last().unwrap().title, "Buy milk");
        assert!(app.draft.is_empty());
    }

    #[test]
    fn toggling_updates_the_summary() {
        let mut app = app();
        assert!(app.set_done(2, true));
        assert_eq!(app.summary(), "2 of 4 done");
        assert!(!app.set_done(2, true));
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
}
