//! Native modal presentations requested by an application.
//!
//! An application returns a [`Dialog`] from [`App::dialog`](crate::app::App::dialog)
//! to show the system alert, or a [`BottomSheet`] from
//! [`App::sheet`](crate::app::App::sheet) to show a native sheet around its own
//! content. Both stay on screen for as long as the hook keeps returning them.

use std::marker::PhantomData;

use crate::element::{Element, RawElement};
use crate::message::Handler;
use crate::ui::Column;

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
    /// How the platform styles the button.
    pub role: DialogRole,
    /// What to tell the application when the button is pressed.
    pub on_tap: Handler<()>,
}

/// The system alert: a title, an optional message and up to three buttons.
///
/// iOS shows a `UIAlertController`; Android shows a Material alert dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog<M = ()> {
    /// The title.
    pub title: String,
    /// The supporting text under the title.
    pub message: Option<String>,
    /// The buttons, in the order they were added.
    pub buttons: Vec<DialogButton>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Dialog<M> {
    /// Creates a dialog with a title and no buttons.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: None,
            buttons: Vec::new(),
            marker: PhantomData,
        }
    }

    /// Sets the supporting text.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Whether the platform may dismiss the dialog without a button press
    /// (Android's back button or a tap outside): only when there is a cancel
    /// button whose message can stand in for that.
    pub fn is_cancelable(&self) -> bool {
        self.cancel_button().is_some()
    }

    /// The button that backs out of the dialog, if any.
    pub(crate) fn cancel_button(&self) -> Option<&DialogButton> {
        self.buttons
            .iter()
            .find(|button| button.role == DialogRole::Cancel)
    }

    /// Drops the message type; the handlers inside are already erased.
    pub(crate) fn erase(self) -> Dialog {
        Dialog {
            title: self.title,
            message: self.message,
            buttons: self.buttons,
            marker: PhantomData,
        }
    }
}

impl<M: Clone + Send + Sync + 'static> Dialog<M> {
    /// Adds an ordinary button that sends `message`.
    pub fn button(self, label: impl Into<String>, message: M) -> Self {
        self.with_button(label, message, DialogRole::Default)
    }

    /// Adds the button that backs out of the dialog. Its message is also sent
    /// when the platform dismisses the dialog itself.
    pub fn cancel(self, label: impl Into<String>, message: M) -> Self {
        self.with_button(label, message, DialogRole::Cancel)
    }

    /// Adds a button for a choice that cannot be undone.
    pub fn destructive(self, label: impl Into<String>, message: M) -> Self {
        self.with_button(label, message, DialogRole::Destructive)
    }

    fn with_button(mut self, label: impl Into<String>, message: M, role: DialogRole) -> Self {
        self.buttons.push(DialogButton {
            label: label.into(),
            role,
            on_tap: Handler::new(move |()| message.clone()),
        });
        self
    }
}

/// A native bottom sheet around application content.
///
/// The platform supplies the surface, rounded corners, dimming and drag
/// handle, and sizes the sheet to its content.
pub struct BottomSheet<M = ()> {
    /// What to tell the application when the user swipes the sheet away or
    /// taps outside it. The application should stop returning the sheet.
    pub on_dismiss: Handler<()>,
    content: RawElement,
    marker: PhantomData<fn() -> M>,
}

impl<M> BottomSheet<M> {
    /// Creates an empty sheet.
    pub fn new() -> Self {
        let content = Column::<M>::new()
            .padding(20)
            .spacing(12)
            .width_percent(1.0);
        Self {
            on_dismiss: Handler::none(),
            content: Element::from(content).into_raw(),
            marker: PhantomData,
        }
    }

    /// Sends `message` when the user dismisses the sheet natively.
    pub fn on_dismiss(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_dismiss = Handler::new(move |()| message.clone());
        self
    }

    /// Appends content to the sheet.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.content.children.push(child.into().into_raw());
        self
    }

    /// The sheet's content tree.
    pub(crate) fn into_content(self) -> RawElement {
        self.content
    }

    /// Drops the message type; the handlers inside are already erased.
    pub(crate) fn erase(self) -> BottomSheet {
        BottomSheet {
            on_dismiss: self.on_dismiss,
            content: self.content,
            marker: PhantomData,
        }
    }
}

impl<M> Default for BottomSheet<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Close,
        Delete,
    }

    #[test]
    fn dialog_collects_buttons_in_order() {
        let dialog = Dialog::new("Delete task?")
            .message("This cannot be undone.")
            .cancel("Cancel", Msg::Close)
            .destructive("Delete", Msg::Delete);
        assert_eq!(dialog.title, "Delete task?");
        assert_eq!(dialog.message.as_deref(), Some("This cannot be undone."));
        assert_eq!(dialog.buttons.len(), 2);
        assert_eq!(dialog.buttons[0].role, DialogRole::Cancel);
        assert_eq!(dialog.buttons[1].role, DialogRole::Destructive);

        let message = dialog.buttons[1].on_tap.message(()).expect("handler");
        assert_eq!(message.downcast_ref::<Msg>(), Some(&Msg::Delete));
    }

    #[test]
    fn only_a_dialog_with_a_cancel_button_can_be_dismissed_natively() {
        let dialog = Dialog::new("Saved").button("OK", Msg::Close);
        assert!(!dialog.is_cancelable());
        assert!(dialog.cancel("Close", Msg::Close).is_cancelable());
    }

    #[test]
    fn a_sheet_reports_its_dismissal() {
        let sheet = BottomSheet::new().on_dismiss(Msg::Close);
        let message = sheet.on_dismiss.message(()).expect("handler");
        assert_eq!(message.downcast_ref::<Msg>(), Some(&Msg::Close));
    }
}
