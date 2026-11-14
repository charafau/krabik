# 8. The session and events

This chapter covers `rust/src/host/`: the three traits a platform implements, the `Session` that
owns a running application, how navigation and native chrome are kept in step with application
state, and how a native event finds its message. The second half reads two of the session's tests
line by line. After this chapter you will know what happens between "the platform called an entry
point" and "the platform was asked to do something", and you can add a new piece of chrome or a new
kind of event.

## Why this layer exists

The runtime (chapter 7) renders one tree into one native root view. An application is more than
that:

- It may have a **navigation stack**. Each screen is a separate native root view inside a native
  navigation controller, with its own tree and its own runtime.
- It has **native chrome**: an app bar, a bottom navigation bar, a floating button, a drawer, a
  snackbar, an alert, a bottom sheet. None of these are in the widget tree. The platform places
  and animates them itself.
- It receives **events** from any of those, and each must become one of the application's
  messages.

The logic that coordinates all this is the same on Android and iOS: compare what the application
wants now with what the platform was last told, and send the difference. Before the restructure
each platform had its own copy of that logic, and the two copies had drifted apart. Now there is
one copy, `Session<P>`, generic over a small `Platform` trait. Android and iOS supply only the
parts that touch native objects. A third implementation, a fake inside the test module, lets the
whole layer run in `cargo test`.

## The three traits

All three are in `rust/src/host/platform.rs` and are `pub(crate)`.

### `Platform`: a bundle of types

```rust,ignore
// rust/src/host/platform.rs
pub(crate) trait Platform: Sized {
    /// Whatever a native call needs for the duration of one entry point: the
    /// JNI environment on Android, the main-thread marker on iOS.
    type Cx<'a>;
    /// The error a native call can fail with.
    type Error;
    /// One native root view and its rendering state.
    type Host: Host<Self>;
    /// The native object that owns navigation, the toolbar and modals.
    type Controller: Controller<Self>;
}
```

`Platform` has no methods. It names four types so that `Session<P>` can be written once.

`Cx<'a>` deserves a word. Every call into native code needs something that is only valid for the
duration of the current call from the platform: on Android the `JNIEnv`, on iOS a token proving
the code runs on the main thread. It cannot be stored in the session. So every session method that
may reach native code takes `cx: &mut P::Cx<'_>` and passes it down. In the fake platform it is
`()`.

`Error` is `jni::errors::Error` on Android. On iOS and in the fake it is `Infallible`: calls
cannot fail, and the iOS entry points unwrap the result with a helper named `infallible`.

### `Host`: one native root view

```rust,ignore
// rust/src/host/platform.rs
pub(crate) trait Host<P: Platform> {
    /// What this host shows.
    fn kind(&self) -> &HostKind;
    /// The engine state of this host.
    fn runtime(&self) -> &Runtime;
    fn runtime_mut(&mut self) -> &mut Runtime;
    /// The actions of a widget inside a recycled row. Such widgets are not
    /// part of the runtime's tree, so the host records them when it binds
    /// the row.
    fn row_actions(&self, id: NodeId) -> Option<&Actions>;
    /// The list and item index a recycled-row widget belongs to.
    fn row_of(&self, id: NodeId) -> Option<(NodeId, usize)>;
    /// Reconciles the host with a new description and applies the changes to
    /// the native views.
    fn render(&mut self, cx: &mut P::Cx<'_>, element: &RawElement) -> Result<(), P::Error>;
}
```

A host is a `Runtime` plus the native views it has created. `render` is the one method with real
platform work in it: call `runtime.update(element, viewport)` and apply the returned operations.
The two `row_*` methods exist because recycled rows live outside the runtime's tree (chapter 7);
the session needs them to resolve events from widgets inside such rows.

```rust,ignore
// rust/src/host/platform.rs
pub(crate) enum HostKind {
    /// The main application root.
    Main,
    /// One screen in the native navigation stack, by screen key.
    Screen(String),
    /// The content of the presented bottom sheet.
    Sheet,
}
```

`HostKind` tells the session which part of the application to ask for when it renders a host:
`App::view`, `App::screen(key)`, or `App::sheet`.

### `Controller`: commands for the native chrome

```rust,ignore
// rust/src/host/platform.rs
pub(crate) trait Controller<P: Platform> {
    fn push(&self, cx: &mut P::Cx<'_>, host: u64, route: &Route) -> Result<(), P::Error>;
    fn pop(&self, cx: &mut P::Cx<'_>, count: usize, transition: ScreenTransition) -> Result<(), P::Error>;
    fn set_root_visible(&self, cx: &mut P::Cx<'_>, visible: bool) -> Result<(), P::Error>;
    fn set_chrome(&self, cx: &mut P::Cx<'_>, chrome: &Chrome) -> Result<(), P::Error>;
    fn show_snackbar(&self, cx: &mut P::Cx<'_>, snackbar: &Snackbar) -> Result<(), P::Error>;
    fn dismiss_snackbar(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    fn set_back_enabled(&self, cx: &mut P::Cx<'_>, enabled: bool) -> Result<(), P::Error>;
    fn show_dialog(&self, cx: &mut P::Cx<'_>, dialog: &Dialog) -> Result<(), P::Error>;
    fn dismiss_dialog(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    fn present_sheet(&self, cx: &mut P::Cx<'_>, host: u64) -> Result<(), P::Error>;
    fn dismiss_sheet(&self, cx: &mut P::Cx<'_>) -> Result<(), P::Error>;
    fn sheet_resized(&self, cx: &mut P::Cx<'_>, height: f32) -> Result<(), P::Error>;
}
```

Eleven commands, all one-directional. The trait's doc comment states the rule: "Rust decides what
is shown; the controller only carries it out." The controller holds no application state and makes
no decisions. The three `dismiss_*` methods are documented as "without reporting back": when Rust
closes something, the platform must not send a "was dismissed" event for it, or the session would
react to its own command.

The reverse direction, the user doing something to the chrome, is not on this trait. It arrives
through the platform's entry points as calls to session methods (below).

## `Session<P>`

```rust,ignore
// rust/src/host/session.rs
pub(crate) struct Session<P: Platform> {
    app: Box<dyn AnyApp>,
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
    /// Whether the controller has been given a navigation stack yet.
    navigation_synced: bool,
    /// The appearance every host resolves its colours for.
    scheme: ColorScheme,
}
```

There is one session per Android activity or iOS scene. The fields fall into three groups:

- **The application**: `app`, with its message type erased (chapter 9).
- **The hosts**: `hosts` by numeric id, `pending` for hosts that have been requested but whose
  native view does not exist yet, and `next_host` to allocate ids. Host `0` (`MAIN_HOST`) is the
  main host and exists from the start.
- **The mirror**: `navigation`, `chrome`, `snackbar`, `dialog`, `back_enabled`. Each is "what the
  controller was last told". None of them is the truth about the application; that is always
  obtained by asking `app` again.

The session never stores application state. This is the same principle as "always re-run `view`",
applied to navigation and chrome: the application's methods are the single source of truth, and
the session's fields exist only to avoid sending the platform something it already has.

### How a host comes to exist

Rust cannot create a native root view inside a native navigation controller by itself; the
controller must do that. So creating a screen is a small handshake:

```text
Session                         Controller (native)           entry point
  |  id = next_host++                 |                            |
  |  pending[id] = Screen("detail")   |                            |
  |--- push(id, route) -------------->|                            |
  |                                   | creates the screen and     |
  |                                   | its root view              |
  |                                   |--- "view for host id" ---->|
  |<------------ attach_host(id, build) ---------------------------|
  |  kind = pending.remove(id)                                     |
  |  hosts[id] = build(kind)                                       |
  |<------------ render_all ---------------------------------------|
```

```rust,ignore
// rust/src/host/session.rs
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
```

Three cases: the host exists already (the platform recreated its view, so the host is replaced by
one built around the new view, with the same id and kind); the host is pending (the normal case);
or it is neither, which means the screen was popped before its view arrived, and the view is
ignored. The bottom sheet uses the same handshake with `present_sheet`.

## `render_all`

Almost everything ends in this method.

```rust,ignore
// rust/src/host/session.rs
pub(crate) fn render_all(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
    let has_navigation = self.app.navigation().is_some();
    for host in self.hosts.values_mut() {
        let element = match host.kind() {
            // A navigation app shows its root as the first screen.
            HostKind::Main if has_navigation => None,
            HostKind::Main => Some(self.app.view()),
            HostKind::Screen(key) => self.app.screen(key).map(|(body, _)| body),
            HostKind::Sheet => self.app.sheet().map(|sheet| sheet.into_content()),
        };
        if let Some(element) = element {
            host.runtime_mut().set_color_scheme(self.scheme);
            host.render(cx, &element)?;
        }
    }
    self.sync(cx)
}
```

For every host: ask the application for that host's description, set the colour scheme, render.
Then `sync`.

Things to notice:

- **Every host renders on every call**, including screens that are covered by another screen. The
  session does not try to work out which host a message affected. This is where the runtime's
  fast path earns its keep: an unaffected host builds its tree, finds it equal, and stops.
- **With navigation, the main host is not rendered** and `Component::view` is never called. The
  first route is the root screen.
- **A host whose description is `None` is skipped**, not cleared. For example a screen key the
  application no longer recognizes keeps its last content until it is popped.
- **Rendering happens before `sync`.** A screen that is about to be popped is rendered one last
  time (if the application still returns it) and then its host is dropped in `sync`. That is
  wasted work, kept for the simplicity of the order.
- `hosts` is a `HashMap`, so the order in which hosts render is unspecified.
- An error from one host stops the loop; the remaining hosts and `sync` do not run.

## `sync`: mirroring application state to the controller

```rust,ignore
// rust/src/host/session.rs
fn sync(&mut self, cx: &mut P::Cx<'_>) -> Result<(), P::Error> {
    if self.controller.is_none() {
        return Ok(());
    }
    let navigation = self.app.navigation().unwrap_or_default();
    let navigated = navigation != self.navigation;
    if navigated {
        self.sync_navigation(cx, &navigation)?;
        self.navigation = navigation;
    }
    self.sync_chrome(cx, navigated)?;
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
```

Five steps, each of the same shape: ask the application, compare with the mirror, send the
difference, update the mirror. Until the platform registers a controller with `set_controller`,
`sync` does nothing and only the main host renders.

### Step 1: navigation

The application returns its whole stack as a list of routes (`key` plus `transition`). The session
turns the difference between two stacks into pops and pushes.

```rust,ignore
// rust/src/host/session.rs
fn sync_navigation(&mut self, cx: &mut P::Cx<'_>, screens: &[Route]) -> Result<(), P::Error> {
    // ...
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
    let restoring = !std::mem::replace(&mut self.navigation_synced, true);
    for screen in &screens[common..] {
        let host = self.next_host;
        self.next_host += 1;
        self.pending.insert(host, HostKind::Screen(screen.key.clone()));
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
```

The two stacks share a prefix of `common` screens, compared by key. Everything in the old stack
after the prefix is popped; everything in the new stack after it is pushed.

```text
old:  home  list  detail  photo          common = 2
new:  home  list  settings
                  ^
      pop(2, photo's transition)   then   push(settings)
```

**Multi-pop.** Going from four screens to two is one `pop(2, ..)`, not two pops. The user sees one
animation, that of the screen they were looking at (the top one). Two sequential native pops would
animate twice or fight each other. The hosts of the popped screens are dropped from `hosts` and
`pending`.

**The "restoring" rule.** `navigation_synced` is `false` until the first stack has been sent. On
that first sync, every push is sent with `ScreenTransition::None`. The reason: a new session does
not always start at the root. On Android a theme change or other configuration change recreates
the activity; the Rust application survives (it is handed over with `Session::into_app`), and the
new session finds an application that is already three screens deep. The user did not navigate;
they should see the screen they were on, at once, not three slide animations. Later pushes use
their own transition.

**Replace.** Changing the top screen (`[home, a]` to `[home, b]`) is a pop of one followed by a
push of one. There is no dedicated replace command.

`set_root_visible(screens.is_empty())` tells the controller whether to show the main host (no
navigation) or the stack.

**Keys must be unique within the stack.** Popped hosts are removed by kind,
`HostKind::Screen(key)`, with `retain`. If the same key were on the stack twice (`[home, item,
item]`), popping the top one would remove the hosts of *both*. So the stack is checked where the
application hands it over (`unique_routes` in `rust/src/app.rs`): a debug build panics, a release
build logs an error and ignores the repeated route. An application that shows the same kind of
screen twice makes the keys differ (`item-7`, `item-9`).

### Step 2: chrome and snackbar

```rust,ignore
// rust/src/host/session.rs
fn sync_chrome(&mut self, cx: &mut P::Cx<'_>, navigated: bool) -> Result<(), P::Error> {
    // ...
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
```

`top_chrome` asks the application for the screen on top of the stack and keeps only its `Chrome`
(app bar, bottom navigation, floating button, drawer, snackbar; chapter 9). Chrome belongs to the
top screen: push a screen and the bars change to that screen's; pop it and they change back. An
application without navigation has no chrome at all (`top_chrome` returns the default).

The whole chrome is sent as one value when it differs from the last one sent, *or* after any
navigation change even if equal, "so the native bars never keep a previous screen's state". The
native navigation controller may have changed the bars on its own during the transition; resending
is cheaper than reasoning about it.

The snackbar is taken out and handled separately, because it is an event-like thing with show and
dismiss commands, not a property to set.

Remember that `Chrome` equality treats handlers as equal when both are set (chapter 2). A chrome
that differs only in its closures is not resent. This is correct because of how chrome events are
resolved (below): never from the stored copy.

### Steps 3 to 5: dialog, sheet, back

**Dialog.** If the application's dialog differs from the one showing: dismiss the old one if there
was one, show the new one if there is one. A dialog whose title or buttons change while it is
showing is therefore closed and reopened.

**Sheet.** Only presence is compared: `app.sheet().is_some()` against `has_sheet()`, which looks
for a `Sheet` host or pending host. To show, the session allocates a host id, marks it pending and
calls `present_sheet(id)`; the handshake above attaches the host, and from then on the sheet's
content is rendered like any other host, so its content can change while it is open. When that
changes the height the content needs, `sync_sheet_height` tells the controller with
`sheet_resized(height)`: the iOS sheet is sized once at presentation and has to be resized by
hand, while Android's wraps its content and ignores the call. To hide, `dismiss_sheet` and drop
the host.

**Back.** `App::back()` returns the message to send for the system back action, or `None`. The
session only tells the controller *whether* there is one (`set_back_enabled`), and only when that
changes. When it is disabled, the platform's own back behaviour applies, which on Android includes
the predictive "leaving the app" animation. That is why the flag is pushed ahead of time and the
session is not asked at the moment of the gesture.

The erased `AnyApp::back` has two sources. `App::back` wins when it returns a message; otherwise,
with more than one route on the stack, it is the message given to `Navigation::on_back`.

**A pop the platform made.** The iOS swipe-back gesture removes the top screen before Rust hears
of it. `Session::native_pop` handles that like a dismissed dialog: it drops the top route and its
host from what the session believes is shown, sets a flag, sends the back message and renders.
The flag makes the next `sync` send the chrome even if it compares equal (the bar belongs to
another screen now) and makes a screen the application kept come back without animation.

## Events: from a native callback to a message

### `NodeEvent` and `Actions`

`rust/src/host/events.rs` defines the vocabulary.

```rust,ignore
// rust/src/host/events.rs
pub(crate) enum NodeEvent {
    Click,
    Text(String),
    Submit,
    Focus { focused: bool, attached: bool },
    Toggle(bool),
    Slider(f32),
    Select(usize),
    Picker(DatePickerValue),
    Scroll(f32),
}

pub(crate) struct Actions {
    pub click: Handler<()>,
    pub text: Handler<String>,
    pub submit: Handler<()>,
    pub focus: Handler<bool>,
    pub toggle: Handler<bool>,
    pub select: Handler<usize>,
    pub slider: Option<SliderAction>,
    pub picker: Option<PickerAction>,
}
```

`NodeEvent` is what a native view can report. `Actions` is what one node wants to hear about: one
handler slot per kind of event. The two are deliberately not per-widget. A switch, a checkbox and a
radio button all report `Toggle`; a radio group, a tab bar and a pager all report `Select`. The
platform entry points need one function per event kind, not one per widget.

`Actions::of(node)` collects a node's handlers:

```rust,ignore
// rust/src/host/events.rs
pub(crate) fn of(node: &UiNode) -> Option<Self> {
    let mut actions = Self::default();
    match &node.kind {
        NodeKind::Widget(widget) => match widget {
            Widget::Button(props) => actions.click = props.on_click.clone(),
            Widget::Switch(props) => actions.toggle = props.on_toggle.clone(),
            Widget::Checkbox(props) | Widget::RadioButton(props) => {
                actions.toggle = props.on_toggle.clone();
            }
            // ... RadioGroup, TabBar, DatePicker, Slider, TextField ...
            Widget::Text(_) | Widget::Progress(_) | Widget::Spinner(_) | Widget::Image(_) => {}
        },
        NodeKind::Pager(props) => actions.select = props.on_select.clone(),
        NodeKind::Root | NodeKind::Column | /* ... */ NodeKind::Recycler(_) => {}
    }
    (actions != Self::default()).then_some(actions)
}
```

This is one of the exhaustive matches on `Widget` (chapter 3): a new widget does not compile until
it has an arm here. A node with no handler set returns `None`.

`Actions::message(&event)` picks the slot for the event and calls the handler. Two events are
adjusted first. A slider reports a raw position; `SliderAction` carries the widget's `min`, `max`
and `step`, and the value is snapped with `snap_slider_value` before the handler sees it. A picker
value is clamped to the widget's date and time ranges with `DatePickerValue::clamped`. Doing this
here means both platforms round identically and the application never receives an out-of-range
value. `Scroll` never produces a message: "scrolling is engine state, not an application event."

### `node_event`

```rust,ignore
// rust/src/host/session.rs
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
    // makes of the message, the control is updated from the state.
    let changes_value = matches!(
        event,
        NodeEvent::Toggle(_) | NodeEvent::Text(_) | NodeEvent::Slider(_)
            | NodeEvent::Select(_) | NodeEvent::Picker(_)
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

fn update(&mut self, cx: &mut P::Cx<'_>, message: Message) -> Result<(), P::Error> {
    self.apply(message);
    self.render_all(cx)
}
```

Every widget event from both platforms enters here with three facts: which host, which node, what
happened. Then:

1. **Find the host.** An event for a host that no longer exists (a popped screen whose view is
   still animating away) is dropped.
2. **Update engine state.** Two events change the runtime regardless of the application: a scroll
   may move a list's window (`Runtime::scroll`), and a focus change may change which row is pinned
   (`track_focus`). Each returns whether a re-render is needed.
3. **Resolve the message** with `actions_for`.
4. **Mark a changed control.** A toggle, text change, slider move, selection or picker change
   means the native view already shows a new value. If the event has a message, the runtime is
   told to update that node on the next render even when its description is unchanged
   (`Runtime::reapply`), so an application that refuses the value gets the control set back.
5. **Act.** With a message: `app.update(message)` and `render_all`. Without a message but with an
   engine change: `render_all` only. With neither: nothing. Most scroll events end here.

### `actions_for`: the main tree, then recycled rows

```rust,ignore
// rust/src/host/session.rs
fn actions_for<P: Platform>(host: &P::Host, id: NodeId) -> Option<Actions> {
    tree::find(host.runtime().tree(), id)
        .and_then(Actions::of)
        .or_else(|| host.row_actions(id).cloned())
}
```

The handler is read from the runtime's *current* tree, the one stored at the end of the last
render. This is the late lookup that chapter 2 promised: the native view holds only an id, and the
closure that runs is always the one from the latest `view()`.

If the id is not in the main tree, it may be a widget inside a recycled row. Those rows are built
by the platform outside `Runtime::update`, so their nodes are in no tree the session can see. When
the platform binds a row it runs `Actions::of` over the row's nodes and stores the results keyed
by node id (`RowBinder::finish` in `rust/src/host/rows.rs`, shared by both platforms). `Host::row_actions`
exposes that table. Row node ids come from `tree::row_root(list, index)` (chapter 4), so they
cannot collide with main-tree ids, and one flat map per host is enough.

The recorded actions are refreshed on every rebind, including the "row unchanged" fast path, for
the same reason the main tree is always replaced: the closures of the latest render must be the
ones that run.

### `track_focus`

```rust,ignore
// rust/src/host/session.rs
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
```

This connects focus events to the two focus mechanisms of chapter 7. `Host::row_of(id)` says
whether the field is inside a recycled row.

- **In a recycled row.** A focus gain, or a blur while the field is still attached to the screen
  (`attached`), is a real user action: record it with `set_row_focus`. A blur with
  `attached == false` means the row is being recycled: park the focus. Either way return `false`.
  The doc comment explains: "focus inside a recycled row is bookkeeping only: the widgets on screen
  did not change, and re-rendering would rebind the list and feed back into focus handling."
- **Not in a known row, but it is the field the row focus points at, and this is a blur.** The row
  was already unbound when the blur arrived. Park.
- **Otherwise** it is a main-tree field: `Runtime::set_focus`, whose result says whether the
  pinned row may have changed.

### Chrome events and the "platform dismissed it" rule

Events from the native chrome have their own session methods, one per source:

| Method | Called when | Handler read from |
|---|---|---|
| `back(cx)` | system back | `app.back()` |
| `toolbar_action(cx, index)` | an app bar action was tapped | `top_chrome().app_bar.actions[index].on_tap` |
| `bottom_navigation_selected(cx, index)` | a bottom bar item was picked | `top_chrome().bottom_navigation.on_select` |
| `chrome_event(cx, ChromeEvent)` | floating button, drawer, snackbar | `top_chrome()` |
| `dialog_closed(cx, button)` | the alert closed | `app.dialog()` |
| `sheet_dismissed(cx)` | the user swiped the sheet away | `app.sheet().on_dismiss` |

All of them read the handler by asking the application again, never from the session's mirror. The
comment in `toolbar_action` says so: "handlers are read from the application's current state,
never from the copy last sent to the platform." The mirror (`self.chrome`) is only replaced when
the chrome compares unequal, and handler equality ignores the closure, so the mirror can hold
closures from many renders ago.

`ChromeEvent` is a small enum (`Fab`, `DrawerSelected(usize)`, `DrawerOpened`, `DrawerClosed`,
`SnackbarAction`, `SnackbarDismissed`) with a `from_code(kind, index)` decoder, because both
platforms report these through one entry point with two integers.

Now the rule. Some native things close themselves: the user swipes a sheet down, taps outside an
alert, a snackbar times out, a drawer is dragged shut. At that moment three parties disagree:

```text
the platform        shows nothing (it closed it)
the session mirror  says "showing"
the application     says "show it" (its state has not changed yet)
```

If the session did nothing special, the next `sync` would compare the application ("show")
against the mirror ("showing"), see no difference, and send nothing. If the application's handler
for the dismissal does not clear the state, the three stay out of step: the application believes
the dialog is up, and it is not.

So each of these methods does three things in a fixed order. `dialog_closed` is the template:

```rust,ignore
// rust/src/host/session.rs
/// The platform has already closed the alert, so the session forgets it
/// first: if the application still returns a dialog, it is shown again.
pub(crate) fn dialog_closed(&mut self, cx: &mut P::Cx<'_>, button: Option<usize>) -> Result<(), P::Error> {
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
```

1. **Make the mirror match reality**: `self.dialog = None`.
2. **Tell the application**, if it has a handler. A platform dismissal with no button (`None`)
   counts as the cancel button.
3. **`render_all`**, always, even without a message. Its `sync` now compares the application
   against the true state of the platform.

The outcome is right in both cases. If the application cleared its state, the dialog is `None` on
both sides and nothing is sent; in particular no `dismiss_dialog` is sent for something already
gone. If the application still returns the dialog, it differs from the mirror's `None`, and it is
shown again. The application's state wins, as it should: an application that returns a dialog
with no cancel button means "this must be answered".

The same pattern for the others:

- `sheet_dismissed`: `forget_sheet()` drops the sheet host, then message, then `render_all`.
- `ChromeEvent::SnackbarAction` / `SnackbarDismissed`: `self.snackbar = None`, then message, then
  `render_all`.
- `ChromeEvent::DrawerOpened` / `DrawerClosed`: the mirror's `drawer.open` is set to what the
  platform now shows, then the `on_open` or `on_close` message, then `render_all`. If the
  application agrees, the chrome equals the mirror and nothing is resent; if it insists on the
  other state, the chrome differs and is sent, which moves the drawer back.

**Rough edge.** The bottom navigation bar does not get this treatment. On Android the native bar
selects the tapped item by itself (the listener in `KrabikBottomNavigation.kt` returns `true`).
`bottom_navigation_selected` does not record that in the mirror. If the application does not
change `selected` in response (or sets no handler), the chrome compares equal and is not resent,
and the bar keeps showing the tapped item while the application says otherwise. The same method,
and `toolbar_action`, also return without rendering when there is no handler.

## The fake platform

The test module of `session.rs` implements `Platform` without any native code:

```rust,ignore
// rust/src/host/session.rs, mod tests
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

/// Records what the session asks the platform to do.
type Log = Rc<RefCell<Vec<String>>>;
struct FakeController(Log);
```

- `FakeHost::render` runs the real `Runtime::update` with a 320 by 640 viewport and
  `ApproximateMeasure`, throws the operations away, and counts the call. So the engine really runs;
  only the native views are missing.
- `FakeController` implements every command by appending a line to a shared log: `"push 2 detail"`,
  `"pop 2 Slide"`, `"chrome HOME fab:Add"`, `"dialog Sure?"`. A push with
  `ScreenTransition::None` is logged as `"push 1 home instantly"`.
- `TestApp` is an application whose state sits behind an `Arc<Mutex<State>>`, so a test can change
  it from outside, "the way a timer or network reply would", and then call `render_all`.
- `attach_pending` plays the native side of the handshake: it attaches a `FakeHost` for every
  pending host and renders.
- `take(&log)` returns the log lines so far and empties the log.

A test is then a script: set state, call a session method, compare the log with the expected
commands. Because the log holds *commands*, the tests check exactly what this layer is responsible
for: sending the right things, in the right order, and nothing else.

## Two tests as documentation

### `leaving_several_screens_is_one_pop`

```rust,ignore
// rust/src/host/session.rs, mod tests
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
```

**Setup.** The application starts three screens deep. The first `render_all` renders nothing
(there is only the main host, and with navigation it is skipped), then `sync` runs. The mirror's
stack is empty, so the common prefix is zero and three screens are pushed, all "instantly" because
this is the first sync (the restoring rule). The chrome of `b` is sent, and since the stack is
longer than one, `back true`. `attach_pending` creates the three hosts and renders them. `take`
discards all of that.

**The change.** The test truncates the application's stack to `["home"]` directly, as if a
message had done it, and calls `render_all`.

**What happens.**

1. The loop renders the hosts. `home`, `a` and `b` all still get a description from
   `TestApp::screen`, so all three render; nothing in them changed, so each takes the fast path.
2. `sync`: the application's stack `[home]` differs from the mirror `[home, a, b]`.
3. `sync_navigation`: the common prefix is 1. `popped` is `[a, b]`. Its last element is `b`, whose
   transition is the default `Slide`. One command: **`pop 2 Slide`**. The hosts for `a` and `b`
   are removed. Nothing to push. **`root false`**: the stack is not empty.
4. `sync_chrome(navigated = true)`: the top screen is now `home`. Sent because of the navigation
   change: **`chrome HOME`**.
5. Dialog and sheet: none before, none now.
6. Back: `TestApp::back` returns a message only when the stack is longer than one. It was enabled,
   now it is not: **`back false`**.

The four log lines are exactly those, in that order. The final assertion checks the bookkeeping:
two hosts remain, main and `home`.

The test documents three things in a dozen lines: multi-pop is one command, the pop uses the top
screen's transition, and the back flag follows the stack.

### `a_dialog_the_app_keeps_is_shown_again_after_it_closes`

```rust,ignore
// rust/src/host/session.rs, mod tests
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
```

`TestApp::dialog` returns `Dialog::new("Sure?").cancel("No", Msg::CloseDialog)` while
`state.dialog` is true. Its `update` handles `Msg::CloseDialog` with
`state.dialog = state.sticky_dialog`. So with `sticky_dialog` set, the application *refuses* to
close the dialog.

**First render.** `sync` finds the application's dialog (`Some`) different from the mirror
(`None`). `sync_dialog` has nothing to dismiss and sends `dialog Sure?`. The mirror now holds the
dialog.

**The platform closes it.** `dialog_closed(None)` models Android's back button or a tap outside.

1. `self.dialog = None`: the mirror now matches the platform.
2. The application still returns the dialog. `button` is `None`, so the cancel button is used; its
   handler yields `Msg::CloseDialog`.
3. `app.update(CloseDialog)` sets `state.dialog = sticky_dialog = true`. Still open.
4. `render_all`, then `sync`: application `Some`, mirror `None`. Different. `sync_dialog` sees an
   empty mirror, so no dismiss; it sends **`dialog Sure?`** again.

The log is exactly that one line. No `dismiss dialog` precedes it, because the session knew the
alert was already gone.

**The application lets go.** With `sticky_dialog` false, the same call goes: mirror cleared,
`CloseDialog` delivered, `state.dialog = false`, `render_all`, `sync` compares `None` with `None`.
Nothing is sent. The log is empty.

Now imagine step 1 missing. In the sticky case, `sync` would compare the application's dialog with
an equal dialog in the mirror and send nothing: the alert would be gone from the screen while the
application believes it is up. In the non-sticky case, `sync` would see `None` against `Some` and
send `dismiss dialog` to a platform that has no dialog. The one line `self.dialog = None` prevents
both.

The neighbours of this test repeat the pattern for the sheet
(`a_sheet_can_be_reopened_after_a_native_dismissal`), the snackbar
(`a_snackbar_the_application_keeps_is_shown_again`) and the drawer
(`the_drawer_is_opened_by_state_and_follows_the_user`).

## Costs to be aware of

Per message, with a stack of `n` screens:

- `App::navigation` is called three times (in `render_all`, in `sync`, in `top_chrome`).
- `App::screen` is called `n + 1` times: once per screen host, and once more for the top screen
  in `top_chrome`, which builds the whole scaffold including the body and keeps only the chrome.
- Every chrome event method calls `top_chrome` once more before the render.
- `App::dialog`, `App::sheet` (twice: once to render the sheet host if there is one, once in
  `sync`) and `App::back` are each called.

These are all pure functions of the application state, so the repetition is safe. It is also not
free for a screen with a large body. Caching one frame's results would be the obvious
optimization; nothing does it today.

## Where this lives

- `rust/src/host/platform.rs`: `Platform`, `Host`, `Controller`, `HostKind`, `ChromeEvent`.
- `rust/src/host/events.rs`: `NodeEvent`, `Actions`, `SliderAction`, `PickerAction`.
- `rust/src/host/session.rs`: `Session` and, from `mod tests` on, the fake platform and fourteen
  tests. Read `session()`, `attach_pending()` and `take()` first; every test uses them.
- `rust/src/host/mod.rs`: the re-exports and a summary.
- The real `Platform` implementations and the entry points that call the session:
  `rust/src/android/` and `rust/src/ios/`, described in [`../platforms/`](../platforms/).

Next: [9. App, navigation, scaffold, presentation](09-app-navigation-scaffold-presentation.md).
