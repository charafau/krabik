//! One running application and its native hosts.

use std::collections::HashMap;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::app::{AnyApp, Environment, Running};
use crate::command::{self, Executor, Jobs, Mailbox, Threads};
use crate::engine::tree::{self, NodeId};
use crate::message::Message;
use crate::navigation::{Route, ScreenTransition};
use crate::presentation::Dialog;
use crate::scaffold::{Chrome, Snackbar};
use crate::style::{Appearance, ColorScheme};

use super::{Actions, ChromeEvent, Controller, Host, HostKind, NodeEvent, Platform};

/// The identifier of the main host, which exists for the whole session.
pub(crate) const MAIN_HOST: u64 = 0;

/// One application instance shared by every native host of an activity or
/// scene.
pub(crate) struct Session<P: Platform> {
    app: Box<dyn AnyApp>,
    /// Where the application's background work delivers its messages.
    mailbox: Link,
    /// Runs commands and timers.
    executor: Arc<dyn Executor>,
    /// The running subscriptions by key.
    timers: Timers,
    /// Whether the application is in the foreground. Timers run only then.
    active: bool,
    hosts: HashMap<u64, P::Host>,
    /// Hosts the controller was asked to create but has not attached yet.
    pending: HashMap<u64, HostKind>,
    next_host: u64,
    controller: Option<P::Controller>,
    /// The navigation stack last mirrored to the controller.
    navigation: Vec<Route>,
    /// The top screen's chrome as the controller last received it, without
    /// the snackbar.
    chrome: Chrome,
    /// The snackbar the controller is showing.
    snackbar: Option<Snackbar>,
    /// The alert the controller is showing.
    dialog: Option<Dialog>,
    /// Whether the controller was last told that back is handled here.
    back_enabled: bool,
    /// The appearance the controller was last asked for.
    appearance: Appearance,
    /// Whether the controller has been given a navigation stack yet.
    navigation_synced: bool,
    /// The device's language, and the appearance every host resolves its
    /// colours for.
    environment: Environment,
    /// The environment the application was last told about.
    told: Option<Environment>,
    /// The height the sheet's content had when the controller last heard
    /// of it; `None` while no sheet host has rendered.
    sheet_height: Option<f32>,
    /// The platform changed the screen stack by itself. The next sync sends
    /// the chrome even if it is equal, and shows a screen the application
    /// kept without animating it back in.
    native_pop: bool,
}

impl<P: Platform> Session<P> {
    /// Creates a session around the main host.
    pub(crate) fn new(running: Running, main: P::Host) -> Self {
        Self::with_executor(running, main, Arc::new(Threads))
    }

    /// Creates a session whose commands and timers run on `executor`.
    pub(crate) fn with_executor(
        running: Running,
        main: P::Host,
        executor: Arc<dyn Executor>,
    ) -> Self {
        let mut hosts = HashMap::new();
        hosts.insert(MAIN_HOST, main);
        // What a new application asked for in `App::init`. The results wait
        // in the mailbox until the platform connects its waker.
        command::run(running.init, &running.mailbox, executor.as_ref());
        Self {
            app: running.app,
            mailbox: Link(running.mailbox),
            executor,
            timers: Timers::default(),
            active: true,
            hosts,
            pending: HashMap::new(),
            next_host: MAIN_HOST + 1,
            controller: None,
            navigation: Vec::new(),
            chrome: Chrome::default(),
            snackbar: None,
            dialog: None,
            back_enabled: false,
            appearance: Appearance::System,
            navigation_synced: false,
            environment: Environment::default(),
            told: running.told,
            native_pop: false,
            sheet_height: None,
        }
    }

    /// Ends the session and hands the application back, so another session
    /// can continue with its state.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub(crate) fn into_app(self) -> Running {
        let Self {
            app, mailbox, told, ..
        } = self;
        Running {
            app,
            // Dropping the rest of the session stops its timers and its
            // wake-ups. Work already started keeps delivering here.
            mailbox: mailbox.0.clone(),
            // The application has started; `init` is not run again.
            init: Jobs::none(),
            told,
        }
    }

    /// Gives the mailbox the way to reach this session on the UI thread:
    /// `wake` must arrange a call to [`Session::drain`] there. Messages that
    /// arrived while no session was connected ask for it at once.
    pub(crate) fn set_waker(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        self.mailbox.0.set_waker(Some(wake));
    }

    /// Applies every message background work has delivered, then renders
    /// once.
    pub(crate) fn drain(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        let parcels = self.mailbox.0.take();
        if parcels.is_empty() {
            return Ok(());
        }
        for parcel in parcels {
            self.apply(parcel);
        }
        self.render_all(cx)
    }

    /// The application moved to the foreground or the background.
    /// Subscriptions run only in the foreground; commands are not affected.
    pub(crate) fn set_active(&mut self, active: bool) {
        self.active = active;
        self.sync_subscriptions();
    }

    /// Continues with the application code of the library at `path`; see
    /// [`crate::hot`]. The state is kept. A library that cannot take the
    /// application over is reported and changes nothing.
    #[cfg(feature = "hot-reload")]
    pub(crate) fn hot_swap(&mut self, cx: &mut P::Cx<'_>, path: &str) -> Result<(), P::Error> {
        let library = match crate::hot::Library::open(path) {
            Ok(library) => library,
            Err(reason) => {
                crate::log::error(&format!("hot reload: {reason}"));
                return Ok(());
            }
        };
        // The application is moved out and back in without anything that
        // can unwind in between.
        // SAFETY: `self.app` is read once and written once; `adopt` returns
        // an application on both paths.
        unsafe {
            let app = std::ptr::read(&self.app);
            let (app, refused) = match library.adopt(app) {
                Ok(app) => (app, None),
                Err((app, reason)) => (app, Some(reason)),
            };
            std::ptr::write(&mut self.app, app);
            if let Some(reason) = refused {
                crate::log::error(&format!("hot reload: {reason}"));
                return Ok(());
            }
        }
        // Timers hold closures of the old code; the render starts the ones
        // the new code asks for.
        self.timers = Timers::default();
        crate::log::error(&format!("hot reload: now running {path}"));
        self.render_all(cx)
    }

    /// Applies one message and starts the work it asks for.
    fn apply(&mut self, message: Message) {
        let jobs = self.app.update(message);
        command::run(jobs, &self.mailbox.0, self.executor.as_ref());
    }

    /// Starts the subscriptions the application now returns and stops the
    /// ones it no longer does.
    fn sync_subscriptions(&mut self) {
        let wanted = if self.active {
            self.app.subscriptions()
        } else {
            Vec::new()
        };
        self.timers.0.retain(|key, (interval, stopped)| {
            let keep = wanted
                .iter()
                .any(|subscription| subscription.key == *key && subscription.interval == *interval);
            if !keep {
                stopped.store(true, Ordering::Relaxed);
            }
            keep
        });
        for subscription in wanted {
            if self.timers.0.contains_key(&subscription.key) {
                continue;
            }
            let stopped = Arc::new(AtomicBool::new(false));
            let mailbox = self.mailbox.0.clone();
            let tick = subscription.tick;
            let running = stopped.clone();
            self.executor.every(
                subscription.interval,
                stopped.clone(),
                Box::new(move || {
                    // A tick that was already due when the timer stopped.
                    if !running.load(Ordering::Relaxed) {
                        mailbox.post(tick());
                    }
                }),
            );
            self.timers
                .0
                .insert(subscription.key, (subscription.interval, stopped));
        }
    }

    /// Records whether the device shows its light or dark appearance. The
    /// next render repaints whatever depends on it.
    pub(crate) fn set_color_scheme(&mut self, scheme: ColorScheme) {
        self.environment.color_scheme = scheme;
    }

    /// Records the user's language as a BCP 47 tag. The next render tells
    /// the application.
    pub(crate) fn set_locale(&mut self, locale: &str) {
        if self.environment.locale != locale {
            self.environment.locale = locale.to_owned();
        }
    }

    /// Tells the application about the environment when it differs from
    /// what it heard last: before the first render, and after a change.
    fn announce_environment(&mut self) {
        if self.told.as_ref() == Some(&self.environment) {
            return;
        }
        self.told = Some(self.environment.clone());
        if let Some(message) = self.app.environment_changed(&self.environment) {
            self.apply(message);
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
        self.announce_environment();
        let has_navigation = self.app.navigation().is_some();
        for host in self.hosts.values_mut() {
            let element = match host.kind() {
                // A navigation app shows its root as the first screen.
                HostKind::Main if has_navigation => None,
                HostKind::Main => Some(match self.app.scaffold() {
                    Some((body, _)) => body,
                    None => self.app.view(),
                }),
                HostKind::Screen(key) => self.app.screen(key).map(|(body, _)| body),
                HostKind::Sheet => self.app.sheet().map(|sheet| sheet.into_content()),
            };
            if let Some(element) = element {
                host.runtime_mut()
                    .set_color_scheme(self.environment.color_scheme);
                host.render(cx, &element)?;
            }
        }
        self.sync_subscriptions();
        self.sync_sheet_height(cx)?;
        self.sync(cx)
    }

    /// Tells the controller when the content of the sheet on screen changed
    /// height, so a sheet that is sized once at presentation can follow.
    fn sync_sheet_height(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        let height = self
            .hosts
            .values()
            .find(|host| *host.kind() == HostKind::Sheet)
            .map(|host| host.runtime().content_height());
        if let (Some(before), Some(now)) = (self.sheet_height, height)
            && (before - now).abs() > 0.5
            && let Some(controller) = self.controller.as_ref()
        {
            controller.sheet_resized(cx, now)?;
        }
        self.sheet_height = height;
        Ok(())
    }

    /// Reconciles the application's navigation, dialog and sheet with the
    /// native controller.
    fn sync(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        if self.controller.is_none() {
            return Ok(());
        }
        let navigation = self.app.navigation().unwrap_or_default();
        let native_pop = std::mem::take(&mut self.native_pop);
        let navigated = navigation != self.navigation;
        if navigated {
            self.sync_navigation(cx, &navigation, native_pop)?;
            self.navigation = navigation;
        }
        self.sync_chrome(cx, navigated || native_pop)?;
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
        // A session starts out following the device, which is also what a
        // new activity or scene does. Asking again for what the platform
        // already shows changes nothing there.
        let appearance = self.app.appearance();
        if appearance != self.appearance {
            if let Some(controller) = self.controller.as_ref() {
                controller.set_appearance(cx, appearance)?;
            }
            self.appearance = appearance;
        }
        Ok(())
    }

    /// The chrome of the screen on top of the stack, read from the
    /// application's current state. Without navigation it is the chrome of
    /// the application's own scaffold, if it has one.
    fn top_chrome(&self) -> Chrome {
        let screen = match self.app.navigation() {
            Some(routes) => routes.last().and_then(|route| self.app.screen(&route.key)),
            None => self.app.scaffold(),
        };
        screen.map(|(_, chrome)| chrome).unwrap_or_default()
    }

    /// Mirrors the top screen's chrome and snackbar.
    ///
    /// The chrome is resent after every navigation change, even when it is
    /// equal, so the native bars never keep a previous screen's state.
    fn sync_chrome(&mut self, cx: &mut P::Cx<'_>, navigated: bool) -> Result<(), P::Error> {
        let Some(controller) = self.controller.as_ref() else {
            return Ok(());
        };
        let mut chrome = self.top_chrome();
        let snackbar = chrome.snackbar.take();
        if navigated || chrome != self.chrome {
            controller.set_chrome(cx, &chrome)?;
            self.chrome = chrome;
        }
        if snackbar != self.snackbar {
            match &snackbar {
                Some(snackbar) => controller.show_snackbar(cx, snackbar)?,
                None => controller.dismiss_snackbar(cx)?,
            }
            self.snackbar = snackbar;
        }
        Ok(())
    }

    fn sync_navigation(
        &mut self,
        cx: &mut P::Cx<'_>,
        screens: &[Route],
        quiet: bool,
    ) -> Result<(), P::Error> {
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
        // The first stack the controller receives is where the application
        // already is, for example after the activity was recreated. It is
        // shown, not replayed as a series of animated pushes.
        let restoring = !std::mem::replace(&mut self.navigation_synced, true) || quiet;
        for screen in &screens[common..] {
            let host = self.next_host;
            self.next_host += 1;
            self.pending
                .insert(host, HostKind::Screen(screen.key.clone()));
            if restoring {
                let mut screen = screen.clone();
                screen.transition = ScreenTransition::None;
                controller.push(cx, host, &screen)?;
            } else {
                controller.push(cx, host, screen)?;
            }
        }
        controller.set_root_visible(cx, screens.is_empty())
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

        // The control already shows the new value. Whatever the application
        // makes of the message, the control is updated from the state. A
        // control nobody listens to is left as the user set it.
        let changes_value = matches!(
            event,
            NodeEvent::Toggle(_)
                | NodeEvent::Text(_)
                | NodeEvent::Slider(_)
                | NodeEvent::Select(_)
                | NodeEvent::Picker(_)
                // The pull left the indicator spinning; it stops unless the
                // application says the refresh is still going.
                | NodeEvent::Refresh
        );
        if changes_value && message.is_some() {
            state.runtime_mut().reapply(id);
        }
        match message {
            Some(message) => self.update(cx, message),
            None if engine_changed => self.render_all(cx),
            None => Ok(()),
        }
    }

    /// Applies a message to the application and shows the result.
    fn update(&mut self, cx: &mut P::Cx<'_>, message: Message) -> Result<(), P::Error> {
        self.apply(message);
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

    /// The platform removed the top screen by itself: the user completed
    /// the system's back gesture.
    ///
    /// The session records the stack the platform now shows and sends the
    /// back message. An application that keeps the screen gets it back, the
    /// way a dialog it keeps is shown again.
    #[cfg_attr(not(any(test, target_os = "ios")), allow(dead_code))]
    pub(crate) fn native_pop(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
        if let Some(route) = self.navigation.pop() {
            let kind = HostKind::Screen(route.key);
            self.hosts.retain(|_, host| *host.kind() != kind);
            self.pending.retain(|_, pending| *pending != kind);
        }
        self.native_pop = true;
        if let Some(message) = self.app.back() {
            self.apply(message);
        }
        self.render_all(cx)
    }

    /// The toolbar item at `index` was tapped.
    pub(crate) fn toolbar_action(
        &mut self,
        cx: &mut P::Cx<'_>,
        index: usize,
    ) -> Result<(), P::Error> {
        // Handlers are read from the application's current state, never from
        // the copy last sent to the platform.
        let message = self
            .top_chrome()
            .app_bar
            .and_then(|bar| bar.actions.get(index)?.on_tap.message(()));
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
            self.apply(message);
        }
        self.render_all(cx)
    }

    /// The user acted on the floating action button, the drawer or the
    /// snackbar of the top screen.
    pub(crate) fn chrome_event(
        &mut self,
        cx: &mut P::Cx<'_>,
        event: ChromeEvent,
    ) -> Result<(), P::Error> {
        // Handlers are read from the application's current state.
        let chrome = self.top_chrome();
        let message = match event {
            ChromeEvent::Fab => chrome
                .floating_action_button
                .and_then(|fab| fab.on_click.message(())),
            ChromeEvent::DrawerSelected(index) => chrome
                .drawer
                .and_then(|drawer| drawer.on_select.message(index)),
            ChromeEvent::DrawerOpened | ChromeEvent::DrawerClosed => {
                // The platform changed the drawer by itself. Remember what it
                // shows now, so the sync below corrects it if the application
                // still says otherwise.
                let open = event == ChromeEvent::DrawerOpened;
                if let Some(shown) = &mut self.chrome.drawer {
                    shown.open = open;
                }
                chrome.drawer.and_then(|drawer| {
                    let handler = if open {
                        drawer.on_open
                    } else {
                        drawer.on_close
                    };
                    handler.message(())
                })
            }
            ChromeEvent::SnackbarAction | ChromeEvent::SnackbarDismissed => {
                // The platform removed the snackbar; if the application keeps
                // returning it, the sync below shows it again.
                self.snackbar = None;
                chrome.snackbar.and_then(|snackbar| {
                    let handler = if event == ChromeEvent::SnackbarAction {
                        snackbar.on_action
                    } else {
                        snackbar.on_dismiss
                    };
                    handler.message(())
                })
            }
        };
        if let Some(message) = message {
            self.apply(message);
        }
        self.render_all(cx)
    }

    /// The user picked the destination at `index` in the bottom navigation bar.
    pub(crate) fn bottom_navigation_selected(
        &mut self,
        cx: &mut P::Cx<'_>,
        index: usize,
    ) -> Result<(), P::Error> {
        // The bar already shows the destination the user picked; if the
        // application keeps another one, the bar is told again.
        if let Some(bar) = self.chrome.bottom_navigation.as_mut() {
            bar.selected = index;
        }
        let message = self
            .top_chrome()
            .bottom_navigation
            .and_then(|bar| bar.on_select.message(index));
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
            self.apply(message);
        }
        self.render_all(cx)
    }
}

/// The session's connection to the application's mailbox. When the session
/// goes away the mailbox stops waking it; messages wait for the next one.
struct Link(Arc<Mailbox>);

impl Drop for Link {
    fn drop(&mut self) {
        self.0.set_waker(None);
    }
}

/// The running subscriptions by key: their interval and the flag that stops
/// them. They stop with the session.
#[derive(Default)]
struct Timers(HashMap<String, (Duration, Arc<AtomicBool>)>);

impl Drop for Timers {
    fn drop(&mut self) {
        for (_, stopped) in self.0.values() {
            stopped.store(true, Ordering::Relaxed);
        }
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
    use crate::command::testing::Inline;
    use crate::command::{Command, Subscription};
    use crate::element::{Element, RawElement};
    use crate::engine::geometry::Size;
    use crate::engine::measure::ApproximateMeasure;
    use crate::engine::render::Op;
    use crate::engine::runtime::Runtime;
    use crate::engine::tree::NodeKind;
    use crate::navigation::{BottomNavItem, BottomNavigation, Navigation};
    use crate::presentation::BottomSheet;
    use crate::scaffold::{AppBar, Drawer, DrawerItem, Fab, Scaffold};
    use crate::style::Icon;
    use crate::ui::{Button, Column, Switch, Text};
    use crate::widgets::Widget;

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
        /// The nodes the last render updated.
        updated: Vec<NodeId>,
    }

    impl FakeHost {
        fn new(kind: HostKind) -> Self {
            Self {
                kind,
                runtime: Runtime::new(Box::new(ApproximateMeasure)),
                renders: 0,
                updated: Vec::new(),
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
            let update = self.runtime.update(element, Size::new(320.0, 640.0));
            self.updated = update
                .ops
                .iter()
                .filter_map(|op| match op {
                    Op::Update(node) => Some(node.id),
                    _ => None,
                })
                .collect();
            self.renders += 1;
            Ok(())
        }
    }

    struct FakeController(Log);

    impl Controller<Fake> for FakeController {
        fn push(&self, _cx: &mut (), host: u64, screen: &Route) -> Result<(), Infallible> {
            let how = match screen.transition {
                ScreenTransition::None => " instantly",
                ScreenTransition::Slide | ScreenTransition::Fade => "",
            };
            self.0
                .borrow_mut()
                .push(format!("push {host} {}{how}", screen.key));
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
        fn set_chrome(&self, _cx: &mut (), chrome: &Chrome) -> Result<(), Infallible> {
            let mut line = format!(
                "chrome {}",
                chrome
                    .app_bar
                    .as_ref()
                    .map_or("-", |bar| bar.title.as_str())
            );
            if let Some(fab) = &chrome.floating_action_button {
                line += &format!(" fab:{}", fab.label);
            }
            if let Some(bar) = &chrome.bottom_navigation {
                line += &format!(" tab:{}", bar.selected);
            }
            if let Some(drawer) = &chrome.drawer {
                line += if drawer.open {
                    " drawer:open"
                } else {
                    " drawer:closed"
                };
            }
            self.0.borrow_mut().push(line);
            Ok(())
        }
        fn show_snackbar(&self, _cx: &mut (), snackbar: &Snackbar) -> Result<(), Infallible> {
            self.0
                .borrow_mut()
                .push(format!("snackbar {}", snackbar.message));
            Ok(())
        }
        fn dismiss_snackbar(&self, _cx: &mut ()) -> Result<(), Infallible> {
            self.0.borrow_mut().push("dismiss snackbar".into());
            Ok(())
        }
        fn set_back_enabled(&self, _cx: &mut (), enabled: bool) -> Result<(), Infallible> {
            self.0.borrow_mut().push(format!("back {enabled}"));
            Ok(())
        }
        fn set_appearance(&self, _cx: &mut (), appearance: Appearance) -> Result<(), Infallible> {
            self.0
                .borrow_mut()
                .push(format!("appearance {appearance:?}"));
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
        fn sheet_resized(&self, _cx: &mut (), height: f32) -> Result<(), Infallible> {
            self.0
                .borrow_mut()
                .push(format!("sheet height {height:.0}"));
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
        fab: bool,
        /// Whether the top screen has a drawer, and whether it is open.
        drawer: Option<bool>,
        snackbar: bool,
        /// Keeps the snackbar up after the platform reports it gone.
        sticky_snackbar: bool,
        picked: Option<usize>,
        /// A switch on every screen, and whether the application lets the
        /// user change it.
        on: bool,
        locked: bool,
        /// The selected bottom navigation destination, when there is a bar.
        tab: Option<usize>,
        /// Leaves the back action to `Navigation::on_back`.
        on_back: bool,
        /// Keeps the top screen when asked to go back.
        stay: bool,
        /// The interval of the clock, while it runs. Each tick is a `Tap`.
        clock: Option<Duration>,
        /// How many lines of text the sheet shows.
        sheet_lines: usize,
        /// Tells one "Deleted" snackbar from the next.
        snackbar_id: u64,
        /// Starts the work of `Msg::Load` when the application starts.
        load_at_start: bool,
        /// The appearance the application asks for.
        appearance: Appearance,
        /// Asks to hear about the device's language and appearance.
        follows_environment: bool,
        /// Every environment the application was told about, in order.
        environments: Vec<(String, ColorScheme)>,
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
        Fab,
        Go(usize),
        DrawerOpened,
        DrawerClosed,
        Undo,
        SnackbarGone,
        Switch(bool),
        Tab(usize),
        /// Starts background work that reports `Loaded`.
        Load,
        Loaded(usize),
        Environment(String, ColorScheme),
    }

    impl Component for TestApp {
        type Msg = Msg;

        fn view(&self) -> Element<Msg> {
            Column::new()
                .background(crate::style::Color::BACKGROUND)
                .into()
        }

        fn update(&mut self, msg: Msg) -> Command<Msg> {
            let mut state = self.state();
            match msg {
                Msg::Load => return Command::perform(|| Msg::Loaded(100)),
                Msg::Loaded(count) => state.clicks += count,
                Msg::Tap => state.clicks += 1,
                Msg::CloseDialog => state.dialog = state.sticky_dialog,
                Msg::CloseSheet => state.sheet = false,
                Msg::Back => {
                    if !state.stay {
                        state.stack.pop();
                    }
                }
                Msg::Fab => state.clicks += 10,
                Msg::Go(index) => {
                    state.picked = Some(index);
                    state.drawer = Some(false);
                }
                Msg::DrawerOpened => state.drawer = Some(true),
                Msg::DrawerClosed => state.drawer = Some(false),
                Msg::Undo => {
                    state.clicks = 0;
                    state.snackbar = false;
                }
                Msg::SnackbarGone => state.snackbar = state.sticky_snackbar,
                Msg::Switch(on) => {
                    if !state.locked {
                        state.on = on;
                    }
                }
                Msg::Tab(index) => {
                    if !state.locked {
                        state.tab = Some(index);
                    }
                }
                Msg::Environment(language, scheme) => state.environments.push((language, scheme)),
            }
            Command::none()
        }
    }

    impl App for TestApp {
        fn init(&mut self) -> Command<Msg> {
            if self.state().load_at_start {
                Command::perform(|| Msg::Loaded(100))
            } else {
                Command::none()
            }
        }

        fn environment_changed(&self, environment: &Environment) -> Option<Msg> {
            self.state()
                .follows_environment
                .then(|| Msg::Environment(environment.language(), environment.color_scheme))
        }

        fn navigation(&self) -> Option<Navigation<Msg>> {
            let state = self.state();
            let navigation = Navigation::new(&state.stack);
            Some(if state.on_back {
                navigation.on_back(Msg::Back)
            } else {
                navigation
            })
        }

        fn screen(&self, key: &str) -> Option<Scaffold<Msg>> {
            let state = self.state();
            let top = state.stack.last() == Some(&key);
            let mut app_bar = AppBar::new(key.to_uppercase());
            if let (Some(subtitle), true) = (state.subtitle, top) {
                app_bar = app_bar.subtitle(subtitle);
            }
            Some(
                Scaffold::new()
                    .app_bar(app_bar)
                    .body(
                        Column::new()
                            .child(Text::new(format!("clicks {}", state.clicks)))
                            .child(Button::new("Tap").on_click(Msg::Tap))
                            .child(Switch::new(state.on).on_toggle(Msg::Switch)),
                    )
                    .bottom_navigation(state.tab.map(|tab| {
                        BottomNavigation::new([
                            BottomNavItem::new("Home", Icon::new().shared("home")),
                            BottomNavItem::new("Search", Icon::new().shared("search")),
                        ])
                        .selected(tab)
                        .on_select(Msg::Tab)
                    }))
                    .floating_action_button(
                        (state.fab && top).then(|| Fab::new("Add").on_click(Msg::Fab)),
                    )
                    .drawer(state.drawer.filter(|_| top).map(|open| {
                        Drawer::new([DrawerItem::new("Inbox"), DrawerItem::new("Archive")])
                            .open(open)
                            .on_select(Msg::Go)
                            .on_open(Msg::DrawerOpened)
                            .on_close(Msg::DrawerClosed)
                    }))
                    .snackbar((state.snackbar && top).then(|| {
                        Snackbar::new("Deleted")
                            .id(state.snackbar_id)
                            .action("Undo", Msg::Undo)
                            .on_dismiss(Msg::SnackbarGone)
                    })),
            )
        }

        fn subscriptions(&self) -> Vec<Subscription<Msg>> {
            self.state()
                .clock
                .map(|interval| Subscription::every("clock", interval, || Msg::Tap))
                .into_iter()
                .collect()
        }

        fn dialog(&self) -> Option<Dialog<Msg>> {
            self.state()
                .dialog
                .then(|| Dialog::new("Sure?").cancel("No", Msg::CloseDialog))
        }

        fn sheet(&self) -> Option<BottomSheet<Msg>> {
            let state = self.state();
            state.sheet.then(|| {
                let mut content = Column::new();
                for line in 0..state.sheet_lines {
                    content = content.child(Text::new(format!("line {line}")));
                }
                BottomSheet::new()
                    .child(content)
                    .on_dismiss(Msg::CloseSheet)
            })
        }

        fn back(&self) -> Option<Msg> {
            let state = self.state();
            (!state.on_back && state.stack.len() > 1).then_some(Msg::Back)
        }

        fn appearance(&self) -> Appearance {
            self.state().appearance
        }
    }

    /// A session around a `TestApp`, the app itself and the controller's log.
    fn session(state: State) -> (Session<Fake>, TestApp, Log) {
        let log = Log::default();
        let app = TestApp::new(state);
        let mut session = Session::<Fake>::with_executor(
            Running::new(Box::new(app.clone())),
            FakeHost::new(HostKind::Main),
            Arc::new(Inline::default()),
        );
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
        // The stack the application starts with is shown, not animated in.
        assert_eq!(
            take(&log),
            ["push 1 home instantly", "root false", "chrome HOME"]
        );

        // Nothing changed, so a second render sends nothing.
        attach_pending(&mut session);
        assert!(take(&log).is_empty());
    }

    #[test]
    fn a_restored_stack_is_shown_at_once_and_later_pushes_animate() {
        // An activity recreated with the same application starts three
        // screens deep.
        let (mut session, app, log) = session(State {
            stack: vec!["home", "a", "b"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        assert_eq!(
            take(&log)[..3],
            [
                "push 1 home instantly",
                "push 2 a instantly",
                "push 3 b instantly"
            ]
        );

        app.state().stack.push("c");
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log)[0], "push 4 c");
    }

    #[test]
    fn the_color_scheme_reaches_every_host() {
        use crate::style::{Color, ColorScheme};
        use crate::tree::NodeKind;
        use crate::widgets::Widget;
        let (mut session, _app, _log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.set_color_scheme(ColorScheme::Dark);
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);

        let screen = session
            .hosts
            .values()
            .find(|host| matches!(host.kind, HostKind::Screen(_)))
            .expect("screen host");
        let NodeKind::Widget(Widget::Text(text)) =
            &screen.runtime.tree().children[0].children[0].kind
        else {
            panic!("expected the text");
        };
        assert_eq!(text.color.argb(), Color::TEXT.dark);
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
        assert_eq!(take(&log), ["chrome HOME"]);
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

    /// A session one screen deep with the controller's first messages read.
    fn on_home(state: State) -> (Session<Fake>, TestApp, Log) {
        let (mut session, app, log) = session(State {
            stack: vec!["home"],
            ..state
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);
        (session, app, log)
    }

    #[test]
    fn the_floating_action_button_follows_state_and_reports_a_press() {
        let (mut session, app, log) = on_home(State::default());

        app.state().fab = true;
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["chrome HOME fab:Add"]);

        session.chrome_event(&mut (), ChromeEvent::Fab).unwrap();
        assert_eq!(app.state().clicks, 10);
        // The button itself did not change, so nothing is resent.
        assert!(take(&log).is_empty());

        app.state().fab = false;
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["chrome HOME"]);
    }

    #[test]
    fn the_drawer_is_opened_by_state_and_follows_the_user() {
        let (mut session, app, log) = on_home(State {
            drawer: Some(false),
            ..State::default()
        });

        // The application opens it.
        app.state().drawer = Some(true);
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["chrome HOME drawer:open"]);

        // The user swipes it shut: the application hears about it and agrees,
        // so the platform is not told what it already shows.
        session
            .chrome_event(&mut (), ChromeEvent::DrawerClosed)
            .unwrap();
        assert_eq!(app.state().drawer, Some(false));
        assert!(take(&log).is_empty());

        // The user opens it and picks a destination, which closes it.
        session
            .chrome_event(&mut (), ChromeEvent::DrawerOpened)
            .unwrap();
        assert_eq!(app.state().drawer, Some(true));
        session
            .chrome_event(&mut (), ChromeEvent::DrawerSelected(1))
            .unwrap();
        assert_eq!(app.state().picked, Some(1));
        assert_eq!(take(&log), ["chrome HOME drawer:closed"]);
    }

    #[test]
    fn a_snackbar_is_shown_once_and_can_be_dismissed_either_way() {
        let (mut session, app, log) = on_home(State {
            clicks: 3,
            ..State::default()
        });

        app.state().snackbar = true;
        session.render_all(&mut ()).unwrap();
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["snackbar Deleted"]);

        // It times out: the application drops it and nothing more is sent.
        session
            .chrome_event(&mut (), ChromeEvent::SnackbarDismissed)
            .unwrap();
        assert!(!app.state().snackbar);
        assert!(take(&log).is_empty());

        // Shown again, its action reaches the application.
        app.state().snackbar = true;
        session.render_all(&mut ()).unwrap();
        session
            .chrome_event(&mut (), ChromeEvent::SnackbarAction)
            .unwrap();
        assert_eq!(app.state().clicks, 0);
        assert_eq!(take(&log), ["snackbar Deleted"]);

        // The application can also take it down itself.
        app.state().snackbar = true;
        session.render_all(&mut ()).unwrap();
        app.state().snackbar = false;
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["snackbar Deleted", "dismiss snackbar"]);
    }

    #[test]
    fn a_snackbar_the_application_keeps_is_shown_again() {
        let (mut session, app, log) = on_home(State {
            snackbar: true,
            sticky_snackbar: true,
            ..State::default()
        });

        // The platform took it down, but the application still returns it.
        session
            .chrome_event(&mut (), ChromeEvent::SnackbarDismissed)
            .unwrap();
        assert!(app.state().snackbar);
        assert_eq!(take(&log), ["snackbar Deleted"]);
    }

    #[test]
    fn chrome_belongs_to_the_top_screen() {
        let (mut session, app, log) = on_home(State {
            fab: true,
            ..State::default()
        });

        // A pushed screen brings its own chrome...
        app.state().stack.push("detail");
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        assert_eq!(
            take(&log)[..3],
            ["push 2 detail", "root false", "chrome DETAIL fab:Add"]
        );

        // ...and leaving it restores the previous screen's.
        app.state().stack.pop();
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log)[2], "chrome HOME fab:Add");
    }

    /// The switch on the screen rendered by host `host`.
    fn switch_of(session: &Session<Fake>, host: u64) -> NodeId {
        tree::preorder(session.hosts[&host].runtime.tree())
            .into_iter()
            .find(|node| matches!(node.kind, NodeKind::Widget(Widget::Switch(_))))
            .expect("the screen has a switch")
            .id
    }

    #[test]
    fn a_control_the_application_overrules_is_set_back() {
        let (mut session, app, _log) = on_home(State {
            locked: true,
            ..State::default()
        });
        let switch = switch_of(&session, 1);

        // The user flips the switch; the native view now shows "on", but the
        // application keeps it off. The description did not change, and the
        // switch is updated all the same.
        session
            .node_event(&mut (), 1, switch, NodeEvent::Toggle(true))
            .unwrap();
        assert!(!app.state().on);
        assert_eq!(session.hosts[&1].updated, [switch]);

        // An ordinary render afterwards leaves it alone.
        session.render_all(&mut ()).unwrap();
        assert!(session.hosts[&1].updated.is_empty());
    }

    #[test]
    fn a_control_the_application_follows_is_updated_once() {
        let (mut session, app, _log) = on_home(State::default());
        let switch = switch_of(&session, 1);

        session
            .node_event(&mut (), 1, switch, NodeEvent::Toggle(true))
            .unwrap();
        assert!(app.state().on);
        assert_eq!(session.hosts[&1].updated, [switch]);
    }

    #[test]
    fn a_press_does_not_update_the_button() {
        let (mut session, _app, _log) = on_home(State::default());
        let button = tree::preorder(session.hosts[&1].runtime.tree())
            .into_iter()
            .find(|node| matches!(node.kind, NodeKind::Widget(Widget::Button(_))))
            .unwrap()
            .id;

        session
            .node_event(&mut (), 1, button, NodeEvent::Click)
            .unwrap();
        assert!(!session.hosts[&1].updated.contains(&button));
    }

    #[test]
    fn a_destination_the_application_overrules_is_selected_again() {
        let (mut session, app, log) = on_home(State {
            tab: Some(0),
            locked: true,
            ..State::default()
        });

        // The bar already shows the second destination; the application
        // stays on the first, so the bar is told.
        session.bottom_navigation_selected(&mut (), 1).unwrap();
        assert_eq!(app.state().tab, Some(0));
        assert_eq!(take(&log), ["chrome HOME tab:0"]);

        // When the application follows, the bar is already right.
        app.state().locked = false;
        session.bottom_navigation_selected(&mut (), 1).unwrap();
        assert_eq!(app.state().tab, Some(1));
        assert!(take(&log).is_empty());
    }

    #[test]
    fn navigation_can_name_the_back_message() {
        let (mut session, app, log) = on_home(State {
            on_back: true,
            ..State::default()
        });
        // On the root screen the platform keeps the back action.
        assert!(!session.back(&mut ()).unwrap());

        app.state().stack.push("detail");
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        assert!(take(&log).contains(&"back true".to_string()));

        assert!(session.back(&mut ()).unwrap());
        assert_eq!(app.state().stack, ["home"]);
        assert!(take(&log).contains(&"pop 1 Slide".to_string()));
    }

    #[test]
    fn a_screen_the_platform_popped_is_not_popped_again() {
        let (mut session, app, log) = on_home(State::default());
        app.state().stack.push("detail");
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);

        // The user completed the system's back gesture: the screen is gone
        // already. The application follows, so only the chrome of the screen
        // that is now on top is sent.
        session.native_pop(&mut ()).unwrap();
        assert_eq!(app.state().stack, ["home"]);
        assert_eq!(take(&log), ["chrome HOME", "back false"]);
        assert!(
            !session
                .hosts
                .values()
                .any(|host| host.kind == HostKind::Screen("detail".into()))
        );
    }

    #[test]
    fn a_screen_the_application_keeps_is_shown_again() {
        let (mut session, app, log) = on_home(State {
            stay: true,
            ..State::default()
        });
        app.state().stack.push("detail");
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        take(&log);

        session.native_pop(&mut ()).unwrap();
        assert_eq!(app.state().stack, ["home", "detail"]);
        // Put back at once, not slid in a second time.
        assert_eq!(
            take(&log),
            ["push 3 detail instantly", "root false", "chrome DETAIL"]
        );
    }

    /// An application with one screen and native chrome around it.
    struct Single;

    impl Component for Single {
        type Msg = Msg;

        fn view(&self) -> Element<Msg> {
            Text::new("unused").into()
        }

        fn update(&mut self, _msg: Msg) -> Command<Msg> {
            Command::none()
        }
    }

    impl App for Single {
        fn scaffold(&self) -> Option<Scaffold<Msg>> {
            Some(
                Scaffold::new()
                    .app_bar(AppBar::new("Notes"))
                    .body(Text::new("body"))
                    .floating_action_button(Fab::new("Add").on_click(Msg::Fab)),
            )
        }
    }

    #[test]
    fn an_application_without_navigation_can_have_chrome() {
        let log = Log::default();
        let mut session = Session::<Fake>::new(
            Running::new(Box::new(Single)),
            FakeHost::new(HostKind::Main),
        );
        session.set_controller(FakeController(log.clone()));
        session.render_all(&mut ()).unwrap();

        assert_eq!(take(&log), ["chrome Notes fab:Add"]);
        // The scaffold's body is what the main host shows.
        let texts: Vec<String> = tree::preorder(session.hosts[&MAIN_HOST].runtime.tree())
            .into_iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Widget(Widget::Text(props)) => Some(props.content.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["body"]);
    }

    #[test]
    #[cfg_attr(debug_assertions, should_panic(expected = "same route key twice"))]
    fn a_repeated_route_key_is_refused() {
        let (mut session, _app, log) = session(State {
            stack: vec!["home", "detail", "home"],
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        // Release builds show the stack without the repeat.
        assert_eq!(
            take(&log)[..2],
            ["push 1 home instantly", "push 2 detail instantly"]
        );
    }

    /// A session whose executor the test keeps, and how often it was woken.
    fn background(state: State) -> (Session<Fake>, TestApp, Arc<Inline>, Arc<Mutex<usize>>) {
        let app = TestApp::new(State {
            stack: vec!["home"],
            ..state
        });
        let executor = Arc::new(Inline::default());
        let mut session = Session::<Fake>::with_executor(
            Running::new(Box::new(app.clone())),
            FakeHost::new(HostKind::Main),
            executor.clone(),
        );
        session.set_controller(FakeController(Log::default()));
        let wakes = Arc::new(Mutex::new(0));
        let counter = wakes.clone();
        session.set_waker(Arc::new(move || *counter.lock().unwrap() += 1));
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        (session, app, executor, wakes)
    }

    fn renders(session: &Session<Fake>) -> usize {
        session.hosts[&1].renders
    }

    #[test]
    fn the_result_of_a_command_comes_back_as_a_message() {
        let (mut session, app, _executor, wakes) = background(State::default());
        let before = renders(&session);

        // The work finishes (here at once) and its message waits in the
        // mailbox; the application has not seen it yet.
        session.update(&mut (), Box::new(Msg::Load)).unwrap();
        assert_eq!(app.state().clicks, 0);
        assert_eq!(*wakes.lock().unwrap(), 1);

        // The platform wakes the UI thread, which applies it and renders.
        session.drain(&mut ()).unwrap();
        assert_eq!(app.state().clicks, 100);
        assert_eq!(renders(&session), before + 2);

        // Nothing waiting: no render.
        session.drain(&mut ()).unwrap();
        assert_eq!(renders(&session), before + 2);
    }

    #[test]
    fn several_results_are_applied_with_one_render() {
        let (mut session, app, _executor, wakes) = background(State::default());
        session.update(&mut (), Box::new(Msg::Load)).unwrap();
        session.update(&mut (), Box::new(Msg::Load)).unwrap();
        // One wake-up is on its way; the second message rides along.
        assert_eq!(*wakes.lock().unwrap(), 1);

        let before = renders(&session);
        session.drain(&mut ()).unwrap();
        assert_eq!(app.state().clicks, 200);
        assert_eq!(renders(&session), before + 1);
    }

    #[test]
    fn a_subscription_runs_while_the_application_returns_it() {
        let (mut session, app, executor, _wakes) = background(State::default());
        assert!(executor.running().is_empty());

        let second = Duration::from_secs(1);
        app.state().clock = Some(second);
        session.render_all(&mut ()).unwrap();
        assert_eq!(executor.running(), [second]);

        // A tick is a message like any other.
        executor.fire();
        session.drain(&mut ()).unwrap();
        assert_eq!(app.state().clicks, 1);
        // Still the same timer: staying subscribed does not restart it.
        assert_eq!(executor.running(), [second]);

        // Another interval under the same key replaces it.
        app.state().clock = Some(second * 2);
        session.render_all(&mut ()).unwrap();
        assert_eq!(executor.running(), [second * 2]);

        app.state().clock = None;
        session.render_all(&mut ()).unwrap();
        assert!(executor.running().is_empty());
    }

    #[test]
    fn subscriptions_pause_in_the_background() {
        let (mut session, _app, executor, _wakes) = background(State {
            clock: Some(Duration::from_secs(1)),
            ..State::default()
        });
        assert_eq!(executor.running().len(), 1);

        session.set_active(false);
        assert!(executor.running().is_empty());

        session.set_active(true);
        assert_eq!(executor.running().len(), 1);
    }

    #[test]
    fn work_outlives_the_session_that_started_it() {
        let (mut session, app, executor, wakes) = background(State {
            clock: Some(Duration::from_secs(1)),
            ..State::default()
        });
        session.update(&mut (), Box::new(Msg::Load)).unwrap();
        assert_eq!(*wakes.lock().unwrap(), 1);

        // The activity is recreated: the session ends with a message still
        // waiting. Its timers stop and it is not woken again.
        let running = session.into_app();
        assert!(executor.running().is_empty());

        // The next session is woken for the waiting message as soon as it
        // connects, and its application has kept its state.
        let mut next = Session::<Fake>::with_executor(
            running,
            FakeHost::new(HostKind::Main),
            executor.clone(),
        );
        let woken = Arc::new(Mutex::new(0));
        let counter = woken.clone();
        next.set_waker(Arc::new(move || *counter.lock().unwrap() += 1));
        assert_eq!(*woken.lock().unwrap(), 1);
        assert_eq!(*wakes.lock().unwrap(), 1);

        next.drain(&mut ()).unwrap();
        assert_eq!(app.state().clicks, 100);
        assert_eq!(executor.running().len(), 1);
    }

    #[test]
    fn the_application_starts_its_work_once() {
        let (session, app, executor, wakes) = background(State {
            load_at_start: true,
            ..State::default()
        });
        // The result was waiting when the platform connected its waker.
        assert_eq!(*wakes.lock().unwrap(), 1);
        let mut session = session;
        session.drain(&mut ()).unwrap();
        assert_eq!(app.state().clicks, 100);

        // The activity is recreated: the application is the same instance,
        // so the session that continues with it starts nothing.
        let mut next = Session::<Fake>::with_executor(
            session.into_app(),
            FakeHost::new(HostKind::Main),
            executor,
        );
        let woken = Arc::new(Mutex::new(0));
        let counter = woken.clone();
        next.set_waker(Arc::new(move || *counter.lock().unwrap() += 1));
        next.drain(&mut ()).unwrap();
        assert_eq!(*woken.lock().unwrap(), 0);
        assert_eq!(app.state().clicks, 100);
    }

    #[test]
    fn the_application_hears_about_the_environment_when_it_changes() {
        let (mut session, app, _log) = session(State {
            stack: vec!["home"],
            follows_environment: true,
            ..State::default()
        });
        let told = |app: &TestApp| app.state().environments.clone();
        let (pl, en) = ("pl".to_string(), "en".to_string());

        // Before the first render, with what the platform reported so far.
        session.set_locale("pl-PL");
        session.render_all(&mut ()).unwrap();
        assert_eq!(told(&app), [(pl.clone(), ColorScheme::Light)]);

        // A render that changes nothing says nothing.
        session.render_all(&mut ()).unwrap();
        assert_eq!(told(&app).len(), 1);

        session.set_color_scheme(ColorScheme::Dark);
        session.render_all(&mut ()).unwrap();
        assert_eq!(told(&app)[1..], [(pl.clone(), ColorScheme::Dark)]);

        // The activity is recreated with the same environment: the
        // application already knows.
        let recreate = |session: Session<Fake>, locale: &str| {
            let mut next = Session::<Fake>::with_executor(
                session.into_app(),
                FakeHost::new(HostKind::Main),
                Arc::new(Inline::default()),
            );
            next.set_controller(FakeController(Log::default()));
            next.set_color_scheme(ColorScheme::Dark);
            next.set_locale(locale);
            next.render_all(&mut ()).unwrap();
            next
        };
        let session = recreate(session, "pl-PL");
        assert_eq!(told(&app).len(), 2);

        // The user changed the device language, which recreates it too.
        recreate(session, "en-GB");
        assert_eq!(told(&app)[2..], [(en, ColorScheme::Dark)]);
    }

    #[test]
    fn the_application_chooses_its_appearance() {
        let (mut session, app, log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        let asked = |log: &Log| {
            take(log)
                .into_iter()
                .filter(|entry| entry.starts_with("appearance"))
                .collect::<Vec<_>>()
        };

        // Following the device is what the platform does by itself.
        session.render_all(&mut ()).unwrap();
        assert!(asked(&log).is_empty());

        app.state().appearance = Appearance::Dark;
        session.render_all(&mut ()).unwrap();
        assert_eq!(asked(&log), ["appearance Dark"]);

        // Asked once, not on every render.
        session.render_all(&mut ()).unwrap();
        assert!(asked(&log).is_empty());

        app.state().appearance = Appearance::System;
        session.render_all(&mut ()).unwrap();
        assert_eq!(asked(&log), ["appearance System"]);
    }

    #[test]
    fn an_application_that_does_not_follow_the_environment_is_left_alone() {
        let (mut session, app, _log) = session(State {
            stack: vec!["home"],
            ..State::default()
        });
        session.set_locale("pl-PL");
        session.render_all(&mut ()).unwrap();
        assert!(app.state().environments.is_empty());
    }

    /// Builds the fixture application in `tests/fixtures/hot-app` with the
    /// given features and returns the library, copied to a name of its own:
    /// loading the same path twice gives the same library.
    #[cfg(feature = "hot-reload")]
    fn fixture_library(features: &str) -> String {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let target = root.join("target/hot-fixture");
        let built = std::process::Command::new(env!("CARGO"))
            .arg("build")
            .arg("--manifest-path")
            .arg(root.join("tests/fixtures/hot-app/Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .args(["--features", features])
            // The test's own flags are not the fixture's.
            .env_remove("RUSTFLAGS")
            .env_remove("CARGO_ENCODED_RUSTFLAGS")
            .status()
            .expect("cargo runs");
        assert!(built.success(), "the fixture builds");
        let extension = if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        };
        let copy = target.join(format!(
            "hot_app_{}.{extension}",
            features.replace(',', "_")
        ));
        std::fs::copy(target.join(format!("debug/libhot_app.{extension}")), &copy)
            .expect("the library is copied");
        copy.to_str().expect("a utf-8 path").to_owned()
    }

    /// What the fixture shows, and the message of its button.
    #[cfg(feature = "hot-reload")]
    fn fixture_screen(session: &Session<Fake>) -> (String, Message) {
        use crate::element::ElementKind;
        use crate::widgets::Widget;
        let view = session.app.view();
        let ElementKind::Widget(Widget::Text(text)) = &view.children[0].kind else {
            panic!("the first child is a text");
        };
        let ElementKind::Widget(Widget::Button(button)) = &view.children[1].kind else {
            panic!("the second child is a button");
        };
        (
            text.content.clone(),
            button.on_click.message(()).expect("a handler"),
        )
    }

    /// One test, because the versions are built one after the other into
    /// the same target directory.
    #[cfg(feature = "hot-reload")]
    #[test]
    fn new_application_code_takes_over_the_running_state() {
        use crate::hot::Library;

        let first = fixture_library("");
        let second = fixture_library("v2");
        let other_layout = fixture_library("v3");

        // The application starts as version one, which lives in a library
        // with its own copy of the framework, as an app on a device does.
        let app = Library::open(&first)
            .expect("version one loads")
            .create(c"fixture_new");
        let mut session = Session::<Fake>::with_executor(
            Running::new(app),
            FakeHost::new(HostKind::Main),
            Arc::new(Inline::default()),
        );
        session.render_all(&mut ()).unwrap();
        let (text, add) = fixture_screen(&session);
        assert_eq!(text, "Count: 0");
        session.update(&mut (), add).unwrap();
        assert_eq!(fixture_screen(&session).0, "Count: 1");
        let renders = session.hosts[&MAIN_HOST].renders;

        // Version two takes the same value over: the count is kept, the
        // label is the new code's, and the screen is rendered again.
        session.hot_swap(&mut (), &second).unwrap();
        let (text, add) = fixture_screen(&session);
        assert_eq!(text, "Total: 1");
        assert_eq!(session.hosts[&MAIN_HOST].renders, renders + 1);
        // `update` is new code too.
        session.update(&mut (), add).unwrap();
        assert_eq!(fixture_screen(&session).0, "Total: 11");

        // A library whose state has another layout is refused, and the
        // application carries on as it was.
        let renders = session.hosts[&MAIN_HOST].renders;
        session.hot_swap(&mut (), &other_layout).unwrap();
        assert_eq!(fixture_screen(&session).0, "Total: 11");
        assert_eq!(session.hosts[&MAIN_HOST].renders, renders);

        // So is a file that is not such a library.
        session.hot_swap(&mut (), "/nonexistent/libapp.so").unwrap();
        assert_eq!(fixture_screen(&session).0, "Total: 11");
    }

    #[test]
    fn a_snackbar_with_a_new_id_is_shown_again() {
        let (mut session, app, log) = on_home(State {
            snackbar: true,
            ..State::default()
        });

        // The same snackbar returned again is the one already showing.
        session.render_all(&mut ()).unwrap();
        assert!(take(&log).is_empty());

        // A second deletion while the first notice is still up.
        app.state().snackbar_id = 1;
        session.render_all(&mut ()).unwrap();
        assert_eq!(take(&log), ["snackbar Deleted"]);
    }

    #[test]
    fn a_sheet_whose_content_grows_is_resized() {
        let (mut session, app, log) = session(State {
            sheet: true,
            sheet_lines: 1,
            ..State::default()
        });
        session.render_all(&mut ()).unwrap();
        attach_pending(&mut session);
        // Presenting it is not a resize: the platform measures it then.
        assert!(
            !take(&log)
                .iter()
                .any(|call| call.starts_with("sheet height"))
        );

        // Same content, no call.
        session.render_all(&mut ()).unwrap();
        assert!(take(&log).is_empty());

        app.state().sheet_lines = 4;
        session.render_all(&mut ()).unwrap();
        let calls = take(&log);
        assert_eq!(calls.len(), 1);
        assert!(calls[0].starts_with("sheet height "), "{calls:?}");
    }
}
