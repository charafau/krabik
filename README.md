# krabik

**Scaffold, build, and run cross-platform Rust mobile apps.**

`krabik` creates projects with a shared Rust core and native Android and iOS
shells, then drives the rest of the toolchain for you: installing Rust targets,
listing and booting devices, building native libraries, generating UniFFI
bindings, and installing and launching the app.



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

> `run`, `release`, `bindings`, and `watch` must be executed from the root of a
> generated project.

## What you need

At minimum you need Rust and `rustup`. Android and iOS builds have additional
requirements.

| Requirement | Needed for | Notes |
| --- | --- | --- |
| Rust 1.88+ with `rustup` | everything | Edition 2024. `rustup` installs the mobile targets. |
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
cargo install --path .
```

Or install directly from the repository:

```sh
cargo install --git https://github.com/charafau/krabik.git
```

This installs the `krabik` binary. Verify it with:

```sh
krabik --version
```

## Commands

| Command | Description | Run from |
| --- | --- | --- |
| `krabik doctor` | Check the development environment. | anywhere |
| `krabik create <name> [--package <prefix>]` | Scaffold a new project in `./<name>`. | anywhere |
| `krabik toolchain` | Install the Rust targets needed for Android and iOS. | anywhere |
| `krabik devices [-a]` | List devices. `-a`/`--all` also shows offline devices and shutdown simulators. | anywhere |
| `krabik launch <device> [--platform android\|ios]` | Boot an Android emulator or an iOS simulator. | anywhere |
| `krabik run [device] [--platform android\|ios] [--abi <abi>]` | Build, install, and launch a debug build. | project root |
| `krabik release [--platform android\|ios]` | Build release artifacts. | project root |
| `krabik bindings [--platform android\|ios]` | Generate Kotlin or Swift UniFFI bindings. | project root |
| `krabik watch [device] [--platform android\|ios\|both]` | Rebuild and relaunch on every Rust change. | project root |

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

`watch` keeps the app in sync with your Rust code. It builds once up front, then
rebuilds, reinstalls, and relaunches on every change.

```sh
krabik watch                       # Android (the default)
krabik watch --platform ios        # iOS simulator
krabik watch --platform both       # Android and iOS
```

- Watched paths: `rust/src`, both bindgen crates' sources, their `Cargo.toml`
  files, and `rust/build.rs`.
- The target directory is not watched, so builds do not retrigger themselves.
- Events from a single save are debounced into one build.
- A build failure is reported but does not stop the watch; the next save is
  picked up normally.
- The Android device is resolved once and reused for the whole session. On iOS
  the simulator is resolved on each build, so pass a simulator name or UDID to
  keep the target stable. On Android the optional argument is a device serial,
  model, or AVD name.
- Native Android or iOS source changes are not watched. Rebuild those manually.

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

Notes:

- The generated `rust-toolchain.toml` declares all Android and iOS targets, so
  `rustup` installs them on the first build. Running `krabik toolchain` first is
  optional.
- UniFFI crates are pinned to the same git revision because
  `uniffi-bindgen-kotlin-jni` is not published on crates.io.
- Android builds link against NDK API 24, matching the generated `minSdk`.
  Nothing machine-specific is written into the project; the NDK location is
  read from `ANDROID_NDK_HOME` at build time.
- The generated project is a single repository. The Rust crate is created with
  `--vcs none`, so `git init` in the project root is all you need.
- iOS projects target iOS 16.0 and use Swift 5.9.

## Environment variables

| Variable | Purpose |
| --- | --- |
| `ANDROID_NDK_HOME` | Path to the Android NDK. Required for Android `run` and `release`. |
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
- `src/android.rs` — Android project creation, devices, build, and release
- `src/ios.rs` — iOS project creation, simulators, build, and release
- `src/rust.rs` — Rust crate generation and toolchain checks
- `src/naming.rs` — project name validation and identifier derivation
- `src/watch.rs` — the file-watching rebuild loop

Tests live in `tests/`. The `create` tests use a fake `android`/`xcodegen`
toolchain, so project generation can be verified without a full mobile SDK
installation.

CI (`.github/workflows/ci.yml`) runs formatting, clippy, and tests on Linux and
macOS, plus the test suite against the minimum supported Rust version (1.88).

The development toolchain is declared in `rust-toolchain.toml`, and the minimum
supported version is declared in `Cargo.toml` as `rust-version`.
