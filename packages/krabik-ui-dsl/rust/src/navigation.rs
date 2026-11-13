//! Navigation state shared between an application and the native hosts.
//!
//! The application owns the route stack; the runtime reads [`Navigation`] and
//! mirrors it with native navigation controllers. A screen's content is built
//! on demand through [`App::screen`](crate::app::App::screen).

/// How a screen enters and leaves the native navigation stack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScreenTransition {
    /// Slides in from the trailing edge and reverses when popped.
    #[default]
    Slide,
    /// Cross-fades between the two screens.
    Fade,
    /// Replaces the screen without animating.
    None,
}

impl ScreenTransition {
    /// The stable code shared with the native controllers.
    #[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
    pub(crate) const fn code(self) -> i32 {
        match self {
            Self::Slide => 0,
            Self::Fade => 1,
            Self::None => 2,
        }
    }
}

/// One screen in a navigation stack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Screen {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The title shown by the native navigation bar.
    pub title: String,
    /// The native transition used when this screen is pushed.
    pub transition: ScreenTransition,
}

impl Screen {
    /// Creates a screen with a key and a navigation-bar title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            transition: ScreenTransition::Slide,
        }
    }

    /// Chooses the native transition used when this screen is pushed.
    pub fn transition(mut self, transition: ScreenTransition) -> Self {
        self.transition = transition;
        self
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_default_to_a_slide_transition() {
        let screen = Screen::new("home", "Home");
        assert_eq!(screen.transition, ScreenTransition::Slide);
        assert_eq!(ScreenTransition::Slide.code(), 0);
        assert_eq!(ScreenTransition::Fade.code(), 1);
        assert_eq!(ScreenTransition::None.code(), 2);
    }

    #[test]
    fn the_transition_can_be_chosen() {
        let screen = Screen::new("fade", "Fade").transition(ScreenTransition::Fade);
        assert_eq!(screen.transition, ScreenTransition::Fade);
    }
}
