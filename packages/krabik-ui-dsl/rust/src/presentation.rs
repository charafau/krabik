//! Native modal presentations requested by an application.
//!
//! The runtime reads [`Presentation`] and asks the platform to present a native
//! dialog or bottom sheet. The content is built by
//! [`App::presentation_view`](crate::app::App::presentation_view).

use crate::ui::{Column, CornerRadii, Element};

/// The kind of native modal to present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum PresentationKind {
    /// A centered dialog with a native dimming layer.
    Dialog = 0,
    /// A bottom sheet with a native grabber and detents where available.
    Sheet = 1,
}

/// One modal currently presented by the application.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Presentation {
    /// A stable identifier used to match the modal across renders.
    pub key: String,
    /// Whether to present a dialog or a bottom sheet.
    pub kind: PresentationKind,
    /// The action dispatched when the native host dismisses the modal.
    pub dismiss_action: String,
}

impl Presentation {
    /// Creates a presentation request.
    pub fn new(
        key: impl Into<String>,
        kind: PresentationKind,
        dismiss_action: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            kind,
            dismiss_action: dismiss_action.into(),
        }
    }
}

/// The content of a native dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct Dialog(pub(crate) Column);

impl Dialog {
    /// Creates an empty dialog card.
    pub fn new() -> Self {
        Self(
            Column::new()
                .padding(20)
                .spacing(12)
                .width_percent(1.0)
                .background(0xFFFF_FFFF)
                .corner_radius(16)
                .elevation(8),
        )
    }

    /// Appends content to the dialog.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0 = self.0.child(child);
        self
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Dialog> for Element {
    fn from(dialog: Dialog) -> Self {
        dialog.0.into()
    }
}

/// The content of a native bottom sheet.
#[derive(Clone, Debug, PartialEq)]
pub struct BottomSheet(pub(crate) Column);

impl BottomSheet {
    /// Creates an empty bottom-sheet card.
    pub fn new() -> Self {
        Self(
            Column::new()
                .padding(20)
                .spacing(12)
                .width_percent(1.0)
                .background(0xFFFF_FFFF)
                .corner_radii(CornerRadii::new(20.0, 20.0, 0.0, 0.0))
                .elevation(8),
        )
    }

    /// Appends content to the sheet.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0 = self.0.child(child);
        self
    }
}

impl Default for BottomSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BottomSheet> for Element {
    fn from(sheet: BottomSheet) -> Self {
        sheet.0.into()
    }
}
