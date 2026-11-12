//! Application interface.
//!
//! Applications describe their UI and respond to user interactions by
//! implementing the [`App`] trait. The platform renderers query the current
//! UI tree through [`App::view`] and forward native events through
//! [`App::handle`].

use std::sync::OnceLock;

use crate::ui::Element;

/// A user interaction dispatched by a native widget.
///
/// Actions are borrowed for the duration of the [`App::handle`] call. They may
/// be built dynamically by the application, such as
/// `format!("note:delete:{index}")` for a row in a list.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event<'a> {
    /// A button was pressed.
    Click {
        /// The action configured on the button.
        action: &'a str,
    },
    /// The text of an input changed.
    TextChanged {
        /// The action configured on the input.
        action: &'a str,
        /// The new text of the input.
        value: &'a str,
    },
    /// The user submitted an input from the keyboard.
    Submitted {
        /// The action configured on the input.
        action: &'a str,
    },
    /// An input gained or lost focus.
    FocusChanged {
        /// The action configured on the input.
        action: &'a str,
        /// Whether the input is now focused.
        focused: bool,
    },
    /// A switch, checkbox or radio button was toggled.
    Toggled {
        /// The action configured on the control.
        action: &'a str,
        /// Whether the control is now on.
        checked: bool,
    },
    /// A slider value changed.
    ValueChanged {
        /// The action configured on the slider.
        action: &'a str,
        /// The new value.
        value: f32,
    },
    /// The selection of a radio group changed.
    Selected {
        /// The action configured on the radio group.
        action: &'a str,
        /// The index of the newly selected option.
        index: usize,
    },
    /// The date of a date picker changed.
    DateChanged {
        /// The action configured on the date picker.
        action: &'a str,
        /// The newly selected date.
        date: crate::ui::DateValue,
    },
    /// The user asked to go back, usually from Android's system back button.
    Back,
}

impl Event<'_> {
    /// The action the event was dispatched for.
    pub fn action(&self) -> &str {
        match self {
            Self::Click { action }
            | Self::TextChanged { action, .. }
            | Self::Submitted { action }
            | Self::FocusChanged { action, .. }
            | Self::Toggled { action, .. }
            | Self::ValueChanged { action, .. }
            | Self::Selected { action, .. }
            | Self::DateChanged { action, .. } => action,
            Self::Back => "",
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

    /// The native navigation stack, when the app uses one.
    ///
    /// The application owns this state; the runtime mirrors it with native
    /// navigation controllers. Defaults to no navigation.
    fn navigation(&self) -> Option<crate::navigation::Navigation> {
        None
    }

    /// Builds the content of one navigation screen.
    ///
    /// Called once per screen host. The default returns `None`, which is only
    /// valid when [`App::navigation`] is also `None`.
    fn screen(&self, _key: &str) -> Option<Element> {
        None
    }

    /// The native dialog or bottom sheet currently presented, if any.
    fn presentation(&self) -> Option<crate::presentation::Presentation> {
        None
    }

    /// Builds the content of the currently presented modal.
    fn presentation_view(&self) -> Option<Element> {
        None
    }
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
