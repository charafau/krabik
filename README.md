<p align="center">
  <img src="assets/krabik.svg" alt="krabik logo" width="180">
</p>

<h1 align="center">krabik</h1>

<p align="center">
  <strong>Build cross-platform Rust mobile apps with native UI.</strong>
</p>

This repository is a Cargo workspace with the krabik toolchain and libraries:

| Package | Description |
| --- | --- |
| [`packages/krabik-cli`](packages/krabik-cli/) | The `krabik` CLI: scaffold, build, run, and watch Android and iOS apps with a shared Rust core and UniFFI bindings. |
| [`packages/krabik-ui-android`](packages/krabik-ui-android/) | Declarative Android view bindings for Rust, created through direct JNI. |

## Quick start

```sh
cargo install --path packages/krabik-cli

krabik doctor          # check your development environment
krabik create my-app   # scaffold a new project in ./my-app
cd my-app
krabik run             # build, install and launch the app
```

See [`packages/krabik-cli/README.md`](packages/krabik-cli/README.md) for the full
command reference and platform requirements.

## Using the UI library

`krabik-ui-android` turns a plain Rust element tree into real Android framework
views. It is application-agnostic: screens, navigation, and click semantics
stay in your app. See
[`packages/krabik-ui-android/README.md`](packages/krabik-ui-android/README.md)
for the component catalogue and the Android bridge setup.

The [`example/`](packages/krabik-ui-android/example/) app inside that package is
a complete Android widget browser and the integration reference.

## Development

```sh
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

`packages/krabik-cli` is the default workspace member, so `cargo run` at the
repository root runs the CLI:

```sh
cargo run -- create my-app
```

The UI example is a standalone Cargo project and is not part of the workspace.
Build it with:

```sh
cargo check --manifest-path packages/krabik-ui-android/example/rust/Cargo.toml
```
