//! Native modal presentations requested by an application.
//!
//! The runtime reads [`Presentation`] and asks the platform to present a native
//! dialog or bottom sheet. The content is built by
//! [`App::presentation_view`](crate::app::App::presentation_view).

/// The kind of native modal to present.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationKind {
    /// A centered dialog with a native dimming layer.
    Dialog,
    /// A bottom sheet with a native grabber and detents where available.
    Sheet,
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
