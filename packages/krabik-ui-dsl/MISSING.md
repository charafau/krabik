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

- Available now for development, on the Android emulator and the iOS
  simulator: `shared/hot-reload.sh` puts changed `view` and `update` code into
  the running app and keeps its state (about 3 s on Android, about 1 s on
  iOS). See [docs/design/hot-reload.md](docs/design/hot-reload.md).
- Still missing: devices, and changes to the layout of the app's state or
  message type, which need a restart.
