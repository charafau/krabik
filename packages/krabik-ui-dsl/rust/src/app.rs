//! Application interface.
//!
//! Applications describe their UI and respond to user interactions by
//! implementing the [`App`] trait. The platform renderers query the current
//! UI tree through [`App::view`] and forward native events through
//! [`App::handle`].

use std::sync::OnceLock;

use crate::ui::Element;

/// A user interaction dispatched by a native widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event<'a> {
    /// A button was pressed.
    Click {
        /// The action configured on the button.
        action: &'static str,
    },
    /// The text of an input changed.
    TextChanged {
        /// The action configured on the input.
        action: &'static str,
        /// The new text of the input.
        value: &'a str,
    },
}

impl Event<'_> {
    /// The action the event was dispatched for.
    pub fn action(&self) -> &'static str {
        match self {
            Self::Click { action } | Self::TextChanged { action, .. } => action,
        }
    }
}

/// An interactive application driven by the UI runtime.
pub trait App: Send + 'static {
    /// Returns the declarative description of the current screen.
    fn view(&self) -> Element;

    /// Handles a user interaction dispatched by a native widget.
    ///
    /// Returns `true` if the state changed and the UI should be re-rendered.
    fn handle(&mut self, event: Event<'_>) -> bool;
}

type AppFactory = Box<dyn Fn() -> Box<dyn App> + Send + Sync>;

static APP_FACTORY: OnceLock<AppFactory> = OnceLock::new();

/// Registers the global application factory.
///
/// Platform root views invoke this factory when creating their Rust runtime state.
/// If no factory is registered, platform views fall back to the built-in gallery demo.
pub fn set_app_factory<F>(factory: F)
where
    F: Fn() -> Box<dyn App> + Send + Sync + 'static,
{
    let _ = APP_FACTORY.set(Box::new(factory));
}

/// Creates a new application instance.
///
/// Uses the registered factory if one exists, otherwise instantiates
/// [`crate::gallery::GalleryApp`].
pub fn create_app() -> Box<dyn App> {
    if let Some(factory) = APP_FACTORY.get() {
        factory()
    } else {
        Box::new(crate::gallery::GalleryApp::default())
    }
}
