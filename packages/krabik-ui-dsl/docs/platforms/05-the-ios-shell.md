# The iOS shell

This chapter covers the Swift half of the framework: four files in `shared/ios/` plus the C
header, and the two small files each app adds. The shell is much thinner than Android's because
Rust creates the widgets itself. Swift owns only what UIKit expresses as view controllers
(navigation, alert, sheet) and the chrome views. Afterwards you will know what each Swift method
does, why the back button is custom, why the snackbar and drawer are hand-made, and where a call
is deferred to keep out of Rust's lock.

A reminder: Rust owns the application, the navigation stack and all state. A *host* is one view
that Rust renders into. The *controller* (`KrabikHostController`) owns the chrome and carries out
what Rust decides.

## The files

```text
shared/ios/
  KrabikIOS.h                 the C functions Swift can call (chapter 1)
  KrabikHostController.swift  navigation, tab bar, FAB, drawer,
                              snackbar, alert, sheet
  KrabikChrome.swift          KrabikSnackbarView and KrabikDrawerView
  RustRootView.swift          class RustHostView: one Rust-rendered view
  ContentView.swift           the SwiftUI wrapper around the controller
```

A naming mismatch to be aware of: the file `RustRootView.swift` declares the class
`RustHostView`. The Rust doc comments on `krabik_ios_create` also say `RustRootView`. On Android
the class really is `RustRootView`; on iOS it was renamed and the file name was not.

These files are not a framework or a Swift package. Each app's Xcode project compiles the
directory as part of the app target (`- path: ../../shared/ios` in `project.yml`; chapter 6).

## `ContentView.swift`

```swift
// shared/ios/ContentView.swift
private struct HostView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> KrabikHostController {
        KrabikHostController()
    }

    func updateUIViewController(_ controller: KrabikHostController, context: Context) {}
}

struct ContentView: View {
    var body: some View {
        HostView()
            .ignoresSafeArea(.container)
    }
}
```

The apps use the SwiftUI app life cycle (`@main struct ...: App`), so the UIKit controller is
wrapped in a `UIViewControllerRepresentable`. `ignoresSafeArea` lets the controller reach the
screen edges; the tab bar needs to cover the home-indicator area. Each screen keeps its own
content inside the safe area (see `hostController` below).

## `RustRootView.swift`: `RustHostView`

A `RustHostView` is a `UIView` that Rust fills with widgets. It stores the session handle, its
host id, and the size it last rendered at. It has three ways of being bound.

```swift
// shared/ios/RustRootView.swift (trimmed)
/// Binds the main host created by the session.
func bindMain(session: UInt64) { /* host = 0 */ }

/// Binds a child host and attaches it to the session.
///
/// The native controller creates hosts while Rust holds the session lock,
/// so the attach itself is deferred to the next main-queue turn.
func bind(session: UInt64, host: UInt64) {
    self.session = session
    self.host = host
    renderedSize = .zero
    DispatchQueue.main.async { [weak self] in
        guard let self, self.session == session, self.host == host else { return }
        krabik_ios_attach_host(session, host, Unmanaged.passUnretained(self).toOpaque())
        self.setNeedsLayout()
    }
}

/// Binds a child host, attaches it and renders it at its current size
/// right away. Only valid when Rust is not in the middle of a render.
func bindNow(session: UInt64, host: UInt64) { /* attach, then krabik_ios_render */ }
```

- `bindMain` is for host 0, which `krabik_ios_create` already created.
- `bind` is for navigation screens. Rust calls `krabikPush` from inside a render, with the lock
  held. `krabik_ios_attach_host` needs that lock, so the attach waits one turn. On Android the
  same delay comes for free from the asynchronous fragment transaction; here it is explicit.
- `bindNow` is for the sheet, which is already inside a deferred block and needs its content
  rendered immediately so its height can be measured.

Rendering follows the size:

```swift
// shared/ios/RustRootView.swift (trimmed)
override func layoutSubviews() {
    super.layoutSubviews()
    guard session != 0, bounds.width > 0, bounds.height > 0 else { return }
    guard bounds.size != renderedSize, !pendingRender else { return }

    // UIKit can run layout synchronously from inside a Rust event handler
    // (for example when a focused field is removed from its superview and
    // resigns first responder). Rendering there would re-enter the state
    // mutex, so coalesce and defer to the next main-queue turn.
    pendingRender = true
    DispatchQueue.main.async { [weak self] in
        // ... if the size still differs from renderedSize: krabik_ios_render(...)
    }
}
```

This is the iOS counterpart of `RustRootView.onMeasure` on Android: Rust re-renders when the
view's size changes (rotation, the keyboard, a tab bar appearing). It is deferred for the same
reason as everything else on this platform: `layoutSubviews` can run while Rust holds the lock.
`pendingRender` coalesces several layout passes into one render.

`deinit` calls `krabik_ios_destroy_host` for child hosts. This is why Rust holds the view weakly:
a strong reference from Rust would keep `deinit` from ever running.

## `KrabikHostController.swift`

One `UIViewController` owns everything. Its view hierarchy:

```text
 KrabikHostController.view
 +-- navigation.view            UINavigationController (child view controller)
 |     root controller          wraps mainHost (host 0)
 |     pushed controllers       one per screen, each wraps a RustHostView
 +-- tabBar                     UITabBar, pinned to the bottom edge
 +-- fab                        UIButton, bottom trailing corner
 +-- snackbar (when shown)      KrabikSnackbarView, inserted below the drawer
 +-- drawer                     KrabikDrawerView, hidden until opened
```

### Start-up

```swift
// shared/ios/KrabikHostController.swift (trimmed)
override func viewDidLoad() {
    super.viewDidLoad()

    mainHost = RustHostView(frame: view.bounds)
    session = krabik_ios_create(
        Unmanaged.passUnretained(mainHost).toOpaque(),
        Double(view.bounds.width),
        Double(view.bounds.height),
        traitCollection.userInterfaceStyle == .dark
    )
    mainHost.bindMain(session: session)

    navigation = UINavigationController(rootViewController: hostController(mainHost))
    navigation.setNavigationBarHidden(true, animated: false)
    navigation.interactivePopGestureRecognizer?.isEnabled = false
    // ... add as child; install the tab bar, FAB and drawer

    krabik_ios_set_controller(session, Unmanaged.passUnretained(self).toOpaque())
}
```

The session is created first, then the chrome is built, and registering the controller comes
last because `krabik_ios_set_controller` renders and synchronises at once: Rust immediately calls
back `krabikPush`, `krabikSetToolbar` and the rest.

The app must have called its install function before this runs, or `create_app` panics. The
app's `init()` does that (see the end of the chapter).

### The navigation controller and the custom back button

The navigation stack is a real `UINavigationController`, so pushes and pops get the system
animation and the system navigation bar. Its root controller holds the main host. An app that
uses navigation never shows that root: its first screen is already a pushed controller, and the
navigation bar stays hidden until there are at least two controllers.

```swift
// shared/ios/KrabikHostController.swift (trimmed)
@objc func krabikPush(_ host: Int64, transition: Int) {
    let hostView = RustHostView(frame: .zero)
    hostView.bind(session: session, host: UInt64(host))
    let vc = hostController(hostView)
    // Rust owns the stack, so the system back button (which pops on its
    // own) is replaced by one that asks Rust. The first screen has none.
    vc.navigationItem.hidesBackButton = true
    if navigation.viewControllers.count > 1 {
        let back = UIBarButtonItem(
            image: UIImage(systemName: "chevron.backward"),
            style: .plain,
            target: self,
            action: #selector(backTapped)
        )
        back.accessibilityLabel = "Back"
        vc.navigationItem.leftBarButtonItem = back
    }
    // ... push with the requested transition
}
```

**Why the back button is custom.** The system back button pops the view controller by itself.
Rust owns the navigation stack; if UIKit popped on its own, the screen and Rust's stack would
disagree, and the app would have no chance to say "no" (an unsaved-changes prompt, for example)
or to do something else on back. So the system button is hidden and replaced by one whose action
is `krabik_ios_on_back(session)`. Rust runs `App::back()`, the app updates its stack, and the
session then calls `krabikPop`. This is the same reason the Android shell does not use the
fragment back stack.

For the same reason the interactive pop gesture (swipe from the left edge) is disabled. That is a
real loss of native feel: there is no swipe-back on iOS. Supporting it would mean letting UIKit
start the pop and reconciling with Rust afterwards.

Transitions: code 0 is the system slide, code 1 adds a `CATransition` fade to the navigation
controller's layer and pushes without animation, code 2 pushes without animation.

`krabikPop(count, transition)` finds the target controller `count` places below the top and pops
to it in one step. `krabikSetRootVisible` shows or hides the main host and the navigation bar.

`hostController(_:)` is the helper that wraps a host view in a plain `UIViewController`, pinning
the view to the controller's safe-area guide. That one constraint set is what keeps Rust content
clear of the status bar, the home indicator, the navigation bar and, through
`additionalSafeAreaInsets`, the tab bar. Rust never deals with insets.

### The navigation bar

`krabikSetToolbar` configures the top controller's `navigationItem`: the title (or a two-line
title view when there is a subtitle) and the right bar button items. An action with an icon
becomes a `UIButton` custom view showing icon and label; one without becomes a plain titled item.
Each carries its index in `tag`, and `toolbarTapped` reports it with
`krabik_ios_on_toolbar_action`.

Icons are resolved by `toolbarImage`: an SF Symbol name first, then an image in the app's asset
catalogue.

One piece of this method has no visible effect: it sets `backButtonTitle` on the previous
controller from `backTitle`, but the system back button that would show it is hidden. The custom
back button always has the accessibility label "Back". So `AppBar`'s back title does nothing on
iOS today.

### The tab bar

The bottom navigation is a real `UITabBar`, added to the controller's own view rather than to a
screen, so it stays put while screens push and pop above it. It is pinned to the bottom edge and
extends 49 points above the safe area (`tabBarHeight`).

`krabikSetBottomNavigation` rebuilds the items only when labels or icons change, sets the
selected item, hides the bar when there are no items, and sets
`navigation.additionalSafeAreaInsets.bottom` so every screen's content ends above the bar. A tap
arrives in the `UITabBarDelegate` method, which calls
`krabik_ios_on_bottom_navigation_selected`. Setting `selectedItem` from code does not call the
delegate, so no silent flag is needed here.

Why not a `UITabBarController`? It would own one child controller per tab and switch between them
itself. In this framework a tab switch is an app message that changes the navigation stack; Rust
decides what the screen shows.

### The floating action button

iOS has no floating action button. The shell uses a `UIButton` with the filled configuration, a
capsule shape and a shadow, pinned to the bottom trailing corner. Two constraints place it above
the tab bar or above the safe area; `placeFloatingActionButton` activates the right one whenever
the tab bar is shown or hidden. A tap reports chrome event 0.

### The alert

```swift
// shared/ios/KrabikHostController.swift (trimmed)
let alert = UIAlertController(title: title, message: message.isEmpty ? nil : message,
                              preferredStyle: .alert)
var hasCancel = false
for index in labels.indices {
    let role = roles.indices.contains(index) ? roles[index].intValue : 0
    let style: UIAlertAction.Style
    switch role {
    // UIKit allows a single cancel action.
    case 1 where !hasCancel:
        hasCancel = true
        style = .cancel
    case 2:
        style = .destructive
    default:
        style = .default
    }
    alert.addAction(UIAlertAction(title: labels[index], style: style) { [weak self] _ in
        self?.dialogClosed(button: index)
    })
}
```

The dialog is the system `UIAlertController`. Roles map to action styles; only the first cancel
role becomes `.cancel` because UIKit raises an exception for two. An alert without buttons gets
an "OK" button that reports `-1`, since it could otherwise never be closed.

`topPresenter()` walks the chain of presented controllers so an alert can appear above an open
sheet. Unlike Android there is no `cancelable` parameter: a UIKit alert cannot be dismissed by
tapping outside it.

### The sheet with a content-sized detent

```swift
// shared/ios/KrabikHostController.swift (trimmed)
@objc func krabikPresentSheet(_ host: Int64) {
    pendingSheet = host
    DispatchQueue.main.async { [weak self] in
        guard let self, self.pendingSheet == host else { return }
        self.pendingSheet = nil

        let hostView = RustHostView(frame: self.view.bounds)
        hostView.backgroundColor = .clear
        hostView.bindNow(session: self.session, host: UInt64(host))
        let contentHeight = CGFloat(krabik_ios_content_height(self.session, UInt64(host)))

        let controller = self.hostController(hostView, background: .systemBackground)
        controller.modalPresentationStyle = .pageSheet
        if let presentation = controller.sheetPresentationController {
            presentation.detents = [
                .custom { context in min(contentHeight, context.maximumDetentValue) },
            ]
            presentation.prefersGrabberVisible = true
        }
        controller.presentationController?.delegate = self
        self.sheet = controller
        self.topPresenter().present(controller, animated: true)
    }
}
```

A *detent* is a height at which a sheet rests. The system offers medium and large; the sheet
should be exactly as tall as its content. The steps:

1. Rust calls `krabikPresentSheet` under the lock, so the work is deferred one turn.
   `pendingSheet` lets a dismiss that arrives in between cancel it.
2. The host view is attached and rendered immediately (`bindNow`) at the full screen size.
3. `krabik_ios_content_height` asks how tall that content came out.
4. A custom detent returns that height, capped at the maximum the system allows.

Custom detents need iOS 16, which is the apps' deployment target.

The height is measured once, at presentation. If the sheet's content grows later, the detent does
not follow.

When the user swipes the sheet away or taps outside it, UIKit has already dismissed it;
`presentationControllerDidDismiss` tells Rust with `krabik_ios_on_modal_dismiss`. The session
forgets the sheet first, so if the app still returns a sheet it is presented again.

### Appearance changes

```swift
// shared/ios/KrabikHostController.swift
override func traitCollectionDidChange(_ previous: UITraitCollection?) {
    super.traitCollectionDidChange(previous)
    if previous?.userInterfaceStyle != traitCollection.userInterfaceStyle, session != 0 {
        krabik_ios_set_color_scheme(session, traitCollection.userInterfaceStyle == .dark)
    }
}
```

The system chrome (navigation bar, tab bar, alert, sheet background) uses dynamic system colours
and re-themes itself. The colours Rust paints are fixed values the engine resolved, so Rust must
be told. It re-renders, and the diff updates only the nodes whose colours differ. Nothing is
recreated. Compare Android, where the activity is recreated and the app retained.

## `KrabikChrome.swift`: snackbar and drawer

**Why they are custom.** Material has a `Snackbar` and a `NavigationView` in a `DrawerLayout`.
UIKit has neither control. To give `Scaffold` the same abilities on both platforms, the shell
contains two small views with the same behaviour. They are plain UIKit and know nothing about
Rust: the controller owns them and translates what the user does into chrome events. They look
like iOS, not like Material, but they are not system components and will not track future system
styling.

### `KrabikSnackbarView`

A rounded, shadowed bar with a message label (up to two lines) and an optional action button.
Its colours are dynamic (`UIColor { traits in ... }`), dark on a light screen and light on a dark
one. It exposes one callback, `onAction`.

The controller does the rest in `krabikShowSnackbar`:

- inserts the bar below the drawer and constrains it above the floating action button, the tab
  bar or the safe area, whichever is showing;
- fades it in and posts a VoiceOver announcement;
- starts a four-second timer;
- adds a downward swipe recogniser.

The timer and the swipe both end in `snackbarLeftByItself`, which removes the bar and reports
chrome event 5. The action button removes it and reports event 4. `krabikDismissSnackbar`
removes it and reports nothing, the same contract as Android: Rust is only told about things it
did not ask for.

### `KrabikDrawerView`

A full-size view with two parts: a dimming `UIControl` and a 300-point panel holding an
inset-grouped `UITableView` of destinations. The panel's leading constraint is `-300` when
closed and `0` when open; `setOpen(_:animated:)` animates the constraint and the dim. The view is
hidden when closed so it does not intercept touches.

- Tapping the dim, swiping left, or the VoiceOver escape gesture closes it and calls `onClose`
  (chrome event 3).
- Picking a row calls `onSelect` (event 1) and does not close the drawer: Rust decides, as on
  Android.
- `setOpen` itself reports nothing, "the caller knows why it asked".
- `accessibilityViewIsModal` is set while open so VoiceOver stays inside.

The controller opens it for the user from two places: a menu button and a left screen-edge pan.
Both go through `openDrawerForUser`, which reports event 2. The menu button is installed by
`updateMenuButton` in the place where the back button would be, and only on the first screen
(`controllers.count == 2`: the hidden root plus one screen). On a pushed screen the leading edge
belongs to back.

## What an app adds

An app's `ios/Sources/` folder holds two files.

```c
// example/ios/Sources/Bridging.h
#include "../../../shared/ios/KrabikIOS.h"

/// Registers the showcase app with the framework. Call once at startup, before any
/// root view is created.
void krabik_ui_dsl_example_install(void);
```

The bridging header is how Swift learns about C functions. It includes the framework's header
and declares the one function the app's own crate exports. The project setting
`SWIFT_OBJC_BRIDGING_HEADER` points at it.

```swift
// example/ios/Sources/KrabikUiDslExampleApp.swift
@main
struct KrabikUiDslExampleApp: App {
    init() {
        krabik_ui_dsl_example_install()
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
```

`init()` registers the app factory before any view exists. `ContentView` comes from
`shared/ios`. On the Rust side:

```rust,ignore
// example/rust/src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn krabik_ui_dsl_example_install() {
    ShowcaseApp::install();
}
```

The app crate is built as a static library (`crate-type = ["staticlib", "cdylib"]` in
`example/rust/Cargo.toml`) and linked into the app. Because the crate depends on
`krabik-ui-dsl`, that one `.a` file contains the framework's `krabik_ios_*` functions too.

There is no Swift code per widget and nothing to register when a widget is added. The remaining
folders are `Resources/` (bundled assets, loaded by `load_asset` in `rust/src/ios/images.rs`) and
`UITests/` (XCUITests).

## Rough edges in this directory

- File name `RustRootView.swift`, class name `RustHostView`.
- No swipe-back gesture.
- `AppBar`'s back title has no effect; the back button's accessibility label is fixed.
- The value `krabik_ios_on_back` returns is discarded.
- The sheet's height is fixed at presentation.
- The snackbar and drawer are custom views.
- In `KrabikSnackbarView.init`, the comment "Announced like an alert, so VoiceOver reads it when
  it appears" sits above `accessibilityViewIsModal = false`, which does not do that. The
  announcement is posted by the controller.
- The controller's `deinit` destroys the session, but Rust holds a strong reference to the
  controller for as long as the session exists, so that `deinit` cannot run first (chapter 1).

## Where this lives

- `shared/ios/KrabikHostController.swift`: `viewDidLoad`, `krabikPush`, `krabikPop`,
  `krabikSetToolbar`, `krabikSetBottomNavigation`, `krabikShowDialog`, `krabikPresentSheet`,
  `hostController`, `traitCollectionDidChange`.
- `shared/ios/KrabikChrome.swift`: `KrabikSnackbarView`, `KrabikDrawerView.setOpen`.
- `shared/ios/RustRootView.swift`: `RustHostView.bind`, `bindNow`, `layoutSubviews`.
- `shared/ios/KrabikIOS.h`: the C interface.
- `example/ios/Sources/`: `Bridging.h`, `KrabikUiDslExampleApp.swift`.

Next: [Building and CI](06-building-and-ci.md).
