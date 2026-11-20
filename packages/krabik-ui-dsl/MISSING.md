# Missing or deferred framework capabilities

This rewrite intentionally uses the current Krabik APIs without modifying
`krabik-ui-dsl`. The following gaps are handled with documented compromises
and are candidates for a later framework pass.

## Pull-to-refresh

- Missing: a native pull-to-refresh container/gesture.
- Current handling: explicit refresh action in the app bar.

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
