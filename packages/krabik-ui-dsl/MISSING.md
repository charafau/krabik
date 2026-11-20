# Missing or deferred framework capabilities

An application rewrite listed the gaps it had to work around. Most are closed
now (startup work, tappable boxes, language and appearance, pull to refresh,
SVG assets; see section 9 of [TODO.md](TODO.md)). What is left:

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
- Dioxus, framework in rust does support it. How it works, what it would take
  here and what to build first are written up in
  [docs/design/hot-reload.md](docs/design/hot-reload.md). Nothing is built yet,
  and the write-up is from reading, not from a device run.
