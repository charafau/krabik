//! Navigation state shared between an application and the native hosts.
//!
//! The application owns the route stack; the runtime reads [`Navigation`] and
//! mirrors it with native navigation controllers. A screen's content is built
//! on demand through [`App::screen`](crate::app::App::screen).

/// One screen in a navigation stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The title shown by the native navigation bar.
    pub title: String,
}

impl Screen {
    /// Creates a screen with a key and a navigation-bar title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
        }
    }
}

/// The visible navigation stack, ordered from the root to the active screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Navigation {
    /// The screens currently in the stack.
    pub screens: Vec<Screen>,
}

impl Navigation {
    /// Creates a navigation stack from its screens.
    pub fn new(screens: Vec<Screen>) -> Self {
        Self { screens }
    }

    /// The screens in the stack.
    pub fn screens(&self) -> &[Screen] {
        &self.screens
    }
}
