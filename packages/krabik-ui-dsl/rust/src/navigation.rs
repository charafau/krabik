//! Navigation state shared between an application and the native hosts.
//!
//! The application owns the route stack; the runtime reads [`Navigation`] and
//! mirrors it with native navigation controllers. A screen's content is built
//! on demand through [`App::screen`](crate::app::App::screen).

use std::marker::PhantomData;

use crate::message::Handler;
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
pub struct Screen<M = ()> {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The title shown by the native navigation bar.
    pub title: String,
    /// The native transition used when this screen is pushed.
    pub transition: ScreenTransition,
    /// Extra native toolbar configuration, when the screen needs it.
    pub toolbar: Option<Toolbar<M>>,
    /// The native bottom navigation bar shown under this screen, if any.
    pub bottom_navigation: Option<BottomNavigation<M>>,
}

impl<M> Screen<M> {
    /// Creates a screen with a key and a navigation-bar title.
    pub fn new(key: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            transition: ScreenTransition::Slide,
            toolbar: None,
            bottom_navigation: None,
        }
    }

    /// Chooses the native transition used when this screen is pushed.
    pub fn transition(mut self, transition: ScreenTransition) -> Self {
        self.transition = transition;
        self
    }

    /// Configures the native toolbar for this screen.
    pub fn toolbar(mut self, toolbar: Toolbar<M>) -> Self {
        self.toolbar = Some(toolbar);
        self
    }

    /// Shows a native bottom navigation bar under this screen.
    ///
    /// Give every top-level destination the same bar so it stays in place
    /// while the content changes.
    pub fn bottom_navigation(mut self, bar: BottomNavigation<M>) -> Self {
        self.bottom_navigation = Some(bar);
        self
    }

    /// Drops the message type; the handlers inside are already erased.
    pub(crate) fn erase(self) -> Screen {
        Screen {
            key: self.key,
            title: self.title,
            transition: self.transition,
            toolbar: self.toolbar.map(Toolbar::erase),
            bottom_navigation: self.bottom_navigation.map(BottomNavigation::erase),
        }
    }
}

/// One destination in a [`BottomNavigation`] bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomNavItem {
    /// The text label.
    pub label: String,
    /// The icon.
    pub icon: Icon,
}

impl BottomNavItem {
    /// Creates an item with a label and an icon.
    pub fn new(label: impl Into<String>, icon: Icon) -> Self {
        Self {
            label: label.into(),
            icon,
        }
    }
}

/// The host-level native bottom navigation bar.
///
/// iOS shows a system tab bar and Android a Material bottom navigation bar,
/// each at its platform height and pinned to the bottom edge of the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomNavigation<M = ()> {
    /// The destinations in display order.
    pub items: Vec<BottomNavItem>,
    /// The index of the selected destination.
    pub selected: usize,
    /// Reported with the index of the destination the user picked.
    pub on_select: Handler<usize>,
    marker: PhantomData<fn() -> M>,
}

impl<M> BottomNavigation<M> {
    /// Creates a bar with the given destinations.
    pub fn new(items: impl IntoIterator<Item = BottomNavItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
            selected: 0,
            on_select: Handler::none(),
            marker: PhantomData,
        }
    }

    /// Sets the selected destination.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Reports the index of the destination the user picked.
    pub fn on_select(mut self, message: impl Fn(usize) -> M + Send + Sync + 'static) -> Self
    where
        M: 'static,
    {
        self.on_select = Handler::new(message);
        self
    }

    fn erase(self) -> BottomNavigation {
        BottomNavigation {
            items: self.items,
            selected: self.selected,
            on_select: self.on_select,
            marker: PhantomData,
        }
    }
}

/// An action shown on the native toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarAction {
    /// The visible text. Always present.
    pub label: String,
    /// The optional icon.
    pub icon: Icon,
    /// What to tell the application when the item is tapped.
    pub on_tap: Handler<()>,
}

/// Extra configuration for the host-level native toolbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Toolbar<M = ()> {
    /// An optional second line under the title.
    pub subtitle: Option<String>,
    /// An optional back-button title.
    pub back_title: Option<String>,
    /// Whether the toolbar is hidden.
    pub hidden: bool,
    /// Right-side actions.
    pub actions: Vec<ToolbarAction>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Default for Toolbar<M> {
    fn default() -> Self {
        Self {
            subtitle: None,
            back_title: None,
            hidden: false,
            actions: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<M> Toolbar<M> {
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

    fn erase(self) -> Toolbar {
        Toolbar {
            subtitle: self.subtitle,
            back_title: self.back_title,
            hidden: self.hidden,
            actions: self.actions,
            marker: PhantomData,
        }
    }
}

impl<M: Clone + Send + Sync + 'static> Toolbar<M> {
    /// Adds a text-only action that sends `message`.
    pub fn action(self, label: impl Into<String>, message: M) -> Self {
        self.action_with_icon(label, Icon::new(), message)
    }

    /// Adds an action with an icon that sends `message`.
    pub fn action_with_icon(mut self, label: impl Into<String>, icon: Icon, message: M) -> Self {
        self.actions.push(ToolbarAction {
            label: label.into(),
            icon,
            on_tap: Handler::new(move |()| message.clone()),
        });
        self
    }
}

/// The visible navigation stack, ordered from the root to the active screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Navigation<M = ()> {
    /// The screens currently in the stack.
    pub screens: Vec<Screen<M>>,
}

impl<M> Navigation<M> {
    /// Creates a navigation stack from its screens.
    pub fn new(screens: Vec<Screen<M>>) -> Self {
        Self { screens }
    }

    /// The screens in the stack.
    pub fn screens(&self) -> &[Screen<M>] {
        &self.screens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Msg {
        Edit,
        Search,
        Tab(usize),
    }

    #[test]
    fn screens_default_to_a_slide_transition() {
        let screen = Screen::<Msg>::new("home", "Home");
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
                .action("Edit", Msg::Edit)
                .action_with_icon("Search", Icon::new().sf("magnifyingglass"), Msg::Search),
        );
        let toolbar = screen.toolbar.expect("toolbar");
        assert_eq!(toolbar.subtitle.as_deref(), Some("12 new"));
        assert_eq!(toolbar.back_title.as_deref(), Some("Back"));
        assert_eq!(toolbar.actions.len(), 2);
        assert_eq!(toolbar.actions[0].label, "Edit");
        assert_eq!(toolbar.actions[1].icon.ios_name(), Some("magnifyingglass"));
        let message = toolbar.actions[1].on_tap.message(()).expect("handler");
        assert_eq!(message.downcast_ref::<Msg>(), Some(&Msg::Search));
    }

    #[test]
    fn screens_can_show_bottom_navigation() {
        let screen = Screen::new("home", "Home").bottom_navigation(
            BottomNavigation::new([
                BottomNavItem::new("Home", Icon::new().drawable("ic_home").sf("house.fill")),
                BottomNavItem::new("Search", Icon::new().shared("search")),
            ])
            .selected(1)
            .on_select(Msg::Tab),
        );
        let bar = screen.bottom_navigation.expect("bottom navigation");
        assert_eq!(bar.selected, 1);
        assert_eq!(bar.items[0].icon.android_name(), Some("ic_home"));
        assert_eq!(bar.items[1].icon.ios_name(), Some("search"));
        let message = bar.on_select.message(1).expect("handler");
        assert_eq!(message.downcast_ref::<Msg>(), Some(&Msg::Tab(1)));
    }

    #[test]
    fn the_transition_can_be_chosen() {
        let screen = Screen::<Msg>::new("fade", "Fade").transition(ScreenTransition::Fade);
        assert_eq!(screen.transition, ScreenTransition::Fade);
    }
}
