//! Native modal presentations requested by an application.
//!
//! An application returns a [`Dialog`] from [`App::dialog`](crate::app::App::dialog)
//! to show the system alert, or a [`BottomSheet`] from
//! [`App::sheet`](crate::app::App::sheet) to show a native sheet around its own
//! content. Both stay on screen for as long as the hook keeps returning them.

use crate::ui::{Column, Element};

/// How a dialog button is styled and placed by the platform.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DialogRole {
    /// An ordinary choice.
    #[default]
    Default,
    /// Backs out without changing anything. Also used when the platform
    /// dismisses the dialog itself, such as Android's back button.
    Cancel,
    /// A choice that deletes or otherwise cannot be undone.
    Destructive,
}

impl DialogRole {
    /// The stable code shared with the native controllers.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) const fn code(self) -> i32 {
        match self {
            Self::Default => 0,
            Self::Cancel => 1,
            Self::Destructive => 2,
        }
    }
}

/// One button of a [`Dialog`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DialogButton {
    /// The visible text.
    pub label: String,
    /// The action dispatched as [`Event::Click`](crate::app::Event::Click).
    pub action: String,
    /// How the platform styles the button.
    pub role: DialogRole,
}

/// The system alert: a title, an optional message and up to three buttons.
///
/// iOS shows a `UIAlertController`; Android shows a Material alert dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog {
    /// The title.
    pub title: String,
    /// The supporting text under the title.
    pub message: Option<String>,
    /// The buttons, in the order they were added.
    pub buttons: Vec<DialogButton>,
}

impl Dialog {
    /// Creates a dialog with a title and no buttons.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            buttons: Vec::new(),
        }
    }

    /// Sets the supporting text.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Adds an ordinary button.
    pub fn button(self, label: impl Into<String>, action: impl Into<String>) -> Self {
        self.with_button(label, action, DialogRole::Default)
    }

    /// Adds the button that backs out of the dialog.
    pub fn cancel(self, label: impl Into<String>, action: impl Into<String>) -> Self {
        self.with_button(label, action, DialogRole::Cancel)
    }

    /// Adds a button for a choice that cannot be undone.
    pub fn destructive(self, label: impl Into<String>, action: impl Into<String>) -> Self {
        self.with_button(label, action, DialogRole::Destructive)
    }

    fn with_button(
        mut self,
        label: impl Into<String>,
        action: impl Into<String>,
        role: DialogRole,
    ) -> Self {
        self.buttons.push(DialogButton {
            label: label.into(),
            action: action.into(),
            role,
        });
        self
    }

    /// The action dispatched when the platform dismisses the dialog without a
    /// button press: the cancel button's action.
    pub fn dismiss_action(&self) -> Option<&str> {
        self.buttons
            .iter()
            .find(|button| button.role == DialogRole::Cancel)
            .map(|button| button.action.as_str())
    }
}

/// A native bottom sheet around application content.
///
/// The platform supplies the surface, rounded corners, dimming and drag
/// handle; the application supplies the content.
#[derive(Clone, Debug, PartialEq)]
pub struct BottomSheet {
    /// The action dispatched when the user swipes the sheet away or taps
    /// outside it. The application should stop returning the sheet in response.
    pub dismiss_action: String,
    pub(crate) content: Column,
}

impl BottomSheet {
    /// Creates an empty sheet that dispatches `dismiss_action` when the user
    /// dismisses it natively.
    pub fn new(dismiss_action: impl Into<String>) -> Self {
        Self {
            dismiss_action: dismiss_action.into(),
            content: Column::new().padding(20).spacing(12).width_percent(1.0),
        }
    }

    /// Appends content to the sheet.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.content = self.content.child(child);
        self
    }

    /// The sheet's content tree.
    pub fn into_content(self) -> Element {
        self.content.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_collects_buttons_in_order() {
        let dialog = Dialog::new("Delete task?")
            .message("This cannot be undone.")
            .cancel("Cancel", "close")
            .destructive("Delete", "delete");
        assert_eq!(dialog.title, "Delete task?");
        assert_eq!(dialog.message.as_deref(), Some("This cannot be undone."));
        assert_eq!(dialog.buttons.len(), 2);
        assert_eq!(dialog.buttons[0].role, DialogRole::Cancel);
        assert_eq!(dialog.buttons[1].role, DialogRole::Destructive);
        assert_eq!(dialog.buttons[1].action, "delete");
    }

    #[test]
    fn native_dismissal_uses_the_cancel_action() {
        let dialog = Dialog::new("Saved").button("OK", "ok");
        assert_eq!(dialog.dismiss_action(), None);
        let dialog = dialog.cancel("Close", "close");
        assert_eq!(dialog.dismiss_action(), Some("close"));
    }

    #[test]
    fn sheet_keeps_its_dismiss_action() {
        let sheet = BottomSheet::new("sheet:close");
        assert_eq!(sheet.dismiss_action, "sheet:close");
    }
}
