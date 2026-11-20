# krabik-ui-dsl documentation

This documentation has two jobs: teach you to build an app with the framework, and explain how
the framework itself works and why it is built the way it is. It is long on purpose. Pick the
part that matches what you want to do.

| If you want to… | Read |
|---|---|
| Build an app, step by step | [Part 1: Tutorial](tutorial/README.md) |
| Understand the Rust engine: elements, tree, layout, diff, events | [Part 2a: How the engine works](internals/README.md) |
| Understand the Android and iOS halves, the build and the macros | [Part 2b: How the platforms work](platforms/README.md) |
| Know why a decision was made, or what changed and when | [Part 3: Design](#part-3-design) |
| Change the framework: add a widget, add chrome, test, debug | [Part 4: Recipes](recipes/README.md) |
| Map it onto Flutter or React Native: which tree is which, what replaces `setState` | [Part 5: Comparisons](comparisons/README.md) |

## Suggested reading order

If you know Flutter or React Native, start with [Part 5](comparisons/README.md): it explains the
engine by comparison and then points back here.

If you are new to the codebase and want the whole picture:

1. [Tutorial 1: Your first app](tutorial/01-your-first-app.md), to see what an app looks like.
2. [The big picture](internals/01-the-big-picture.md), one page on how a description becomes
   native views and how an event comes back.
3. [Why it is built this way](design/decisions.md), twelve decisions with their reasons.
4. [One tap, end to end](internals/10-one-tap-end-to-end.md) and
   [One tap on each platform](platforms/08-one-tap-on-each-platform.md), which follow a single
   tap through every function.
5. The rest of Parts 2a and 2b, in order, when you want to open a particular file and know what
   it is for.

## Part 1: Tutorial

Builds one small Notes app from an empty folder: a counter, widgets and layout, lists,
components, navigation and the scaffold, dialogs and sheets, colours and dark mode, the `view!`
macro, testing, running on both platforms, and background work and timers. Every chapter ends with the complete code so far.
[Start here](tutorial/README.md).

## Part 2a: How the engine works

A tour of everything under `rust/src/` that is not platform code: elements and typed messages,
widgets, the node tree and stable identity, layout with Taffy, diffing into operations, the
runtime, the session that both platforms share, and the commands and mailbox behind background
work. [Index](internals/README.md).

## Part 2b: How the platforms work

How Rust talks to Android (JNI) and iOS (a C interface and `objc2`), the two renderers, the
Kotlin and Swift shells, the Gradle and Xcode builds, CI, and the procedural macros.
[Index](platforms/README.md).

## Part 3: Design

- [Why it is built this way](design/decisions.md): each decision, the problem it answers, the
  alternatives, and what it costs.
- [Hot reload](design/hot-reload.md): new application code in a running app during development.
  How to use it, how it works, what was measured, and its limits.
- [What was done, in order](design/history.md): the work commit by commit, from the first
  native-UI bug fixes through the restructure to the scaffold, with how each step was checked.

## Part 4: Recipes

Checklists for changing the framework: [add a widget](recipes/add-a-widget.md),
[add a piece of native chrome](recipes/add-a-chrome-slot.md),
[write tests at each level](recipes/write-tests.md) and
[find out why something renders wrongly](recipes/debug-a-render.md).

## Part 5: Comparisons

The framework next to two you may already know. [The index](comparisons/README.md) has the seven
ideas that matter most and a three-way table of concepts.
[Compared to Flutter](comparisons/01-compared-to-flutter.md) goes through the trees, one update
step by step, identity and keys, state, layout, painting, threads, input and tooling.
[Compared to React Native](comparisons/02-compared-to-react-native.md) does the same against
Fiber, the shadow tree, Yoga, JSI and the mounting layer.

## How to trust what you read

Code excerpts are quoted from the source with their file path. The complete Rust listings in the
tutorial are compiled as part of `cargo test`, so they cannot silently go stale; shorter
excerpts are not compiled and may lag behind the code. Where the documentation describes
something that was not verified (the CI workflow has never run; device builds were compiled but
not run on a phone), it says so. Known gaps and rough edges are listed in
[`TODO.md`](../TODO.md).
