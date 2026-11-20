# Missing or deferred framework capabilities

This rewrite intentionally uses the current Krabik APIs without modifying
`krabik-ui-dsl`. The following gaps are handled with documented compromises
and are candidates for a later framework pass.

## Custom illustration pipeline

- Available now: `.svg` assets (cargo feature `svg`), including SVG blur
  filters, and `Stack` to put content over an illustration.
- Still missing, on purpose: a blur style on native views. It was prototyped
  and removed because Android and iOS cannot produce the same result; see
  decision 12 in `docs/design/decisions.md`.

## Pixel-perfect Flutter parity

- Missing: Flutter-specific layout, animation, gesture and image-cache behavior.
- Current handling: native platform presentation with equivalent application
  behavior rather than pixel-identical UI.

## Hot reload

- Flutter and react native support hot reload, krabik doesn't.
- Dioxus, framework in rust does support it. Check how it works.
