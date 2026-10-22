# mobile_cli

Scaffold and build cross-platform Rust mobile apps. `create` generates a project
containing a UniFFI-based Rust library plus Android (Gradle/Kotlin) and iOS
(XcodeGen/Swift) app shells, and the CLI drives the rest of the toolchain:
installing Rust targets, listing and launching emulators, building native
libraries, generating Kotlin/Swift UniFFI bindings, and installing and
launching the app on a device or simulator.

Commands exit non-zero on failure, so they can be scripted and used in CI.

## Prerequisites

- Rust toolchain (edition 2024, Rust 1.88+)
- `rustup` (used to install the Android/iOS targets)

Android:

- Android SDK with NDK — `ANDROID_NDK_HOME` must point at an NDK installation
  (required by `run` and `release`)
- `adb` on your `PATH`
- A JDK on your `PATH` — Gradle needs it. When `JAVA_HOME` is set it has to
  point at an existing JDK, otherwise Gradle fails with a confusing error.
- Google's Android CLI (`android`) — used to generate the Gradle project shell
  and to manage emulators. It must support the `--namespace` and
  `--application-id` options of `android create`.

iOS:

- Xcode (`xcodebuild`, `xcrun simctl`)
- [xcodegen](https://github.com/yonaskolb/XcodeGen) (`brew install xcodegen`)

Run `mobile_cli doctor` to check what is installed and what is missing. It
reports the Rust toolchain, the Android CLI, `adb`, a JDK and a valid
`JAVA_HOME`, the Android SDK, the NDK, the Xcode tools and the Rust
compilation targets that are missing.

## Install

```sh
cargo install --path .
```

## Usage

| Command                       | Description                                                  |
| ----------------------------- | ------------------------------------------------------------ |
| `doctor`                      | Check the development environment                            |
| `create <name> [--package <p>]` | Scaffold a new project (`./<name>/rust`, `android`, `ios`); `--package` overrides the `com.example` prefix for the app/bundle id |
| `toolchain`                   | Install the Rust targets needed for Android and iOS          |
| `devices [-a/--all]`          | List online devices (use `-a` to include offline ones)       |
| `launch <emulator>`           | Boot an emulator/simulator (`--platform android\|ios`)       |
| `run [device]`                | Build, install and launch a debug build (`--abi` overrides the detected one) |
| `release`                     | Build release artifacts (`--platform android\|ios`)          |
| `bindings`                    | Generate UniFFI Kotlin/Swift bindings                        |
| `watch [device]`              | Watch the Rust sources and rebuild on every change (`--platform android\|ios\|both`) |

Typical workflow:

```sh
mobile_cli doctor          # verify your environment
mobile_cli toolchain       # install Android/iOS Rust targets
mobile_cli create my_app                      # scaffold a new project
mobile_cli create my_app --package io.acme    # ...with a custom package prefix
cd my_app
mobile_cli devices -a      # see what is available
mobile_cli launch Pixel_10 # boot an emulator
mobile_cli run             # build, install and launch on it
```

`run`, `release` and `bindings` are run from inside a project directory.
`release` builds release Rust libraries for all Android targets (assembles no
APK) and an unsigned iOS app for physical devices.

## Watching

`watch` keeps the app in sync with the Rust sources: every change rebuilds
the library, reinstalls the app and launches it again.

```sh
cd my_app
mobile_cli watch                          # Android
mobile_cli watch --platform ios           # iOS simulator
mobile_cli watch --platform both         # both
```

The project is built once up front, so the app is up to date when the watch
starts. Afterwards only the changed crate is recompiled, which takes a few
seconds.

- The Rust sources of the project are watched: `rust/src`, the manifests and
  the build script. The target directory is not watched, the build writes to
  it and every write would trigger another build.
- The events of a single save are collected, so a file that is written in
  several steps only triggers one build.
- A build that fails is reported, but does not stop the watch. The next save
  is picked up as usual.
- The device is resolved once, so the watch does not ask for it again.

## Project names

The project name has to work as a Cargo package name, as a Rust crate
identifier, as a Java/Kotlin package segment and as part of an iOS bundle
identifier, so it must

- start with a lowercase ASCII letter,
- contain only lowercase ASCII letters, digits, `-` and `_`,
- not be a Rust or Java keyword.

Hyphens are allowed because Cargo package names use them (`my-app`); they are
translated to underscores wherever an identifier requires it. The identifiers
derived from the name are used consistently across the generated project:

| Name        | Cargo package | Rust/Kotlin id | Java package          | iOS bundle id         |
| ----------- | ------------- | -------------- | --------------------- | --------------------- |
| `my-app`    | `my-app`      | `my_app`       | `com.example.my_app`  | `com.example.my-app`  |
| `my_app`    | `my_app`      | `my_app`       | `com.example.my_app`  | `com.example.my-app`  |
| `demo`      | `demo`        | `demo`         | `com.example.demo`    | `com.example.demo`    |

## Generated project layout

```
my_app/
├── rust/                  # UniFFI library (cdylib/staticlib)
│   ├── uniffi-bindgen/    # Kotlin bindings generator crate
│   └── uniffi-bindgen-swift/  # Swift bindings generator crate
├── android/               # Gradle project (generated by the Android CLI)
├── ios/                   # XcodeGen project.yml + Swift sources
└── rust-toolchain.toml    # The toolchain and targets the project needs
```

Notes:

- The project is created in a staging directory and only moved into place once
  every part was generated successfully, so a failed `create` leaves nothing
  behind and can simply be retried.
- The generated crates pin all `uniffi` crates to the same git revision
  (`uniffi-bindgen-kotlin-jni` is not published on crates.io).
- The Rust crate is not a repository of its own — the project is a single
  repository, so `git init` in the project root is all that is needed.
- `rust-toolchain.toml` declares the Android and iOS compilation targets, so
  rustup installs them on the first build. The project does not depend on the
  machine being set up beforehand — `mobile_cli toolchain` is not required.
- Android builds link against NDK API 24, matching the generated `minSdk`.
  Linker paths are resolved from `ANDROID_NDK_HOME` at build time — nothing
  machine-specific is written into the project.
- `run` builds the Rust library for the architecture of the device it is
  installed on. The ABI is read from the device (`ro.product.cpu.abi`), and
  can be overridden with `--abi arm64-v8a|armeabi-v7a|x86_64|x86`.

## Development

```sh
cargo test                          # unit + integration tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

CI (`.github/workflows/ci.yml`) runs the same three checks on Linux and macOS
on every push and pull request, and additionally runs the tests against the
minimum supported Rust version.

The CLI logic lives in `src/lib.rs` (`mobile_cli::run`), with `src/main.rs` as
a thin wrapper, so behavior can be tested without spawning a process. Project
names are validated and every identifier is derived from them in `src/naming.rs`
(`mobile_cli::naming::ProjectName`).

The toolchain this project is developed with is declared in
`rust-toolchain.toml`, and the minimum supported version in `Cargo.toml` as
`rust-version`.

`tests/create.rs` runs `create` end to end with a fake `android`/`xcodegen`
toolchain, so the generated project is verified without a full Android/Xcode
installation.
