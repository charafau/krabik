# Missing or deferred framework capabilities

An application rewrite listed the gaps it had to work around. Most are closed
now (startup work, tappable boxes, language and appearance, pull to refresh,
SVG assets; see section 9 of [TODO.md](TODO.md)). What is left:

## Custom illustration pipeline

- Available now: `.svg` assets (cargo feature `svg`), including SVG blur
  filters, and `Stack` to put content over an illustration.
- Still missing, on purpose: a blur style on native views. It was prototyped
  and removed because Android and iOS cannot produce the same result; see
  decision 12 in `docs/design/decisions.md`.

## Pixel-perfect Flutter parity

- Missing: Flutter-specific layout, animation, gesture and image-cache behavior.
- Current handling: native platform presentation with equivalent application
  behavior rather than pixel-identical UI.
- What an application coming from Flutter will miss is listed under
  "Compared with Flutter" below.

## Hot reload

- Available now for development, on the Android emulator and the iOS
  simulator: `krabik watch` puts changed `view` and `update` code into
  the running app and keeps its state (about 3 s on Android, about 1 s on
  iOS). See [docs/design/hot-reload.md](docs/design/hot-reload.md).
- Still missing: devices, and changes to the layout of the app's state or
  message type, which need a restart.

## Project tooling

- Available now: `krabik create <name> --with-dsl`, `krabik run` and
  `krabik watch` ([krabik-cli](../krabik-cli)).
- Still missing: a published framework. A created project refers to a krabik
  checkout by absolute path. `krabik release` does not handle such a project,
  and it has no UI test targets.

## Compared with Flutter

The source was read against Flutter's feature set on 2026-11-21. The list is
ordered by how soon an application meets the gap. The first entry was done
first; the second is the next piece of work. "Here" says what exists today.

| # | Flutter | Here | Missing |
|---|---|---|---|
| 1 | `TextStyle`, `RichText`, custom fonts | nine font weights, italic, underline and strikethrough, letter spacing, line height, `Text::rich` with spans, fonts from a file on `Text`, `Button` and spans, and the system text size on both platforms (**done**, see section 10 of [TODO.md](TODO.md)) | spans that can be tapped (links); a font file on `TextField` and the other controls; font families with several weights; text shadows; a way for the application to read the text scale or opt a text out of it; selectable text |
| 2 | `MediaQuery`, `SafeArea`, `WidgetsBindingObserver` | the hosts keep content in the safe area; Android resizes for the keyboard; `Environment` carries locale and colour scheme | the screen size, insets and orientation for the application; a callback for foreground and background; any keyboard avoidance on iOS |
| 3 | `GestureDetector`, `Dismissible`, `Draggable` | tap (`Button`, `Container::on_tap`), long press (`Container::on_long_press`), the pager's swipe, pull to refresh, swiping a list row away and dragging it (`List::on_dismiss`, `on_reorder`) | double tap, free drag and pan, pinch |
| 4 | `FocusNode`, `ScrollController`, `NotificationListener` | focus and scroll position are the platform's; `on_focus` reports focus | focusing a field from code, moving to the next field, scrolling to an offset or an item, a scroll event (so no "load more at the end") |
| 5 | `GridView`, `Wrap`, `Positioned`, `ConstrainedBox`, `MainAxisAlignment.spaceAround/Evenly` | flexbox `Column`/`Row`, `Stack` with `.positioned`, `Wrap`, `Grid`, `width`/`height`/percent, minimum and maximum sizes, `aspect_ratio`, all six distributions (**done**, section 11 of [TODO.md](TODO.md)) | a staggered grid, a cell that spans several columns |
| 6 | `shared_preferences`, `RestorationMixin` | nothing; state is lost when Android ends the process | key-value storage, and saving and restoring the application's state |
| 7 | `TextEditingController`, `inputFormatters`, `Form` | `TextField` with keyboard type, IME action, secure and multiline | maximum length, selection and cursor control, formatters and validation helpers |
| 8 | `PopupMenuButton`, `Tooltip`, `showGeneralDialog` | alert dialog, bottom sheet, snackbar, drawer; popup and context menus (`Menu`), tooltips | a full-screen modal, a free overlay, sub-menus |
| 9 | `AnimationController`, `Hero`, `Transform` | implicit animation of frame and decoration, fade and scale on enter and exit, slide or fade between screens | explicit animations, shared-element transitions, rotate/scale/translate |
| 10 | `Image` with `loadingBuilder`/`errorBuilder`, `Icon` | asset, URL and SVG images with caching; icons in the app bar, drawer and navigation, and in content with `Image::icon` | a placeholder and an error image |
| 11 | `Semantics`, `Directionality` | `accessibility_label`; native controls bring their own semantics | roles, hints and actions; right-to-left layout |
| 12 | `flutter_test` (`pumpWidget`, `tap`, `find`), golden tests | `update` and `view` are plain Rust; the framework's own tests drive a fake platform | a public way for an application's tests to tap a widget and read the result; image comparisons |
| 13 | `ThemeData`, `CustomPaint`, desktop and web | adaptive colours; SVG | not planned: a theme object is the application's own module of constants, and custom painting and other targets do not fit native widgets (decision 2) |

## Missing widgets

The table above is about capabilities. This one is about widgets Flutter has
and this framework does not, read from the source on 2026-11-21 and ordered by
how soon an application wants them. A row leaves the table when it is built.

| Group | Flutter | Note |
|---|---|---|
| Buttons | an icon inside the native `Button` | `IconButton` with `.text(..)` is the way for now. |
| Embedded platform views | map, camera preview | Not built in. An application embeds its own with `NativeView`; `WebView` is built in. |
| Sliver effects | `SliverAppBar` (collapsing header) | Not built: both shells pin the content below the bar. "A collapsing app bar" in [TODO.md](TODO.md) says what it needs. Sticky section headers exist (`List::sticky_header`). |

Built so far, in section 11 of [TODO.md](TODO.md): `Wrap`, `Grid`, an icon in
content (`Image::icon`), `Divider`, `Card`, `ListTile`, `Avatar`, `Badge`,
`Chip`, `IconButton`, filled and outlined buttons, `ExpansionTile`, `Stepper`,
`NumberStepper`, `SearchBar`, `Autocomplete`, `Menu` with `MenuButton`, context
menus and the app bar's menu, `Dropdown`, `SegmentedControl`, tooltips, `RangeSlider`, copyable text
(`Text::selectable`), `NativeView`, `WebView`, and on a recycled list swipe to
dismiss, drag to reorder and sticky headers.

Not planned: `CustomPaint`, `Hero` and `AnimatedBuilder`. A canvas, a
shared-element transition and a per-frame builder do not fit widgets that the
platform draws and animates itself (decision 2).

Smaller items that are not Flutter features, and the known rough edges, are in
[TODO.md](TODO.md).
