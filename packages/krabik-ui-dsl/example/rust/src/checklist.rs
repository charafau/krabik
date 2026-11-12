//! The checklist demo, kept as one page of the widget showcase.
//!
//! It retains the original example's behavior: fixed tasks with per-task
//! switches, a notes input, dynamic rows and per-note delete actions.

use krabik_ui_dsl::app::Event;
use krabik_ui_dsl::ui::{
    Align, Button, Column, EdgeInsets, Element, Row, ScrollView, Spacer, Switch, Text, TextField,
};

use crate::pages::{COLOR_DONE, COLOR_MUTED, COLOR_SURFACE, COLOR_TEXT, caption};

/// One fixed checklist task.
struct Task {
    title: &'static str,
    action: &'static str,
    done: bool,
}

/// The state of the checklist page.
pub(crate) struct Checklist {
    tasks: Vec<Task>,
    notes: Vec<String>,
    draft: String,
    hide_done: bool,
}

impl Checklist {
    /// Creates the checklist with its initial tasks.
    pub(crate) fn new() -> Self {
        Self {
            tasks: vec![
                Task {
                    title: "Review the renderer",
                    action: "task:0",
                    done: true,
                },
                Task {
                    title: "Register the app factory",
                    action: "task:1",
                    done: false,
                },
                Task {
                    title: "Ship the checklist",
                    action: "task:2",
                    done: false,
                },
            ],
            notes: Vec::new(),
            draft: String::new(),
            hide_done: false,
        }
    }

    pub(crate) fn summary(&self) -> String {
        let done = self.tasks.iter().filter(|task| task.done).count();
        let note_word = if self.notes.len() == 1 {
            "note"
        } else {
            "notes"
        };
        format!(
            "{} of {} tasks done · {} {note_word}",
            done,
            self.tasks.len(),
            self.notes.len()
        )
    }

    fn task_row(&self, task: &Task) -> Element {
        let title = if task.done {
            format!("✓ {}", task.title)
        } else {
            task.title.to_string()
        };
        Row::new()
            .key(task.action)
            .width_percent(1.0)
            .spacing(8)
            .align_items(Align::Center)
            .padding_each(EdgeInsets::symmetric(12, 10))
            .background(if task.done { COLOR_DONE } else { COLOR_SURFACE })
            .corner_radius(10)
            .child(Text::new(title).size(16).color(COLOR_TEXT))
            .child(Spacer::new())
            .child(
                Switch::new(task.done)
                    .action(task.action)
                    .accessibility_label(task.title),
            )
            .into()
    }

    /// One note row with a delete button whose action is built at runtime.
    fn note_row(&self, index: usize, note: &str) -> Element {
        Row::new()
            .width_percent(1.0)
            .spacing(8)
            .align_items(Align::Center)
            .padding_each(EdgeInsets::symmetric(12, 8))
            .background(COLOR_SURFACE)
            .corner_radius(8)
            .child(Text::new(format!("• {note}")).size(15).color(COLOR_TEXT))
            .child(Spacer::new())
            .child(
                Button::new("Delete")
                    .action(format!("note:delete:{index}"))
                    .accessibility_label(format!("Delete {note}")),
            )
            .into()
    }

    fn add_note(&mut self) -> bool {
        let note = self.draft.trim().to_string();
        if note.is_empty() {
            return false;
        }
        self.notes.push(note);
        self.draft.clear();
        true
    }

    fn toggle(&mut self, action: &str, checked: bool) -> bool {
        for task in &mut self.tasks {
            if task.action == action {
                task.done = checked;
                return true;
            }
        }
        if action == "tasks:hide-done" {
            self.hide_done = checked;
            return true;
        }
        false
    }

    /// Builds the checklist page.
    pub(crate) fn view(&self) -> Element {
        let mut column = Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .align_items(Align::Start)
            .child(super::pages::back_button())
            .child(Text::new("Checklist").size(30).bold().color(COLOR_TEXT))
            .child(caption(&self.summary()))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(Text::new("Hide completed").size(16).color(COLOR_MUTED))
                    .child(Spacer::new())
                    .child(
                        Switch::new(self.hide_done)
                            .action("tasks:hide-done")
                            .accessibility_label("Hide completed"),
                    ),
            );

        for task in &self.tasks {
            if self.hide_done && task.done {
                continue;
            }
            column = column.child(self.task_row(task));
        }

        column = column
            .child(Spacer::new().height(8))
            .child(Text::new("Notes").size(20).bold().color(COLOR_TEXT))
            .child(caption(
                "Notes are dynamic rows with per-row delete actions built at runtime.",
            ))
            .child(
                Row::new()
                    .width_percent(1.0)
                    .spacing(8)
                    .align_items(Align::Center)
                    .child(
                        TextField::new(self.draft.clone())
                            .placeholder("Add a note")
                            .action("notes:draft")
                            .submit_action("notes:add")
                            .grow(1.0),
                    )
                    .child(Button::new("Add").action("notes:add")),
            );

        if !self.notes.is_empty() {
            column = column.child(Button::new("Clear notes").action("notes:clear"));
            for (index, note) in self.notes.iter().enumerate() {
                column = column.child(self.note_row(index, note));
            }
        }

        ScrollView::new()
            .width_percent(1.0)
            .height_percent(1.0)
            .child(column)
            .into()
    }

    /// Handles a checklist event. Returns whether the UI should re-render.
    pub(crate) fn handle(&mut self, event: Event<'_>) -> bool {
        match event {
            Event::TextChanged {
                action: "notes:draft",
                value,
            } => {
                self.draft = value.to_string();
                true
            }
            Event::Click { action } => {
                if let Some(index) = action
                    .strip_prefix("note:delete:")
                    .and_then(|index| index.parse::<usize>().ok())
                    && index < self.notes.len()
                {
                    self.notes.remove(index);
                    return true;
                }
                match action {
                    "notes:add" => self.add_note(),
                    "notes:clear" => {
                        self.notes.clear();
                        true
                    }
                    _ => false,
                }
            }
            Event::Submitted {
                action: "notes:add",
            } => self.add_note(),
            Event::Toggled { action, checked } => self.toggle(action, checked),
            _ => false,
        }
    }
}
