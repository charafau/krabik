# 4. Navigation, toolbar, dialogs and sheets

Some parts of a screen are not widgets you place in a layout: the navigation
stack, the toolbar, the bottom navigation bar, the system alert and the
bottom sheet. The platform owns how they look and animate. Your app only says
*what* should be shown, through optional methods on `App`:

```rust
pub trait App: Component + Send + 'static {
    fn navigation(&self) -> Option<Navigation<Self::Msg>> { None }
    fn screen(&self, key: &str) -> Option<Element<Self::Msg>> { None }
    fn dialog(&self) -> Option<Dialog<Self::Msg>> { None }
    fn sheet(&self) -> Option<BottomSheet<Self::Msg>> { None }
    fn back(&self) -> Option<Self::Msg> { None }
}
```

Each of these is computed from your state, exactly like `view()`. There is no
`push()` or `show_dialog()` call: you change your state and the framework
works out what to push, pop, show or dismiss.

## A navigation stack

Keep the stack in your state and describe it:

```rust
pub struct MyApp {
    stack: Vec<Route>,   // starts as vec![Route::Home]
}

impl App for MyApp {
    fn navigation(&self) -> Option<Navigation<Msg>> {
        Some(Navigation::new(
            self.stack.iter().map(|route| Screen::new(route.key(), route.title())).collect(),
        ))
    }

    fn screen(&self, key: &str) -> Option<Element<Msg>> {
        match key {
            "home" => Some(self.home()),
            "detail" => Some(self.detail()),
            _ => None,
        }
    }

    fn back(&self) -> Option<Msg> {
        (self.stack.len() > 1).then_some(Msg::Back)
    }
}
```

To open a screen, push onto `self.stack` in `update`; to go back, pop. The
framework compares the old and new stacks:

- A new screen at the end is pushed with the platform's transition
  (`UINavigationController` on iOS, a fragment transition on Android).
- Removed screens are popped. Popping several at once, for example "back to
  home", is a single animated pop.
- A screen that is covered keeps its native views, so going back shows it
  exactly as the user left it, including scroll position.

A screen is identified by its `key`. With navigation, `Component::view` is not
shown: the first screen in the stack is the root.

Each screen can choose how it appears:

```rust
Screen::new("photo", "Photo").transition(ScreenTransition::Fade)
```

## Back

`back()` answers one question: "if the user goes back now, which message
should I get?" Return `None` when there is nothing to go back to, and the
platform does what it normally does, which on Android means leaving the app
with the system's predictive back animation.

```rust
fn back(&self) -> Option<Msg> {
    // Close what is on top first: the dialog, the sheet, then a screen.
    (self.dialog_open || self.sheet_open || self.stack.len() > 1).then_some(Msg::Back)
}
```

The same message is sent by Android's back button or gesture and by the back
arrow in the toolbar on both platforms.

## Toolbar

Every screen gets the native toolbar with its title. Add to it per screen:

```rust
Screen::new("inbox", "Inbox").toolbar(
    Toolbar::new()
        .subtitle("3 unread")
        .action("Refresh", Msg::Refresh)
        .action_with_icon(
            "Search",
            Icon::new().drawable("ic_search").sf("magnifyingglass"),
            Msg::Search,
        ),
)
```

`Toolbar::new().hidden(true)` hides it for a full-screen page.

An `Icon` names a drawable resource on Android and an SF Symbol (or asset) on
iOS; `.shared(name)` sets one name for both.

## Bottom navigation

The bar is declared on the screen, like the toolbar. Give every top-level
screen the same bar so it stays in place while the content changes:

```rust
Screen::new("home", "Home").bottom_navigation(
    BottomNavigation::new([
        BottomNavItem::new("Home", Icon::new().drawable("ic_home").sf("house.fill")),
        BottomNavItem::new("Search", Icon::new().drawable("ic_search").sf("magnifyingglass")),
    ])
    .selected(self.tab)
    .on_select(Msg::Tab),
)
```

iOS shows the system tab bar, Android a Material bottom navigation bar, each
at its native height and running to the bottom edge of the screen.

## The system alert

```rust
fn dialog(&self) -> Option<Dialog<Msg>> {
    self.confirming_delete.then(|| {
        Dialog::new("Delete task?")
            .message("This cannot be undone.")
            .cancel("Cancel", Msg::CancelDelete)
            .destructive("Delete", Msg::ConfirmDelete)
    })
}
```

The alert is shown while `dialog()` returns `Some`. A button sends its
message; your `update` then clears `confirming_delete`, and the alert is gone
on the next render. It is the real system alert: `UIAlertController` on iOS,
a Material alert dialog on Android.

If the dialog has a `cancel` button, Android lets the user dismiss it with
back or a tap outside, which sends that button's message. Without one, the
alert can only be closed with a button.

## The bottom sheet

A sheet has your own content inside the platform's sheet:

```rust
fn sheet(&self) -> Option<BottomSheet<Msg>> {
    self.filters_open.then(|| {
        BottomSheet::new()
            .on_dismiss(Msg::CloseFilters)
            .child(Text::new("Filters").size(20).bold())
            .child(Switch::new(self.only_open).on_toggle(Msg::OnlyOpen))
            .child(Button::new("Done").on_click(Msg::CloseFilters))
    })
}
```

The platform supplies the surface, rounded corners, dimming and drag handle
and makes the sheet as tall as its content. `on_dismiss` is sent when the user
swipes the sheet away or taps outside it, so your state can follow.

Next: [how it works inside](05-how-it-works.md).
