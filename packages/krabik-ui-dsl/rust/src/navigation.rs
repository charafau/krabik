//! Navigation state shared between an application and the native hosts.
//!
//! The application owns the route stack; the runtime reads [`Navigation`] and
//! mirrors it with native navigation controllers. A screen's content is built
//! on demand through [`App::screen`](crate::app::App::screen).

use crate::ui::Icon;

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
    /// Extra native toolbar configuration, when the screen needs it.
    pub toolbar: Option<Toolbar>,
}

impl Screen {
    /// Creates a screen with a key and a navigation-bar title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            transition: ScreenTransition::Slide,
            toolbar: None,
        }
    }

    /// Chooses the native transition used when this screen is pushed.
    pub fn transition(mut self, transition: ScreenTransition) -> Self {
        self.transition = transition;
        self
    }

    /// Configures the native toolbar for this screen.
    pub fn toolbar(mut self, toolbar: Toolbar) -> Self {
        self.toolbar = Some(toolbar);
        self
    }
}

/// An action shown on the native toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarAction {
    /// The action dispatched when the item is tapped.
    pub action: String,
    /// The visible text. Always present.
    pub label: String,
    /// The optional icon.
    pub icon: Icon,
}

impl ToolbarAction {
    /// Creates a text-only toolbar action.
    pub fn new(action: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            label: label.into(),
            icon: Icon::new(),
        }
    }

    /// Adds an icon to this action.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
}

/// Extra configuration for the host-level native toolbar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Toolbar {
    /// An optional second line under the title.
    pub subtitle: Option<String>,
    /// An optional back-button title.
    pub back_title: Option<String>,
    /// Whether the toolbar is hidden.
    pub hidden: bool,
    /// Right-side actions.
    pub actions: Vec<ToolbarAction>,
}

impl Toolbar {
    /// Creates an empty toolbar configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the subtitle.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Sets the back-button title.
    pub fn back_title(mut self, title: impl Into<String>) -> Self {
        self.back_title = Some(title.into());
        self
    }

    /// Hides the toolbar for this screen.
    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    /// Adds a text-only action.
    pub fn action(mut self, action: impl Into<String>, label: impl Into<String>) -> Self {
        self.actions.push(ToolbarAction::new(action, label));
        self
    }

    /// Adds an action with an icon.
    pub fn action_with_icon(
        mut self,
        action: impl Into<String>,
        label: impl Into<String>,
        icon: Icon,
    ) -> Self {
        self.actions
            .push(ToolbarAction::new(action, label).icon(icon));
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
    fn screens_can_configure_a_toolbar() {
        let screen = Screen::new("profile", "Profile").toolbar(
            Toolbar::new()
                .subtitle("12 new")
                .back_title("Back")
                .hidden(false)
                .action("edit", "Edit")
                .action_with_icon("search", "Search", Icon::new().sf("magnifyingglass")),
        );
        let toolbar = screen.toolbar.expect("toolbar");
        assert_eq!(toolbar.subtitle.as_deref(), Some("12 new"));
        assert_eq!(toolbar.back_title.as_deref(), Some("Back"));
        assert_eq!(toolbar.actions.len(), 2);
        assert_eq!(toolbar.actions[0].label, "Edit");
        assert_eq!(toolbar.actions[1].icon.ios_name(), Some("magnifyingglass"));
    }

    #[test]
    fn the_transition_can_be_chosen() {
        let screen = Screen::new("fade", "Fade").transition(ScreenTransition::Fade);
        assert_eq!(screen.transition, ScreenTransition::Fade);
    }
}
