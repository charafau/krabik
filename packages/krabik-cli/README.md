<p align="center">
  <img src="https://raw.githubusercontent.com/charafau/krabik/main/assets/krabik.svg" alt="krabik logo" width="180">
</p>

<h1 align="center">krabik</h1>

<p align="center">
  <strong>Scaffold, build, and run cross-platform Rust mobile apps.</strong>
</p>

`krabik` creates projects with a shared Rust core and native Android and iOS
shells, then drives the rest of the toolchain for you: installing Rust targets,
listing and booting devices, building native libraries, generating UniFFI
bindings, and installing and launching the app.

A project is either a UniFFI project, where native Kotlin and Swift UIs call a
Rust library, or, with `--with-dsl`, a
[krabik-ui-dsl](../krabik-ui-dsl) app, where the UI itself is written in Rust
and `krabik watch` hot reloads it.

## Quick start

```sh
krabik doctor          # check your development environment
krabik create my-app   # scaffold a new project in ./my-app
cd my-app
krabik devices -a      # list connected devices, emulators and simulators
krabik launch Pixel_10 # boot an Android emulator
krabik run             # build, install and launch the app
```

For iOS:

```sh
krabik launch "iPhone 16" --platform ios
krabik run --platform ios
```

> `run`, `release`, `bindings`, `watch`, and `clean` must be executed from the
> root of a generated project.

## What you need

At minimum you need Rust and `rustup`. Android and iOS builds have additional
requirements.

| Requirement | Needed for | Notes |
| --- | --- | --- |
| Rust 1.99.1+ with `rustup` | everything | Edition 2024. `rustup` installs the mobile targets. |
| `cargo` | everything | Comes with Rust. |
| Android CLI (`android`) | `create`, `launch`, `devices -a` | Must support `android create --namespace` and `--application-id`. |
| Android SDK | Android builds | `ANDROID_HOME` or `ANDROID_SDK_ROOT`; also detected through `android info`. |
| Android NDK | Android builds | `ANDROID_NDK_HOME` must point at an NDK installation. |
| `adb` on `PATH` | Android devices, `run`, `watch` | Used to find and talk to devices. |
| JDK on `PATH` | Android Gradle builds | If `JAVA_HOME` is set, it must point at a valid JDK. |
| Xcode tools (`xcodebuild`, `xcrun`) | iOS builds | Requires macOS. |
| [XcodeGen](https://github.com/yonaskolb/XcodeGen) | iOS projects and builds | `brew install xcodegen`. |

Run `krabik doctor` to check everything at once. It reports the Rust toolchain,
the Android CLI and SDK, `adb`, the JDK and `JAVA_HOME`, the NDK, the Xcode
tools, and any missing Rust compilation targets. It exits with an error if a
check fails.

```sh
krabik doctor
```

## Install

Install from a local checkout:

```sh
git clone https://github.com/charafau/krabik.git
cd krabik
cargo install --path packages/krabik-cli
```

Or install directly from the repository:

```sh
cargo install --git https://github.com/charafau/krabik.git --package krabik-cli
```

This installs the `krabik` binary. Verify it with:

```sh
krabik --version
```

## Commands

| Command | Description | Run from |
| --- | --- | --- |
| `krabik doctor` | Check the development environment. | anywhere |
| `krabik create <name> [--package <prefix>] [--with-dsl [--dsl-path <dir>]]` | Scaffold a new project in `./<name>`. | anywhere |
| `krabik toolchain` | Install the Rust targets needed for Android and iOS. | anywhere |
| `krabik devices [-a]` | List devices. `-a`/`--all` also shows offline devices and shutdown simulators. | anywhere |
| `krabik launch <device> [--platform android\|ios]` | Boot an Android emulator or an iOS simulator. | anywhere |
| `krabik run [device] [--platform android\|ios] [--abi <abi>]` | Build, install, and launch a debug build. | project root |
| `krabik release [--platform android\|ios]` | Build release artifacts. | project root |
| `krabik bindings [--platform android\|ios]` | Generate Kotlin or Swift UniFFI bindings. | project root |
| `krabik watch [device] [--platform android\|ios\|both]` | Hot reload a krabik-ui-dsl app, or rebuild and relaunch, on every Rust change. | project root |
| `krabik clean [--purge]` | Remove build artifacts; `--purge` also removes cached Android directories. | project root |

### Create a project

```sh
krabik create my-app
krabik create my-app --package io.acme
```

`--package` defaults to `com.example`. It is used as the prefix for the Android
application id and the iOS bundle id.

The command creates the Rust crate, the Android Gradle project, and the iOS
XcodeGen project. It is atomic: the project is built in a staging directory and
only moved into place when every part was generated successfully. A failed
`create` leaves nothing behind and can simply be retried.

The target directory must not exist yet.

#### A krabik-ui-dsl app

```sh
krabik create my-app --with-dsl
krabik create my-app --with-dsl --dsl-path ~/src/krabik/packages/krabik-ui-dsl
```

`--with-dsl` creates a [krabik-ui-dsl](../krabik-ui-dsl) app instead: a Rust
crate with a small counter app, and Android and iOS shells that only load it.
There are no UniFFI bindings, and the `android` CLI is not needed.

The framework is not published yet, so the project refers to a local krabik
checkout by absolute path: the Rust crate (`rust/Cargo.toml`), the Kotlin host
(`android/settings.gradle.kts`), and the Swift host and build script
(`ios/project.yml`, `ios/Sources/Bridging.h`). `--dsl-path` names the
`krabik-ui-dsl` directory of that checkout; by default it is the checkout the
CLI was built from. The project only builds on a machine that has the checkout
at that path. If it moves, change those four files.

If you installed the CLI with `cargo install --git`, pass `--dsl-path` with a
checkout of your own: the default is then a checkout inside cargo's cache
(`~/.cargo/git/checkouts`), which cargo may remove.

In such a project `run` and `watch` work as described below, `bindings` and
`release` are not supported yet.

### Work with devices

```sh
krabik devices          # online devices and booted simulators
krabik devices -a       # also include offline AVDs and shutdown simulators
krabik launch Pixel_10                  # Android emulator
krabik launch "iPhone 16 Pro" --platform ios
```

`run` and `watch` pick a device automatically:

- **Android:** the only online device is used. If several are online, you are
  asked to choose. Pass a model name, AVD name, or `adb` serial to select one
  explicitly.
- **iOS:** the booted simulator is used, falling back to the first available
  one. Pass a simulator name or UDID to select one explicitly.

### Run a debug build

```sh
krabik run
krabik run Pixel_7
krabik run --platform ios
krabik run "iPhone 16 Pro" --platform ios
```

On Android, `run` detects the ABI of the connected device
(`ro.product.cpu.abi`) and builds the Rust library for that architecture. It
then copies the shared library into `jniLibs`, generates Kotlin bindings,
installs the debug app with Gradle, and launches `MainActivity`.

Use `--abi` to build for a different architecture:

```sh
krabik run --abi arm64-v8a
krabik run --abi armeabi-v7a
krabik run --abi x86_64
krabik run --abi x86
```

On iOS, `run` builds for the simulator, generates the Swift bindings, creates
the Xcode project, and installs and launches the app in the simulator.
`--abi` is only supported on Android.

In a krabik-ui-dsl project Gradle and Xcode build the Rust library themselves,
so `run` only installs the app with Gradle (for the ABI of the device, or
`--abi`) or builds it with `xcodebuild`, and launches it.

### Build a release

```sh
krabik release                      # Android APK
krabik release --platform ios       # iOS .app for a physical device
```

Android release builds compile the Rust library for all four Android targets,
generate the Kotlin bindings, and assemble the APK:

```text
android/app/build/outputs/apk/release/app-release-unsigned.apk
```

iOS release builds are compiled for physical devices and are unsigned:

```text
ios/build/Build/Products/Release-iphoneos/<App>.app
```

You will need to sign the resulting artifact before distributing it.

### Generate bindings only

```sh
krabik bindings                    # Kotlin
krabik bindings --platform ios     # Swift
```

Kotlin bindings are written to `android/app/src/main/java`; Swift bindings,
headers, and the module map are written to `ios/Generated`.

### Watch while developing

`watch` keeps the app in sync with your Rust code. What it does depends on the
project:

- **Hot reload**, for a krabik-ui-dsl app whose `rust/Cargo.toml` enables the
  `hot-reload` feature of `krabik-ui-dsl` (projects made with `--with-dsl` do).
  The crate is rebuilt and the library is handed to the running app, which
  continues with the new code and keeps its state. This takes a second or
  three instead of a reinstall.
- **Rebuild**, for every other project, including a krabik-ui-dsl app without
  that feature: it builds once up front, then rebuilds, reinstalls, and
  relaunches on every change.

```sh
krabik watch                       # Android (the default)
krabik watch --platform ios        # iOS simulator
krabik watch --platform both       # Android and iOS
```

- Watched paths: `rust/src`, `rust/Cargo.toml` and `rust/build.rs`; in a
  UniFFI project also both bindgen crates' sources and their `Cargo.toml`
  files. Paths that do not exist are skipped.
- The target directory is not watched, so builds do not retrigger themselves.
- Events from a single save are debounced into one build.
- A build failure is reported but does not stop the watch; the next save is
  picked up normally.
- The Android device is resolved once and reused for the whole session. On iOS
  the simulator is resolved on each build, so pass a simulator name or UDID to
  keep the target stable. On Android the optional argument is a device serial,
  model, or AVD name.
- Native Android or iOS source changes are not watched. Rebuild those manually.

For hot reload:

- Start the app first with `krabik run`; `watch` does not install or launch it.
  The first reload after `watch` starts builds the crate for the device, which
  can take half a minute once.
- It works on the Android emulator and the iOS simulator, with a debug build.
  Android needs `adb` and `ANDROID_NDK_HOME`.
- The state survives as long as the app's state and message types keep their
  layout. After adding a field or a variant the app refuses the library: this
  only shows in the app's log (`adb logcat -s krabik`), and `krabik run`
  restarts the app with the new code.
- A failing build is reported and the app keeps running the code it has.
- More about how it works and its limits:
  [hot-reload.md](../krabik-ui-dsl/docs/design/hot-reload.md).

### Clean the project

```sh
krabik clean
krabik clean --purge
```

`clean` removes `rust/target`, the Android build directories
(`android/build` and `android/app/build`), the generated Kotlin bindings, the
copied `.so` files, `ios/build`, and `ios/Generated`. Sources, configuration,
and the generated Xcode project are kept.

`--purge` additionally removes the cached Android directories
(`android/.gradle` and `android/.kotlin`).

## Project naming

A project name has to be valid as a Cargo package name, a Rust crate identifier,
a Java/Kotlin package segment, and part of an iOS bundle identifier. It must:

- start with a lowercase ASCII letter,
- contain only lowercase ASCII letters, digits, `-`, and `_`,
- be at most 64 characters long,
- not be a Rust or Java keyword.

Hyphens become underscores wherever an identifier is required. Underscores
become hyphens in iOS bundle identifiers.

| Name | Directory | Cargo package | Rust/Kotlin id | Android package | iOS bundle id |
| --- | --- | --- | --- | --- | --- |
| `my-app` | `my-app/` | `my-app` | `my_app` | `com.example.my_app` | `com.example.my-app` |
| `my_app` | `my_app/` | `my_app` | `my_app` | `com.example.my_app` | `com.example.my-app` |
| `demo` | `demo/` | `demo` | `demo` | `com.example.demo` | `com.example.demo` |

The `--package` prefix must be dot-separated valid Java identifiers, for example
`com.example` or `io.acme`.

## Generated project layout

```text
my-app/
├── rust/                          # Shared Rust library (cdylib + staticlib)
│   ├── src/lib.rs                 # Exported UniFFI functions
│   ├── Cargo.toml
│   ├── build.rs                   # Generates the Android JNI scaffolding
│   ├── uniffi-bindgen/            # Kotlin bindings generator
│   └── uniffi-bindgen-swift/      # Swift bindings generator
├── android/                       # Gradle app shell
│   └── app/src/main/
│       ├── java/…/MainActivity.kt
│       └── jniLibs/               # Rust .so files per ABI
├── ios/                           # XcodeGen project and SwiftUI app
│   ├── project.yml
│   ├── Sources/
│   └── Generated/                 # Swift bindings and C headers
└── rust-toolchain.toml            # Toolchain and required Rust targets
```

Both generated apps call `get_greeting()` from Rust and display the result.

With `--with-dsl`:

```text
my-app/
├── rust/                          # The app: a Rust library (cdylib + staticlib)
│   ├── src/lib.rs                 # A counter, `hot_reload!`, the two install functions
│   └── Cargo.toml                 # Depends on krabik-ui-dsl by absolute path
├── android/                       # Gradle app shell
│   ├── settings.gradle.kts        # Includes the framework's Kotlin library by absolute path
│   └── app/src/main/java/…/
│       ├── MainActivity.kt        # Loads the library and installs the app
│       └── AppBridge.kt           # Declares the install function implemented in Rust
├── ios/                           # XcodeGen project
│   ├── project.yml                # Compiles the framework's Swift host, builds ../rust
│   ├── Sources/                   # <App>App.swift and Bridging.h
│   └── Signing.xcconfig
└── rust-toolchain.toml            # Toolchain and required Rust targets
```

The app is a counter written with krabik-ui-dsl. Gradle and Xcode build the
Rust library as part of their own build, so there is no `jniLibs` directory and
no generated bindings. The Xcode project is generated and ignored by git. The
Gradle wrapper, the manifest and the resources are copied from
`example-todo-minimal/android` in the checkout. What each file is for is
explained in
[the tutorial](../krabik-ui-dsl/docs/tutorial/10-running-on-android-and-ios.md).

Notes:

- The generated `rust-toolchain.toml` declares all Android and iOS targets, so
  `rustup` installs them on the first build. Running `krabik toolchain` first is
  optional.
- UniFFI crates are pinned to the same git revision because
  `uniffi-bindgen-kotlin-jni` is not published on crates.io. A `--with-dsl`
  project does not use UniFFI.
- Android builds link against NDK API 24, matching the generated `minSdk`.
  Nothing machine-specific is written into the project; the NDK location is
  read from `ANDROID_NDK_HOME` at build time.
- The generated project is a single repository. The Rust crate is created with
  `--vcs none`, so `git init` in the project root is all you need.
- A `--with-dsl` project contains the absolute path of the krabik checkout, so
  a clone of it on another machine needs the checkout at the same path.
- iOS projects target iOS 16.0 and use Swift 5.9.

## Environment variables

| Variable | Purpose |
| --- | --- |
| `ANDROID_NDK_HOME` | Path to the Android NDK. Required for Android `run` and `release`, and for hot reload with `watch`. |
| `ANDROID_HOME` / `ANDROID_SDK_ROOT` | Android SDK location. `android info` is preferred when available. |
| `JAVA_HOME` | JDK used by Gradle. Optional, but if set it must point at an existing JDK. |
| `NO_COLOR` | Disables colored output. Colors are also disabled when stdout is not a terminal. |

## Troubleshooting

| Problem | Fix |
| --- | --- |
| `ANDROID_NDK_HOME is not set` | Install the NDK and export `ANDROID_NDK_HOME`. |
| `no online Android devices found` | Start an emulator with `krabik launch <name>` or connect a device. |
| `could not determine the ABI of …` | The device is still booting. Wait for it to be ready and retry. |
| `JAVA_HOME points to a missing directory` | Unset `JAVA_HOME` or point it at a valid JDK. |
| `xcodegen is not installed` | Run `brew install xcodegen`. |
| `no Rust sources found` | Run `watch` from the root of a generated project. |
| `… already exists` | Choose another name or remove the existing directory. |
| Gradle cannot find the SDK | Set `ANDROID_HOME` or configure the SDK in the Android CLI. |
| `krabik-ui-dsl not found at …` or `… is not a krabik-ui-dsl directory` | `create --with-dsl` needs a krabik checkout. Pass its `packages/krabik-ui-dsl` directory with `--dsl-path`. |
| `the path … contains …, which a project cannot refer to` | The checkout's path has a character that cannot be written into the project's build files. Move the checkout. |
| `release builds are not supported for krabik-ui-dsl projects yet` | Build the release with Gradle (`./gradlew :app:assembleRelease`) or Xcode. |
| `a krabik-ui-dsl project has no UniFFI bindings to generate` | Nothing to do: such a project has no bindings. |
| `watch`: `the library was not delivered` | The app is not installed or not running on that device. Start it with `krabik run` first. |
| `watch` says `reloaded` but the app does not change | The app refused the library because its state or message type changed (`adb logcat -s krabik`). Restart it with `krabik run`. |
| A `--with-dsl` project stopped building after the checkout moved | Fix the path in `rust/Cargo.toml`, `android/settings.gradle.kts`, `ios/project.yml` and `ios/Sources/Bridging.h`. |

## Developing krabik

```sh
cargo test                            # unit and integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The CLI is a library plus a thin binary, so behavior can be tested without
spawning a process:

- `src/lib.rs` — argument parsing and command dispatch (`krabik::run`)
- `src/main.rs` — the `krabik` binary
- `src/android.rs` — Android project creation, devices, build, release, and cleanup
- `src/ios.rs` — iOS project creation, simulators, build, release, and cleanup
- `src/rust.rs` — Rust crate generation, toolchain checks, and cleanup
- `src/naming.rs` — project name validation and identifier derivation
- `src/dsl.rs` — the template of a krabik-ui-dsl project (`create --with-dsl`)
- `src/hot.rs` — hot reload: building the app's crate and delivering the library to the running app
- `src/watch.rs` — the file-watching loop, which hot reloads or rebuilds
- `src/remove.rs` — helpers for removing generated files and directories

Tests live in `tests/`. They use a fake toolchain (`android`, `xcodegen`,
`cargo`, `adb`, `gradlew` as shell scripts that log their arguments, in
`tests/common`), so project generation, `run` and `watch` can be verified
without a full mobile SDK installation. The `--with-dsl` tests create projects
from a small fake framework directory, and one of them from this checkout.

CI (`.github/workflows/ci.yml`) runs formatting, clippy, and tests on Linux and
macOS, plus the test suite against the minimum supported Rust version (1.99.1).

The development toolchain is declared in `rust-toolchain.toml`, and the minimum
supported version is declared in `Cargo.toml` as `rust-version`.
