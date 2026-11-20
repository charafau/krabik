# Hot reload: what Dioxus does and what it would take here

Flutter and React Native apply a code change to a running app in about a second and keep its
state. Here every change is a native rebuild and a reinstall. This document looks at how Dioxus
does it for Rust, maps that onto this framework, and recommends what to build.

**Status: a proposal from reading, not from running.** The Dioxus facts below come from the
`subsecond` 0.7.10 documentation and source comments, read on 2026-11-21. Nothing was tried on a
device or a simulator: the `dx` tool is not installed on the machine this was written on, and it
expects to build and package the app itself (see "What stands in the way"). Statements that need
an experiment are marked **to verify**.

## Two different things are called hot reload

1. **Reloading a description.** React Native's Fast Refresh and Dioxus's RSX hot reload send a
   new description of the UI to the running app, which interprets it. No machine code changes.
   It needs an interpreter for the description, and it only covers what the description language
   can express.
2. **Patching compiled code.** The changed functions are recompiled, loaded into the running
   process, and calls are redirected to them. Flutter does this in its VM. Dioxus 0.7 does it for
   native Rust with a crate called `subsecond`.

A screen here is a Rust function that builds an `Element`, written with builders or with the
`view!` macro, which expands to the same builders (`rust/macros/src/view.rs`). There is no
description to interpret, so only the second kind applies.

## How `subsecond` works

- **Calls go through a jump table.** The app wraps a call in `subsecond::call(|| ...)`. After a
  patch, the call lands in the newest version of the function. The documentation says it "works
  by detouring function calls through a jump table" and "does not modify your process memory".
- **A patch is a new dynamic library.** The changed crate is recompiled and linked into a small
  library whose undefined symbols resolve to the running program. `subsecond::apply_patch` loads
  it and updates the jump table. On Android the library is loaded from memory rather than from a
  file, "to circumvent permissions issues".
- **A stale frame unwinds to the nearest `call`.** Code that was already running when the patch
  arrived panics with a `HotFnPanic`, which the next `call` up the stack catches and retries.
- **The tool does the building.** "A third-party tool that implements the Subsecond compiler and
  protocol is required"; that tool is the Dioxus CLI, run as `dx serve --hotpatch`. Its linker
  wrapper is "not available as a standalone tool".
- **Patches arrive over a websocket.** `dioxus_devtools::connect_subsecond()` is documented as
  "intended to be used by non-dioxus projects that want to use hotpatching". The Dioxus guide
  says Bevy and Iced already use it.
- **It is a debug feature.** It is only enabled when `debug_assertions` are on.

### What it cannot do

| Limit | Consequence here |
|---|---|
| Only the "tip" crate is patched: "the crate in which your main.rs is located". Changes in other crates are ignored. | The app crate is the tip, which is right: edits to the framework would still need a rebuild. An app split into several crates reloads only one. |
| "Does not support hot-reloading of structs." A changed layout with old state crashes. | Adding a field to the app's state or a variant to `Msg` needs a restart. Editing `view` and `update` bodies does not. |
| Changes to static initializers are not seen; thread-locals in the tip crate reset. | Little effect: an app here keeps its state in the `App` value. |
| iOS devices are "currently not supported due to code-signing requirements". | Simulator only on iOS. |

## How it maps onto this framework

The shape fits well, because the application already is three functions the runtime calls:

- `Session` calls the app through the `AnyApp` trait in a handful of places: `view`, `update`,
  `screen`, `scaffold`, `navigation`, `dialog`, `sheet`, `subscriptions`
  (`rust/src/host/session.rs`). Wrapping those calls in `subsecond::call` behind a cargo feature
  is a small, contained change.
- State survives by construction. The `App` value is not rebuilt by a patch; only the code that
  reads it changes. This is the same reason an Android activity recreation keeps the app today.
- `Session::render_all` already rebuilds every host from the application. A patch handler needs
  to call it on the UI thread. There is no entry point for that from outside an event today:
  `drain` returns early when the mailbox is empty. One new entry point per platform, or a
  "rendered again" marker posted through the mailbox and its waker, would do it.
- The patch listener is a background thread, started once per process when the feature is on.

## What stands in the way

These are the open questions, in the order I would test them.

1. **The app is not an executable. To verify.** `subsecond` uses the `main` symbol "as a
   sentinel for the current executable" to work out where the program was loaded, and "requires
   coordination from the CLI to export it". Here the app and the framework are one `cdylib`
   loaded with `System.loadLibrary` on Android, and a static library linked into a Swift
   executable on iOS (`docs/platforms/06-building-and-ci.md`). On Android the process's `main`
   belongs to the Android runtime, not to our library. Whether the address arithmetic holds for a
   library loaded by a host process is not documented.
2. **`dx` wants to own the build. To verify.** `dx serve` builds, packages and installs an app
   from its own project layout. Here Gradle and Xcode drive Cargo (`shared/android/rust.gradle.kts`,
   `shared/build-rust-ios.sh`). Either `dx` can be pointed at an existing native project, or the
   patch build has to be reproduced: compile the app crate with the same flags, link the patch
   against the running library's symbols, produce the jump table. The second is the real cost of
   this project, and it follows a format that is still marked experimental.
3. **Reaching the device.** The websocket has to connect from the device to the development
   machine: `adb reverse` for an emulator or a phone, nothing extra for the iOS simulator. How
   the address is configured for `connect_subsecond` is not stated in its documentation.
4. **"The tip crate" for a `cdylib`.** The documentation defines it by `main.rs`. Our app crates
   have a `lib.rs`.
5. **Versions.** crates.io shows `subsecond` and `dioxus-cli` at `0.8.0-alpha.3` next to the
   0.7.10 documentation, so the interface is moving.

## A cheaper alternative: restart quickly, keep the state

Most of the day-to-day cost is not the restart, it is getting back to the screen being worked on.
That can be had without patching code:

- `krabik-cli` already has a `watch` command that rebuilds, reinstalls and relaunches on a change
  to the Rust sources. It only knows UniFFI projects today, not this package's layout.
- The app would opt into restoring its state: an optional `App::save` / `App::restore` pair
  (bytes out, bytes in), written by the session when the process is asked to stop for a reload
  and read by `set_app_factory`'s instance on the next start. Navigation is application state
  here, so the stack comes back with it.

It handles every kind of change, including new struct fields (the app decides what to do with
old bytes), needs no third-party tool, and also gives Android process-death restoration, which is
missing today. It is slower: a debug build, an install and a launch, where a patch takes a
second or two.

## Recommendation

1. **Build the restart path first.** Teach `krabik-cli watch` this package's project layout and
   add opt-in state save and restore. It is useful on both platforms and on devices, and nothing
   in it depends on an experimental format.
2. **Run a two-day spike on `subsecond` before committing to it.** Answer open questions 1 and 2
   on the Android emulator with `example-todo-minimal`: wrap the `AnyApp` calls, start the
   listener, change a string in `view`, and see whether the patch applies and the count survives.
   If the `main` sentinel or the build ownership cannot be worked around, stop there.
3. **If the spike works**, ship it as a development-only cargo feature of the framework, document
   the limits from the table above, and keep the restart path for the changes a patch cannot
   apply.

## Sources

- `subsecond` crate documentation, version 0.7.10: <https://docs.rs/subsecond/latest/subsecond/>
- `subsecond` source and README: <https://github.com/DioxusLabs/dioxus/tree/main/packages/subsecond>
- Dioxus 0.7 hot-reload guide: <https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload>
- `dioxus_devtools::connect_subsecond`: <https://docs.rs/dioxus-devtools/latest/dioxus_devtools/fn.connect_subsecond.html>
