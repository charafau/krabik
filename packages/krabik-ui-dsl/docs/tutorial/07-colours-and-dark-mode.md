# 7. Colours and dark mode

This chapter explains the `Color` type, gives Notes a small palette of its own, and follows what
happens on each platform when the user switches between light and dark appearance. Afterwards you
will know how to make an app look right in both, and why Android and iOS take different routes to
the same result.

## A colour is two colours

Every method that takes a colour (`.background(..)`, `.color(..)`, `.border(.., ..)`) takes
`impl Into<Color>`. A `Color` holds two values (`rust/src/style.rs`):

```rust,ignore
/// A colour, with a value for the light appearance and one for the dark.
pub struct Color {
    /// The ARGB value in the light appearance.
    pub light: u32,
    /// The ARGB value in the dark appearance.
    pub dark: u32,
}
```

There are three ways to get one.

**A plain number** converts into a colour that is the same in both appearances:

```rust,ignore
Text::new("Always blue").color(0xFF21_96F3)
```

The format is `0xAARRGGBB`: alpha first, then red, green, blue, two hex digits each. `FF` alpha is
opaque. The underscore is only a digit separator. The alpha byte is easy to forget: `0x2196F3`
has an alpha of `00` and is fully transparent, which looks exactly like "my background does not
work".

**`Color::adaptive(light, dark)`** gives a value for each appearance:

```rust,ignore
const CARD: Color = Color::adaptive(0xFFFF_F4C2, 0xFF3A_3320);
```

It is a `const fn`, so colours can be constants. `Color::fixed(argb)` is the explicit form of the
number conversion, for the same reason.

**The named colours** are adaptive colours the framework defines for the common roles:

| Constant | Role | Light | Dark |
| -------- | ---- | ----- | ---- |
| `Color::TEXT` | default text | `0xFF000000` | `0xFFE6E1E5` |
| `Color::SECONDARY_TEXT` | captions, hints | `0xFF666666` | `0xFFA8A8A8` |
| `Color::BACKGROUND` | the page | `0xFFFFFFFF` | `0xFF121212` |
| `Color::SURFACE` | a card or field on the page | `0xFFF4F4F4` | `0xFF1E1E1E` |
| `Color::OUTLINE` | borders, dividers | `0xFFD0D0D0` | `0xFF3A3A3A` |
| `Color::ACCENT` | sliders, progress, spinners | `0xFF2196F3` | `0xFF64B5F6` |

Widgets use them as defaults. A `Text`, a `Button` label and a `TextField` are `Color::TEXT`
unless you say otherwise (`#[widget(default = Color::TEXT)]` on the `color` field of each
properties struct), and a `Slider` is `Color::ACCENT`. So an app that sets no colours at all is
already readable in dark mode. Before the dark-theme work (commit `fd7e2b5`) the default text
colour was fixed black; the commit message notes the change.

The rule of thumb: **any colour you write as a bare number is a decision to ignore dark mode.**
That is right for a brand colour on a button and wrong for a card background. Dark text on a
fixed light card looks fine by day and unreadable at night, because the text adapts and the card
does not.

## Who paints the page?

Notes never sets a page background, yet its screens are white by day and dark by night. The
platform paints it, underneath everything Rust draws:

- On Android each navigation screen is backed with the theme's background colour
  (`themeBackground` in `shared/android/src/main/java/dev/krabik/ui/KrabikScreenFragment.kt`
  reads `android.R.attr.colorBackground`). The example apps use the theme
  `Theme.Material3.DayNight.NoActionBar`, which has a light and a dark variant.
- On iOS each screen's view controller uses `.systemBackground`
  (`hostController(_:background:)` in `shared/ios/KrabikHostController.swift`).

So leave the page alone and colour the things on it. There is one mismatch to know about.
`Color::BACKGROUND` and the other named colours are fixed numbers chosen by the framework. They
are not the platform's dynamic system colours. The dark `Color::BACKGROUND` is `0xFF121212`,
while iOS's dark system background is black. If you paint a full-screen container with
`Color::BACKGROUND`, it will not be exactly the colour the platform would have used. Use
`Color::BACKGROUND` for a patch that should *look like* the page (the showcase uses it for a
bordered box), not to repaint the page.

## A palette for Notes

Scattering `Color::adaptive(..)` calls through view code makes a redesign painful. Put the app's
colours in one module and give them names that say what they are for, not what they look like:

```rust
use krabik_ui_dsl::ui::{Color, ColorScheme, Text};

/// The app's colours. Each has a value for the light and for the dark
/// appearance; the framework picks one when it builds the tree.
mod palette {
    use krabik_ui_dsl::ui::Color;

    /// Captions and counts. The framework's own secondary text colour.
    pub const MUTED: Color = Color::SECONDARY_TEXT;
    /// The card a note sits on: paper yellow by day, dim brown by night.
    pub const CARD: Color = Color::adaptive(0xFFFF_F4C2, 0xFF3A_3320);
    /// The hairline around a card.
    pub const CARD_EDGE: Color = Color::adaptive(0xFFE8_D98A, 0xFF5A_5030);
    /// Text on a card. Dark brown on yellow, pale yellow on brown.
    pub const ON_CARD: Color = Color::adaptive(0xFF3E_2F00, 0xFFF3_E7B3);
}

/// Small secondary text.
fn caption<M>(text: &str) -> Text<M> {
    Text::new(text).size(14).color(palette::MUTED)
}

/// What a colour is in each appearance; handy in a test.
fn both(color: Color) -> (u32, u32) {
    (
        color.resolve(ColorScheme::Light).argb(),
        color.resolve(ColorScheme::Dark).argb(),
    )
}
```

Three habits are packed into that:

- **Name by role.** `CARD`, not `YELLOW`. In dark mode it is not yellow.
- **Define text and its background together.** `ON_CARD` only makes sense on `CARD`. Choosing the
  pair in one place is how you keep contrast in both appearances.
- **Re-export a framework colour under your own name** when it happens to be right (`MUTED`).
  View code then says `palette::MUTED` everywhere, and changing it later is a one-line edit.

The example apps follow the same shape with plain constants at the top of the file, for instance
`COLOR_SURFACE` and `COLOR_DONE` in `example-todo-minimal/rust/src/lib.rs`.

Now use it. The note row gets the card colours and a hairline border:

```rust,ignore
/// One note in the list.
fn note_row(note: &Note) -> Element<Msg> {
    Row::new()
        .width_percent(1.0)
        .height(ROW_HEIGHT)
        .padding_each(EdgeInsets::symmetric(12, 6))
        .spacing(8)
        .align_items(Align::Center)
        .background(palette::CARD)
        .border(1, palette::CARD_EDGE)
        .corner_radius(10)
        .child(
            Text::new(note.text.clone())
                .color(palette::ON_CARD)
                .max_lines(1)
                .ellipsis()
                .grow(1),
        )
        .child(
            Button::new("Open")
                .on_click(Msg::Open(note.id))
                .color(palette::ON_CARD)
                .accessibility_label(format!("Open {}", note.text)),
        )
        .into()
}
```

`caption` changes as shown in the block above. Those are all the edits. `Color::SURFACE` and
`Color::SECONDARY_TEXT` no longer appear in view code; every colour the views use now comes from
`palette`.

A test that both values of a pair differ protects against the classic slip of pasting the same
number twice:

```rust,ignore
#[test]
fn the_card_colours_change_with_the_appearance() {
    let (light, dark) = both(palette::CARD);
    assert_ne!(light, dark);
    let (light, dark) = both(palette::ON_CARD);
    assert_ne!(light, dark);
}
```

## How a colour reaches the screen

Your `view` returns colours that still hold both values. The native renderers never see those.
In between, the engine resolves every colour for the current appearance while it builds the tree
(`Runtime::update` in `rust/src/engine/runtime.rs` calls `tree::resolve_colors`):

```rust,ignore
/// Picks the value for `scheme` in every colour of the tree.
///
/// The renderers only ever see resolved colours, and a change of appearance
/// reaches them as ordinary updates of the nodes whose colours differ.
pub(crate) fn resolve_colors(node: &mut UiNode, scheme: crate::style::ColorScheme) {
```

(from `rust/src/engine/tree.rs`)

"Resolving" replaces both values with the chosen one (`Color::resolve`), so `argb()` can return
either. A node's background, gradient and border are resolved in `Style::resolve_colors`; a
widget's own colours, such as a text colour, in a `resolve_colors` method that
`#[derive(Widget)]` generates for every field of type `Color`.

```text
   view()                   engine                   renderer
   ------                   ------                   --------
   background:              resolve for Dark:        set the view's
     light 0xFFFFF4C2  -->    0xFF3A3320        -->  background to
     dark  0xFF3A3320                                0xFF3A3320
```

The important consequence is in the doc comment: a change of appearance is an ordinary update.
The engine holds a `ColorScheme`. When it changes, the next tree has different resolved colours
in some nodes, the diff finds exactly those nodes, and each produces one "update this view"
operation. Nothing is rebuilt, nothing moves, and no renderer contains code for theme changes.
The test `a_new_color_scheme_updates_only_the_nodes_whose_colors_change` in
`rust/src/engine/runtime.rs` shows it on a three-node tree: two nodes with adaptive colours
update, the one with a fixed colour does not.

This is also why the design puts resolution in the engine and not in the platform code: there is
one implementation, it is tested on a desktop, and both renderers stay unaware of it.

## What happens when the user switches

The engine needs to be told the appearance, at startup and whenever it changes. Here the two
platforms differ, and the difference is deliberate.

### iOS: tell Rust, repaint

UIKit notifies the host view controller when the interface style changes. The controller passes
it on (`shared/ios/KrabikHostController.swift`):

```swift
/// Colours Rust paints are fixed values, not dynamic system colours, so
/// Rust is told when the appearance changes and repaints them.
override func traitCollectionDidChange(_ previous: UITraitCollection?) {
    super.traitCollectionDidChange(previous)
    if previous?.userInterfaceStyle != traitCollection.userInterfaceStyle, session != 0 {
        krabik_ios_set_color_scheme(session, traitCollection.userInterfaceStyle == .dark)
    }
}
```

On the Rust side that entry point sets the scheme and renders (`rust/src/ios/entry.rs`):

```rust,ignore
session.set_color_scheme(scheme(dark));
infallible(session.render_all(&mut mtm));
```

Everything stays in place: the same native views, the same session, the same app. The native
chrome (navigation bar, tab bar, alerts, the sheet's surface) is made of system controls that
restyle themselves. The views Rust painted get their new colours through the diff.

### Android: recreate the activity, keep the app

On Android, a change of the night mode is a *configuration change*, and the system's default
response is to destroy the activity and create a new one so that every view is inflated again
with the new theme's resources. The framework lets that happen. The reason is the native chrome:
the Material toolbar, bottom bar, dialogs and sheets take their colours from the theme when they
are created, and recreating the activity is the reliable way to re-theme all of them.

Rotation is treated differently. The app's manifest lists it under `android:configChanges`
(`example-todo-minimal/android/app/src/main/AndroidManifest.xml`), so rotation only resizes the
activity. Night mode is *not* in that list, on purpose.

Recreating the activity would normally lose everything. The Rust session is tied to the activity
and is torn down with it. What must not be lost is your app: its notes, its open screen, its
draft. So the app instance is moved out of the dying session and into the new one. The steps:

```text
   old activity                         new activity
   ------------                         ------------
   onDestroy()
     isChangingConfigurations is true
     root.destroy(retain = true)
        Rust: remove native views,
        end the session, keep the
        app in a map, return its id
     id -> RetainedApp (a ViewModel)
                                        onCreate()
                                          id <- RetainedApp
                                          RustRootView(retainedApp = id)
                                             Rust: take the app out of the
                                             map, start a new session with
                                             it, for the new night mode
                                          the stack is shown at once,
                                          without replaying pushes
```

The Kotlin side (`KrabikActivity.kt`):

```kotlin
override fun onDestroy() {
    // A configuration change the manifest does not handle, such as a
    // switch between light and dark, recreates the activity so every
    // native view picks up the new resources. The application's state
    // must not be lost with it.
    retained.id = root.destroy(retain = isChangingConfigurations)
    super.onDestroy()
}
```

`retained` is a `RetainedApp`, a `ViewModel` with one field. A `ViewModel` is Android's mechanism
for exactly this: it outlives a configuration change and is cleared when the activity is finished
for real. When that happens without the app having been picked up, `onCleared` tells Rust to drop
it (`nativeDropRetainedApp`), so nothing leaks.

The Rust side of the pick-up (`native_create` in `rust/src/android/entry.rs`):

```rust,ignore
// An activity recreated by a configuration change, such as a switch
// between light and dark, continues with the application it had.
let app = retained_apps()
    .lock()
    .expect("retained apps")
    .remove(&retained_app)
    .unwrap_or_else(crate::app::create_app);
let mut core = Session::new(app, main);
core.set_color_scheme(if night {
    ColorScheme::Dark
} else {
    ColorScheme::Light
});
```

If there is a retained app, use it; otherwise call your factory. Either way the new session knows
the night mode before its first render.

Then the session asks the retained app for its `navigation()`, as always. The app says
`["notes", "note:2"]`, because that is still its state. This is the "first stack is shown
immediately" rule from chapter 5: the user finds themselves on the same detail screen, now dark,
with no slide animation.

What survives is exactly what lives in your app's state. What lived only in the old native views
and the old session is created afresh; from the source, that includes scroll positions. This is
one more argument for the rule that the app owns all state: state you own comes through a
recreation without any code on your part.

Both platforms have a UI test that switches the appearance and checks that the screen and a
choice made on it are still there: `switchingToDarkKeepsTheScreenAndItsState` in
`example/android/app/src/androidTest/java/com/example/krabik_ui_dsl/ShowcaseTest.kt` and
`testSwitchingToDarkKeepsTheScreenAndItsState` in `example/ios/UITests/ShowcaseUITests.swift`.

### The iOS stand-ins

The two custom iOS views from chapter 6 are not painted by Rust, so they handle dark mode
themselves with UIKit dynamic colours. `KrabikSnackbarView` in `shared/ios/KrabikChrome.swift`
inverts: a dark bar in light mode, a light bar in dark mode.

## Trying it

On an Android emulator the night mode can be switched from a shell; this is the command the
Android UI test uses:

```sh
adb shell cmd uimode night yes
adb shell cmd uimode night no
```

On an iOS simulator use the Settings app, or Features > Toggle Appearance in the Simulator menu.
The iOS UI test sets `XCUIDevice.shared.appearance`.

Check three things in each appearance: text on cards, text on the page, and any colour you wrote
as a number.

## The code so far

The file is chapter 6's with these changes:

- the `palette` module and the `caption` function from the block above;
- `note_row` as shown above;
- the `both` helper and the test, if you want them. With `both`, add `ColorScheme` to the
  `use krabik_ui_dsl::ui::{..}` list at the top. Without it, remove `Color` from that list,
  because only the `palette` module names it then.

Chapter 8 ends with the complete final file.

## Where this lives

- `rust/src/style.rs`: `Color`, `ColorScheme`, the named colours, `Style::resolve_colors`,
  `dim_color` (how disabled decoration is dimmed in both values).
- `rust/src/engine/tree.rs`: `resolve_colors`. `rust/src/engine/runtime.rs`: `set_color_scheme`
  and the test named above.
- `rust/src/host/session.rs`: `set_color_scheme`, `into_app`, and the tests
  `the_color_scheme_reaches_every_host` and
  `a_restored_stack_is_shown_at_once_and_later_pushes_animate`.
- `rust/src/android/entry.rs`: `native_create`, `native_destroy`, `native_drop_retained_app`.
- `shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt`: `onDestroy` and `RetainedApp`.
- `shared/ios/KrabikHostController.swift`: `traitCollectionDidChange`.

Next: [8. The view macro](08-the-view-macro.md).
