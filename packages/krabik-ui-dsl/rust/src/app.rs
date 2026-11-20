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
//!     fn update(&mut self, msg: Msg) -> Command<Msg> {
//!         match msg {
//!             Msg::Increment => self.count += 1,
//!             Msg::Name(name) => self.name = name,
//!         }
//!         Command::none()
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
//! `Msg::Child(msg)` to `child.update(msg).map(Msg::Child)`.
//!
//! `update` returns a [`Command`]: work for the runtime to do off the UI
//! thread, whose result comes back as another message. Most messages return
//! [`Command::none`]. See [`crate::command`].

use std::sync::{Arc, OnceLock};

use crate::command::{Command, Jobs, Mailbox, Subscription};
use crate::element::{Element, RawElement};
use crate::message::Message;
use crate::navigation::{Navigation, Route};
use crate::presentation::{BottomSheet, Dialog};
use crate::scaffold::{Chrome, Scaffold};
use crate::style::{Appearance, ColorScheme};

/// What the device tells the application about itself: the user's language
/// and whether the screen is light or dark.
///
/// The application hears about it through [`App::environment_changed`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    /// The user's preferred language as a BCP 47 tag, such as `en-US` or
    /// `pl-PL`. Empty when the platform did not report one.
    pub locale: String,
    /// The appearance colours are resolved for.
    pub color_scheme: ColorScheme,
}

impl Environment {
    /// The language part of [`Environment::locale`], in lower case: `pl`
    /// for `pl-PL`.
    pub fn language(&self) -> String {
        self.locale
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase()
    }
}

/// A piece of UI with its own state and messages.
pub trait Component {
    /// Everything that can happen to this component.
    type Msg: 'static;

    /// Applies a message to the state. The UI is rebuilt afterwards.
    ///
    /// Returns the work this message starts, or [`Command::none`]. It must
    /// not block: it runs on the UI thread.
    fn update(&mut self, msg: Self::Msg) -> Command<Self::Msg>;

    /// Describes the UI for the current state.
    fn view(&self) -> Element<Self::Msg>;
}

/// The root component of an application, plus the native chrome around it.
///
/// Every method has a default, so a single-screen app only implements
/// [`Component`].
pub trait App: Component + Send + 'static {
    /// The work to start when the application starts, such as loading what
    /// the first screen shows. Its result arrives as a message, like the
    /// result of a command returned from [`Component::update`].
    ///
    /// Called once for every instance of the application, before its first
    /// render. An Android activity that is recreated (a rotation, a theme
    /// change) keeps its instance, so this does not run again.
    fn init(&mut self) -> Command<Self::Msg> {
        Command::none()
    }

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

    /// The message to send because the device's language or appearance is
    /// `environment`, or `None` when the application does not follow them.
    ///
    /// Called before the first render and again whenever a value changes
    /// while the application runs, so an application that keeps its strings
    /// in Rust can switch language without a restart:
    ///
    /// ```ignore
    /// fn environment_changed(&self, environment: &Environment) -> Option<Msg> {
    ///     Some(Msg::Language(environment.language()))
    /// }
    /// ```
    fn environment_changed(&self, _environment: &Environment) -> Option<Self::Msg> {
        None
    }

    /// The appearance the application shows: the device's, or light or
    /// dark whatever the device shows. This is how a System / Light / Dark
    /// setting is implemented; the application keeps the choice in its
    /// state and returns it here.
    ///
    /// The native chrome follows too. The colour scheme that results is
    /// reported through [`App::environment_changed`].
    fn appearance(&self) -> Appearance {
        Appearance::System
    }

    /// What should keep sending messages in the current state, such as a
    /// clock. Compared by key after every message; see [`Subscription`].
    fn subscriptions(&self) -> Vec<Subscription<Self::Msg>> {
        Vec::new()
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
    fn init(&mut self) -> Jobs;
    fn environment_changed(&self, environment: &Environment) -> Option<Message>;
    fn view(&self) -> RawElement;
    fn update(&mut self, message: Message) -> Jobs;
    fn appearance(&self) -> Appearance;
    fn subscriptions(&self) -> Vec<Subscription<()>>;
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
    fn init(&mut self) -> Jobs {
        App::init(self).into_jobs()
    }

    fn view(&self) -> RawElement {
        Component::view(self).into_raw()
    }

    fn update(&mut self, message: Message) -> Jobs {
        // Every handler in this app's tree was built from `A::Msg`.
        match message.downcast::<A::Msg>() {
            Ok(message) => Component::update(self, *message).into_jobs(),
            Err(_) => Jobs::none(),
        }
    }

    fn environment_changed(&self, environment: &Environment) -> Option<Message> {
        App::environment_changed(self, environment).map(|message| Box::new(message) as Message)
    }

    fn appearance(&self) -> Appearance {
        App::appearance(self)
    }

    fn subscriptions(&self) -> Vec<Subscription<()>> {
        App::subscriptions(self)
            .into_iter()
            .map(Subscription::erase)
            .collect()
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

/// An application and the mailbox its background work delivers to.
///
/// The two stay together when an activity is recreated: the session ends,
/// the application and its mailbox are kept, and the next session continues
/// with both.
pub(crate) struct Running {
    pub(crate) app: Box<dyn AnyApp>,
    pub(crate) mailbox: Arc<Mailbox>,
    /// The work [`App::init`] asked for, until the first session starts it.
    /// Empty for an application that already had a session.
    pub(crate) init: Jobs,
    /// The environment the application was last told about, so a session
    /// that continues with it only reports what changed in between.
    pub(crate) told: Option<Environment>,
}

impl Running {
    /// A new instance of an application, which has not started yet.
    pub(crate) fn new(mut app: Box<dyn AnyApp>) -> Self {
        let init = app.init();
        Self {
            app,
            mailbox: Arc::new(Mailbox::default()),
            init,
            told: None,
        }
    }
}

/// Creates a new instance of the registered application.
///
/// # Panics
///
/// Panics when no application was registered: the host must call the app's
/// install function (which calls [`set_app_factory`]) before it creates the
/// first root view.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn create_app() -> Running {
    let factory = APP_FACTORY.get().expect(
        "krabik: no application registered; call set_app_factory before creating a root view",
    );
    Running::new(factory())
}
