# The Android shell

This chapter covers the Kotlin half of the framework: the library module in `shared/android/`
(package `dev.krabik.ui`) and the three small files each app adds on top of it. It explains how
the activity lays out its containers, why screens are fragments managed by hand, how each piece
of Material chrome is driven, what the "silent" setters on the widget classes are for, and why a
theme switch recreates the activity while a rotation does not.

A reminder: Rust owns the application, the navigation stack and all state. A *host* is one
`RustRootView` that Rust renders into. The *controller* (`KrabikHostController`) owns the chrome
and only carries out what Rust decides.

## The files

```text
shared/android/src/main/java/dev/krabik/ui/
  KrabikActivity.kt          the activity apps subclass; RetainedApp
  KrabikHostController.kt    screens, toolbar, bottom bar, FAB, drawer,
                             snackbar, alert, sheet
  KrabikScreenFragment.kt    one navigation screen; themeBackground()
  KrabikSheetFragment.kt     the bottom sheet
  RustRootView.kt            the host view; every `external fun`;
                             frames and animations
  KrabikRecycler.kt          object KrabikRecyclers + the RowAdapter
  KrabikRows.kt              snapshot/fade for row transitions
  KrabikStyle.kt             backgrounds, borders, ripples, curves
  KrabikViews.kt             scroll containers, progress bars
  KrabikBottomNavigation.kt  the host-level bottom navigation bar
  KrabikTextInput.kt, KrabikSwitch.kt, KrabikChecks.kt,
  KrabikSeekBar.kt, KrabikRadioGroup.kt, KrabikTabBar.kt,
  KrabikPager.kt, KrabikDatePicker.kt, KrabikImages.kt
                             one class per interactive widget,
                             plus image loading
shared/android/src/main/res/
  anim/krabik_*.xml          seven screen-transition animations
  drawable/krabik_ic_*.xml   back arrow and menu icons
```

Two file names do not match their contents: `KrabikRecycler.kt` declares `object
KrabikRecyclers`, and `KrabikChecks.kt` declares `KrabikCheckBox` and `KrabikRadioButton`.

## `KrabikActivity`

An app's `MainActivity` extends this class. It is an `AppCompatActivity` that builds its whole view
hierarchy in code; there are no layout XML files. AppCompat is what lets the application choose
light or dark by itself (`KrabikHostController.setAppearance`).

### Start-up order

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt (trimmed)
override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(null)

    loadNativeLibrary()
    installApp()
    retained = ViewModelProvider(this)[RetainedApp::class.java]
    root = RustRootView(this, retainedApp = retained.id)
    retained.id = 0L
    // ... build the containers, then the controller
}
```

Three things here are deliberate.

`super.onCreate(null)` discards any saved instance state. Android would otherwise restore the
fragments of the previous activity instance, and those fragments carry a session handle and host
id that no longer exist. Starting clean is safe because the Rust app rebuilds its screens from
its own state.

`loadNativeLibrary()` and `installApp()` are the two hooks a subclass overrides. The order is
fixed: the library must be loaded before the first `external fun` is called, and the app factory
must be registered before `nativeCreate` asks for an app. Constructing `RustRootView` is what
calls `nativeCreate`.

`retained.id` is the id of an app parked by the previous activity instance (see "Theme changes"
below). It is passed to the root view once and then cleared.

### The layout of containers

```text
 DrawerLayout
 +-- column: LinearLayout (vertical)
 |   +-- toolbar: android.widget.Toolbar            (wrap height)
 |   +-- content: FrameLayout                       (weight 1)
 |   |   +-- mainContainer: FrameLayout
 |   |   |   +-- root: RustRootView   (host 0, the main root)
 |   |   +-- navContainer: FrameLayout  (screen fragments go here)
 |   |   +-- fab: ExtendedFloatingActionButton (bottom end, 16dp margin)
 |   +-- bottomNavigation: KrabikBottomNavigation   (wrap height)
 +-- drawer: NavigationView (gravity START)
```

`mainContainer` and `navContainer` overlap on purpose. The comment in the source gives the
reason: revealing the root during a pop must not resize the outgoing screen. `navContainer` is
added later, so it draws above the root. The floating action button is above both. The drawer is
a sibling of the whole column, so it slides over the bars too.

An app without navigation renders into `root`. An app with navigation renders every screen,
including the first, into fragments inside `navContainer`, and `mainContainer` is hidden.

### Edge to edge

`drawEdgeToEdge` calls `WindowCompat.setDecorFitsSystemWindows(window, false)`, which lets the
app draw behind the status bar and the gesture area, and chooses dark or light system-bar icons
from the current night mode. It then installs one insets listener on the column:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt
column.setPadding(bars.left, 0, bars.right, keyboard)
toolbar.setPadding(0, bars.top, 0, 0)
bottomNavigation.setPadding(0, 0, 0, if (keyboard > 0) 0 else bars.bottom)
content.setPadding(
    0,
    if (toolbar.visibility == View.VISIBLE) 0 else bars.top,
    0,
    if (bottomNavigation.visibility == View.VISIBLE || keyboard > 0) 0 else bars.bottom,
)
```

The toolbar extends under the status bar and the bottom navigation under the gesture area, as
Material screens do. When either bar is hidden, the content takes that padding instead. The
bottom padding of the column equals the keyboard height, so the content area shrinks when the
keyboard opens and a focused field stays visible. Because the padding depends on which bars are
showing, the controller calls `requestApplyInsets()` whenever it shows or hides one.

Rust never sees insets. It is given the size of the `RustRootView`, which is already inside the
padded content area.

### Back

There are two `OnBackPressedCallback`s.

`backCallback` forwards the system back action to Rust (`root.onBackPressed()`). It starts
disabled and is switched by the controller's `setBackEnabled`, which Rust calls whenever the
answer to "does `App::back()` return a message?" changes. While it is disabled, Android handles
back itself, including the predictive "back to home" animation. This is why the manifest sets
`android:enableOnBackInvokedCallback="true"`.

`drawerBack` is registered second, so the dispatcher asks it first. It is enabled only while the
drawer is open and closes it.

One rough edge: `handleOnBackPressed` ignores the boolean `root.onBackPressed()` returns. If the
callback is enabled but Rust answers "not handled" (the event was dropped by the try-lock, say),
the back press does nothing.

### Theme changes and the retained app

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikActivity.kt
override fun onDestroy() {
    retained.id = root.destroy(retain = isChangingConfigurations)
    super.onDestroy()
}

internal class RetainedApp : ViewModel() {
    var id = 0L

    override fun onCleared() {
        if (id != 0L) {
            RustRootView.nativeDropRetainedApp(id)
            id = 0L
        }
    }
}
```

Adaptive colours are resolved in the engine, so in principle a switch between light and dark
could be an ordinary re-render. On Android it is not, because the Material chrome (toolbar,
bottom navigation, dialogs, the drawer) takes its colours from theme resources that are read
when a view is created. Recreating the activity is the supported way to re-theme them. So the
activity is allowed to be recreated, and the Rust app is carried across:

1. `onDestroy` runs with `isChangingConfigurations == true`. `root.destroy(retain = true)` calls
   `nativeDestroy`, which parks the app and returns its id.
2. The id is stored in `RetainedApp`, a `ViewModel`. A `ViewModel` outlives a configuration
   change.
3. The new activity's `onCreate` passes the id to the new `RustRootView`, and `nativeCreate`
   takes the app back.
4. If the activity is finished instead, `onCleared` drops the parked app.

The app comes back with its state and navigation stack. The session treats the first stack it
gives a new controller as "restoring" and shows it without replaying push animations
(`sync_navigation` in `rust/src/host/session.rs`).

## `KrabikHostController`

The controller is constructed at the end of `onCreate` with references to all the containers. Its
`init` block wires the listeners, hides the bars, and, last, calls `main.setController(this)`.
That call goes to `nativeSetController`, which renders and synchronises immediately, so by the
time the constructor returns Rust has already pushed the first screens and set the chrome.

### Screens are fragments managed by hand

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt
/**
 * The screen stack, root first. Screens are plain fragments added to
 * [navContainer]; the fragment back stack is not used, so the system back
 * button can never pop a screen without Rust knowing.
 */
private val screens = ArrayList<KrabikScreenFragment>()
```

The obvious Android design would be `addToBackStack`. It was rejected because the fragment
manager would then pop a screen by itself when the user presses back, and Rust, which owns the
navigation stack, would be out of step with what is on screen. Managing the list by hand also
fixed the page transitions (first round of work): each transaction states exactly which fragment
animates and how.

`push(hostId, transition)`:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikHostController.kt (trimmed)
val fragment = KrabikScreenFragment.newInstance(session, hostId)
val previous = screens.lastOrNull()
val transaction = activity.supportFragmentManager.beginTransaction().setReorderingAllowed(true)
if (previous != null) {
    when (transition) {
        TRANSITION_SLIDE -> transaction.setCustomAnimations(
            R.anim.krabik_slide_in_right, R.anim.krabik_slide_out_left)
        TRANSITION_FADE -> transaction.setCustomAnimations(
            R.anim.krabik_fade_in, R.anim.krabik_hold)
    }
}
transaction.add(navContainer.id, fragment)
if (previous != null) {
    transaction.hide(previous)
}
transaction.commitAllowingStateLoss()
screens.add(fragment)
```

The new screen is *added* above the old one and the old one is *hidden*, not removed. Its view
stays alive, so a later pop reveals it exactly as the user left it, scroll position included. The
first screen gets no animation (`previous == null`).

`commitAllowingStateLoss()` is asynchronous: the fragment's view is created on a later main-loop
turn. That matters. Rust calls `push` while it holds the session lock. If the fragment's
`RustRootView` were constructed right there, its `nativeAttachHost` would block on the same lock.
The asynchronous commit is what keeps this from deadlocking.

`pop(count, transition)` removes `count` fragments from the list. The ones between the top and
the target are hidden already and are removed in a quiet transaction. The top one is removed with
the animation of the transition it was pushed with, and the target is shown.

`setRootVisible(visible)` switches between `mainContainer` and `navContainer`. When the last
screen is popped with an animation, hiding `navContainer` at once would cut the animation off, so
it is hidden by a delayed `Runnable` after `transitionMs + 40` milliseconds. `transitionMs` is
read from the integer resource `krabik_transition_ms` (280), which the seven
`res/anim/krabik_*.xml` files use for their `android:duration` as well, so the two cannot differ.
A Rust test (`a_screen_transition_takes_the_same_time_on_both_platforms`) compares it with the
iOS constant.

`KrabikScreenFragment` itself is small. `onCreateView` constructs a `RustRootView` with the
session handle and host id from its arguments and gives it an opaque background from the theme
(`themeBackground`), so the outgoing screen does not show through the incoming one.

### Toolbar

`setToolbar` applies the title, subtitle, back title and actions of the top screen.

- The navigation icon is decided by `updateNavigationIcon`: a back arrow when more than one
  screen is on the stack, a menu icon when the root screen has a drawer, nothing otherwise. One
  click listener serves both: back calls `main.onBackPressed()`, menu opens the drawer.
- Actions are rebuilt as menu items on every call. An action with an icon gets a custom action
  view (icon plus label in a horizontal row), because a framework menu item shows either its icon
  or its text, not both.
- Icons are looked up by name in the *app's* resources:
  `resources.getIdentifier(icon, "drawable", activity.packageName)`. That is why the example app
  ships `ic_add.xml`, `ic_home.xml` and so on in its own `res/drawable`.
- The toolbar is hidden when Rust says so or when there are no screens. An app that does not use
  navigation therefore never shows a toolbar.

### Bottom navigation

`setBottomNavigation` forwards to `KrabikBottomNavigation.configure` and hides the bar when
there are no items. The class extends Material's `BottomNavigationView`. It rebuilds its menu
only when labels or icon names change, and it applies the selection with a `silent` flag set so
the selection listener does not report Rust's own choice back to Rust.

### Floating action button

`setFab(label, icon, visible)` sets the text, content description, icon and visibility of the
`ExtendedFloatingActionButton`. Its click listener reports `EVENT_FAB` through
`main.onChromeEvent`.

### Drawer

`setDrawer(labels, icons, selected, present, open)` rebuilds the `NavigationView` menu, locks
the drawer closed when the screen has none, and opens or closes it to match `open`.

The controller keeps `drawerOpen`, what Rust last asked for. The drawer listener reports
`EVENT_DRAWER_OPENED` or `EVENT_DRAWER_CLOSED` only when the actual state differs from that, so a
change Rust requested is not echoed and a change the user made (a swipe) is reported. Picking an
item reports `EVENT_DRAWER_SELECTED` and returns `false`: the drawer does not close or highlight
by itself. Rust decides, and the next `setDrawer` carries it out.

### Snackbar

`showSnackbar(message, action)` dismisses any current snackbar, creates a Material `Snackbar`
anchored above the floating action button or the bottom bar if either is visible, and shows it
for `LENGTH_LONG`. The action button reports `EVENT_SNACKBAR_ACTION`. The dismissal callback
reports `EVENT_SNACKBAR_DISMISSED` only for a timeout or a swipe, and only if this snackbar is
still the current one. A snackbar that Rust replaced or removed reports nothing.

### Alert

`showDialog` builds a `MaterialAlertDialogBuilder`. Android places alert buttons by slot rather
than by order, so the roles are mapped: the cancel-role button takes the negative slot, the first
other button the positive slot, the next the neutral slot. A destructive button is coloured with
the theme's error colour after the dialog is shown.

Three slots hold one cancel button and two others. When the application gives more than that,
`showDialog` does not use the slots: it stacks every button as a list under the message, in the
order given, with the destructive one in the error colour, so no choice is dropped. A dismissal
by back or an outside tap is allowed only when `cancelable` is true and reports `-1`.

### Bottom sheet

`presentSheet(hostId)` shows a `KrabikSheetFragment`, a Material `BottomSheetDialogFragment`
whose view is a `RustRootView` for that host. Material supplies the surface, scrim and drag
behaviour. The sheet wraps its content: the root view is measured with a non-exact height, and
in that case it reports the height Rust rendered (`nativeContentHeight`; see `onMeasure` below).
Swiping the sheet away, tapping the scrim and pressing back all arrive in `onCancel`, which calls
`onModalDismiss`.

## `RustRootView`

This is the host view: a `ViewGroup` whose children are created and positioned by Rust. It has
three jobs.

### Session and host identity

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt
class RustRootView(
    context: Context,
    sessionHandle: Long = 0L,
    host: Long = 0L,
    retainedApp: Long = 0L,
) : ViewGroup(context), View.OnClickListener {

    private val ownsSession = sessionHandle == 0L
```

Constructed without a session handle, it *creates* the session (`nativeCreate`) and is host 0.
Constructed with one, it attaches itself as a child host (`nativeAttachHost`). `destroy()` does
the matching thing: the owner destroys the session, a child would call `nativeDestroyHost`.

In practice only the owner's `destroy()` is ever called (from `KrabikActivity.onDestroy`). The
screen and sheet fragments do not call it; their comment says the session owns the host, and
indeed the session drops a popped screen's host itself in `sync_navigation`. So
`nativeDestroyHost` is currently unreachable on Android.

### Measuring and rendering

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt (trimmed)
override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
    val width = MeasureSpec.getSize(widthMeasureSpec)
    val available = MeasureSpec.getSize(heightMeasureSpec)

    if (handle != 0L && width > 0 && available > 0 &&
        (width != renderedWidth || available != renderedHeight)) {
        renderedWidth = width
        renderedHeight = available
        nativeRender(handle, hostId, width, available)
    }

    wrapsContent = MeasureSpec.getMode(heightMeasureSpec) != MeasureSpec.EXACTLY
    val height = if (!wrapsContent || handle == 0L) {
        available
    } else {
        val content = nativeContentHeight(handle, hostId)
        if (available > 0) minOf(content, available) else content
    }
    setMeasuredDimension(width, height)
    // ... measure each child at exactly the size Rust gave it
}
```

Rust renders when the view's size changes, and only then from Kotlin's side. This is how
rotation works: the manifest keeps the activity alive across a rotation, Android re-measures the
view with the new size, and `nativeRender` lays everything out again for the new viewport.

Children are positioned with **translation**, not with their layout position. `onLayout` places
every child at `(0, 0)` with the size from its layout params; `setFrame` writes `translationX`
and `translationY`. The KDoc on `applyFrame` explains the benefit: layout params are only
reassigned when the size changes, so moving a view does not schedule another layout pass.

### Frames and animations

- `applyFrame` (one view) and `applyLayouts` (a batch, addressed by node id) both end in
  `applyFrameInternal`. Without an animation it sets the frame. With one it runs a
  `ValueAnimator` that interpolates translation and size, cancelling any animator already running
  for that view.
- `registerView` / `unregisterView` maintain `nodeViews`, the node-id-to-view map the batch call
  needs. References are weak.
- `startEnter` fades and scales a new view in. `startExit` does the reverse and removes the view
  when the animation ends. `pendingExits` remembers views that are animating out, keyed by node
  id, so that if the same node is created again before the exit finishes, `registerView` can
  detach the old view immediately.

### The `external fun`s and their wrappers

Every `external fun` is private and has a public wrapper that checks `handle != 0L` first, for
example `onCheckedChanged(nodeId, checked)`. Widgets and the controller call the wrappers. The
view is also the `OnClickListener` for stock buttons:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/RustRootView.kt
override fun onClick(view: View) {
    val nodeId = view.tag as? Long ?: return
    nativeOnClick(handle, hostId, nodeId)
}
```

## `KrabikRecycler.kt`: lists

`KrabikRecyclers.create` builds a `RecyclerView` with a `LinearLayoutManager` and a
`RowAdapter`. Item animations are turned off: the comment explains that they detach and rebind
holders on every range change, which clears row state and makes focus and actions flicker.

The adapter owns no data. It knows a count, an optional fixed row height and a revision:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikRecycler.kt (trimmed)
override fun onBindViewHolder(holder: Holder, position: Int) {
    val width = recycler?.width?.takeIf { it > 0 } ?: /* fallbacks */
    val measured = root.bindRow(nodeId, position, holder.id, holder.host, width)
    applyHeight(holder.host, rowHeight(measured))
}

fun update(count: Int, itemHeightPx: Int, revision: Long) {
    // ... notifyItemRangeInserted / notifyItemRangeRemoved for a changed count
    if ((heightChanged || revision != this.revision) && count > 0) {
        this.revision = revision
        notifyItemRangeChanged(0, count)
    }
}
```

Each `Holder` is an empty `FrameLayout` with a process-unique numeric id. Binding asks Rust to
build the row into that frame and uses the returned height when rows are not fixed-height.

When the revision changes (the app's description was rebuilt), the adapter notifies a change for
the whole range. `RecyclerView` only rebinds the rows currently attached, and Rust's unchanged-row
fast path (chapter 2) makes each of those rebinds nearly free when nothing changed. This is the
mechanism by which an app state change reaches rows the recycler already owns.

`onViewAttachedToWindow` calls `restoreRowFocus`; `onViewRecycled` calls `unbindRow`.

`KrabikRows` provides the optional row transition: `snapshot` draws the holder into a bitmap and
puts it in an overlay `ImageView`; `fadeOut` brings the overlay to the front and fades it away
while the new content sits underneath.

## `KrabikStyle`

`KrabikStyle.apply` receives every decoration value of a node in one call and turns it into
drawables.

- It keeps a `ViewStyle` per view in a `WeakHashMap`: the current values, a running animator, and
  the reusable `GradientDrawable`, mask and `RippleDrawable`.
- Without an animation, `applyValues` builds or updates one `GradientDrawable` for background,
  gradient, corner radii and border. For buttons (`ripple = true`) it wraps the shape in a
  `RippleDrawable` so the native press feedback survives. If a node has no decoration at all, the
  view's original background is restored.
- With an animation, a `ValueAnimator` interpolates between the last applied values and the
  target (`interpolate`), using an `ArgbEvaluator` for colours.
- `interpolator(kind, p1..p4)` decodes the packed curve: kind 1 is a cubic bezier
  (`PathInterpolator`), kind 2 a spring (`SpringInterpolator`, a small damped-spring formula in
  this file), and kind 0 one of the four named curves. Parameters are fixed-point with a scale of
  10,000. `RustRootView` uses the same function for frame and enter/exit animations.

## The widget classes and their silent setters

Each interactive widget is a subclass of a stock Android widget with the constructor
`(Context, RustRootView, nodeId)`. It attaches its own listener and reports events with its node
id through the root view.

The recurring problem these classes solve: Android listeners fire for programmatic changes too.
When Rust writes `checked = true` to a switch because the app's state says so,
`OnCheckedChangeListener` fires exactly as if the user had tapped it. Reported back to Rust, that
would run the app's `on_toggle` a second time, for a change the app itself made. A **silent
setter** applies a value from Rust without reporting it:

```kotlin
// shared/android/src/main/java/dev/krabik/ui/KrabikSwitch.kt
fun setCheckedSilently(checked: Boolean) {
    if (isChecked == checked) {
        return
    }
    silent = true
    try {
        isChecked = checked
    } finally {
        silent = false
    }
}
```

The early return also matters: writing an unchanged value is skipped, so a re-render does not
restart animations or move a cursor.

The classes, briefly:

- **`KrabikTextInput`** (`EditText`): owns the text watcher, the editor-action listener (submit)
  and the focus listener. `setTextSilently` removes the watcher, sets the text, restores the
  selection and re-adds the watcher. `configure` applies keyboard type, password mode, multiline
  and the IME action, and returns early when nothing changed. Focus changes are reported with
  `post { ... }`, one main-loop turn later, together with `isAttachedToWindow`, because
  `RecyclerView` moves focus during a layout pass and Rust needs to tell recycling from a real
  blur.
- **`KrabikSwitch`**, **`KrabikCheckBox`**, **`KrabikRadioButton`**: the same pattern as above
  with `setCheckedSilently`.
- **`KrabikSeekBar`** (`SeekBar`): maps the integer range 0 to 1000 onto the slider's
  `min..max`, snaps to the step, and only reports changes made by the user (`fromUser`).
  `setValueSilently` sets the progress.
- **`KrabikRadioGroup`** (`RadioGroup`): rebuilds its buttons only when the labels change;
  selection, colour and spacing are updated in place under the `silent` flag.
- **`KrabikTabBar`** (`LinearLayout` of `TextView`s): a tap updates the highlight at once and
  reports the index; the app sends the selection back down on the next render.
- **`KrabikPager`** (`FrameLayout`): a hand-written horizontal pager. Its pages are ordinary
  Rust-rendered children laid out side by side. It handles drag, fling and snapping with an
  `OverScroller` and reports a page only after a user gesture settles. A page change from Rust
  scrolls without reporting.
- **`KrabikDatePicker`** (`FrameLayout`): a compact field that opens the platform date or time
  dialog, or an inline `DatePicker`/`TimePicker`. Values set from Rust are applied under
  `silent`.
- **`KrabikViews`** (object): creates scroll containers (a `ScrollView` or
  `HorizontalScrollView` subclass with one `FrameLayout` child, the content host) and progress
  bars. `ScrollReporter` coalesces scroll callbacks to one per frame with `postOnAnimation` and
  reads the node id from the view's tag.
- **`KrabikImages`** (object): loads assets and URLs into an `ImageView` with Coil, and reads an
  asset's size from its header for layout.

Silent setters cannot catch everything, because some echoes come from inside Android (setting an
input type can call `setText`). The try-lock in the entry points (chapter 2) catches the rest.

## How an app uses it

An app's Android project has three framework-specific pieces. The example app shows them all.

**A `MainActivity`** that names the native library and the bridge:

```kotlin
// example/android/app/src/main/java/com/example/krabik_ui_dsl/MainActivity.kt
class MainActivity : KrabikActivity() {
    override fun loadNativeLibrary() {
        System.loadLibrary("krabik_ui_dsl_example")
    }

    override fun installApp() {
        ExampleBridge.install()
    }
}
```

**A bridge object** with the one `external fun` the app's crate exports:

```kotlin
// example/android/app/src/main/java/com/example/krabik_ui_dsl/ExampleBridge.kt
object ExampleBridge {
    /** Registers the example app factory with the DSL runtime. */
    external fun install()
}
```

```rust,ignore
// example/rust/src/lib.rs
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_example_krabik_1ui_1dsl_ExampleBridge_install<'caller>(
    _env: EnvUnowned<'caller>,
    _this: JObject<'caller>,
) {
    ShowcaseApp::install();
}
```

The symbol name follows JNI mangling: an underscore in a package or class name is written `_1`,
hence `krabik_1ui_1dsl`. `ShowcaseApp::install` calls `set_app_factory`. The app's library is one
`.so` that contains both the app and the framework (the app crate depends on `krabik-ui-dsl` and
is built as a `cdylib`), so loading it also provides every `Java_dev_krabik_ui_RustRootView_*`
symbol.

**A manifest** with two attributes that matter:

```xml
<!-- example/android/app/src/main/AndroidManifest.xml (trimmed) -->
<application
    android:enableOnBackInvokedCallback="true"
    android:theme="@style/Theme.KrabikUiDsl">
    <activity
        android:name=".MainActivity"
        android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation"
        android:windowSoftInputMode="adjustResize">
```

`configChanges` lists the changes the activity handles itself instead of being recreated:
rotation, resizing, split screen, a hardware keyboard. For those the view is re-measured
and Rust re-renders at the new size. No state is lost and no views are rebuilt.

**`uiMode` is deliberately not in the list.** A switch between light and dark must recreate the
activity so that Material chrome picks up the other theme's resources; the retained app carries
the Rust state across. Adding `uiMode` would keep the old toolbar and dialog colours while the
Rust-drawn content switched.

The theme is `Theme.Material3.DayNight.NoActionBar` (`res/values/themes.xml`): Material 3 so the
chrome looks native, `DayNight` so it follows the system, `NoActionBar` because the activity
supplies its own toolbar.

## Rough edges in this directory

- The class comments of `KrabikActivity` and `RustRootView` predate the floating action button,
  drawer and snackbar, and still describe the root view as "the only Android widget the Rust UI
  owns".
- `ExampleBridge` and the example's `MainActivity` say they install "the checklist app"; the
  example is the widget showcase (`ShowcaseApp`).
- `closeDrawerForBack` returns a boolean nobody reads.
- The identical comment in both fragments' `onDestroyView` talks about screens in the sheet
  fragment.

## Where this lives

- `KrabikActivity.kt`: `onCreate`, `drawEdgeToEdge`, `onDestroy`, `RetainedApp`.
- `KrabikHostController.kt`: `push`, `pop`, `setRootVisible`, `setToolbar`, `setDrawer`,
  `showSnackbar`, `showDialog`, `presentSheet`.
- `RustRootView.kt`: `onMeasure`, `applyLayouts`, `applyFrameInternal`, the `external fun`s.
- `KrabikRecycler.kt`: `RowAdapter.onBindViewHolder`, `RowAdapter.update`.
- `KrabikStyle.kt`: `apply`, `applyValues`, `interpolator`.
- `example/android/app/src/main/`: `MainActivity.kt`, `ExampleBridge.kt`, `AndroidManifest.xml`.

Next: [The iOS renderer](04-the-ios-renderer.md).
