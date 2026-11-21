# Hot reload

Flutter and React Native put a code change into a running app in about a second and keep its
state. This framework can now do the same during development, on the Android emulator and the
iOS simulator, for changes to an application's `view` and `update` code.

This document says how to use it, how it works, what was measured, where it stops, and why it
does not use Dioxus's `subsecond`.

## Using it

1. Enable the feature in the app's `rust/Cargo.toml`:

   ```toml
   krabik-ui-dsl = { path = "...", features = ["hot-reload"] }
   ```

2. Opt in once in the app's crate, with the type that implements `App`:

   ```rust,ignore
   krabik_ui_dsl::hot_reload!(Notes);
   ```

3. Start the app, then watch it with [krabik](../../../krabik-cli) from the app's directory and
   edit the Rust sources:

   ```sh
   cd example-todo-minimal
   krabik run      # or: krabik run --platform ios
   krabik watch    # or: krabik watch --platform ios
   ```

`example-todo-minimal` is set up this way, and so is every app made with
`krabik create <name> --with-dsl`, which writes steps 1 and 2 for you. The other apps in this
package (`harness`, `example`, `example-todo`) do not enable the feature; `krabik watch` in one
of those rebuilds and relaunches the app on every change instead. `krabik watch` hot reloads when the app's `rust/Cargo.toml`
enables the feature; it needs `adb` on the `PATH` and `ANDROID_NDK_HOME` for Android, and the
app running on a booted simulator for iOS.

## How it works

An application is a library, and the runtime reaches it through one trait object
(`Session.app: Box<dyn AnyApp>`). That is all the mechanism needs:

```text
  save a file
      |
  krabik watch           cargo build of the app's crate (the framework is a cached dependency)
      |                  copy the library into a directory inside the app's storage, new name
      v
  watcher thread         sees the file, wakes every session (the mailbox's waker)
      |
  Session::drain         on the UI thread: Session::hot_swap(path)
      |
  hot::Library::open     dlopen; look up the two functions hot_reload! exports
  Library::adopt         compare fingerprints; hand the application value over
      |
  render_all             the same state, drawn by the new code
```

- **Adoption.** `hot_reload!(App)` exports `krabik_hot_fingerprint` and `krabik_hot_adopt` from
  the app's library. `adopt` takes the pointer of the running application value, treats it as
  its own `App` type, and returns a trait object whose methods are the new code. Nothing is
  copied or rebuilt, which is why the state survives.
- **The fingerprint** is the size and alignment of the application type and hashes of its name
  and its message type's name. A library whose fingerprint differs is refused, the reason goes
  to the log, and the app carries on with the code it has.
- **The old library stays loaded.** Handlers and commands made by old code may still run. Each
  reload leaks one library, which is fine for a development session.
- **The new library has its own copy of the framework.** It is used only to build descriptions
  and handlers. Its session maps and platform entry points are never called; the running
  session stays in the library the app started with.
- **Delivery.** A process may only load code from its own storage. The watcher polls
  `code_cache/krabik-hot` on Android and `tmp/krabik-hot` in the app's container on iOS.
  `krabik watch` writes under another name and renames, so a library is never seen half
  written. No socket and no permission is involved.

The code is `rust/src/hot.rs`, `Session::hot_swap` and the two lines in `Session::drain` and
`Session::set_waker` (`rust/src/host/session.rs`), all behind the cargo feature `hot-reload`.
The watcher only starts in a debug build.

## What was tried, and what it showed

Run on 2026-11-21 with `example-todo-minimal`.

| Check | Android emulator (Galaxy S24 AVD, API 37, arm64) | iOS simulator (iPhone 17) |
|---|---|---|
| Change a label and a button title in `view`, with a todo added and one ticked | New text shown, todos kept, same process id | New text shown, added todo kept |
| Use the app after the reload | Ticking a todo updates the count through the new code | Ticking a todo updates the count through the new code |
| Change `update` (prefix new todos) | New todos get the prefix | not tried |
| Add a field to the state struct | Refused, with the two fingerprints in logcat; app keeps running | not tried on the simulator |
| Panic in reloaded code | The process aborts (SIGABRT); the panic message is not in logcat | not tried |
| Save to new code on screen | about 3 s (three runs: 3.8, 3.0, 3.0) | about 1.2 s (0.6 s build, 0.6 s until seen) |

Where the Android time goes: about 2.7 s is the incremental build, nearly all of it linking an
88 MB debug library; delivering and loading takes about 0.2 s. The simulator's library is
10 MB and links in 0.6 s. The first build for a target is a full build (27 s for the simulator).

The same path is covered without a device by a Rust test,
`new_application_code_takes_over_the_running_state` in `rust/src/host/session.rs`. It builds
the fixture app in `rust/tests/fixtures/hot-app` three times and swaps the versions in a session
with the fake platform: state kept, `view` and `update` replaced, another layout refused, a
missing file refused. It runs with `cargo test --features hot-reload`.

## Limits

- **Emulator and simulator only.** An iOS device does not load unsigned code. An Android device
  was not tried; the emulator logs an SELinux audit line for executing a library from the app's
  data directory, which stricter builds may deny.
- **The state type and the message type must keep their layout.** Add a field or a variant and
  the library is refused: restart the app. The check is by size, alignment and name, so a change
  that keeps all three, such as swapping two fields of the same type, is not caught and will
  misbehave.
- **The framework must not change.** The running session is old framework code, and it trusts
  that the new library's copy lays out `Element`, handlers and messages the same way. That holds
  when both are built from the same sources by the same compiler. After editing the framework,
  reinstall.
- **A panic in reloaded code aborts the process** without a message in logcat. It crosses from
  one copy of the Rust runtime into another, which cannot be unwound.
- **A new instance of the app is old code.** `set_app_factory` belongs to the library the app
  started with. An activity that is recreated keeps its reloaded application; a second scene on
  iOS, or an activity started fresh in the same process, starts from the old factory and then
  takes the newest library when it is woken.
- **Statics in the app's crate start again** in each library. State that should survive belongs
  in the `App` value.
- **`krabik watch` reports "reloaded" when the library was delivered.** A refusal is only in
  the app's log (`adb logcat -s krabik`).

## Why not Dioxus's `subsecond`

Dioxus 0.7 patches running Rust with a crate called `subsecond`. It redirects calls through a
jump table to functions in a small patch library, and unwinds stale frames to the nearest
`subsecond::call`. Its documentation (version 0.7.10, read on 2026-11-21) says two things that
decided against it here:

- "A third-party tool that implements the Subsecond compiler and protocol is required", and the
  linker wrapper that builds the patches is "not available as a standalone tool": in practice
  the Dioxus CLI, `dx`, which also wants to build and package the app itself. Here Gradle and
  Xcode drive Cargo.
- It locates the program through the `main` symbol of "the current executable" and patches "the
  crate in which your main.rs is located". An app here is a library loaded by a Java or Swift
  process and has neither.

`subsecond` is the finer tool: it patches single functions, so it avoids relinking the whole
library, and it copes with frames that are on the stack when the patch arrives. This framework
does not need that. Its application is entered from the top for every message and every render,
so replacing the whole application object between two of those is enough, and that takes
nothing but `cargo build` and `dlopen`.

Both stop at the same wall: neither can change the layout of live state.

## What is left

- Make the Android link faster. Most of the three seconds is linking debug information; a
  development profile with `debug = "line-tables-only"` or split debug info should bring it
  close to the simulator's time. Not tried.
- Report a refusal back to `krabik watch`.
- A restart that restores state, for the changes a reload cannot apply: an opt-in
  `save`/`restore` pair on `App`, which would also give Android process-death restoration.
- Run the layout and panic checks on the iOS simulator, and try an Android device.

## Sources

- `subsecond` crate documentation: <https://docs.rs/subsecond/latest/subsecond/>
- `subsecond` source and README: <https://github.com/DioxusLabs/dioxus/tree/main/packages/subsecond>
- Dioxus 0.7 hot-reload guide: <https://dioxuslabs.com/learn/0.7/essentials/ui/hotreload>
