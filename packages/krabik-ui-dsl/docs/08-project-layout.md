# 8. Project layout, building and testing

## Layout

```
packages/krabik-ui-dsl/
  rust/                 the framework: a plain Rust library
    src/
      app.rs            Component and App
      element.rs        Element<M> and the shared builder methods
      style.rs          layout and decoration values
      message.rs        typed messages, erased
      navigation.rs     Screen, Toolbar, BottomNavigation
      presentation.rs   Dialog, BottomSheet
      widgets/          one file per widget
      engine/           tree, layout, measure, render (diff), runtime
      host/             the session shared by both platforms
      android/          the Android renderer (JNI)
      ios/              the iOS renderer (UIKit through objc2)
  shared/
    android/            the Kotlin library every Android app includes
    ios/                the Swift host every iOS app compiles
  harness/              the gallery the framework is developed against
  example/              the widget showcase
  example-todo/         a todo app
  example-todo-minimal/ the smallest complete app
  docs/                 this guide
```

Every app has the same three directories:

```
<app>/
  rust/      the app crate: depends on ../../rust, builds the native library
  android/   a Gradle project: MainActivity, a bridge object, resources, tests
  ios/       an Xcode project (generated from project.yml): the App struct, tests
```

## Creating a new app

The quickest start is to copy `example-todo-minimal/` and rename:

1. **Rust** (`rust/Cargo.toml`, `rust/src/lib.rs`): the package name, your
   `Component`, and the two install functions (chapter 1).
2. **Android** (`android/`): `applicationId` in `app/build.gradle.kts`, the
   library name in `MainActivity.loadNativeLibrary`, and the bridge object
   whose name matches the JNI install function.
3. **iOS** (`ios/project.yml`): the project name, bundle id and `-l<library>`
   flag; the install function in `Sources/Bridging.h` and the App struct.
   Then run `xcodegen generate`.

An Android app's whole build file is about 45 lines; the shared parts live in
`shared/android`.

## Building and running

Rust toolchains for the targets:

```sh
rustup target add aarch64-linux-android aarch64-apple-ios-sim
```

**Android** (needs the Android SDK and NDK). Gradle builds the Rust crate
itself:

```sh
cd example/android
./gradlew :app:installDebug -PkrabikAbis=arm64-v8a
```

`-PkrabikAbis` chooses the ABIs to build; it defaults to the architecture of
the machine running Gradle, which matches a local emulator.

**iOS** (needs Xcode). Open `example/ios/KrabikUiDslExample.xcodeproj` and
run, or:

```sh
cd example/ios
xcodebuild build -project KrabikUiDslExample.xcodeproj -scheme KrabikUiDslExample \
  -destination 'platform=iOS Simulator,name=iPhone 17'
```

A build phase runs `cargo build` for the simulator target first. After
changing `project.yml`, regenerate the project with `xcodegen generate`
(`brew install xcodegen`); the generated project is committed, so building
does not need XcodeGen.

## Testing

There are three layers, from fastest to slowest.

**Unit tests, on your machine.** The engine, the session and every app's
logic are plain Rust:

```sh
cargo test --manifest-path rust/Cargo.toml          # the framework
cargo test --manifest-path example/rust/Cargo.toml  # an app
```

The framework's tests cover layout, diffing, list windowing, message mapping
and the session (navigation, dialog and sheet syncing, event resolution)
against a fake platform.

**Lints for the platform code.** The Android and iOS renderers only compile
for their targets, so lint them there:

```sh
cargo clippy --manifest-path rust/Cargo.toml --target aarch64-linux-android -- -D warnings
cargo clippy --manifest-path rust/Cargo.toml --target aarch64-apple-ios-sim -- -D warnings
```

**UI tests, on an emulator or simulator.** They drive the real apps through
native widgets:

```sh
cd example/android && ./gradlew :app:connectedDebugAndroidTest -PkrabikAbis=arm64-v8a

cd example/ios && xcodebuild test -project KrabikUiDslExample.xcodeproj \
  -scheme KrabikUiDslExample -destination 'platform=iOS Simulator,name=iPhone 17'
```

The harness has its own suites in `harness/android` and `harness/ios`.

CI (`.github/workflows/ci.yml`) runs the unit tests and lints and builds
every Android APK and iOS static library. It does not run the UI tests.

## Conventions

- **The app owns all state.** Widgets hold none; every value comes from
  `view()` and every change goes through `update()`.
- **Rust decides, native carries out.** Kotlin and Swift never decide what is
  shown.
- **One list of widgets.** `enum Widget` is matched exhaustively everywhere;
  do not add a `_ =>` arm to a match on it.
- **State set from Rust is applied silently**, so it is not reported back as
  user input.
