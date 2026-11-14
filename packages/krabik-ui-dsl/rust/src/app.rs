//! The application interface.
//!
//! An application is a [`Component`]: state, a message type describing
//! everything that can happen to it, an `update` that applies a message and a
//! `view` that describes the UI for the current state. Widgets carry the
//! message to send, so there is no string matching and the compiler checks
//! every event:
//!
//! ```ignore
//! #[derive(Clone)]
//! enum Msg { Increment, Name(String) }
//!
//! impl Component for Counter {
//!     type Msg = Msg;
//!
//!     fn update(&mut self, msg: Msg) {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!             Msg::Name(name) => self.name = name,
//!         }
//!     }
//!
//!     fn view(&self) -> Element<Msg> {
//!         Column::new()
//!             .child(Text::new(format!("{}", self.count)))
//!             .child(Button::new("+1").on_click(Msg::Increment))
//!             .child(TextField::new(&self.name).on_change(Msg::Name))
//!             .into()
//!     }
//! }
//! ```
//!
//! A larger application is built from smaller components: the parent keeps
//! the child in its state, shows `child.view().map(Msg::Child)` and forwards
//! `Msg::Child(msg)` to `child.update(msg)`.

use std::sync::OnceLock;

use crate::element::{Element, RawElement};
use crate::message::Message;
use crate::navigation::{Navigation, Route};
use crate::presentation::{BottomSheet, Dialog};
use crate::scaffold::{Chrome, Scaffold};

/// A piece of UI with its own state and messages.
pub trait Component {
    /// Everything that can happen to this component.
    type Msg: 'static;

    /// Applies a message to the state. The UI is rebuilt afterwards.
    fn update(&mut self, msg: Self::Msg);

    /// Describes the UI for the current state.
    fn view(&self) -> Element<Self::Msg>;
}

/// The root component of an application, plus the native chrome around it.
///
/// Every method has a default, so a single-screen app only implements
/// [`Component`].
pub trait App: Component + Send + 'static {
    /// The native navigation stack, when the app uses one.
    ///
    /// The application owns this state; the runtime mirrors it with native
    /// navigation controllers. With navigation, [`Component::view`] is not
    /// shown: the first screen is the root.
    fn navigation(&self) -> Option<Navigation<Self::Msg>> {
        None
    }

    /// The native chrome around [`Component::view`] for an app without a
    /// navigation stack: an app bar, a floating action button, a drawer, a
    /// snackbar. The scaffold's body is shown in place of `view`.
    ///
    /// Not used when [`App::navigation`] returns a stack; each screen then
    /// brings its own scaffold.
    fn scaffold(&self) -> Option<Scaffold<Self::Msg>> {
        None
    }

    /// One navigation screen, by its key: its content and the native chrome
    /// around it. Content alone converts into a screen without chrome:
    /// `Some(self.page().into())`.
    fn screen(&self, _key: &str) -> Option<Scaffold<Self::Msg>> {
        None
    }

    /// The system alert to show, if any. It stays up while this returns
    /// `Some`; a button sends its message and the application closes the
    /// alert by returning `None` afterwards.
    fn dialog(&self) -> Option<Dialog<Self::Msg>> {
        None
    }

    /// The native bottom sheet to show, if any.
    fn sheet(&self) -> Option<BottomSheet<Self::Msg>> {
        None
    }

    /// The message to send for the system back action, or `None` to let the
    /// platform handle it (which leaves the app from its root screen).
    ///
    /// An app with a navigation stack usually sets
    /// [`Navigation::on_back`] instead; this method is for other rules, such
    /// as closing an in-page panel first. When it returns `Some`, it wins.
    fn back(&self) -> Option<Self::Msg> {
        None
    }
}

/// An [`App`] with its message type erased, as the runtime holds it.
pub(crate) trait AnyApp: Send {
    fn view(&self) -> RawElement;
    fn update(&mut self, message: Message);
    fn navigation(&self) -> Option<Vec<Route>>;
    fn screen(&self, key: &str) -> Option<(RawElement, Chrome)>;
    /// The content and chrome of an app without navigation that has a
    /// scaffold.
    fn scaffold(&self) -> Option<(RawElement, Chrome)>;
    fn dialog(&self) -> Option<Dialog>;
    fn sheet(&self) -> Option<BottomSheet>;
    fn back(&self) -> Option<Message>;
}

impl<A: App> AnyApp for A {
    fn view(&self) -> RawElement {
        Component::view(self).into_raw()
    }

    fn update(&mut self, message: Message) {
        // Every handler in this app's tree was built from `A::Msg`.
        if let Ok(message) = message.downcast::<A::Msg>() {
            Component::update(self, *message);
        }
    }

    fn navigation(&self) -> Option<Vec<Route>> {
        App::navigation(self).map(|navigation| unique_routes(navigation.routes))
    }

    fn scaffold(&self) -> Option<(RawElement, Chrome)> {
        App::scaffold(self).map(Scaffold::erase)
    }

    fn screen(&self, key: &str) -> Option<(RawElement, Chrome)> {
        App::screen(self, key).map(Scaffold::erase)
    }

    fn dialog(&self) -> Option<Dialog> {
        App::dialog(self).map(Dialog::erase)
    }

    fn sheet(&self) -> Option<BottomSheet> {
        App::sheet(self).map(BottomSheet::erase)
    }

    fn back(&self) -> Option<Message> {
        if let Some(message) = App::back(self) {
            return Some(Box::new(message));
        }
        let navigation = App::navigation(self)?;
        if navigation.routes.len() > 1 {
            navigation.on_back.message(())
        } else {
            None
        }
    }
}

/// Keeps the first route of every key.
///
/// A screen's native host is found by its key, so a key that appears twice
/// would make one pop drop both screens. That is a mistake in the
/// application: debug builds stop there, release builds report it and show
/// the stack without the repeated route.
fn unique_routes(mut routes: Vec<Route>) -> Vec<Route> {
    let mut seen = std::collections::HashSet::new();
    let before = routes.len();
    routes.retain(|route| seen.insert(route.key.clone()));
    if routes.len() != before {
        debug_assert!(
            false,
            "a navigation stack contains the same route key twice; keys must be unique"
        );
        crate::log::error(
            "a navigation stack contains the same route key twice; ignoring the repeat",
        );
    }
    routes
}

type AppFactory = Box<dyn Fn() -> Box<dyn AnyApp> + Send + Sync>;

static APP_FACTORY: OnceLock<AppFactory> = OnceLock::new();

/// Registers the application. Call once at startup, before any native root
/// view is created; each activity or scene then gets its own instance.
pub fn set_app_factory<A: App>(factory: impl Fn() -> A + Send + Sync + 'static) {
    let _ = APP_FACTORY.set(Box::new(move || Box::new(factory())));
}

/// Creates a new instance of the registered application.
///
/// # Panics
///
/// Panics when no application was registered: the host must call the app's
/// install function (which calls [`set_app_factory`]) before it creates the
/// first root view.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn create_app() -> Box<dyn AnyApp> {
    let factory = APP_FACTORY.get().expect(
        "krabik: no application registered; call set_app_factory before creating a root view",
    );
    factory()
}
