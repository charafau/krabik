# Missing or deferred framework capabilities

This rewrite intentionally uses the current Krabik APIs without modifying
`krabik-ui-dsl`. The following gaps are handled with documented compromises
and are candidates for a later framework pass.

## Pull-to-refresh

- Missing: a native pull-to-refresh container/gesture.
- Current handling: explicit refresh action in the app bar.

## Whole-app appearance override

- Missing: an app-selected System/Light/Dark mode that updates the DSL colour
  scheme and native app bars, tab bars and other platform chrome.
- Current handling: the preference and selection are persisted and restored,
  but the effective appearance continues to follow the device because Krabik's
  host does not expose a colour-scheme override. The Appearance screen says so.

## Custom illustration pipeline

- Missing: SVG rendering and effects such as blur.
- Current handling: onboarding uses native cards, simplified graphics and a
  rasterized logo asset.

## Pixel-perfect Flutter parity

- Missing: Flutter-specific layout, animation, gesture and image-cache behavior.
- Current handling: native platform presentation with equivalent application
  behavior rather than pixel-identical UI.

## Hot reload

- Flutter and react native support hot reload, krabik doesn't.
- Dioxus, framework in rust does support it. Check how it works.
