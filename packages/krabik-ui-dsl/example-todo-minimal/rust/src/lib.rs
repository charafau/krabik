//! The minimal todo example: a standalone app built with the UI DSL.
//!
//! It keeps only the core todo flow — add, complete, delete and clear — and
//! renders the list with a `List::recycled()` on both platforms. The fuller
//! widget showcase lives in the sibling `example-todo` app.
//!
//! The views are written with the `view!` macro. It is only syntax for the
//! builder calls the other examples write by hand.

use std::sync::Arc;

use jni::EnvUnowned;
use jni::objects::JObject;
use krabik_ui_dsl::app::{App, Component, set_app_factory};
use krabik_ui_dsl::scaffold::{AppBar, Scaffold};
use krabik_ui_dsl::ui::{
    Align, Button, Checkbox, Color, Column, EdgeInsets, Element, List, Row, Spacer, Text, TextField,
};
use krabik_ui_dsl::view;

const COLOR_MUTED: Color = Color::adaptive(0xFF66_6666, 0xFFA0_A4A8);
const COLOR_SURFACE: Color = Color::adaptive(0xFFF4_F4F4, 0xFF2A_2B2E);
const COLOR_DONE: Color = Color::adaptive(0xFFE6_F4EA, 0xFF1E_3A2A);

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

/// Everything the user can do in the app.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The text in the "new task" field changed.
    Draft(String),
    /// Adds the draft as a task.
    Add,
    /// Marks the task with this id done or not done.
    SetDone(u64, bool),
    /// Deletes the task with this id.
    Delete(u64),
    /// Removes every completed task.
    ClearCompleted,
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
        set_app_factory(Self::new);
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

impl Component for MinimalTodoApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Draft(text) => self.draft = text,
            Msg::Add => {
                self.add_todo();
            }
            Msg::SetDone(id, done) => {
                self.set_done(id, done);
            }
            Msg::Delete(id) => {
                self.delete_todo(id);
            }
            Msg::ClearCompleted => {
                self.clear_completed();
            }
        }
    }

    fn view(&self) -> Element<Msg> {
        let rows = self.visible_rows();
        let done = self.todos.iter().filter(|todo| todo.done).count();

        view! {
            Column(
                padding: 24,
                spacing: 12,
                width_percent: 1.0,
                height_percent: 1.0,
                align_items: Align::Start,
            ) {
                caption(&self.summary())
                Row(width_percent: 1.0, spacing: 8, align_items: Align::Center) {
                    TextField(
                        self.draft.clone(),
                        placeholder: "What needs doing?",
                        on_change: Msg::Draft,
                        on_submit: Msg::Add,
                        grow: 1.0,
                    )
                    Button("Add", on_click: Msg::Add)
                }
                Row(width_percent: 1.0, align_items: Align::Center) {
                    Spacer
                    Button("Clear completed", on_click: Msg::ClearCompleted, enabled: done > 0)
                }
                if rows.is_empty() {
                    caption("Nothing here yet.")
                }
                List(
                    rows,
                    item_height: ROW_HEIGHT,
                    buffer: 2,
                    key: |row| row.id.to_string(),
                    row: todo_row,
                    recycled,
                )
            }
        }
    }
}

impl App for MinimalTodoApp {
    /// One screen with a native app bar. Without a navigation stack the
    /// chrome is described here and the body is the component's view.
    fn scaffold(&self) -> Option<Scaffold<Msg>> {
        Some(
            Scaffold::new()
                .app_bar(AppBar::new("Todo"))
                .body(self.view()),
        )
    }
}

fn todo_row(row: &TodoRow) -> Element<Msg> {
    let id = row.id;
    view! {
        Row(
            width_percent: 1.0,
            height: ROW_HEIGHT,
            padding_each: EdgeInsets::symmetric(10, 6),
            spacing: 8,
            align_items: Align::Center,
            background: if row.done { COLOR_DONE } else { COLOR_SURFACE },
            corner_radius: 10,
        ) {
            Checkbox(
                row.title.clone(),
                row.done,
                on_toggle: move |done| Msg::SetDone(id, done),
                size: 15,
                grow: 1.0,
            )
            Button(
                "Delete",
                on_click: Msg::Delete(id),
                accessibility_label: format!("Delete {}", row.title),
            )
        }
    }
}

fn caption(content: &str) -> Text<Msg> {
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

    #[test]
    fn messages_drive_the_same_flow() {
        let mut app = app();
        app.update(Msg::Draft("Buy milk".to_string()));
        app.update(Msg::Add);
        assert_eq!(app.todos.last().unwrap().title, "Buy milk");
        let id = app.todos.last().unwrap().id;

        app.update(Msg::SetDone(id, true));
        assert_eq!(app.summary(), "2 of 5 done");
        app.update(Msg::Delete(id));
        assert_eq!(app.todos.len(), 4);
        app.update(Msg::ClearCompleted);
        assert!(app.todos.iter().all(|todo| !todo.done));
    }
}
