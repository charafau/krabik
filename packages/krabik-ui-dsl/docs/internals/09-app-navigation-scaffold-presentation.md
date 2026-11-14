# 9. App, navigation, scaffold, presentation

This chapter covers the files an application author sees: `rust/src/app.rs`, `navigation.rs`,
`scaffold.rs`, `presentation.rs`, `style.rs`, `ui.rs` and `lib.rs`. For each public type it shows
the typed form the application writes and the erased form the session works with, and where one
becomes the other. After reading it you will know the complete public surface, how the application
object is created and held, and how colours follow the light and dark appearance.

## The public surface in one example

This is a complete application as far as the core is concerned. (A real one also needs the two
small platform entry points that call `set_app_factory`; those are in
[`../platforms/`](../platforms/).)

```rust
use krabik_ui_dsl::app::{App, Component};
use krabik_ui_dsl::ui::{Button, Column, Element, Text};

#[derive(Clone)]
enum Msg {
    Add,
}

struct Counter {
    count: i32,
}

impl Component for Counter {
    type Msg = Msg;

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Add => self.count += 1,
        }
    }

    fn view(&self) -> Element<Msg> {
        Column::new()
            .child(Text::new(format!("{}", self.count)))
            .child(Button::new("+1").on_click(Msg::Add))
            .into()
    }
}

impl App for Counter {}
```

Two traits. `Component` is the state machine; `App` adds the optional hooks for native chrome, and
here uses all the defaults.

## `lib.rs`: what is public and what is not

```rust,ignore
// rust/src/lib.rs (some comments trimmed)
// Lets `view!`, which names this crate by its full path, work inside it too.
extern crate self as krabik_ui_dsl;

pub mod app;
pub(crate) mod element;
pub mod engine;
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
mod host;
mod message;
pub mod navigation;
pub mod presentation;
pub mod scaffold;
pub mod style;
pub mod ui;
pub mod widgets;

#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
mod animation;
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod frame_batch;

pub use app::{App, Component, set_app_factory};
pub use krabik_ui_dsl_macros::view;
// The engine modules keep their short public paths, e.g. `krabik_ui_dsl::runtime`.
pub use engine::{geometry, layout, measure, render, runtime, tree};

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
```

Reading the visibility:

- **Public for applications**: `app`, `navigation`, `presentation`, `scaffold`, `style`, `ui`,
  `widgets`, and the `view!` macro.
- **Public but not for applications**: `engine` and its six short aliases. The engine is public so
  it can be inspected and tested from outside, and the aliases explain why engine files import
  `crate::geometry::Size` and not `crate::engine::geometry::Size`.
- **Private**: `element` and `message` (their public types are re-exported through `ui`), `host`,
  `animation`, `frame_batch`, and the two platform modules.

`host`, `animation` and `frame_batch` are compiled on every target although only device builds use
them. The `cfg_attr(..., allow(dead_code))` lines silence the warnings that would cause on a
laptop. The reason is stated next to each: "it stays compiled everywhere so its tests run on the
host."

`extern crate self as krabik_ui_dsl` lets code inside the crate refer to the crate by its external
name. The `view!` macro expands to paths starting with `krabik_ui_dsl::`, and without this line it
could not be used in the crate's own tests.

## `ui.rs`: a facade

```rust,ignore
// rust/src/ui.rs
pub(crate) use crate::element::ElementKind;
pub use crate::element::{Element, RawElement};
pub use crate::message::Handler;
pub use crate::style::*;
pub use crate::widgets::*;
```

Five lines of re-exports and four hundred lines of tests. Applications import everything they need
to describe a UI from `krabik_ui_dsl::ui`. The definitions live elsewhere; this file only decides
what the vocabulary is. The tests are builder-level checks ("a slider's step is optional", "the
`view!` macro builds the same tree as the builders") that do not belong to any single widget file.

## `app.rs`

### `Component` and `App`

```rust,ignore
// rust/src/app.rs
pub trait Component {
    /// Everything that can happen to this component.
    type Msg: 'static;

    /// Applies a message to the state. The UI is rebuilt afterwards.
    fn update(&mut self, msg: Self::Msg);

    /// Describes the UI for the current state.
    fn view(&self) -> Element<Self::Msg>;
}

pub trait App: Component + Send + 'static {
    fn navigation(&self) -> Option<Navigation> { None }
    fn screen(&self, _key: &str) -> Option<Scaffold<Self::Msg>> { None }
    fn dialog(&self) -> Option<Dialog<Self::Msg>> { None }
    fn sheet(&self) -> Option<BottomSheet<Self::Msg>> { None }
    fn back(&self) -> Option<Self::Msg> { None }
}
```

`Component` is deliberately tiny and has no connection to the framework's runtime. Nothing calls a
child component automatically. A parent holds its child as a field, calls `child.view()` inside
its own `view` (with `.map(Msg::Child)`, chapter 2) and `child.update(msg)` inside its own
`update`. The trait gives that convention a name and a shape. `Msg: 'static` is required because
messages are boxed as `dyn Any`.

`App` is the root component plus five hooks, all with defaults. Every hook takes `&self` and
returns a description. None of them can change state; only `update` can. That is what allows the
session to call them as often as it likes (chapter 8).

| Hook | Returns | The session uses it to |
|---|---|---|
| `view` (from `Component`) | the UI when there is no navigation | render the main host |
| `navigation` | the whole route stack | diff against the native stack; push and pop |
| `screen(key)` | one screen: body and chrome | render that screen's host; set the bars |
| `dialog` | the alert, while it should be up | show or dismiss the system alert |
| `sheet` | the bottom sheet, while it should be up | present or dismiss; render its content |
| `back` | the message for system back, or `None` | enable or disable back handling; deliver it |

With navigation, `view` is not shown; the comment on `navigation` says "the first screen is the
root". An application with navigation still has to implement `view` because `Component` requires
it. The showcase's `view` carries the comment "Not shown: with navigation, the first screen is the
root".

`App` requires `Send + 'static` because the session, which owns the application, is stored in a
global table shared with platform entry points.

### `AnyApp`: the application with its type erased

`Session` holds a `Box<dyn AnyApp>`. It cannot hold a `Box<dyn App>`, because `App` has an
associated type and methods that mention it. `AnyApp` is the same interface with every `Msg`
removed:

```rust,ignore
// rust/src/app.rs
pub(crate) trait AnyApp: Send {
    fn view(&self) -> RawElement;
    fn update(&mut self, message: Message);
    fn navigation(&self) -> Option<Vec<Route>>;
    fn screen(&self, key: &str) -> Option<(RawElement, Chrome)>;
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
        App::navigation(self).map(|navigation| navigation.routes)
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
        App::back(self).map(|message| Box::new(message) as Message)
    }
}
```

This blanket implementation is the single boundary between typed and untyped code. Every method is
one line that calls the typed hook and erases the result:

| Typed | Erased | By |
|---|---|---|
| `Element<Msg>` | `RawElement` | `Element::into_raw` |
| `Scaffold<Msg>` | `(RawElement, Chrome)` | `Scaffold::erase` |
| `Dialog<Msg>` | `Dialog` (that is, `Dialog<()>`) | `Dialog::erase` |
| `BottomSheet<Msg>` | `BottomSheet<()>` | `BottomSheet::erase` |
| `Msg` (from `back`) | `Box<dyn Any>` | `Box::new` |
| `Box<dyn Any>` (into `update`) | `Msg` | `downcast` |

Nothing is converted at run time in the first four rows. The handlers inside these values were
already erased when the builder methods created them (chapter 2); the `M` parameter is a marker.
`erase` and `into_raw` only change the type the compiler sees. The last row is the one place a
type is recovered.

The method names collide on purpose (`view` on both traits), which is why the bodies use the
qualified form `Component::view(self)` and `App::dialog(self)`.

### The app factory

```rust,ignore
// rust/src/app.rs
type AppFactory = Box<dyn Fn() -> Box<dyn AnyApp> + Send + Sync>;

static APP_FACTORY: OnceLock<AppFactory> = OnceLock::new();

pub fn set_app_factory<A: App>(factory: impl Fn() -> A + Send + Sync + 'static) {
    let _ = APP_FACTORY.set(Box::new(move || Box::new(factory())));
}

pub(crate) fn create_app() -> Box<dyn AnyApp> {
    let factory = APP_FACTORY.get().expect(
        "krabik: no application registered; call set_app_factory before creating a root view",
    );
    factory()
}
```

The library is linked into a native app whose entry point is not Rust's `main`. The first Rust code
that runs is a platform entry point inside `krabik-ui-dsl`, which has no way to know the
application's type. So the application crate registers a constructor in a process-wide slot, and
the library calls it when a native root view is created.

- It is a *factory*, not an instance, because one process can have several activities or scenes.
  As the doc says: "each activity or scene then gets its own instance."
- The typed factory is wrapped in a closure that boxes its result as `dyn AnyApp`. This is where
  the application's concrete type disappears.
- `OnceLock::set` fails if the slot is already filled, and `let _ =` discards that. A second call
  to `set_app_factory` is silently ignored: the first registration wins. That is harmless when the
  same application registers twice (an entry point called more than once) and surprising if two
  different ones were ever registered.
- `create_app` panics with an explanatory message if nothing was registered. It is `pub(crate)`
  and called only by the platform modules.

The showcase registers itself like this (`example/rust/src/lib.rs`):

```rust,ignore
pub fn install() {
    set_app_factory(Self::new);
}
```

`install` is then called from two small exported functions in the same file, one with the C ABI
for the iOS shell and one JNI function for the Android shell.

## `navigation.rs`

```rust,ignore
// rust/src/navigation.rs
pub enum ScreenTransition { Slide, Fade, None }      // default: Slide

pub struct Route {
    /// A stable identifier used to match the screen across renders.
    pub key: String,
    /// The native transition used when this screen is pushed.
    pub transition: ScreenTransition,
}

pub struct Navigation {
    pub routes: Vec<Route>,
}
```

A navigation stack is a list of routes, root first. A route is a key and a transition. It contains
no title and no content: those come from `App::screen(key)`, built on demand. The comment at the
top of the file states the ownership: "the application owns the route stack; the runtime reads
`Navigation` and mirrors it with native navigation controllers."

In practice the application keeps a `Vec` of keys in its state and maps it:

```rust,ignore
// example/rust/src/lib.rs
fn navigation(&self) -> Option<Navigation> {
    Some(Navigation::new(self.stack.iter().map(|key| {
        Route::new(*key).transition(match *key {
            "transition-fade" => ScreenTransition::Fade,
            "transition-none" => ScreenTransition::None,
            _ => ScreenTransition::Slide,
        })
    })))
}
```

Pushing a screen is `self.stack.push("detail")` in `update`. Popping is `self.stack.pop()`. There
is no `navigate()` call anywhere: the session sees a different list on the next render and issues
the native pushes and pops (chapter 8).

`Navigation::new` takes anything that iterates over things convertible into a `Route`, and there
are `From` implementations for `&str`, `String`, `&String` and `&&str`, so
`Navigation::new(["home", "detail"])` and `Navigation::new(&self.stack)` both work.

Navigation carries no messages, so it has no type parameter and nothing to erase;
`AnyApp::navigation` only unwraps the `Vec<Route>`.

`ScreenTransition::code()` returns the integer (0, 1, 2) the native controllers receive. It is the
only place the numbering is defined on the Rust side.

Two things to keep in mind, both from chapter 8: the key identifies a screen within the stack, so
keys should be unique in a stack; and `screen(key)` is called for every screen on the stack on
every render, not only the top one.

The file also defines `BottomNavigation<M>` and `BottomNavItem`. They are chrome and are
re-exported from `scaffold`, which is where applications import them from, although they are
defined in this file.

## `scaffold.rs`

`App::screen` returns a `Scaffold`: one screen's body and everything native around it.

```rust,ignore
// rust/src/scaffold.rs
pub struct Scaffold<M = ()> {
    pub app_bar: Option<AppBar<M>>,
    pub body: Element<M>,
    pub bottom_navigation: Option<BottomNavigation<M>>,
    pub floating_action_button: Option<Fab<M>>,
    pub drawer: Option<Drawer<M>>,
    pub snackbar: Option<Snackbar<M>>,
}
```

```text
+--------------------------------+
| app bar            [action]    |  AppBar: title, subtitle, actions
+--------------------------------+
|                                |
|  body                          |  Element<M>: rendered by the widget
|  (Rust-rendered widgets)       |  tree in this screen's host
|                                |
|                        ( + )   |  Fab
|  [ snackbar message   UNDO ]   |  Snackbar
+--------------------------------+
|  home     search     profile   |  BottomNavigation
+--------------------------------+
   drawer slides over from the leading edge
```

Only the body is drawn by the engine. Everything else is *chrome*: drawn, placed and animated by
the platform, described by plain data. The module comment gives the reason: these pieces belong
"to the host rather than to the widget tree". A native app bar animates with the navigation
transition; a bottom bar sits above the system gesture area; a snackbar times out by itself. A
widget inside the scrollable body could do none of that properly.

### The slot setters accept `Option`

```rust,ignore
// rust/src/scaffold.rs
pub fn snackbar(mut self, snackbar: impl Into<Option<Snackbar<M>>>) -> Self {
    self.snackbar = snackbar.into();
    self
}
```

Every slot takes `impl Into<Option<X>>`. Both `X` and `Option<X>` convert into `Option<X>`, so the
two common forms read naturally:

```rust,ignore
Scaffold::new()
    .app_bar(AppBar::new("Inbox"))                                        // always
    .snackbar(self.deleted.then(|| Snackbar::new("Deleted")))             // while state says so
```

The second form is how chrome becomes state-driven: the snackbar is in the scaffold exactly while
`self.deleted` is true.

A body alone is also a screen: `impl<M> From<Element<M>> for Scaffold<M>` lets a hook return
`Some(self.page().into())`.

### `Scaffold::erase` and `Chrome`

```rust,ignore
// rust/src/scaffold.rs
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

/// The native chrome of a screen, with the message type erased.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Chrome {
    pub app_bar: Option<AppBar>,
    pub bottom_navigation: Option<BottomNavigation>,
    pub floating_action_button: Option<Fab>,
    pub drawer: Option<Drawer>,
    pub snackbar: Option<Snackbar>,
}
```

`erase` splits the scaffold along the line between the two rendering systems. The body goes to the
screen's host and through the engine. The `Chrome` goes to the session, which compares it with the
last one sent and forwards it to the controller.

`Chrome` derives `PartialEq`, and that one derive is the chrome "diff": the session resends the
whole value when it is not equal to the previous one. Chrome is small, so there is no finer
granularity. `Default` is "no chrome", which a controller must treat as "hide everything".

### The erase pattern

Every chrome type uses the same construction. `Fab` is the shortest:

```rust,ignore
// rust/src/scaffold.rs
pub struct Fab<M = ()> {
    pub label: String,
    pub icon: Icon,
    pub on_click: Handler<()>,
    marker: PhantomData<fn() -> M>,
}

impl<M> Fab<M> {
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
```

- The struct has a type parameter `M` that defaults to `()` and appears only in a `PhantomData`
  marker. The handler field is already `Handler<()>`: a payload type, not a message type.
- The builder method that sets a handler is the only place `M` is used: it takes an `M` and stores
  an erased closure.
- `erase` moves every field into the same struct with `M = ()`. Written `Fab` in the return type,
  thanks to the default.

So `Fab<Msg>` and `Fab` are the same data. The parameter exists so that
`Scaffold<Msg>::floating_action_button` only accepts a button built with the right message type.
Unlike widgets, there is no separate properties struct and builder; one struct plays both parts,
because chrome is not stored in a tree of mixed kinds.

`AppBar`, `Drawer`, `Snackbar`, `BottomNavigation`, `Dialog` and `BottomSheet` follow the same
pattern. Their data, in short:

- `AppBar`: `title`, `subtitle`, `back_title`, and `actions`, each an `AppBarAction` with a label,
  an `Icon` and an `on_tap` handler. The back button is not described here; the platform shows
  it, and `back_title` only sets its label "where the platform shows one".
- `Fab`: label, icon, `on_click`.
- `Drawer`: `items` (label and icon), `selected`, `open`, and three handlers: `on_select(index)`,
  `on_open`, `on_close`. `open` is application state; the two handlers are how the application
  learns that the user opened or closed it by gesture.
- `Snackbar`: `message`, an optional `action` label with `on_action`, and `on_dismiss` for the
  timeout or swipe.
- `BottomNavigation`: `items`, `selected`, `on_select(index)`.

Honest platform notes from the doc comments: Android has native controls for all of them. iOS has
no floating action button, drawer or snackbar, so the iOS shell shows "a filled capsule button",
"a side panel with the same behaviour" and "a small banner with the same behaviour".

## `presentation.rs`

### `Dialog`

```rust,ignore
// rust/src/presentation.rs
pub struct Dialog<M = ()> {
    pub title: String,
    pub message: Option<String>,
    pub buttons: Vec<DialogButton>,
    marker: PhantomData<fn() -> M>,
}

pub struct DialogButton {
    pub label: String,
    pub role: DialogRole,       // Default, Cancel, Destructive
    pub on_tap: Handler<()>,
}
```

A dialog is the system alert: `UIAlertController` on iOS, a Material alert dialog on Android. It
is data only; the platform draws it. Buttons are added with `.button(label, msg)`,
`.cancel(label, msg)` and `.destructive(label, msg)`, which differ in the `DialogRole` they
record. The role tells the platform how to style and place the button.

The cancel role has a second job. `cancel_button()` finds the first button with that role, and the
session uses its message when the platform closes the dialog without a button press
(`dialog_closed(None)`, chapter 8). `is_cancelable()` is "has a cancel button", and the platform
allows dismissal by back button or outside tap only when it is true. A dialog without a cancel
button can only be closed by one of its buttons.

The lifecycle is state-driven, in the words of the `App::dialog` doc: "it stays up while this
returns `Some`; a button sends its message and the application closes the alert by returning
`None` afterwards."

The struct doc says "up to three buttons". Nothing in the builder enforces that limit.

### `BottomSheet`

```rust,ignore
// rust/src/presentation.rs
pub struct BottomSheet<M = ()> {
    pub on_dismiss: Handler<()>,
    content: RawElement,
    marker: PhantomData<fn() -> M>,
}

pub fn new() -> Self {
    let content = Column::<M>::new().padding(20).spacing(12).width_percent(1.0);
    Self {
        on_dismiss: Handler::none(),
        content: Element::from(content).into_raw(),
        marker: PhantomData,
    }
}

pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
    self.content.children.push(child.into().into_raw());
    self
}
```

A sheet is half chrome, half content. The surface, corners, dimming and drag handle are native.
The content is an ordinary element tree: a padded `Column` that `child` appends to. The session
renders that content into its own host (`HostKind::Sheet`) through a normal runtime, and the
native sheet asks `Runtime::content_height` how tall to be (chapter 7).

`on_dismiss` is the message for a native dismissal (swipe down, tap on the scrim, back). The
application is expected to clear the state that makes `sheet()` return `Some`; if it does not, the
session presents the sheet again (chapter 8).

`BottomSheet` has two conversions out of the typed form: `erase` (keeps the handler, used by
`sync` and `sheet_dismissed`) and `into_content` (keeps only the element, used by `render_all`).

## `style.rs`

This file holds every value type shared by widgets. Most are plain data with obvious meanings:
`Length` (`Auto`, `Points`, `Percent`), `EdgeInsets`, `Align`, `Justify`, `FontWeight`,
`TextAlign`, `Border`, `CornerRadii`, `Gradient`, `Animation`, `Transition`, `AnimationCurve`,
`Icon`, and `Style`, the struct every element carries. Chapters 4 to 6 used them. Three things
deserve an explanation here.

### `Color` and `ColorScheme`

```rust,ignore
// rust/src/style.rs
pub enum ColorScheme { Light, Dark }

pub struct Color {
    /// The ARGB value in the light appearance.
    pub light: u32,
    /// The ARGB value in the dark appearance.
    pub dark: u32,
}

impl Color {
    pub const fn fixed(argb: u32) -> Self { Self { light: argb, dark: argb } }
    pub const fn adaptive(light: u32, dark: u32) -> Self { Self { light, dark } }

    /// The colour with the value for `scheme` in both appearances.
    pub const fn resolve(self, scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::fixed(self.light),
            ColorScheme::Dark => Self::fixed(self.dark),
        }
    }

    /// The ARGB value of a colour the engine has resolved.
    pub const fn argb(self) -> u32 {
        self.light
    }
}

impl From<u32> for Color {
    fn from(argb: u32) -> Self { Self::fixed(argb) }
}
```

A colour is always a pair. There is no enum with a "fixed" and an "adaptive" case; a fixed colour
is a pair of equal values. That keeps `Color` a `Copy` type with trivial equality.

The life of a colour:

1. **In the application** it is written as a number, a named constant or an explicit pair. Every
   builder that takes a colour accepts `impl Into<Color>`, so `.background(0xFF21_96F3)` compiles
   through `From<u32>`.
2. **In the element tree** it is stored unresolved.
3. **In `Runtime::build`**, `tree::resolve_colors` replaces every colour in the node tree with
   `color.resolve(scheme)`: a pair whose two values are both the one for the current appearance.
4. **In the diff**, a colour that resolved differently from last time makes its node unequal and
   produces an `Update`.
5. **In the renderer**, `color.argb()` reads the value. It returns `self.light`, which is correct
   because after step 3 both fields are equal.

```rust
use krabik_ui_dsl::ui::{Color, ColorScheme};

fn main() {
    let card = Color::adaptive(0xFFF4_F4F4, 0xFF1E_1E1E);
    let in_dark = card.resolve(ColorScheme::Dark);
    assert_eq!(in_dark, Color::fixed(0xFF1E_1E1E));
    assert_eq!(in_dark.argb(), 0xFF1E_1E1E);

    // A plain number is the same in both appearances.
    let brand = Color::from(0xFF21_96F3_u32);
    assert_eq!(brand.resolve(ColorScheme::Dark), brand);
}
```

Why resolve in the engine and not in the renderers? Three reasons. A theme change needs no code of
its own: it is a diff like any other, and only the nodes whose colours differ are touched. Both
renderers stay ignorant of schemes. And it is testable on a laptop
(`a_new_color_scheme_updates_only_the_nodes_whose_colors_change`).

**Rough edge.** `argb()` on a colour that has *not* been resolved returns the light value without
complaint. Inside the renderers that cannot happen. Anyone reading a colour from an element (not a
node), for instance in a new piece of chrome, would get the light value in dark mode. Chrome
carries no colours today, so the question has not come up.

The named colours are constants with a light and a dark value:

| Constant | Meant for |
|---|---|
| `Color::TEXT` | default text |
| `Color::SECONDARY_TEXT` | captions, hints |
| `Color::BACKGROUND` | the page background |
| `Color::SURFACE` | a card or field on the background |
| `Color::OUTLINE` | borders and dividers |
| `Color::ACCENT` | sliders, progress indicators, spinners |

Widget defaults use them (`#[widget(default = Color::TEXT)]`), which is why an application that
sets no colours at all still follows dark mode.

How the scheme arrives: the platform tells the session (`Session::set_color_scheme`), the session
sets it on every host's runtime before each render, and the platform then calls `render_all`. On
Android a theme switch also recreates the activity so that the Material chrome re-themes; the Rust
application is retained across that and handed to the new session. On iOS the appearance change is
reported to the existing session.

`dim_color(color, enabled)` multiplies the alpha of both values by `DISABLED_ALPHA` (0.4) when a
node is disabled. It is a helper for the renderers, for decoration the framework paints itself;
native controls dim themselves.

### `IntoPoints`

```rust,ignore
// rust/src/style.rs
pub trait IntoPoints {
    fn into_points(self) -> f32;
}
impl IntoPoints for f32 { /* ... */ }
impl IntoPoints for i32 { /* ... */ }
```

This is why `.padding(16)` and `.padding(16.0)` both compile. There is exactly one integer type
and one float type with an implementation, so the compiler infers `i32` for the literal `16` and
`f32` for `16.0`. A parameter of type `impl Into<f32>` would not accept `16`, because the standard
library has no lossless conversion from `i32` to `f32`.

### `Style` and `Icon`

`Style` is the struct of everything `style_builder!` can set (chapter 2). Two methods on it matter
to the engine: `needs_view()`, which decides whether a container gets a native view (chapter 4),
and `resolve_colors(scheme)`, which resolves `background`, both ends of `gradient`, and
`border.color`.

`Icon` names a platform image without the core knowing what images exist. It has three optional
names: `android` (a drawable resource), `ios` (an SF Symbol or asset) and `shared` (a fallback for
both). `android_name()` and `ios_name()` return the specific name or the shared one. Icons are only
used by chrome (app bar actions, bottom navigation, floating button, drawer items).

## Summary: where each typed thing is erased

```text
application code          boundary                    session / engine
----------------          --------                    ----------------
Button::on_click(Msg)   -- Handler::new ----------->  Handler<()>
Element<Msg>            -- Element::into_raw ------>  RawElement
child.view().map(f)     -- Handler::map (Wrap) ---->  handlers, wrapped
Scaffold<Msg>           -- Scaffold::erase -------->  (RawElement, Chrome)
Dialog<Msg>             -- Dialog::erase ---------->  Dialog
BottomSheet<Msg>        -- BottomSheet::erase ----->  BottomSheet
App::back() -> Msg      -- Box::new --------------->  Message
impl App for T          -- impl AnyApp for A: App ->  Box<dyn AnyApp>
fn() -> T               -- set_app_factory -------->  AppFactory
                                                           |
update(Msg)             <- downcast::<A::Msg>() ----  Message
```

Everything to the right of the boundary column is free of type parameters for messages. That is
the property that lets one `Session`, one engine and two renderers serve every application.

## Where this lives

- `rust/src/lib.rs`: visibility and re-exports.
- `rust/src/app.rs`: `Component`, `App`, `AnyApp`, `set_app_factory`, `create_app`.
- `rust/src/navigation.rs`: `Navigation`, `Route`, `ScreenTransition`, `BottomNavigation`.
- `rust/src/scaffold.rs`: `Scaffold`, `Chrome`, `AppBar`, `Fab`, `Drawer`, `Snackbar`.
- `rust/src/presentation.rs`: `Dialog`, `DialogRole`, `BottomSheet`.
- `rust/src/style.rs`: `Color`, `ColorScheme`, `Style`, and the other value types.
- `rust/src/ui.rs`: the facade, and builder-level tests.
- `example/rust/src/lib.rs`: a full `App` implementation using every hook.

Next: [10. One tap, end to end](10-one-tap-end-to-end.md).
