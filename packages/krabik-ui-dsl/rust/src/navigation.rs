//! Navigation state shared between an application and the native hosts.
//!
//! The application owns the route stack; the runtime reads [`Navigation`] and
//! mirrors it with native navigation controllers. Each screen, its chrome and
//! its content, is built on demand through
//! [`App::screen`](crate::app::App::screen).

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

/// One entry of the navigation stack: which screen, and how it arrives.
///
/// The screen itself, its chrome and its content, comes from
/// [`App::screen`](crate::app::App::screen) for the route's key.
///
/// Keys must be unique within a stack: the key is how a screen's native host
/// is found again, so the same key twice would make one pop drop both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The native transition used when this screen is pushed.
    pub transition: ScreenTransition,
}

impl Route {
    /// Creates a route that slides in.
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            transition: ScreenTransition::Slide,
        }
    }

    /// Chooses the native transition used when this screen is pushed.
    pub fn transition(mut self, transition: ScreenTransition) -> Self {
        self.transition = transition;
        self
    }
}

impl From<&str> for Route {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl From<String> for Route {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<&String> for Route {
    fn from(key: &String) -> Self {
        Self::new(key.as_str())
    }
}

impl From<&&str> for Route {
    fn from(key: &&str) -> Self {
        Self::new(*key)
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

    pub(crate) fn erase(self) -> BottomNavigation {
        BottomNavigation {
            items: self.items,
            selected: self.selected,
            on_select: self.on_select,
            marker: PhantomData,
        }
    }
}

/// The visible navigation stack, ordered from the root to the active screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Navigation {
    /// The routes currently in the stack.
    pub routes: Vec<Route>,
}

impl Navigation {
    /// Creates a navigation stack from its routes. Plain keys work too:
    /// `Navigation::new(["home", "detail"])`.
    pub fn new(routes: impl IntoIterator<Item = impl Into<Route>>) -> Self {
        Self {
            routes: routes.into_iter().map(Into::into).collect(),
        }
    }

    /// The routes in the stack.
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Msg {
        Tab(usize),
    }

    #[test]
    fn routes_default_to_a_slide_transition() {
        let route = Route::new("home");
        assert_eq!(route.transition, ScreenTransition::Slide);
        assert_eq!(ScreenTransition::Slide.code(), 0);
        assert_eq!(ScreenTransition::Fade.code(), 1);
        assert_eq!(ScreenTransition::None.code(), 2);
        assert_eq!(
            Route::new("photo")
                .transition(ScreenTransition::Fade)
                .transition,
            ScreenTransition::Fade
        );
    }

    #[test]
    fn a_stack_can_be_written_as_plain_keys() {
        let stack = vec!["home".to_owned(), "detail".to_owned()];
        let navigation = Navigation::new(&stack);
        assert_eq!(
            navigation.routes(),
            [Route::new("home"), Route::new("detail")]
        );
        assert_eq!(Navigation::new(["home"]).routes().len(), 1);
    }

    #[test]
    fn bottom_navigation_reports_the_selected_index() {
        let bar = BottomNavigation::new([
            BottomNavItem::new("Home", Icon::new().drawable("ic_home").sf("house.fill")),
            BottomNavItem::new("Search", Icon::new().shared("search")),
        ])
        .selected(1)
        .on_select(Msg::Tab);
        assert_eq!(bar.selected, 1);
        assert_eq!(bar.items[0].icon.android_name(), Some("ic_home"));
        assert_eq!(bar.items[1].icon.ios_name(), Some("search"));
        let message = bar.on_select.message(1).expect("handler");
        assert_eq!(message.downcast_ref::<Msg>(), Some(&Msg::Tab(1)));
    }
}
