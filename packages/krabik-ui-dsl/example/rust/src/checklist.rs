//! The checklist demo, kept as one page of the widget showcase.
//!
//! It retains the original example's behavior: fixed tasks with per-task
//! switches, a notes input, dynamic rows and per-note delete actions.

use krabik_ui_dsl::app::Component;
use krabik_ui_dsl::command::Command;
use krabik_ui_dsl::ui::{
    Align, Button, Column, EdgeInsets, Element, Row, ScrollView, Spacer, Switch, Text, TextField,
};

use crate::pages::{COLOR_DONE, COLOR_MUTED, COLOR_SURFACE, COLOR_TEXT, caption};

/// Everything the user can do on the checklist page.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The text in the "add a note" field changed.
    Draft(String),
    /// Adds the draft as a note.
    AddNote,
    /// Removes every note.
    ClearNotes,
    /// Deletes the note at this index.
    DeleteNote(usize),
    /// Marks the task at this index done or not done.
    SetTask(usize, bool),
    /// Hides or shows completed tasks.
    HideDone(bool),
}

/// One fixed checklist task.
struct Task {
    title: &'static str,
    /// A stable key for the task's row.
    key: &'static str,
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
                    key: "task:0",
                    done: true,
                },
                Task {
                    title: "Register the app factory",
                    key: "task:1",
                    done: false,
                },
                Task {
                    title: "Ship the checklist",
                    key: "task:2",
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

    fn task_row(&self, index: usize, task: &Task) -> Element<Msg> {
        let title = if task.done {
            format!("✓ {}", task.title)
        } else {
            task.title.to_string()
        };
        Row::new()
            .key(task.key)
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
                    .on_toggle(move |done| Msg::SetTask(index, done))
                    .accessibility_label(task.title),
            )
            .into()
    }

    /// One note row with a delete button whose action is built at runtime.
    fn note_row(&self, index: usize, note: &str) -> Element<Msg> {
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
                    .on_click(Msg::DeleteNote(index))
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
}

impl Component for Checklist {
    type Msg = Msg;

    /// Builds the checklist page.
    fn view(&self) -> Element<Msg> {
        let mut column = Column::new()
            .padding(24)
            .spacing(12)
            .width_percent(1.0)
            .align_items(Align::Start)
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
                            .on_toggle(Msg::HideDone)
                            .accessibility_label("Hide completed"),
                    ),
            );

        for (index, task) in self.tasks.iter().enumerate() {
            if self.hide_done && task.done {
                continue;
            }
            column = column.child(self.task_row(index, task));
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
                            .on_change(Msg::Draft)
                            .on_submit(Msg::AddNote)
                            .grow(1.0),
                    )
                    .child(Button::new("Add").on_click(Msg::AddNote)),
            );

        if !self.notes.is_empty() {
            column = column.child(Button::new("Clear notes").on_click(Msg::ClearNotes));
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

    fn update(&mut self, msg: Msg) -> Command<Msg> {
        match msg {
            Msg::Draft(text) => self.draft = text,
            Msg::AddNote => {
                self.add_note();
            }
            Msg::ClearNotes => self.notes.clear(),
            Msg::DeleteNote(index) => {
                if index < self.notes.len() {
                    self.notes.remove(index);
                }
            }
            Msg::SetTask(index, done) => {
                if let Some(task) = self.tasks.get_mut(index) {
                    task.done = done;
                }
            }
            Msg::HideDone(hide) => self.hide_done = hide,
        }
        Command::none()
    }
}
