# Tutorial: building the Notes app

This tutorial builds one small application, a notes app, from an empty folder to a finished app
that runs on Android and iOS. Each chapter adds to the same code and explains the part of
`krabik-ui-dsl` it uses: what it does, why it is built that way, and where it lives in the
source. Chapters 1 to 6 end with the complete file up to that point and chapter 8 with the final
one, so you can start anywhere.

The first nine chapters need only a Rust toolchain: the app is written and tested on your
development machine. Chapter 10 puts it on an emulator and a simulator.

## Chapters

1. [Your first app](01-your-first-app.md): what the framework is, the `Component` and `App`
   traits, a counter, the update loop, and testing with no device.
2. [Widgets and layout](02-widgets-and-layout.md): the flexbox model behind `Column` and `Row`,
   sizing, alignment, text styling, the list of widgets and their events, and the first version
   of Notes.
3. [Lists and state](03-lists-and-state.md): messages that carry data, what a key is and why a
   list needs one, `List` in its windowed and recycled modes, `item_height` and `revision`.
4. [Components](04-components.md): a child component with its own messages, `Element::map`,
   forwarding in `update`, and when a plain view function is the better tool.
5. [Navigation and the scaffold](05-navigation-and-scaffold.md): the navigation stack as state
   that the platform mirrors, `Scaffold` and the app bar, back handling, and a second screen for
   Notes.
6. [Dialogs, sheets and more chrome](06-dialogs-sheets-and-more-chrome.md): the "described by
   state" pattern applied to a dialog, a snackbar with Undo, a bottom sheet, a floating action
   button and a drawer, and which of them are native on each platform.
7. [Colours and dark mode](07-colours-and-dark-mode.md): `Color`, adaptive and named colours, a
   palette for the app, and what each platform does when the appearance changes.
8. [The view macro](08-the-view-macro.md): the rules of `view!`, the same views in both forms,
   when to prefer which, and the complete final app.
9. [Testing](09-testing.md): unit tests on the app, the framework's fake-platform tests, UI tests
   with Espresso and XCUITest, and the pitfalls the existing suites record.
10. [Running on Android and iOS](10-running-on-android-and-ios.md): the layout of an app, the
    install functions, the two host projects and how they build the Rust code, and device builds.

## How the code blocks are marked

A Rust block that is a complete file, with all its `use` lines, is fenced as `rust`. A fragment
that only makes sense inside a larger file, including every excerpt quoted from the framework's
source, is fenced as `rust,ignore`. Excerpts name the file they come from, as a path relative to
the package root (`packages/krabik-ui-dsl`).

## What the app looks like at the end

```text
  list screen                          detail screen
  +-------------------------------+    +-------------------------------+
  | =  Notes                      |    | <  Note               Delete  |
  +-------------------------------+    +-------------------------------+
  | [ Write a note            ]   |    | Welcome to Notes              |
  | 2 notes                       |    |                               |
  | +---------------------------+ |    | [ Details ]                   |
  | | Welcome to Notes   [Open] | |    |                               |
  | +---------------------------+ |    |                               |
  | +---------------------------+ |    +-------------------------------+
  | | Write something..  [Open] | |
  | +---------------------------+ |    "Delete" asks in a system alert,
  |                      ( + Add )|    then a snackbar offers Undo.
  +-------------------------------+    "Details" opens a bottom sheet.
   "=" opens a drawer that picks
   the sort order.
```

## Related documentation

The tutorial stays at the level of "how do I build an app and what does the framework do for
me". The mechanisms underneath (the tree, layout, the diff, the session, the two renderers, the
macros) are described in [`../internals/`](../internals/).

The files `../01-your-first-app.md` to `../08-project-layout.md` are an older guide. Parts of it
describe an earlier API (`Screen`, `Toolbar`, `Navigation<Msg>`); where it and this tutorial
disagree, this tutorial and the source are current.
