//! One running application and its native hosts.

use std::collections::HashMap;

use crate::app::AnyApp;
use crate::engine::tree::{self, NodeId};
use crate::message::Message;
use crate::navigation::Screen;
use crate::presentation::Dialog;

use super::{Actions, Controller, Host, HostKind, NodeEvent, Platform};

/// The identifier of the main host, which exists for the whole session.
pub(crate) const MAIN_HOST: u64 = 0;

/// One application instance shared by every native host of an activity or
/// scene.
pub(crate) struct Session<P: Platform> {
    app: Box<dyn AnyApp>,
    hosts: HashMap<u64, P::Host>,
    /// Hosts the controller was asked to create but has not attached yet.
    pending: HashMap<u64, HostKind>,
    next_host: u64,
    controller: Option<P::Controller>,
    /// The navigation stack last mirrored to the controller.
    navigation: Vec<Screen>,
    /// The alert the controller is showing.
    dialog: Option<Dialog>,
    /// Whether the controller was last told that back is handled here.
    back_enabled: bool,
}

impl<P: Platform> Session<P> {
    /// Creates a session around the main host.
    pub(crate) fn new(app: Box<dyn AnyApp>, main: P::Host) -> Self {
        let mut hosts = HashMap::new();
        hosts.insert(MAIN_HOST, main);
        Self {
            app,
            hosts,
            pending: HashMap::new(),
            next_host: MAIN_HOST + 1,
            controller: None,
            navigation: Vec::new(),
            dialog: None,
            back_enabled: false,
        }
    }

    /// Registers the native controller. Until then only the main host renders.
    pub(crate) fn set_controller(&mut self, controller: P::Controller) {
        self.controller = Some(controller);
    }

    /// One host, for platform code that needs its native state.
    pub(crate) fn host(&self, host: u64) -> Option<&P::Host> {
        self.hosts.get(&host)
    }

    /// One host, for platform code that needs its native state.
    pub(crate) fn host_mut(&mut self, host: u64) -> Option<&mut P::Host> {
        self.hosts.get_mut(&host)
    }

    /// Every host, for teardown.
    pub(crate) fn hosts_mut(&mut self) -> impl Iterator<Item = &mut P::Host> {
        self.hosts.values_mut()
    }

    /// Adopts a host view the controller created.
    ///
    /// `build` wraps the native view. A view for a host that is neither
    /// pending nor known is ignored: its screen was already popped.
    pub(crate) fn attach_host(
        &mut self,
        host: u64,
        build: impl FnOnce(HostKind) -> Result<P::Host, P::Error>,
    ) -> Result<(), P::Error> {
        // A platform may recreate a host's view; keep the id and kind and
        // point the host at the new view.
        let kind = match self.hosts.get(&host) {
            Some(existing) => existing.kind().clone(),
            None => match self.pending.remove(&host) {
                Some(kind) => kind,
                None => return Ok(()),
            },
        };
        self.hosts.insert(host, build(kind)?);
        Ok(())
    }

    /// Forgets a host whose native view was destroyed.
    pub(crate) fn remove_host(&mut self, host: u64) {
        self.hosts.remove(&host);
        self.pending.remove(&host);
    }

    /// Rebuilds every host from the application and mirrors the native chrome.
    pub(crate) fn render_all(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        let has_navigation = self.app.navigation().is_some();
        for host in self.hosts.values_mut() {
            let element = match host.kind() {
                // A navigation app shows its root as the first screen.
                HostKind::Main if has_navigation => None,
                HostKind::Main => Some(self.app.view()),
                HostKind::Screen(key) => self.app.screen(key),
                HostKind::Sheet => self.app.sheet().map(|sheet| sheet.into_content()),
            };
            if let Some(element) = element {
                host.render(cx, &element)?;
            }
        }
        self.sync(cx)
    }

    /// Reconciles the application's navigation, dialog and sheet with the
    /// native controller.
    fn sync(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        if self.controller.is_none() {
            return Ok(());
        }
        let navigation = self.app.navigation().unwrap_or_default();
        if navigation != self.navigation {
            self.sync_navigation(cx, &navigation)?;
            self.navigation = navigation;
        }
        let dialog = self.app.dialog();
        if dialog != self.dialog {
            self.sync_dialog(cx, dialog.as_ref())?;
            self.dialog = dialog;
        }
        let sheet = self.app.sheet().is_some();
        if sheet != self.has_sheet() {
            self.sync_sheet(cx, sheet)?;
        }
        let back_enabled = self.app.back().is_some();
        if back_enabled != self.back_enabled {
            if let Some(controller) = self.controller.as_ref() {
                controller.set_back_enabled(cx, back_enabled)?;
            }
            self.back_enabled = back_enabled;
        }
        Ok(())
    }

    fn sync_navigation(&mut self, cx: &mut P::Cx<'_>, screens: &[Screen]) -> Result<(), P::Error> {
        let Some(controller) = self.controller.as_ref() else {
            return Ok(());
        };
        let common = self
            .navigation
            .iter()
            .zip(screens)
            .take_while(|(old, new)| old.key == new.key)
            .count();

        // Leaving several screens at once is one native pop, animated with
        // the transition of the screen the user is looking at.
        let popped = &self.navigation[common..];
        if let Some(top) = popped.last() {
            controller.pop(cx, popped.len(), top.transition)?;
            for screen in popped {
                let kind = HostKind::Screen(screen.key.clone());
                self.hosts.retain(|_, host| *host.kind() != kind);
                self.pending.retain(|_, pending| *pending != kind);
            }
        }
        for screen in &screens[common..] {
            let host = self.next_host;
            self.next_host += 1;
            self.pending
                .insert(host, HostKind::Screen(screen.key.clone()));
            controller.push(cx, host, screen)?;
        }
        controller.set_root_visible(cx, screens.is_empty())?;
        // Sent on every navigation change, so the native chrome never keeps a
        // previous screen's toolbar or bottom navigation.
        controller.set_chrome(cx, screens.last())
    }

    fn sync_dialog(&mut self, cx: &mut P::Cx<'_>, dialog: Option<&Dialog>) -> Result<(), P::Error> {
        let Some(controller) = self.controller.as_ref() else {
            return Ok(());
        };
        if self.dialog.is_some() {
            controller.dismiss_dialog(cx)?;
        }
        match dialog {
            Some(dialog) => controller.show_dialog(cx, dialog),
            None => Ok(()),
        }
    }

    fn sync_sheet(&mut self, cx: &mut P::Cx<'_>, show: bool) -> Result<(), P::Error> {
        let Some(controller) = self.controller.as_ref() else {
            return Ok(());
        };
        if show {
            let host = self.next_host;
            self.next_host += 1;
            self.pending.insert(host, HostKind::Sheet);
            controller.present_sheet(cx, host)
        } else {
            controller.dismiss_sheet(cx)?;
            self.forget_sheet();
            Ok(())
        }
    }

    /// Whether a sheet is presented or about to be.
    fn has_sheet(&self) -> bool {
        self.hosts
            .values()
            .any(|host| *host.kind() == HostKind::Sheet)
            || self.pending.values().any(|kind| *kind == HostKind::Sheet)
    }

    /// Drops the sheet's host after the sheet closed.
    fn forget_sheet(&mut self) {
        self.hosts.retain(|_, host| *host.kind() != HostKind::Sheet);
        self.pending.retain(|_, kind| *kind != HostKind::Sheet);
    }

    // ----- events -----------------------------------------------------

    /// Handles a native event on the widget `id` of host `host`.
    pub(crate) fn node_event(
        &mut self,
        cx: &mut P::Cx<'_>,
        host: u64,
        id: NodeId,
        event: NodeEvent,
    ) -> Result<(), P::Error> {
        let Some(state) = self.hosts.get_mut(&host) else {
            return Ok(());
        };

        // Engine state that changes with the event itself.
        let mut engine_changed = false;
        match &event {
            NodeEvent::Scroll(offset) => {
                engine_changed = state.runtime_mut().scroll(id, *offset);
            }
            NodeEvent::Focus { focused, attached } => {
                engine_changed = track_focus::<P>(state, id, *focused, *attached);
            }
            _ => {}
        }

        let message = actions_for::<P>(state, id).and_then(|actions| actions.message(&event));
        match message {
            Some(message) => self.update(cx, message),
            None if engine_changed => self.render_all(cx),
            None => Ok(()),
        }
    }

    /// Applies a message to the application and shows the result.
    fn update(&mut self, cx: &mut P::Cx<'_>, message: Message) -> Result<(), P::Error> {
        self.app.update(message);
        self.render_all(cx)
    }

    /// The system back action. Returns whether the application consumed it.
    pub(crate) fn back(&mut self, cx: &mut P::Cx<'_>) -> Result<bool, P::Error> {
        match self.app.back() {
            Some(message) => {
                self.update(cx, message)?;
                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// The toolbar item at `index` was tapped.
    pub(crate) fn toolbar_action(
        &mut self,
        cx: &mut P::Cx<'_>,
        index: usize,
    ) -> Result<(), P::Error> {
        // Handlers are read from the application's current state, never from
        // the copy last sent to the platform.
        let message = self.app.navigation().and_then(|screens| {
            let toolbar = screens.last()?.toolbar.as_ref()?;
            toolbar.actions.get(index)?.on_tap.message(())
        });
        match message {
            Some(message) => self.update(cx, message),
            None => Ok(()),
        }
    }

    /// The system alert closed: by the button at `button`, or by the platform
    /// itself (`None`), which counts as the cancel button.
    ///
    /// The platform has already closed the alert, so the session forgets it
    /// first: if the application still returns a dialog, it is shown again.
    pub(crate) fn dialog_closed(
        &mut self,
        cx: &mut P::Cx<'_>,
        button: Option<usize>,
    ) -> Result<(), P::Error> {
        self.dialog = None;
        let message = self.app.dialog().and_then(|dialog| {
            let button = match button {
                Some(index) => dialog.buttons.get(index),
                None => dialog.cancel_button(),
            };
            button?.on_tap.message(())
        });
        if let Some(message) = message {
            self.app.update(message);
        }
        self.render_all(cx)
    }

    /// The user picked the destination at `index` in the bottom navigation bar.
    pub(crate) fn bottom_navigation_selected(
        &mut self,
        cx: &mut P::Cx<'_>,
        index: usize,
    ) -> Result<(), P::Error> {
        let message = self.app.navigation().and_then(|screens| {
            let bar = screens.last()?.bottom_navigation.as_ref()?;
            bar.on_select.message(index)
        });
        match message {
            Some(message) => self.update(cx, message),
            None => Ok(()),
        }
    }

    /// The user dismissed the bottom sheet natively (swipe, back, scrim).
    ///
    /// The platform has already closed the sheet, so the session forgets it
    /// first: if the application still returns a sheet, it is presented again.
    pub(crate) fn sheet_dismissed(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        self.forget_sheet();
        let message = self
            .app
            .sheet()
            .and_then(|sheet| sheet.on_dismiss.message(()));
        if let Some(message) = message {
            self.app.update(message);
        }
        self.render_all(cx)
    }
}

/// The actions of a widget: from the host's tree, or recorded for a widget
/// inside a recycled row.
fn actions_for<P: Platform>(host: &P::Host, id: NodeId) -> Option<Actions> {
    tree::find(host.runtime().tree(), id)
        .and_then(Actions::of)
        .or_else(|| host.row_actions(id).cloned())
}

/// Records which input has focus. Returns whether the visible tree changed.
///
/// Focus inside a recycled row is bookkeeping only: the widgets on screen did
/// not change, and re-rendering would rebind the list and feed back into
/// focus handling.
fn track_focus<P: Platform>(host: &mut P::Host, id: NodeId, focused: bool, attached: bool) -> bool {
    let row = host.row_of(id);
    let runtime = host.runtime_mut();
    match row {
        Some((list, index)) => {
            if focused || attached {
                runtime.set_row_focus(list, index, id, focused);
            } else {
                // The field lost focus because its row is being torn down,
                // not because the user moved on; keep the logical focus so
                // it can be restored when the row comes back.
                runtime.park_row_focus(id);
            }
            false
        }
        None if !focused && runtime.row_focus().is_some_and(|focus| focus.field == id) => {
            // The row was unbound before the deferred blur arrived.
            runtime.park_row_focus(id);
            false
        }
        None => runtime.set_focus(id, focused),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::convert::Infallible;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::app::{App, Component};
    use crate::element::{Element, RawElement};
    use crate::engine::geometry::Size;
    use crate::engine::measure::ApproximateMeasure;
    use crate::engine::runtime::Runtime;
    use crate::navigation::{Navigation, ScreenTransition, Toolbar};
    use crate::presentation::BottomSheet;
    use crate::ui::{Button, Column, Text};

    /// Records what the session asks the platform to do.
    type Log = Rc<RefCell<Vec<String>>>;

    struct Fake;

    impl Platform for Fake {
        type Cx<'a> = ();
        type Error = Infallible;
        type Host = FakeHost;
        type Controller = FakeController;
    }

    struct FakeHost {
        kind: HostKind,
        runtime: Runtime,
        renders: usize,
    }

    impl FakeHost {
        fn new(kind: HostKind) -> Self {
            Self {
                kind,
                runtime: Runtime::new(Box::new(ApproximateMeasure)),
                renders: 0,
            }
        }
    }

    impl Host<Fake> for FakeHost {
        fn kind(&self) -> &HostKind {
            &self.kind
        }
        fn runtime(&self) -> &Runtime {
            &self.runtime
        }
        fn runtime_mut(&mut self) -> &mut Runtime {
            &mut self.runtime
        }
        fn row_actions(&self, _id: NodeId) -> Option<&Actions> {
            None
        }
        fn row_of(&self, _id: NodeId) -> Option<(NodeId, usize)> {
            None
        }
        fn render(&mut self, _cx: &mut (), element: &RawElement) -> Result<(), Infallible> {
            self.runtime.update(element, Size::new(320.0, 640.0));
            self.renders += 1;
            Ok(())
        }
    }

    struct FakeController(Log);

    impl Controller<Fake> for FakeController {
        fn push(&self, _cx: &mut (), host: u64, screen: &Screen) -> Result<(), Infallible> {
            self.0
                .borrow_mut()
                .push(format!("push {host} {}", screen.key));
            Ok(())
        }
        fn pop(
            &self,
            _cx: &mut (),
            count: usize,
            transition: ScreenTransition,
        ) -> Result<(), Infallible> {
            self.0
                .borrow_mut()
                .push(format!("pop {count} {transition:?}"));
            Ok(())
        }
        fn set_root_visible(&self, _cx: &mut (), visible: bool) -> Result<(), Infallible> {
            self.0.borrow_mut().push(format!("root {visible}"));
            Ok(())
        }
        fn set_chrome(&self, _cx: &mut (), screen: Option<&Screen>) -> Result<(), Infallible> {
            let title = screen.map_or("-", |screen| screen.title.as_str());
            self.0.borrow_mut().push(format!("chrome {title}"));
            Ok(())
        }
        fn set_back_enabled(&self, _cx: &mut (), enabled: bool) -> Result<(), Infallible> {
            self.0.borrow_mut().push(format!("back {enabled}"));
            Ok(())
        }
        fn show_dialog(&self, _cx: &mut (), dialog: &Dialog) -> Result<(), Infallible> {
            self.0.borrow_mut().push(format!("dialog {}", dialog.title));
            Ok(())
        }
        fn dismiss_dialog(&self, _cx: &mut ()) -> Result<(), Infallible> {
            self.0.borrow_mut().push("dismiss dialog".into());
            Ok(())
        }
        fn present_sheet(&self, _cx: &mut (), host: u64) -> Result<(), Infallible> {
            self.0.borrow_mut().push(format!("sheet {host}"));
            Ok(())
        }
        fn dismiss_sheet(&self, _cx: &mut ()) -> Result<(), Infallible> {
            self.0.borrow_mut().push("dismiss sheet".into());
            Ok(())
        }
    }

    /// What the test application currently wants on screen.
    #[derive(Default)]
    struct State {
        stack: Vec<&'static str>,
        dialog: bool,
        sticky_dialog: bool,
        sheet: bool,
        clicks: usize,
        subtitle: Option<&'static str>,
    }

    /// An application whose state the test can also change from outside,
    /// the way a timer or network reply would.
    #[derive(Clone, Default)]
    struct TestApp(Arc<Mutex<State>>);

    impl TestApp {
        fn new(state: State) -> Self {
            Self(Arc::new(Mutex::new(state)))
        }

        fn state(&self) -> std::sync::MutexGuard<'_, State> {
            self.0.lock().unwrap()
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Tap,
        CloseDialog,
        CloseSheet,
        Back,
    }

    impl Component for TestApp {
        type Msg = Msg;

        fn view(&self) -> Element<Msg> {
            Column::new().into()
        }

        fn update(&mut self, msg: Msg) {
            let mut state = self.state();
            match msg {
                Msg::Tap => state.clicks += 1,
                Msg::CloseDialog => state.dialog = state.sticky_dialog,
                Msg::CloseSheet => state.sheet = false,
                Msg::Back => {
                    state.stack.pop();
                }
            }
        }
    }

    impl App for TestApp {
        fn navigation(&self) -> Option<Navigation<Msg>> {
            let state = self.state();
            Some(Navigation::new(
                state
                    .stack
                    .iter()
                    .map(|key| {
                        let screen = Screen::new(*key, key.to_uppercase());
                        match (state.subtitle, state.stack.last() == Some(key)) {
                            (Some(subtitle), true) => {
                                screen.toolbar(Toolbar::new().subtitle(subtitle))
                            }
                            _ => screen,
                        }
                    })
                    .collect(),
            ))
        }

        fn screen(&self, _key: &str) -> Option<Element<Msg>> {
            Some(
                Column::new()
                    .child(Text::new(format!("clicks {}", self.state().clicks)))
                    .child(Button::new("Tap").on_click(Msg::Tap))
                    .into(),
            )
        }

        fn dialog(&self) -> Option<Dialog<Msg>> {
            self.state()
                .dialog
                .then(|| Dialog::new("Sure?").cancel("No", Msg::CloseDialog))
        }

        fn sheet(&self) -> Option<BottomSheet<Msg>> {
            self.state()
                .sheet
                .then(|| BottomSheet::new().on_dismiss(Msg::CloseSheet))
        }

        fn back(&self) -> Option<Msg> {
            (self.state().stack.len() > 1).then_some(Msg::Back)
        }
    }

    /// A session around a `TestApp`, the app itself and the controller's log.
    fn session(state: State) -> (Session<Fake>, TestApp, Log) {
        let log = Log::default();
        let app = TestApp::new(state);
        let mut session =
            Session::<Fake>::new(Box::new(app.clone()), FakeHost::new(HostKind::Main));
        session.set_controller(FakeController(log.clone()));
        (session, app, log)
    }

    /// Plays the controller's part: creates the host views it was asked for.
    fn attach_pending(session: &mut Session<Fake>) {
        let pending: Vec<u64> = session.pending.keys().copied().collect();
        for host in pending {
            session
                .attach_host(host, |kind| Ok(FakeHost::new(kind)))
                .unwrap();
        }
        session.render_all(&mut ()).unwrap();
    }

    fn take(log: &Log) -> Vec<String> {
        std::mem::take(&mut *log.borrow_mut())
    }

    #[test]
    fn the_initial_stack_is_pushed_and_given_chrome() {
        let (mut session, _app, log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["push 1 home", "root false", "chrome HOME"]);

        // Nothing changed, so a second render sends nothing.
        attach_pending(&mut session);
        assert!(take(&log).is_empty());
    }

    #[test]
    fn leaving_several_screens_is_one_pop() {
        let (mut session, app, log) = session(State {
            stack: vec!["home", "a", "b"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);

        app.state().stack.truncate(1);
        session.render_all(&mut ()).unwrap();
        // Back at the root there is nothing left to go back to, so the
        // platform is told to handle back itself again.
        assert_eq!(
            take(&log),
            ["pop 2 Slide", "root false", "chrome HOME", "back false"]
        );
        // The popped screens' hosts are gone; main and home remain.
        assert_eq!(session.hosts.len(), 2);
    }

    #[test]
    fn a_toolbar_change_resends_the_chrome_without_navigating() {
        let (mut session, app, log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);

        app.state().subtitle = Some("3 new");
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["root false", "chrome HOME"]);
    }

    #[test]
    fn back_pops_until_the_root_and_then_declines() {
        let (mut session, _app, log) = session(State {
            stack: vec!["home", "a"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);

        assert!(session.back(&mut ()).unwrap());
        assert_eq!(take(&log)[0], "pop 1 Slide");
        assert!(!session.back(&mut ()).unwrap());
        assert!(take(&log).is_empty());
    }

    #[test]
    fn a_click_reaches_the_application_and_rerenders() {
        let (mut session, _app, _log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);

        let (host, button) = session
            .hosts
            .iter()
            .find(|(_, host)| *host.kind() == HostKind::Screen("home".into()))
            .map(|(id, host)| {
                let button = tree::preorder(host.runtime().tree())
                    .into_iter()
                    .find(|node| Actions::of(node).is_some())
                    .expect("button");
                (*id, button.id)
            })
            .expect("home host");
        let renders = session.hosts[&host].renders;

        session
            .node_event(&mut (), host, button, NodeEvent::Click)
            .unwrap();
        assert_eq!(session.hosts[&host].renders, renders + 1);

        // An event the widget did not ask for changes nothing.
        session
            .node_event(&mut (), host, button, NodeEvent::Toggle(true))
            .unwrap();
        assert_eq!(session.hosts[&host].renders, renders + 1);
    }

    #[test]
    fn a_dialog_the_app_keeps_is_shown_again_after_it_closes() {
        let (mut session, app, log) = session(State {
            dialog: true,
            sticky_dialog: true,
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        assert!(take(&log).contains(&"dialog Sure?".to_string()));

        // The platform closed the alert; the app still wants it.
        session.dialog_closed(&mut (), None).unwrap();
        assert_eq!(take(&log), ["dialog Sure?"]);

        // The app lets go: nothing more to show, and nothing to dismiss
        // because the platform already closed it.
        app.state().sticky_dialog = false;
        session.dialog_closed(&mut (), None).unwrap();
        assert!(take(&log).is_empty());
    }

    #[test]
    fn a_sheet_can_be_reopened_after_a_native_dismissal() {
        let (mut session, app, log) = session(State {
            sheet: true,
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        assert!(take(&log).iter().any(|call| call.starts_with("sheet ")));

        // Swiped away: the app is told, and the native side is not asked to
        // dismiss what is already gone.
        session.sheet_dismissed(&mut ()).unwrap();
        assert!(take(&log).is_empty());
        assert!(!session.has_sheet());

        app.state().sheet = true;
        session.render_all(&mut ()).unwrap();
        assert!(take(&log).iter().any(|call| call.starts_with("sheet ")));
    }
}
