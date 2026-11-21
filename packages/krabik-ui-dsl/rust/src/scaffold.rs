//! The layout of one screen: an app bar, a body and the native pieces around
//! them.
//!
//! [`App::screen`](crate::app::App::screen) returns a [`Scaffold`]. The body
//! is ordinary Rust-rendered content; everything else is native chrome that
//! belongs to the host rather than to the widget tree: the app bar, the
//! bottom navigation bar, a floating action button, a navigation drawer and
//! a snackbar. Each of them reports back with one of the application's
//! messages.
//!
//! The drawer and the snackbar are described by state, like a
//! [`Dialog`](crate::presentation::Dialog): the scaffold contains them while
//! they should be visible, and the platform's own dismissal (a swipe, a
//! timeout) is reported as a message so the application can update its
//! state.

use std::marker::PhantomData;

use crate::element::{Element, RawElement};
use crate::message::Handler;
use crate::ui::Icon;
use crate::widgets::Column;

pub use crate::navigation::{BottomNavItem, BottomNavigation};

/// One screen: a body and the native chrome around it.
pub struct Scaffold<M = ()> {
    /// The native bar at the top, if the screen has one.
    pub app_bar: Option<AppBar<M>>,
    /// The content of the screen.
    pub body: Element<M>,
    /// The native bottom navigation bar, if any.
    pub bottom_navigation: Option<BottomNavigation<M>>,
    /// The floating action button, if any.
    pub floating_action_button: Option<Fab<M>>,
    /// The navigation drawer, if any.
    pub drawer: Option<Drawer<M>>,
    /// The snackbar being shown, if any.
    pub snackbar: Option<Snackbar<M>>,
}

impl<M> Scaffold<M> {
    /// Creates a screen with an empty body and no chrome.
    pub fn new() -> Self {
        Self {
            app_bar: None,
            body: Column::new().into(),
            bottom_navigation: None,
            floating_action_button: None,
            drawer: None,
            snackbar: None,
        }
    }

    /// Sets the content of the screen.
    pub fn body(mut self, body: impl Into<Element<M>>) -> Self {
        self.body = body.into();
        self
    }

    /// Shows the native app bar: the toolbar on Android, the navigation bar
    /// on iOS. Without one the screen has no bar.
    pub fn app_bar(mut self, app_bar: impl Into<Option<AppBar<M>>>) -> Self {
        self.app_bar = app_bar.into();
        self
    }

    /// Shows the native bottom navigation bar under the body.
    ///
    /// Give every top-level destination the same bar so it stays in place
    /// while the content changes.
    pub fn bottom_navigation(mut self, bar: impl Into<Option<BottomNavigation<M>>>) -> Self {
        self.bottom_navigation = bar.into();
        self
    }

    /// Shows a floating action button over the bottom corner of the body.
    pub fn floating_action_button(mut self, button: impl Into<Option<Fab<M>>>) -> Self {
        self.floating_action_button = button.into();
        self
    }

    /// Gives the screen a navigation drawer, opened from the app bar's menu
    /// button or by swiping from the leading edge.
    pub fn drawer(mut self, drawer: impl Into<Option<Drawer<M>>>) -> Self {
        self.drawer = drawer.into();
        self
    }

    /// Shows a brief message at the bottom of the screen while it is `Some`.
    pub fn snackbar(mut self, snackbar: impl Into<Option<Snackbar<M>>>) -> Self {
        self.snackbar = snackbar.into();
        self
    }

    /// Splits the screen into what the widget tree renders and what the
    /// native controller shows. The handlers inside are already erased.
    pub(crate) fn erase(self) -> (RawElement, Chrome) {
        let chrome = Chrome {
            app_bar: self.app_bar.map(AppBar::erase),
            bottom_navigation: self.bottom_navigation.map(BottomNavigation::erase),
            floating_action_button: self.floating_action_button.map(Fab::erase),
            drawer: self.drawer.map(Drawer::erase),
            snackbar: self.snackbar.map(Snackbar::erase),
        };
        (self.body.into_raw(), chrome)
    }
}

impl<M> Default for Scaffold<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Content on its own is a screen with no chrome.
impl<M> From<Element<M>> for Scaffold<M> {
    fn from(body: Element<M>) -> Self {
        Self::new().body(body)
    }
}

/// The native chrome of a screen, with the message type erased.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Chrome {
    pub app_bar: Option<AppBar>,
    pub bottom_navigation: Option<BottomNavigation>,
    pub floating_action_button: Option<Fab>,
    pub drawer: Option<Drawer>,
    pub snackbar: Option<Snackbar>,
}

/// An action shown on the app bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppBarAction {
    /// The visible text. Always present.
    pub label: String,
    /// The optional icon.
    pub icon: Icon,
    /// What to tell the application when the item is tapped.
    pub on_tap: Handler<()>,
    /// Whether the action is a line of the bar's "more" menu and not a
    /// button on the bar.
    pub in_menu: bool,
}

/// Which of `actions` are lines of the "more" menu, as one bit each.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn menu_mask(actions: &[AppBarAction]) -> u64 {
    actions
        .iter()
        .take(64)
        .enumerate()
        .filter(|(_, action)| action.in_menu)
        .fold(0, |mask, (index, _)| mask | 1 << index)
}

/// The native bar at the top of a screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppBar<M = ()> {
    /// The title.
    pub title: String,
    /// An optional second line under the title.
    pub subtitle: Option<String>,
    /// The label of the back button, where the platform shows one.
    pub back_title: Option<String>,
    /// Trailing actions.
    pub actions: Vec<AppBarAction>,
    marker: PhantomData<fn() -> M>,
}

impl<M> AppBar<M> {
    /// Creates an app bar with a title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            back_title: None,
            actions: Vec::new(),
            marker: PhantomData,
        }
    }

    /// Sets the subtitle.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Sets the label of the back button.
    pub fn back_title(mut self, title: impl Into<String>) -> Self {
        self.back_title = Some(title.into());
        self
    }

    fn erase(self) -> AppBar {
        AppBar {
            title: self.title,
            subtitle: self.subtitle,
            back_title: self.back_title,
            actions: self.actions,
            marker: PhantomData,
        }
    }
}

impl<M: Clone + Send + Sync + 'static> AppBar<M> {
    /// Adds a text-only action that sends `message`.
    pub fn action(self, label: impl Into<String>, message: M) -> Self {
        self.action_with_icon(label, Icon::new(), message)
    }

    /// Adds an action with an icon that sends `message`.
    pub fn action_with_icon(mut self, label: impl Into<String>, icon: Icon, message: M) -> Self {
        self.actions.push(AppBarAction {
            label: label.into(),
            icon,
            on_tap: Handler::new(move |()| message.clone()),
            in_menu: false,
        });
        self
    }

    /// Adds a line to the bar's "more" menu that sends `message`.
    ///
    /// The lines are behind one button at the end of the bar: the three
    /// dots of the toolbar on Android, an ellipsis that opens a menu on iOS.
    /// Use it for the actions that do not deserve a place on the bar.
    pub fn menu_item(mut self, label: impl Into<String>, message: M) -> Self {
        self.actions.push(AppBarAction {
            label: label.into(),
            icon: Icon::new(),
            on_tap: Handler::new(move |()| message.clone()),
            in_menu: true,
        });
        self
    }
}

/// A floating action button: the screen's main action.
///
/// Android shows a Material extended floating action button. iOS has no
/// such control, so it shows a filled capsule button in the same place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fab<M = ()> {
    /// The text on the button, also its accessibility label.
    pub label: String,
    /// The optional icon shown before the label.
    pub icon: Icon,
    /// What to tell the application when the button is pressed.
    pub on_click: Handler<()>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Fab<M> {
    /// Creates a button with a label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: Icon::new(),
            on_click: Handler::none(),
            marker: PhantomData,
        }
    }

    /// Shows an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    /// Sends `message` when the button is pressed.
    pub fn on_click(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_click = Handler::new(move |()| message.clone());
        self
    }

    fn erase(self) -> Fab {
        Fab {
            label: self.label,
            icon: self.icon,
            on_click: self.on_click,
            marker: PhantomData,
        }
    }
}

/// One destination in a [`Drawer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawerItem {
    /// The text label.
    pub label: String,
    /// The icon.
    pub icon: Icon,
}

impl DrawerItem {
    /// Creates an item with a label and no icon.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: Icon::new(),
        }
    }

    /// Shows an icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
}

/// A navigation drawer: a panel of destinations that slides in from the
/// leading edge.
///
/// Android shows a Material navigation drawer. iOS has no such control, so
/// it shows a side panel with the same behaviour.
///
/// The application owns whether the drawer is open: set [`Drawer::open`]
/// from state, and update that state in the `on_open` and `on_close`
/// messages, which are sent when the user opens or closes it natively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawer<M = ()> {
    /// The destinations in display order.
    pub items: Vec<DrawerItem>,
    /// The index of the highlighted destination, if any.
    pub selected: Option<usize>,
    /// Whether the drawer is showing.
    pub open: bool,
    /// Reported with the index of the destination the user picked.
    pub on_select: Handler<usize>,
    /// Reported when the user opens the drawer.
    pub on_open: Handler<()>,
    /// Reported when the user closes the drawer.
    pub on_close: Handler<()>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Drawer<M> {
    /// Creates a closed drawer with the given destinations.
    pub fn new(items: impl IntoIterator<Item = DrawerItem>) -> Self {
        Self {
            items: items.into_iter().collect(),
            selected: None,
            open: false,
            on_select: Handler::none(),
            on_open: Handler::none(),
            on_close: Handler::none(),
            marker: PhantomData,
        }
    }

    /// Highlights a destination.
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self
    }

    /// Sets whether the drawer is showing.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
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

    /// Sends `message` when the user opens the drawer.
    pub fn on_open(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_open = Handler::new(move |()| message.clone());
        self
    }

    /// Sends `message` when the user closes the drawer.
    pub fn on_close(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_close = Handler::new(move |()| message.clone());
        self
    }

    fn erase(self) -> Drawer {
        Drawer {
            items: self.items,
            selected: self.selected,
            open: self.open,
            on_select: self.on_select,
            on_open: self.on_open,
            on_close: self.on_close,
            marker: PhantomData,
        }
    }
}

/// A brief message at the bottom of the screen, with an optional action.
///
/// Android shows a Material snackbar. iOS has no such control, so it shows
/// a small banner with the same behaviour. Both dismiss it after a few
/// seconds and report that through `on_dismiss`; the application then stops
/// returning the snackbar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snackbar<M = ()> {
    /// The message.
    pub message: String,
    /// The label of the action button, if there is one.
    pub action: Option<String>,
    /// Tells two snackbars with the same text apart; see [`Snackbar::id`].
    pub id: u64,
    /// What to tell the application when the action is pressed.
    pub on_action: Handler<()>,
    /// What to tell the application when the snackbar goes away by itself or
    /// is swiped away.
    pub on_dismiss: Handler<()>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Snackbar<M> {
    /// Creates a snackbar with a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            action: None,
            id: 0,
            on_action: Handler::none(),
            on_dismiss: Handler::none(),
            marker: PhantomData,
        }
    }

    /// Adds an action button that sends `message`.
    pub fn action(mut self, label: impl Into<String>, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.action = Some(label.into());
        self.on_action = Handler::new(move |()| message.clone());
        self
    }

    /// Identifies this snackbar among ones with the same text.
    ///
    /// A snackbar is shown when it differs from the one that is up. Two
    /// with the same message and action are the same snackbar, so "Saved"
    /// returned again while "Saved" is showing does nothing. Give each
    /// occasion its own id (a counter in the state) and the second one is
    /// shown anew, with its full time on screen.
    pub fn id(mut self, id: u64) -> Self {
        self.id = id;
        self
    }

    /// Sends `message` when the snackbar times out or is swiped away.
    pub fn on_dismiss(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.on_dismiss = Handler::new(move |()| message.clone());
        self
    }

    fn erase(self) -> Snackbar {
        Snackbar {
            message: self.message,
            action: self.action,
            id: self.id,
            on_action: self.on_action,
            on_dismiss: self.on_dismiss,
            marker: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn menu_items_are_actions_marked_for_the_more_menu() {
        let bar = super::AppBar::new("Notes")
            .action("Refresh", 1)
            .menu_item("Settings", 2)
            .action("Save", 3)
            .menu_item("Help", 4);
        let in_menu: Vec<bool> = bar.actions.iter().map(|action| action.in_menu).collect();
        assert_eq!(in_menu, [false, true, false, true]);
        // One bit per action, in order, so an index means the same action
        // on the bar, in the menu and in the application.
        assert_eq!(super::menu_mask(&bar.actions), 0b1010);
    }

    use super::*;
    use crate::widgets::Text;

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Msg {
        Edit,
        Add,
        Tab(usize),
        Go(usize),
        DrawerOpened,
        DrawerClosed,
        Undo,
        SnackbarGone,
    }

    #[test]
    fn content_alone_is_a_screen_without_chrome() {
        let scaffold: Scaffold<Msg> = Element::from(Text::new("Hello")).into();
        let (body, chrome) = scaffold.erase();
        assert_eq!(body, Element::<Msg>::from(Text::new("Hello")).into_raw());
        assert_eq!(chrome, Chrome::default());
    }

    #[test]
    fn every_slot_keeps_its_configuration_and_messages() {
        let scaffold = Scaffold::new()
            .app_bar(
                AppBar::new("Inbox")
                    .subtitle("12 new")
                    .back_title("Back")
                    .action("Edit", Msg::Edit),
            )
            .body(Text::new("Body"))
            .bottom_navigation(
                BottomNavigation::new([BottomNavItem::new("Home", Icon::new())])
                    .on_select(Msg::Tab),
            )
            .floating_action_button(
                Fab::new("Add")
                    .icon(Icon::new().sf("plus"))
                    .on_click(Msg::Add),
            )
            .drawer(
                Drawer::new([DrawerItem::new("Inbox"), DrawerItem::new("Archive")])
                    .selected(1)
                    .open(true)
                    .on_select(Msg::Go)
                    .on_open(Msg::DrawerOpened)
                    .on_close(Msg::DrawerClosed),
            )
            .snackbar(
                Snackbar::new("Deleted")
                    .action("Undo", Msg::Undo)
                    .on_dismiss(Msg::SnackbarGone),
            );
        let (_, chrome) = scaffold.erase();

        let message = |message: Option<crate::message::Message>| {
            *message
                .expect("handler")
                .downcast::<Msg>()
                .expect("the application's message type")
        };
        let app_bar = chrome.app_bar.expect("app bar");
        assert_eq!(app_bar.title, "Inbox");
        assert_eq!(app_bar.subtitle.as_deref(), Some("12 new"));
        assert_eq!(message(app_bar.actions[0].on_tap.message(())), Msg::Edit);
        let bar = chrome.bottom_navigation.expect("bottom navigation");
        assert_eq!(message(bar.on_select.message(0)), Msg::Tab(0));
        let fab = chrome.floating_action_button.expect("fab");
        assert_eq!(fab.icon.ios_name(), Some("plus"));
        assert_eq!(message(fab.on_click.message(())), Msg::Add);
        let drawer = chrome.drawer.expect("drawer");
        assert!(drawer.open);
        assert_eq!(drawer.selected, Some(1));
        assert_eq!(message(drawer.on_select.message(1)), Msg::Go(1));
        assert_eq!(message(drawer.on_open.message(())), Msg::DrawerOpened);
        assert_eq!(message(drawer.on_close.message(())), Msg::DrawerClosed);
        let snackbar = chrome.snackbar.expect("snackbar");
        assert_eq!(snackbar.action.as_deref(), Some("Undo"));
        assert_eq!(message(snackbar.on_action.message(())), Msg::Undo);
        assert_eq!(message(snackbar.on_dismiss.message(())), Msg::SnackbarGone);
    }

    #[test]
    fn a_slot_accepts_an_option_so_state_can_decide() {
        let none: Option<Snackbar<Msg>> = None;
        let (_, chrome) = Scaffold::<Msg>::new().snackbar(none).erase();
        assert!(chrome.snackbar.is_none());
    }
}
