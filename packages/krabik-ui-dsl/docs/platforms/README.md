# Part 2b: how the platforms work

This part explains everything that is specific to Android or iOS: the two renderers written in
Rust, the Kotlin and Swift shells around them, how an app is built and packaged, the CI workflow,
and the procedural macros. After reading it you can open any file under `rust/src/android/`,
`rust/src/ios/`, `shared/`, `rust/macros/`, an app's `android/` or `ios/` folder, or the CI
workflow, and know what it is for and why it has the shape it has.

The platform-independent engine (elements, the node tree, layout, the diff, the runtime and the
session) is covered in [`../internals/`](../internals/). This part only repeats, in one paragraph
where needed, what an `Op`, a host, a session and a controller are.

## Chapters

1. [How Rust talks to the platforms](01-how-rust-talks-to-the-platforms.md): the two bridges side
   by side, JNI on Android and a C interface plus `objc2` on iOS, with a table of every entry
   point in both directions.
2. [The Android renderer](02-the-android-renderer.md): `rust/src/android/` file by file, from the
   JNI entry points through applying each operation to a `View`, cached JNI handles, widgets,
   measurement and recycled rows.
3. [The Android shell](03-the-android-shell.md): the Kotlin library in `shared/android/`
   (`KrabikActivity`, `KrabikHostController`, `RustRootView` and the widget classes) and the three
   files an app adds on top of it.
4. [The iOS renderer](04-the-ios-renderer.md): `rust/src/ios/` file by file, including why every
   event is deferred by one main-queue turn, frame animation with a display link, and the
   collection-view data source written in Rust.
5. [The iOS shell](05-the-ios-shell.md): the Swift files in `shared/ios/` (the host controller,
   the custom snackbar and drawer, the host view) and what an app adds.
6. [Building and CI](06-building-and-ci.md): how Gradle and Xcode each run `cargo`, how targets
   and ABIs are chosen, signing, and the CI workflow job by job.
7. [The macros](07-the-macros.md): `#[derive(Widget)]`, the `widgets!` table and `view!`, each
   with a real input and its full expansion, and how they are tested.
8. [One tap on each platform](08-one-tap-on-each-platform.md): the platform halves of a single
   button tap, from the native listener into the session and from an `Op::Update` back to a
   native setter, on Android and then on iOS.

## Reading order

Chapter 1 is the map; read it first. Chapters 2 and 3 form the Android pair and chapters 4 and 5
the iOS pair; each pair can be read on its own. Chapter 8 is a good check of your understanding
once you have read one pair.

## Honest limits, in one place

These are stated again where they come up:

- The CI workflow has never run. The repository's remote is not GitHub.
- Device builds for iOS were compiled but never run on a phone.
- iOS has no native snackbar or navigation drawer, so those two are small custom views.
- Widget creation is still one `match` per platform, plus one more for updates.
