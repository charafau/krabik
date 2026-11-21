# 10. Running on Android and iOS

This chapter puts the Notes app on an Android emulator and an iOS simulator, and shows how to
edit it while it runs. `krabik create notes --with-dsl` in chapter 1 already made everything that
is needed, so the first two sections are three commands. The rest of the chapter explains what
was created: the three-directory layout of an app, the two install functions your crate exports,
what the Android and iOS host projects contain and how they build the Rust code for you, and how
to build for a real device. Afterwards you will know what each file in the project is for.

Nothing in this chapter changes the app's behaviour.

## Run it

Start an Android emulator, from Android Studio's Device Manager or with the CLI
(`krabik devices -a` lists what you have):

```sh
krabik launch Pixel_10
```

Then, from the project's directory:

```sh
cd notes
krabik run
```

This builds the Rust library and the Kotlin shell, installs the app on the emulator and starts
it. The first build compiles the framework and takes a few minutes; later ones are quick.

iOS is the same with one more argument:

```sh
krabik launch "iPhone 17" --platform ios
krabik run --platform ios
```

`krabik doctor` checks the tools both builds need and says which one is missing.

## Edit it while it runs

Leave the app running and start, in the project's directory:

```sh
krabik watch                  # or: krabik watch --platform ios
```

Now change something in `rust/src/lib.rs`, a label for example, and save. A second or so later
the running app shows the change, and it keeps its state: the notes you added are still there.
The terminal says:

```text
Android: reloaded in 0.7s
```

This is hot reload. `krabik watch` rebuilds your crate and hands the new library to the running
app, which continues with the new `view` and `update` and the state it had. It works because of
two lines `krabik create` wrote for you, the `hot-reload` feature in `rust/Cargo.toml` and
`krabik_ui_dsl::hot_reload!(...)` in `lib.rs`; both are shown below.

Three things to know:

- **A build error does no harm.** `watch` prints it, the app keeps running the code it has, and
  the next save tries again.
- **A change to the fields of the state or to the variants of the message type cannot be
  reloaded.** The running state has the old layout. The app refuses the library and writes that
  to its log (`adb logcat -s krabik`); `watch` does not see the refusal and still reports
  "reloaded". When a change does not show up, start the app again with `krabik run`.
- **It is for the emulator and the simulator**, and for development only.

[`../design/hot-reload.md`](../design/hot-reload.md) has the mechanism and the full list of
limits.

## How the pieces fit

```text
  notes/
    rust-toolchain.toml       the Rust targets the builds need
    rust/                     your crate: the app + the framework compiled in
      src/lib.rs
      Cargo.toml
    android/                  a Gradle project
      app/
        build.gradle.kts      application id, dependencies, "build ../rust"
        src/main/
          AndroidManifest.xml
          java/.../MainActivity.kt   loads the library, installs the app
          java/.../AppBridge.kt      declares the native install function
          res/                       theme, launcher icon
      settings.gradle.kts     includes the framework's Kotlin library
    ios/                      an Xcode project
      project.yml             the project's description (XcodeGen)
      Sources/
        NotesApp.swift        installs the app, shows the host view
        Bridging.h            declares the C install function
      Signing.xcconfig

  <krabik checkout>/packages/krabik-ui-dsl/
    rust/                     the framework's crate
    shared/                   the framework's native halves, for every app
      android/                the Kotlin library dev.krabik.ui (:krabik-ui)
        rust.gradle.kts       builds an app's Rust crate from Gradle
      ios/                    the Swift host, compiled into every iOS app
      build-rust-ios.sh       builds an app's Rust crate from Xcode
```

The host projects are deliberately almost empty. All the native code that matters (the root
view, the navigation controller, the widgets Rust drives) lives once, in `shared/`. Before the
restructure each app carried its own copy of those files; commits `5b81899` and `c976d12` turned
them into one Kotlin library module and one set of Swift files, which is why an app's shell is
now a handful of short files.

The framework is not published anywhere yet. Your project therefore refers to the krabik checkout
by its absolute path, in four files: `rust/Cargo.toml`, `android/settings.gradle.kts`,
`ios/project.yml` and `ios/Sources/Bridging.h`. The listings below write that path as
`/path/to/krabik-ui-dsl`. The project builds on a machine that has the checkout at that path; if
the checkout moves, change those four files.

At run time the flow is the same on both platforms:

```text
  1. the OS starts the app's native entry point
       Android: MainActivity (a KrabikActivity)
       iOS:     the SwiftUI App struct
  2. the native code loads, or is linked with, your Rust library
  3. it calls your install function
       -> set_app_factory(NotesApp::new)
  4. the shared host creates its root view
       -> Rust creates a session, calls the factory,
          renders the first tree
```

Step 3 must come before step 4. Chapter 1 quoted the panic you get otherwise: "no application
registered; call set_app_factory before creating a root view".

## The Rust side: two install functions

The native code cannot call `NotesApp::install()` directly; it can only call functions with a C
or JNI signature. So the crate exports one of each. They are at the bottom of
`notes/rust/src/lib.rs`, before the tests (with `Counter` in place of `NotesApp` until you
renamed it in chapter 2):

```rust,ignore
use jni::EnvUnowned;
use jni::objects::JObject;

/// Called by the iOS host before it creates the root view.
#[unsafe(no_mangle)]
pub extern "C" fn notes_install() {
    NotesApp::install();
}

/// Called by the Android host before it creates the root view.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_notes_AppBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    NotesApp::install();
}
```

The second one is why `rust/Cargo.toml` depends on the `jni` crate.

This is the same pair every example app has; compare the end of
`example-todo-minimal/rust/src/lib.rs`.

- `#[unsafe(no_mangle)]` keeps the function's name as written in the compiled library, so native
  code can find it by name.
- The iOS function is plain C: no arguments, no result. Its name is yours to choose; it has to
  match the declaration in `Bridging.h` below.
- The Android function's name is dictated by JNI: `Java_`, then the package with dots replaced
  by underscores, then the class, then the method. `Java_com_example_notes_AppBridge_install`
  is the method `install` of `com.example.notes.AppBridge`. An underscore *inside* a package
  or class name is written `_1`, which is why the examples, whose package is
  `com.example.krabik_ui_dsl`, export `Java_com_example_krabik_1ui_1dsl_MinimalTodoBridge_install`.
  `krabik create` does this for you: a project called `my-app` gets the package
  `com.example.my_app` and the function `Java_com_example_my_1app_AppBridge_install`.

Both functions compile on every target, including your desktop, because the `jni` crate is plain
Rust. So `cargo test` keeps working unchanged.

The framework's own entry points (create a root view, report a click, and so on) are also
exported from your library, because your crate compiles the framework in. On Android they are
JNI functions bound to the shared `dev.krabik.ui.RustRootView` class
(`rust/src/android/entry.rs`); on iOS they are the C functions declared in
`shared/ios/KrabikIOS.h`. You never call them. They are the subject of
[`../internals/`](../internals/).

Just above the install functions is the line that makes hot reload possible:

```rust,ignore
// Lets `krabik watch` hand a rebuilt library to the running app. The state
// is kept as long as the layout of `NotesApp` and `Msg` stays the same.
krabik_ui_dsl::hot_reload!(NotesApp);
```

It exports two more functions from the library, which the framework uses to check that a new
library has the same state layout and to hand the running app over to it. The macro only exists
when the `hot-reload` feature of `krabik-ui-dsl` is enabled, which `rust/Cargo.toml` does. For
a build you ship, remove the feature and this line. (The generated `impl Default` has nothing
to do with any of this; it is there because clippy asks for one next to a `new()` without
arguments.)

The `crate-type` from chapter 1 now earns its place:

```toml
[lib]
crate-type = ["staticlib", "cdylib"]
```

Android loads a shared library at run time, so it needs the `cdylib` (`libnotes.so`). iOS links
a static library into the app binary, so it needs the `staticlib` (`libnotes.a`).

## Toolchains

`rust-toolchain.toml` in the project's directory lists the compilation targets:

```toml
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

rustup reads this file for any crate below that directory and installs the targets on first use,
so there is nothing to install by hand. The framework's package has a file like it, which is
why the examples inside the checkout need none of their own. `krabik toolchain` installs the
targets explicitly, if you prefer that. On an Intel Mac, add `"x86_64-apple-ios"` to the list for
the simulator.

Beyond Rust you need:

- **Android**: JDK 17, the Android SDK with platform 37 (`compileSdk = 37` in the build files),
  and an NDK.
- **iOS**: Xcode with a simulator, and [XcodeGen](https://github.com/yonaskolb/XcodeGen)
  (`brew install xcodegen`), which generates the Xcode project.

`krabik doctor` checks all of these.

## The Android host

`krabik create` wrote the five files described below. Everything else in `notes/android` (the
Gradle wrapper, `gradle.properties`, `gradle/libs.versions.toml`, the top-level
`build.gradle.kts`, the manifest and the launcher icons) is the same in every app and was copied
from `example-todo-minimal/android` in the checkout.

### `settings.gradle.kts`

Two lines matter. The rest is repository boilerplate.

```kotlin
rootProject.name = "notes"

include(":app")

// The framework's Android library, in a local krabik checkout.
include(":krabik-ui")
project(":krabik-ui").projectDir = file("/path/to/krabik-ui-dsl/shared/android")
```

The framework's Kotlin code is pulled in as a Gradle module from a path outside the project. It
is a normal Android library with the namespace `dev.krabik.ui` (`shared/android/build.gradle.kts`),
and it brings its own dependencies: AndroidX fragment and activity, Material, RecyclerView.

### `app/build.gradle.kts`

```kotlin
plugins {
  alias(libs.plugins.android.application)
}

android {
  namespace = "com.example.notes"
  compileSdk = 37

  defaultConfig {
    applicationId = "com.example.notes"
    minSdk = 24
    targetSdk = 37
    versionCode = 1
    versionName = "1.0"
  }

  buildTypes {
    release {
      isMinifyEnabled = false
    }
  }
  compileOptions {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
  }
}

kotlin {
  jvmToolchain(17)
}

dependencies {
  // The framework's Android half.
  implementation(project(":krabik-ui"))
}

// Builds ../rust for Android and packages the library with the app.
extra["krabikMinSdk"] = android.defaultConfig.minSdk
apply(from = project(":krabik-ui").projectDir.resolve("rust.gradle.kts"))
```

`namespace` is the Kotlin package of your two classes and must agree with the JNI function name
in Rust. `applicationId` is the identity of the installed app. `krabik create` sets both from
`--package` (default `com.example`) and the project's name.

The last two lines are what connect Gradle to cargo. They are explained below.

### `MainActivity.kt` and `AppBridge.kt`

`app/src/main/java/com/example/notes/MainActivity.kt`:

```kotlin
package com.example.notes

import dev.krabik.ui.KrabikActivity

/** Loads the app's library, which includes the framework, and installs the app. */
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("notes")
    }

    override fun installApp() {
        AppBridge.install()
    }
}
```

`app/src/main/java/com/example/notes/AppBridge.kt`:

```kotlin
package com.example.notes

/** Installs the app. [MainActivity] calls this once the library is loaded. */
object AppBridge {
    /** Registers the app with the framework; implemented in `rust/src/lib.rs`. */
    external fun install()
}
```

That is all the Kotlin an app contains. `KrabikActivity`
(`shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt`) does the rest, and it calls your
two overrides in the order that matters, at the top of `onCreate`:

```kotlin
loadNativeLibrary()
installApp()
retained = ViewModelProvider(this)[RetainedApp::class.java]
root = RustRootView(this, retainedApp = retained.id)
```

Three names have to agree across languages, and a mismatch in any of them is a crash at startup,
not a compile error. They agree in a new project; this table is for the day you rename
something:

| In Rust | In Kotlin | If they differ |
| ------- | --------- | -------------- |
| package `name = "notes"` in `Cargo.toml` | `System.loadLibrary("notes")` | `UnsatisfiedLinkError`: library not found |
| `Java_com_example_notes_AppBridge_install` | package `com.example.notes`, object `AppBridge`, method `install` | `UnsatisfiedLinkError`: no implementation found |
| `Icon::new().drawable("ic_add")` | `res/drawable/ic_add.xml` | no crash; the icon is missing |

The library file is named after the Cargo package with dashes turned into underscores
(`rustLibraryName` in `rust.gradle.kts`), so a package called `my-notes` is loaded with
`System.loadLibrary("my_notes")`.

### The manifest and resources

`app/src/main/AndroidManifest.xml` is the copied one. Three attributes in it are part of how the
framework behaves, so do not drop them:

```xml
<application
    ...
    android:enableOnBackInvokedCallback="true"
    android:theme="@style/Theme.KrabikUiDsl">
    <!-- The Rust session lives as long as the activity, so rotation and
         similar changes resize the app instead of recreating it. -->
    <activity
        android:name=".MainActivity"
        android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation"
        android:exported="true"
        android:windowSoftInputMode="adjustResize">
```

- `android:configChanges` makes rotation resize the running activity instead of recreating it.
  Night mode is absent from the list on purpose (chapter 7).
- `android:enableOnBackInvokedCallback="true"` opts in to the predictive back gesture, which
  works with `App::back` returning `None` on the root screen (chapter 5).
- `android:windowSoftInputMode="adjustResize"` lets the layout make room for the keyboard.

The theme in `res/values/themes.xml` is one line, and its parent is what gives the native chrome
a light and a dark variant:

```xml
<style name="Theme.KrabikUiDsl" parent="Theme.Material3.DayNight.NoActionBar" />
```

`app_name` in `res/values/strings.xml` is the name under the launcher icon. It is `notes`;
change it to `Notes`.

One thing is missing from a new project: the icon the floating button of chapter 6 asks for.
Icons are looked up by resource name at run time
(`resources.getIdentifier(icon, "drawable", packageName)` in `KrabikHostController.kt`), so the
drawable has to exist in *your* app. Copy
`example/android/app/src/main/res/drawable/ic_add.xml` from the checkout to
`notes/android/app/src/main/res/drawable/ic_add.xml`.

### How Gradle builds the Rust code

`shared/android/rust.gradle.kts` is applied by every app. Its header says what it does:

```kotlin
// Every build that packages native libraries first runs `cargo build` for the
// selected Android ABIs and copies the resulting `.so` files into
// `build/rustJniLibs/<debug|release>`, which is registered as a native
// library folder of that build type. Debug and release therefore never
// package each other's library. The ABI defaults to the architecture of the machine
// running Gradle, which matches local emulators; override it with
// `-PkrabikAbis=arm64-v8a,x86_64` (or the KRABIK_ABIS environment variable).
```

In more detail:

- It expects the crate at `../rust` relative to the Android project and fails with a clear
  message if `Cargo.toml` is not there.
- It registers two tasks, `buildRustLibsDebug` and `buildRustLibsRelease`, and makes the
  Android tasks that merge native libraries depend on them. You never run them yourself; a normal
  `assembleDebug` triggers the right one.
- For each ABI it runs `cargo build --manifest-path ../rust/Cargo.toml --target <triple>` (plus
  `--release` for a release build) with the NDK's clang set as the linker, then copies
  `lib<name>.so` into `app/build/rustJniLibs/<debug|release>/<abi>/`.
- The linker is chosen for the app's minimum API level, which is what
  `extra["krabikMinSdk"] = android.defaultConfig.minSdk` passes in.
- It finds the NDK through `ANDROID_NDK_HOME` or `ANDROID_NDK_ROOT`, or takes the newest one in
  the SDK's `ndk/` directory. It finds the SDK through `ANDROID_HOME`, `ANDROID_SDK_ROOT` or
  `sdk.dir` in `local.properties`.
- The task's inputs include the framework's sources (`rust/src` at the package root), not only
  your crate's. A change to the framework therefore rebuilds your app's library too.

The ABI default deserves a second look. An emulator on an Apple-silicon Mac runs `arm64-v8a`
images; on an Intel machine, `x86_64`. The script picks the one that matches the machine running
Gradle (`hostArchitectureAbi`), which is right for the local emulator and builds only one
architecture, so it is fast.

### What `krabik run` does, and doing it by hand

`krabik run` asks the emulator for its ABI and then runs, in `notes/android`:

```sh
./gradlew app:installDebug -PkrabikAbis=arm64-v8a
adb shell am start -n com.example.notes/com.example.notes.MainActivity
```

The first command builds the Rust library and the Kotlin, and installs the APK. The second starts
the app; tapping its launcher icon does the same. You can run both yourself, and
`./gradlew :app:assembleDebug` builds the APK without installing it. Opening `notes/android` in
Android Studio works as well; the Rust build runs as part of the normal Gradle build there.

A new project has no UI tests. Chapter 9 shows the tests; to run them the app module needs the
`androidTest` dependencies and the `testInstrumentationRunner` line that
`example-todo-minimal/android/app/build.gradle.kts` has, after which
`./gradlew :app:connectedDebugAndroidTest` runs them.

## The iOS host

`krabik create` wrote four files into `notes/ios` and generated the Xcode project from them.

### `project.yml`

The Xcode project is generated from this file by XcodeGen. The parts that differ per app are the
names; the structure is identical for every app.

```yaml
name: Notes
options:
  bundleIdPrefix: com.example
  deploymentTarget:
    iOS: "16.0"
  createIntermediateGroups: true

# `Local.xcconfig` is optional: put DEVELOPMENT_TEAM = <your team id> there to
# run on a device. The simulator needs no signing identity.
configFiles:
  Debug: Signing.xcconfig
  Release: Signing.xcconfig
settings:
  base:
    SWIFT_VERSION: "5.9"
    GENERATE_INFOPLIST_FILE: YES
    CODE_SIGN_STYLE: Automatic
    INFOPLIST_KEY_UILaunchScreen_Generation: YES
    INFOPLIST_KEY_UIApplicationSceneManifest_Generation: YES
    # The Rust build phase runs cargo, which reads and writes outside the
    # script's declared inputs.
    ENABLE_USER_SCRIPT_SANDBOXING: NO

targets:
  Notes:
    type: application
    platform: iOS
    sources:
      - path: Sources
      # The framework's Swift host, in a local krabik checkout.
      - path: "/path/to/krabik-ui-dsl/shared/ios"
    preBuildScripts:
      # Builds ../rust so the app never links a stale library.
      - name: Build Rust library
        basedOnDependencyAnalysis: false
        script: |
          "/path/to/krabik-ui-dsl/shared/build-rust-ios.sh"
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: com.example.notes
        PRODUCT_NAME: Notes
        RUST_LIB: notes
        SWIFT_OBJC_BRIDGING_HEADER: Sources/Bridging.h
        LIBRARY_SEARCH_PATHS:
          - $(SRCROOT)/../rust/target/xcode/$(PLATFORM_NAME)-$(CONFIGURATION)
        OTHER_LDFLAGS:
          - -l$(RUST_LIB)
```

Read the application target from top to bottom; it is the whole integration.

- `sources` compiles two directories into the app: your `Sources` and `shared/ios`, the
  framework's Swift host (`KrabikHostController.swift`, `RustRootView.swift`,
  `KrabikChrome.swift`, `ContentView.swift`). There is no Swift package or framework target; the
  shared files are part of every app.
- `preBuildScripts` runs `shared/build-rust-ios.sh` before anything is compiled.
  `basedOnDependencyAnalysis: false` means "run it on every build"; cargo decides whether there
  is anything to do.
- `RUST_LIB: notes` is the library's name without `lib` and `.a`. It is used twice: by the
  script, to know which file cargo produced, and by `OTHER_LDFLAGS: -l$(RUST_LIB)`, to link it.
- `LIBRARY_SEARCH_PATHS` points the linker at the directory the script writes to.
- `SWIFT_OBJC_BRIDGING_HEADER` makes the C declarations in `Bridging.h` visible to Swift.

There is no UI test target. To run the XCUITest suite of chapter 9, add a `UITests` directory and
the `scheme` and `bundle.ui-testing` target that `example-todo-minimal/ios/project.yml` has.

### `Sources/Bridging.h`

```c
// Exposes the framework's C interface and this app's install function to Swift.

#include "/path/to/krabik-ui-dsl/shared/ios/KrabikIOS.h"

/// Registers the app with the framework. Call once at startup, before any
/// root view is created. Implemented in `rust/src/lib.rs`.
void notes_install(void);
```

The include pulls in the framework's C interface, which the shared Swift files call. The last
line declares your install function. Its name must match the `extern "C" fn` in Rust; if it does
not, the build fails at link time with an undefined symbol.

### `Sources/NotesApp.swift`

```swift
import SwiftUI

@main
struct NotesApp: App {
    init() {
        // Registers the app with the framework before any view is created.
        notes_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
```

This Swift struct shares a name with the Rust struct of the tutorial only by coincidence; the two
never meet.
`init` runs before `body`, which gives the required order: install first, views second.
`ContentView` is the shared file `shared/ios/ContentView.swift`. It wraps the framework's
`KrabikHostController` (a UIKit view controller) so SwiftUI can show it, edge to edge:

```swift
struct ContentView: View {
    var body: some View {
        // The host reaches the screen edges so the tab bar can cover the
        // home-indicator area; each screen keeps its content in the safe area.
        HostView()
            .ignoresSafeArea(.container)
    }
}
```

SwiftUI is used for these few lines of app lifecycle only. Every view the user sees is UIKit,
created from Rust.

Icons on iOS need no files: `.sf("plus")` names an SF Symbol, which ships with the system.

### The generated project

`Notes.xcodeproj` is written by XcodeGen from `project.yml`. `krabik create` ran it once, and
`krabik run --platform ios` runs it again before every build, so the project always matches the
file. That is why the project is listed in `ios/.gitignore`: it is build output. After a change
to `project.yml` you can also run it yourself, from `notes/ios`:

```sh
xcodegen generate
```

The examples in the checkout do it the other way round and commit their generated projects, so
that building them does not require XcodeGen.

### How Xcode builds the Rust code

`shared/build-rust-ios.sh` is short. Xcode hands it the platform and architectures it is
building for, and the script maps them to a Rust target:

```sh
for arch in $ARCHS; do
  case "$PLATFORM_NAME-$arch" in
    iphoneos-arm64) target=aarch64-apple-ios ;;
    iphonesimulator-arm64) target=aarch64-apple-ios-sim ;;
    iphonesimulator-x86_64) target=x86_64-apple-ios ;;
    # ...
  esac
  cargo build --target "$target" $flags
  libraries="$libraries target/$target/$profile/lib$RUST_LIB.a"
done

# One file for the linker, whether Xcode asked for one architecture or two.
lipo -create $libraries -output "$out/lib$RUST_LIB.a"
```

So the same project builds for a simulator on Apple silicon, a simulator on Intel, and a device,
with no setting to flip: whatever destination you pick in Xcode decides the target. The result
goes to `rust/target/xcode/<platform>-<configuration>/libnotes.a`, the directory in
`LIBRARY_SEARCH_PATHS`. A Release build in Xcode passes `--release` to cargo.

The script prepends `$HOME/.cargo/bin` to `PATH`, because Xcode runs build phases with a minimal
environment in which `cargo` would otherwise not be found.

### What `krabik run --platform ios` does, and doing it by hand

`krabik run --platform ios` generates the project, builds it with `xcodebuild` for the booted
simulator, installs the app with `xcrun simctl install` and starts it with
`xcrun simctl launch`.

By hand, the simplest way is to open `notes/ios/Notes.xcodeproj` in Xcode, choose a simulator and
press Run. From the command line, use the name of a simulator you have installed
(`xcrun simctl list devices available` prints them):

```sh
cd notes/ios
xcodebuild build -project Notes.xcodeproj -scheme Notes \
    -destination 'platform=iOS Simulator,name=iPhone 17'
```

With a UI test target in place, `xcodebuild test` with the same arguments builds and runs the UI
tests. The simulator name depends on the installed Xcode (the READMEs use `iPhone 17`); the CI
workflow avoids the question by picking the first available iPhone by id.

## Building for a real device

### iOS

A device needs a signed app, and signing needs your Apple developer team. That is a per-developer
setting, so it is kept out of the repository. `Signing.xcconfig` consists of one optional
include:

```text
// Signing settings for this app. Create `Local.xcconfig` next to this file
// with `DEVELOPMENT_TEAM = <your team id>` to build for a device; it is
// ignored by git.
#include? "Local.xcconfig"
```

Create `notes/ios/Local.xcconfig`:

```text
DEVELOPMENT_TEAM = ABCDE12345
```

with your own team id. `Local.xcconfig` is listed in `ios/.gitignore`. The `?` in
`#include?` makes the include optional, so the project still builds for the simulator on a
machine without the file. With `CODE_SIGN_STYLE: Automatic` in `project.yml`, Xcode handles
certificates and profiles from there. Then open the project in Xcode, select your phone as the
destination and run. The build script picks `aarch64-apple-ios` for it. (`krabik run` and
`krabik watch` only know simulators.)

An honest caveat: device builds have been compiled (the CI workflow has a step that builds the
example's static library for `aarch64-apple-ios`) but, as far as the project's notes say, not
yet run on a physical phone. Expect to be the first to find out.

### Android

A debug build installs on a device without extra signing setup: `krabik run <serial>` with the
phone's `adb` serial does it and asks the phone for its ABI. With Gradle directly, the one thing
to check is the ABI. Phones are `arm64-v8a`. On an Apple-silicon Mac that is already the default.
On an Intel machine the default is `x86_64`, which matches the emulator and not the phone, so say
which ones you want:

```sh
./gradlew :app:installDebug -PkrabikAbis=arm64-v8a
```

Several ABIs can be listed, separated by commas, to build one APK for both the emulator and a
phone. The script deletes the library directories of ABIs the current build does not target, so a
stale library from an earlier build is never packaged.

### Release builds

`krabik release` does not support a krabik-ui-dsl project yet. Build a release with the platform
tools: `./gradlew :app:assembleRelease` in `notes/android` (signing is configured in
`app/build.gradle.kts` as in any Android project), and Product > Archive in Xcode. Both pass
`--release` to cargo. Before you ship, remove the `hot-reload` feature from `rust/Cargo.toml`
and the `hot_reload!` line from `lib.rs`.

## When it does not start

| Symptom | Likely cause |
| ------- | ------------ |
| Panic: "no application registered" | The install function was not called before the first root view. On Android, `installApp()` is not overridden; on iOS, the call is missing from the App struct's `init`. |
| Android: `UnsatisfiedLinkError` for the library | The name in `System.loadLibrary` does not match the Cargo package name, or the library was built for a different ABI than the device's. |
| Android: `UnsatisfiedLinkError` for `install` | The JNI function name in Rust does not match the Kotlin package, object and method. |
| iOS: undefined symbol at link time | `RUST_LIB` does not match the Cargo package name, or the function declared in `Bridging.h` does not exist in Rust under that name. |
| iOS: `cargo: command not found` in the build log | Rust is not installed in `$HOME/.cargo/bin`. |
| `krabik watch` says "reloaded" and nothing changes | The app refused the library because the state or message type changed; `adb logcat -s krabik` says so. Start it again with `krabik run`. |
| `krabik watch`: "the library was not delivered" | The app is not installed or not running on that device. Start it with `krabik run` first. |
| A build cannot find `krabik-ui-dsl` | The krabik checkout moved. Fix its path in the four files named under "How the pieces fit". |
| A floating button or action without its icon | The drawable is not in the app's `res/drawable`, or the SF Symbol name is misspelt. Icons fail silently. |

## Creating the next app

```sh
krabik create shopping --with-dsl --package io.acme
```

That is the whole list. The command derives every name in this chapter from the two arguments:
the Cargo package, the two install functions, the Kotlin package and application id, the bundle
identifier, `RUST_LIB` and the Swift struct. `--dsl-path` names the `krabik-ui-dsl` directory of
another checkout than the one the CLI was built from. If you installed the CLI with
`cargo install --git`, always pass it: the default is then a checkout inside cargo's cache.

## Where to go from here

You have built an app that uses every layer of the framework: components and typed messages,
flexbox layout, a windowed list, a child component, a navigation stack owned by Rust, native
chrome described by state, adaptive colours, the `view!` macro, tests at three levels, and two
host projects. The tutorial explained what each layer does for an app. How each one works inside
(the tree and its identifiers, the Taffy integration, the diff and its operations, the session,
the two renderers, the macros) is the subject of [`../internals/`](../internals/).

For more examples of each widget and each piece of chrome in use, read `example/rust/src/pages.rs`
alongside the running showcase app.

## Where this lives

- `packages/krabik-cli/src/dsl.rs` (in the repository root): the template of every file
  `krabik create --with-dsl` writes; `src/hot.rs` next to it: what `krabik watch` does.
- `example-todo-minimal/`: `rust/Cargo.toml`, `rust/src/lib.rs` (install functions at the end),
  `android/`, `ios/`. The same layout written by hand, with UI tests.
- `shared/android/rust.gradle.kts`: the Gradle side of the Rust build.
- `shared/android/build.gradle.kts`: the `:krabik-ui` library module.
- `shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt`: `loadNativeLibrary`,
  `installApp`, `onCreate`.
- `shared/build-rust-ios.sh`: the Xcode side of the Rust build.
- `shared/ios/ContentView.swift`, `shared/ios/KrabikIOS.h`.
- `rust-toolchain.toml`: the targets.
- `rust/src/hot.rs` and `docs/design/hot-reload.md`: hot reload.
- `example-todo-minimal/README.md`: the same build commands for that app.

Back to the [index](README.md).
