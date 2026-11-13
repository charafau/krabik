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
use crate::navigation::{Navigation, Screen};
use crate::presentation::{BottomSheet, Dialog};

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

    /// The content of one navigation screen, by its key.
    fn screen(&self, _key: &str) -> Option<Element<Self::Msg>> {
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
    fn back(&self) -> Option<Self::Msg> {
        None
    }
}

/// An [`App`] with its message type erased, as the runtime holds it.
pub(crate) trait AnyApp: Send {
    fn view(&self) -> RawElement;
    fn update(&mut self, message: Message);
    fn navigation(&self) -> Option<Vec<Screen>>;
    fn screen(&self, key: &str) -> Option<RawElement>;
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

    fn navigation(&self) -> Option<Vec<Screen>> {
        App::navigation(self)
            .map(|navigation| navigation.screens.into_iter().map(Screen::erase).collect())
    }

    fn screen(&self, key: &str) -> Option<RawElement> {
        App::screen(self, key).map(Element::into_raw)
    }

    fn dialog(&self) -> Option<Dialog> {
        App::dialog(self).map(Dialog::erase)
    }

    fn sheet(&self) -> Option<BottomSheet> {
        App::sheet(self).map(BottomSheet::erase)
    }

    fn back(&self) -> Option<Message> {
        App::back(self).map(|message| Box::new(message) as Message)
    }
}

type AppFactory = Box<dyn Fn() -> Box<dyn AnyApp> + Send + Sync>;

static APP_FACTORY: OnceLock<AppFactory> = OnceLock::new();

/// Registers the application. Call once at startup, before any native root
/// view is created; each activity or scene then gets its own instance.
pub fn set_app_factory<A: App>(factory: impl Fn() -> A + Send + Sync + 'static) {
    let _ = APP_FACTORY.set(Box::new(move || Box::new(factory())));
}

/// Creates a new application instance.
///
/// Uses the registered factory if one exists, otherwise the built-in gallery.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn create_app() -> Box<dyn AnyApp> {
    match APP_FACTORY.get() {
        Some(factory) => factory(),
        None => Box::new(crate::gallery::GalleryApp::default()),
    }
}
