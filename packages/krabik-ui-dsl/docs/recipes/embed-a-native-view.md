# Embed a native view of your own

The framework draws what Android and iOS have in common. Some things an application needs are
not in that set and never will be: a map, a video player, a camera preview, a chart from a
platform library. For those the application makes the view itself, in Kotlin and in Swift, and
Rust places it in the screen like any other widget.

This recipe embeds a map. The same three steps work for anything.

## What the pieces are

```text
Rust                      Kotlin / Swift (the application's own code)
----                      -------------------------------------------
NativeView::new("map")    KrabikNativeViews.register("map") { ... }
  .prop("lat", "50.06")     -> a view, told its properties as strings
  .on_event(Msg::Map)       <- strings the view sends
```

- **A name** connects the two sides. Rust asks for a view by name; each platform has a factory
  registered under it.
- **Properties** go from Rust to the view as strings. The view's `update` is called with all of
  them when the view is created and again whenever one changes.
- **Events** go from the view to Rust as strings, through the `emit` function the factory is
  given. Rust receives them in `on_event`.

Rust owns the frame. The view fills whatever size the description gives; it is never asked how
large it wants to be, so give a `NativeView` a width and a height or let a parent stretch it.
Without either it is 200 points square.

## Step 1: describe it in Rust

```rust,ignore
NativeView::new("map")
    .prop("lat", self.place.lat.to_string())
    .prop("lon", self.place.lon.to_string())
    .height(240)
    .corner_radius(12)
    .on_event(Msg::MapMoved)
```

`Msg::MapMoved(String)` receives whatever the view sends. Parse it in `update`; a small format
of your own, such as `"50.06,19.94"`, is enough.

## Step 2: make the view on Android

Register the factory before the first screen is shown. The activity's `installApp` is a good
place:

```kotlin
class MainActivity : KrabikActivity() {
    override fun installApp() {
        KrabikNativeViews.register("map") { context, emit ->
            object : KrabikNativeView {
                private val map = MapView(context).apply {
                    onCreate(null)
                    onResume()
                    getMapAsync { google ->
                        google.setOnCameraIdleListener {
                            val at = google.cameraPosition.target
                            emit("${at.latitude},${at.longitude}")
                        }
                    }
                }
                override val view: View = map

                override fun update(props: Map<String, String>) {
                    val lat = props["lat"]?.toDoubleOrNull() ?: return
                    val lon = props["lon"]?.toDoubleOrNull() ?: return
                    map.getMapAsync { it.moveCamera(CameraUpdateFactory.newLatLng(LatLng(lat, lon))) }
                }
            }
        }
        AppBridge.install()
    }
}
```

The map library, its API key and its permissions are the application's business, in the
application's own Gradle file and manifest.

## Step 3: make the view on iOS

```swift
@main
struct NotesApp: App {
    init() {
        notes_install()
        KrabikNativeViews.register("map") { emit in MapView(emit: emit) }
    }
    // ...
}

final class MapView: NSObject, KrabikNativeView, MKMapViewDelegate {
    private let map = MKMapView()
    private let emit: (String) -> Void
    var view: UIView { map }

    init(emit: @escaping (String) -> Void) {
        self.emit = emit
        super.init()
        map.delegate = self
    }

    func update(_ props: [String: String]) {
        guard let lat = Double(props["lat"] ?? ""), let lon = Double(props["lon"] ?? "") else { return }
        map.setCenter(CLLocationCoordinate2D(latitude: lat, longitude: lon), animated: true)
    }

    func mapView(_ mapView: MKMapView, regionDidChangeAnimated animated: Bool) {
        emit("\(mapView.centerCoordinate.latitude),\(mapView.centerCoordinate.longitude)")
    }
}
```

## What to keep in mind

- **`update` is told everything, every time.** Compare with what the view shows before doing
  work, as the map above would do before moving its camera. `update` also runs after an event the
  view itself caused, with whatever the application made of it.
- **A view is kept while its node is.** Give the `NativeView` a `.key(..)` if its position among
  its siblings can change, as for any widget. Changing the name at the same position replaces the
  view.
- **A name nothing is registered under** shows an empty view and is logged once, on both
  platforms.
- **Strings only.** A property or an event that is not a string is serialised by you. That keeps
  the bridge one function wide on each platform.
- **In a recycled list** a native view is created and dropped with its row, like every other
  widget of the row. A heavy view, such as a map or a web page, does not belong there.
- **Lifecycle is yours.** The framework adds and removes the view. A view that needs to hear
  about `onPause` or about the scene going to the background observes that itself.

## The one the framework brings

`WebView::new(url)` is a `NativeView` whose factory the framework registers on both platforms
(`android.webkit.WebView`, `WKWebView`), under the name `krabik.webview`. `on_navigate` reports
the address of every page that finished loading. An Android application needs the `INTERNET`
permission to load anything from the network.

## Where the code is

| What | File |
| ---- | ---- |
| `NativeView`, `WebView` | `rust/src/widgets/native_view.rs` |
| The Android registry, host and web view | `shared/android/src/main/java/dev/krabik/ui/KrabikNativeViews.kt` |
| The iOS registry, host and web view | `shared/ios/KrabikNativeViews.swift` |
| The renderers | `rust/src/android/widgets/native_view.rs`, `rust/src/ios/widgets/native_view.rs` |
| A star rating and a video, registered by an app | `example/android/.../NativeViews.kt`, `example/ios/Sources/NativeViews.swift` |
