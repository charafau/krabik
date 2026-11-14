# Building and CI

This chapter explains how an app turns into an APK or an iOS app: how Gradle and Xcode each run
`cargo`, how the Rust target is chosen, where the compiled library ends up, how signing is set
up, and what each CI job does. Afterwards you can read any build file in the package and predict
what a build will do.

There is one principle behind all of it: **each native build tool builds the Rust library itself,
as part of its normal build**. There is no separate "build Rust first" step to remember, on either
platform.

## What gets built

Each app (`harness/`, `example/`, `example-todo/`, `example-todo-minimal/`) has the same layout:

```text
example/
  rust/      the app crate; depends on ../../rust (the framework)
  android/   a Gradle project
  ios/       an Xcode project (generated from project.yml)
```

The app crate declares two library types:

```toml
# example/rust/Cargo.toml
[lib]
# The staticlib is linked into the iOS app; the cdylib is loaded by the
# Android app and exports the DSL's JNI entry points plus the install call.
crate-type = ["staticlib", "cdylib"]

[dependencies]
krabik-ui-dsl = { path = "../../rust" }
jni = "0.22"

[workspace]
```

- `cdylib` produces `libkrabik_ui_dsl_example.so` for Android, loaded with
  `System.loadLibrary`.
- `staticlib` produces `libkrabik_ui_dsl_example.a` for iOS, linked into the app binary.

The framework (`rust/Cargo.toml`) is a plain Rust library, an `rlib`. It is compiled *into* each
app's library. That is why one `.so` contains both the app and every `Java_dev_krabik_ui_*`
symbol, and one `.a` contains the app and every `krabik_ios_*` symbol.

The empty `[workspace]` table makes each app crate its own workspace, so cargo does not adopt a
parent one. The framework has a workspace too, containing itself and `rust/macros`. Every app
therefore has its own `target/` directory and its own `Cargo.lock`.

One oddity in the dependencies: the framework's iOS crates are declared under
`[target.'cfg(target_os = "ios")'.dependencies]`, but `jni` is an ordinary dependency, compiled
for every target including iOS and the host. The Android module itself is behind
`#[cfg(target_os = "android")]` in `rust/src/lib.rs`, so nothing uses `jni` elsewhere. The app
crates also export their JNI `install` function on every target.

## The Rust toolchain

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
targets = [
    "aarch64-apple-ios",
    "aarch64-apple-ios-sim",
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "i686-linux-android",
    "x86_64-linux-android",
]
```

When `cargo` runs anywhere under the package directory, rustup reads this file, uses the stable
channel, and installs the listed targets if they are missing. A *target* is a
CPU-and-operating-system combination the compiler can produce code for:

| Target | Used for |
| --- | --- |
| `aarch64-apple-ios` | a real iPhone or iPad |
| `aarch64-apple-ios-sim` | the simulator on an Apple-silicon Mac |
| `aarch64-linux-android` | ABI `arm64-v8a`: phones, and emulators on Apple silicon |
| `armv7-linux-androideabi` | ABI `armeabi-v7a`: old 32-bit phones |
| `x86_64-linux-android` | ABI `x86_64`: emulators on Intel machines and on CI |
| `i686-linux-android` | ABI `x86`: old 32-bit emulators |

The list also has `x86_64-apple-ios`, the simulator on an Intel Mac, which the iOS build script
can ask for (see below).

## Android

### The framework module: `shared/android/build.gradle.kts`

```kotlin
// shared/android/build.gradle.kts (trimmed)
plugins {
  alias(libs.plugins.android.library)
}

android {
  namespace = "dev.krabik.ui"
  compileSdk = 37
  defaultConfig { minSdk = 24 }
}

// Several app builds include this same directory; keep their outputs apart.
layout.buildDirectory.set(rootProject.layout.buildDirectory.dir("krabik-ui"))

dependencies {
  // Part of the public API: apps subclass `KrabikActivity`.
  api(libs.androidx.core.ktx)
  api(libs.androidx.fragment)
  api(libs.androidx.activity.ktx)
  api(libs.material)

  implementation(libs.androidx.recyclerview)
  implementation(libs.coil)
  implementation(libs.coil.network.okhttp)
}
```

The Kotlin half of the framework is an Android *library* module. It is not published anywhere;
each app includes the directory as a source module. Three details:

- `api` versus `implementation`: an `api` dependency is visible to the app that uses the library.
  Apps subclass `KrabikActivity`, a `FragmentActivity`, so they need those types. RecyclerView and
  Coil are internal.
- The build directory is redirected into the *including* app's build directory. Four Gradle
  projects include this one source directory; without the redirect they would all write to
  `shared/android/build` and overwrite each other.
- `libs.` refers to a version catalogue. The module has none of its own: it uses the catalogue of
  whichever app includes it (`<app>/android/gradle/libs.versions.toml`). So each app pins the
  versions, and the four catalogues must stay in step by hand.

The module has no `AndroidManifest.xml`; the `namespace` line is enough for a library with no
components.

### An app's `settings.gradle.kts`

```kotlin
// example/android/settings.gradle.kts (trimmed)
rootProject.name = "krabik-ui-dsl"

include(":app")

// The framework's Android library lives outside this project.
include(":krabik-ui")
project(":krabik-ui").projectDir = file("../../shared/android")
```

This is how a directory outside the project becomes the module `:krabik-ui`. The rest of the
file is the standard repository setup (`google()`, `mavenCentral()`) and the toolchain resolver
plugin, which lets Gradle download JDK 17 if it is missing.

### An app's `build.gradle.kts`

```kotlin
// example/android/app/build.gradle.kts (trimmed)
android {
  namespace = "com.example.krabik_ui_dsl"
  compileSdk = 37
  defaultConfig {
    applicationId = "com.example.krabik_ui_dsl_example"
    minSdk = 24
    targetSdk = 37
  }
  // View finders shared by the instrumented tests of every app.
  sourceSets["androidTest"].kotlin.srcDir(project(":krabik-ui").projectDir.resolve("../android-test"))
}

dependencies {
  implementation(project(":krabik-ui"))
  // ... androidTest dependencies
}

// Builds ../rust for Android and packages the library with the app.
extra["krabikMinSdk"] = android.defaultConfig.minSdk
apply(from = project(":krabik-ui").projectDir.resolve("rust.gradle.kts"))
```

- All four apps share the Kotlin namespace `com.example.krabik_ui_dsl` (which is why the JNI
  install symbols all start with `Java_com_example_krabik_1ui_1dsl_`) but have different
  `applicationId`s, so they can be installed side by side.
- `shared/android-test/` is added as a test source directory. It holds view finders used by every
  app's Espresso tests.
- The last two lines hand the minimum API level to the Rust build script and apply it.

There is no plugin for Kotlin in the `plugins` block. The top-level build file puts the Kotlin
Gradle plugin on the classpath, and its comment says the Android Gradle Plugin (9.x here)
compiles Kotlin itself; the classpath entry pins the compiler version.

### `shared/android/rust.gradle.kts`

This script is applied to every app module. It defines one task type and hooks it into the
Android build. Reading it top to bottom:

**ABI selection.**

```kotlin
// shared/android/rust.gradle.kts
fun hostArchitectureAbi(): String {
  val architecture = System.getProperty("os.arch").lowercase()
  return if (architecture == "aarch64" || architecture == "arm64") "arm64-v8a" else "x86_64"
}

fun configuredAbis(): List<String> =
  (providers.gradleProperty("krabikAbis").orNull
    ?: providers.environmentVariable("KRABIK_ABIS").orNull
    ?: hostArchitectureAbi())
    .split(',')
    .map { it.trim() }
    .filter { it.isNotEmpty() }
```

An *ABI* is Android's name for a CPU architecture. By default the script builds exactly one: the
one matching the machine running Gradle, because that is what a local emulator uses. Building all
four on every run would quadruple the Rust build time for no benefit during development. To build
others, pass `-PkrabikAbis=arm64-v8a,x86_64` or set `KRABIK_ABIS`. The table `rustAndroidTargets`
maps each ABI to its Rust target, its clang prefix and the name of cargo's linker variable.

A release build for the store must therefore name its ABIs explicitly; the default would ship
only the build machine's architecture.

**NDK lookup.** Rust can compile for Android by itself but needs the NDK's linker. The script
finds it in this order:

1. `ANDROID_NDK_HOME` or `ANDROID_NDK_ROOT`;
2. otherwise the SDK (`ANDROID_HOME`, `ANDROID_SDK_ROOT`, or `sdk.dir` in `local.properties`),
   and inside it `ndk-bundle`, or the highest version under `ndk/` (`compareVersions` compares
   the dotted numbers).

`ndkPrebuiltDirectory` then picks the host toolchain directory (`darwin-x86_64`,
`linux-x86_64`, ...), and `ndkClang` builds the compiler path, for example
`aarch64-linux-android24-clang`. The number in that name is the minimum API level, taken from
`krabikMinSdk`: the NDK provides one compiler wrapper per API level, and linking against the
app's minimum guarantees the library loads on the oldest supported device.

**The crate.** `rustCrateDirectory` is `../rust` relative to the Android project. The script
fails early with a clear message if `Cargo.toml` is not there. `cargoPackageName` reads the
`[package]` name with a small line parser, and hyphens become underscores to get the library
file name.

**The task.**

```kotlin
// shared/android/rust.gradle.kts (trimmed)
abis.get().forEachIndexed { index, abi ->
  execOperations.exec {
    workingDir = crateDirectory
    environment(linkerVariables.get()[index], linkers.get()[index])
    val arguments = mutableListOf(
      "cargo", "build",
      "--manifest-path", manifestFile.path,
      "--target", triples.get()[index],
    )
    if (profile.get() == "release") {
      arguments += "--release"
    }
    commandLine(*arguments.toTypedArray())
  }

  val library = File(
    crateDirectory,
    "target/${triples.get()[index]}/${profile.get()}/lib${libraryName.get()}.so",
  )
  val abiDirectory = File(outputRoot, abi).apply { mkdirs() }
  library.copyTo(File(abiDirectory, library.name), overwrite = true)
}
```

For each ABI: run `cargo build --target <triple>` with one environment variable set, for example
`CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=/path/to/aarch64-linux-android24-clang`, then copy the
resulting `.so` to `app/build/rustJniLibs/<debug|release>/<abi>/`. The script registers that
folder as a native-library folder of the matching build type, so debug and release each package
their own library and neither can pick up the other's. It is under `build/`, so it is generated
and never committed. Before building, the task deletes subdirectories of ABIs that are not
wanted this time, so a stale library cannot be packaged.

No cargo plugin (`cargo-ndk` and the like) is involved; it is plain cargo with a linker variable.

**Inputs and outputs.** `BuildRustLibs` declares its inputs: the crate's `src`, `Cargo.toml`,
`Cargo.lock`, `build.rs`, and, as `dependencySources`, the framework's `rust/src` and
`rust/macros/src` with both of their `Cargo.toml` files. Gradle skips the task when none of them
changed. The framework and its macro crate are listed because an app crate compiles them in;
without those inputs, editing either would not rebuild the app's library. (Cargo would notice,
but Gradle would never run cargo.)

**Hooking in.**

```kotlin
// shared/android/rust.gradle.kts
tasks.matching {
  it.name.startsWith("merge") &&
    (it.name.endsWith("JniLibFolders") || it.name.endsWith("NativeLibs"))
}.configureEach {
  val buildProfile = if (name.contains("Release")) "release" else "debug"
  rustBuildTasks[buildProfile]?.let { rustTask -> dependsOn(rustTask) }
}
```

Two tasks are registered, `buildRustLibsDebug` and `buildRustLibsRelease`. The Android tasks
that collect native libraries are made to depend on the matching one. So
`./gradlew :app:assembleDebug` builds the Rust library in debug and
`./gradlew :app:assembleRelease` builds it in release, with no extra command.

**Minified builds.** Rust finds the Kotlin classes, constructors and methods by name, which R8
cannot see. The library module therefore ships `consumer-rules.pro`, applied automatically to
every app that depends on it; it keeps the `dev.krabik.ui` package whole and every `native`
method. A minified build of the showcase was installed and driven by hand through several
screens to check this. The instrumented suites cannot run against a minified app (the test APK
expects library classes that minification removes from the app), so that check is manual. Two
things the rules do not cover: an app's drawables that Rust names as icons are found with
`getIdentifier`, so resource shrinking would remove them unless the app keeps them; and an app's
own classes that only Rust refers to need the app's own rules.

## iOS

### `project.yml` and XcodeGen

An `.xcodeproj` is a large generated file that is unpleasant to edit or review. Each app
describes its project in a short `project.yml`, and the tool XcodeGen generates the Xcode project
from it (`xcodegen generate` in the app's `ios/` directory). The generated project is committed,
so building does not require XcodeGen; only changing the project structure does.

```yaml
# example/ios/project.yml (trimmed)
name: KrabikUiDslExample
options:
  deploymentTarget:
    iOS: "16.0"

configFiles:
  Debug: Signing.xcconfig
  Release: Signing.xcconfig
settings:
  base:
    CODE_SIGN_STYLE: Automatic
    # The Rust build phase runs cargo, which reads and writes outside the
    # script's declared inputs.
    ENABLE_USER_SCRIPT_SANDBOXING: NO

targets:
  KrabikUiDslExample:
    type: application
    platform: iOS
    sources:
      - path: Sources
      # The Swift host shared by every app.
      - path: ../../shared/ios
      - path: Resources
        buildPhase: resources
    preBuildScripts:
      # Builds ../rust so the app never links a stale library.
      - name: Build Rust library
        basedOnDependencyAnalysis: false
        script: |
          "$SRCROOT/../../shared/build-rust-ios.sh"
    settings:
      base:
        RUST_LIB: krabik_ui_dsl_example
        SWIFT_OBJC_BRIDGING_HEADER: Sources/Bridging.h
        LIBRARY_SEARCH_PATHS:
          - $(SRCROOT)/../rust/target/xcode/$(PLATFORM_NAME)-$(CONFIGURATION)
        OTHER_LDFLAGS:
          - -l$(RUST_LIB)
```

What each part does:

- `sources`: the app's own `Sources/`, plus `shared/ios` compiled straight into the app target.
  There is no framework target and no Swift package.
- `preBuildScripts`: the Rust build runs before compilation. `basedOnDependencyAnalysis: false`
  makes Xcode run it on every build; cargo does its own up-to-date check, which is fast.
- `ENABLE_USER_SCRIPT_SANDBOXING: NO`: recent Xcode versions sandbox build scripts to their
  declared inputs and outputs. Cargo reads and writes the whole crate and its `target/`
  directory, so the sandbox is turned off.
- `RUST_LIB`, `LIBRARY_SEARCH_PATHS`, `OTHER_LDFLAGS`: the library's name, the directory the
  script writes it to, and the linker flag `-lkrabik_ui_dsl_example` that links it.
- `deploymentTarget` 16.0: required by the sheet's custom detent (chapter 5).

A second target, `KrabikUiDslExampleUITests`, holds the XCUITests. The four apps' `project.yml`
files differ only in names, bundle identifiers and `RUST_LIB`.

### `shared/build-rust-ios.sh`

```sh
# shared/build-rust-ios.sh (trimmed)
cd "$SRCROOT/../rust"

if [ "$CONFIGURATION" = "Release" ]; then
  profile=release
  flags=--release
else
  profile=debug
  flags=
fi

out="target/xcode/$PLATFORM_NAME-$CONFIGURATION"
mkdir -p "$out"

libraries=
for arch in $ARCHS; do
  case "$PLATFORM_NAME-$arch" in
    iphoneos-arm64) target=aarch64-apple-ios ;;
    iphonesimulator-arm64) target=aarch64-apple-ios-sim ;;
    iphonesimulator-x86_64) target=x86_64-apple-ios ;;
    *)
      echo "error: no Rust target for $PLATFORM_NAME on $arch" >&2
      exit 1
      ;;
  esac
  cargo build --target "$target" $flags
  libraries="$libraries target/$target/$profile/lib$RUST_LIB.a"
done

lipo -create $libraries -output "$out/lib$RUST_LIB.a"
```

**How it picks targets.** Xcode sets environment variables for build scripts. The script uses
four: `SRCROOT` (the project directory), `CONFIGURATION` (`Debug` or `Release`), `PLATFORM_NAME`
(`iphoneos` for a device, `iphonesimulator` for a simulator) and `ARCHS` (the CPU architectures
of this build). The pair of platform and architecture selects the Rust target:

| `PLATFORM_NAME` | arch | Rust target |
| --- | --- | --- |
| `iphoneos` | `arm64` | `aarch64-apple-ios` |
| `iphonesimulator` | `arm64` | `aarch64-apple-ios-sim` |
| `iphonesimulator` | `x86_64` | `x86_64-apple-ios` |

A device and an Apple-silicon simulator are both arm64, yet they are different targets: the
binaries are marked for different platforms and the linker refuses to mix them. That is why the
platform name is part of the decision and why the output directory includes it.

`ARCHS` can hold two values (a simulator build for both Mac architectures). The script builds
each and `lipo -create` merges them into one file. With a single architecture `lipo` copies it.
Either way the linker finds exactly one `lib<name>.a` in
`target/xcode/<platform>-<configuration>/`, the directory `LIBRARY_SEARCH_PATHS` names.

The first line of real work, `export PATH="$HOME/.cargo/bin:$PATH"`, is there because Xcode runs
scripts with a minimal `PATH` that does not include cargo.

The script was added in commit `db4639f` so that one project builds for a device, an
Apple-silicon simulator or an Intel simulator without editing anything.

### Signing: `Signing.xcconfig` and `Local.xcconfig`

```text
// example/ios/Signing.xcconfig
// Signing settings for this app. Create `Local.xcconfig` next to this file
// with `DEVELOPMENT_TEAM = <your team id>` to build for a device; it is
// ignored by git.
#include? "Local.xcconfig"
```

An `.xcconfig` file is a list of build settings. `#include?` with a question mark includes a
file only if it exists. So:

- The **simulator** needs no signing identity. A fresh clone builds and runs there with no setup.
- A **device** needs a development team. Signing is automatic (`CODE_SIGN_STYLE: Automatic`), so
  Xcode creates the certificate and profile, but it must know which team. That one value goes in
  `Local.xcconfig`, which the package's `.gitignore` excludes, so a team id is never committed.

### Device versus simulator, honestly

Device builds were compiled, and the CI workflow has a step that builds the example's static
library for `aarch64-apple-ios`. No app has been run on a physical iPhone. Anything that only
shows on real hardware (signing, performance, the real keyboard, safe areas on particular models)
is untested.

## The CI workflow

The file is `.github/workflows/ci.yml` at the repository root. It covers the whole repository;
this section describes the parts about this package.

**It has not run yet.** The repository's remote is not GitHub, so no job in this file has ever
executed. Every command in it was written from local runs. Expect to fix things the first time it
does run: action versions, runner images, SDK package names and simulator names are the usual
suspects.

It triggers on every push and pull request, and `concurrency` cancels an older run on the same
branch when a newer one starts.

### `check` (Linux and macOS)

Formatting, lints and unit tests, on both operating systems. After the repository's root
workspace it handles each standalone project by manifest path:

```yaml
# .github/workflows/ci.yml
- run: cargo fmt --manifest-path packages/krabik-ui-dsl/rust/Cargo.toml --all --check
# `--workspace` includes the macro crate next to the library
- run: cargo clippy --manifest-path packages/krabik-ui-dsl/rust/Cargo.toml --workspace --all-targets -- -D warnings
- run: cargo test --manifest-path packages/krabik-ui-dsl/rust/Cargo.toml --workspace
```

followed by `fmt`, `clippy` and `test` for the harness and each of the three example crates.
These run for the *host* target. That compiles the engine, the session (with its fake-platform
tests), the macros and each app's logic, but neither platform renderer: `rust/src/android/` and
`rust/src/ios/` are behind `cfg(target_os)`. `-D warnings` turns every lint into a failure.

### `msrv`

Runs the root workspace's tests with the minimum supported Rust version, 1.99.1 (the root
`Cargo.toml` declares `rust-version = "1.99.1"`). The environment variable `RUSTUP_TOOLCHAIN`
overrides toolchain files. This job does not run anything in this package.

### `android`

Builds a debug APK of all four apps for `arm64-v8a`:

```yaml
# .github/workflows/ci.yml
- name: Assemble the dev harness debug APK
  working-directory: packages/krabik-ui-dsl/harness/android
  run: ./gradlew :app:assembleDebug --no-daemon -PkrabikAbis=arm64-v8a
```

It installs the Rust Android target, JDK 17, the Android platform 37, build tools and NDK
27.2.12479018. An APK build compiles three things the `check` job cannot: the Android renderer
(Rust for an Android target), the Kotlin library, and the link between them. As the comment
says, it also proves that each app's `.so` exports the JNI `install` symbol its Kotlin bridge
declares. Well, nearly: a missing JNI symbol is only detected when the method is first called, so
what the build really proves is that everything compiles and links.

The ABI is passed explicitly because the runner is x86_64 and the default would build for that.

### `ios`

Runs on macOS. First it lints the iOS renderer, which no other job compiles:

```yaml
# .github/workflows/ci.yml
- run: cargo clippy --manifest-path packages/krabik-ui-dsl/rust/Cargo.toml --target aarch64-apple-ios-sim -- -D warnings
```

Then it builds the example's static library for a device (`aarch64-apple-ios`), "a real device
is a different target from the simulator; keep it building", and each app's static library for
the simulator. It does not run Xcode.

The `android` job has the matching step for the Android renderer
(`cargo clippy --target aarch64-linux-android`) before it assembles the apps.

### `android-ui`

Runs every app's Espresso tests on an emulator. The emulator and the runner are both x86_64, so
the apps are built with `-PkrabikAbis=x86_64`. The job enables KVM (hardware acceleration for the
emulator on Linux runners) and uses the `android-emulator-runner` action with API level 35:

```yaml
# .github/workflows/ci.yml
script: |
  cd packages/krabik-ui-dsl/harness/android && ./gradlew :app:connectedDebugAndroidTest --no-daemon -PkrabikAbis=x86_64
  cd packages/krabik-ui-dsl/example/android && ./gradlew :app:connectedDebugAndroidTest --no-daemon -PkrabikAbis=x86_64
  # ... example-todo, example-todo-minimal
```

Each line starts with a `cd` from the repository root. That is only correct if the action runs
each line as a separate shell command starting from the workspace root. If it ran the script as
one shell session, the second `cd` would fail. This is one of the things to check on the first
real run.

`disable-animations: false` leaves animations on, so the tests exercise the real transitions.
Test reports are uploaded when the job fails.

### `ios-ui`

Runs every app's XCUITests on a simulator. Because the image's simulators change over time, the
job picks the first available iPhone by parsing `xcrun simctl list` and stores its identifier:

```yaml
# .github/workflows/ci.yml
- name: Test the dev harness
  working-directory: packages/krabik-ui-dsl/harness/ios
  run: xcodebuild test -project KrabikUiDsl.xcodeproj -scheme KrabikUiDsl -destination "platform=iOS Simulator,id=$SIMULATOR"
```

There is no separate Rust build step here: each Xcode project runs `build-rust-ios.sh` in its
build phase. This job is therefore the only one that exercises that script.

### What CI covers and what it does not

| | Unit tests | Lints | Compiles | Runs |
| --- | --- | --- | --- | --- |
| Engine, session, macros | `check` | `check` | `check` | `check` |
| Android renderer (Rust) | none | none | `android` | `android-ui` |
| Kotlin shell | none | none | `android` | `android-ui` |
| iOS renderer (Rust) | none | `ios` | `ios` | `ios-ui` |
| Swift shell | none | none | `ios-ui` | `ios-ui` |
| Device build (iOS) | none | none | `ios` (Rust only) | none |

The platform renderers have no unit tests; they are covered only by the UI test suites.

## Where this lives

- `rust-toolchain.toml`: channel and targets.
- `rust/Cargo.toml`, `<app>/rust/Cargo.toml`: crate types, dependencies, workspaces.
- `shared/android/build.gradle.kts`: the `:krabik-ui` library module.
- `shared/android/rust.gradle.kts`: `configuredAbis`, `androidNdkDirectory`, `BuildRustLibs`.
- `<app>/android/settings.gradle.kts`, `<app>/android/app/build.gradle.kts`,
  `<app>/android/gradle/libs.versions.toml`.
- `<app>/ios/project.yml`, `<app>/ios/Signing.xcconfig`, `shared/build-rust-ios.sh`.
- `../../.github/workflows/ci.yml`.

Next: [The macros](07-the-macros.md).
