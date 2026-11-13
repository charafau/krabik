//! A mobile UI framework: describe the interface in Rust, draw it with native
//! Android and iOS widgets.
//!
//! An application implements [`Component`] (state, messages, `update`, `view`)
//! and [`App`], builds its view from the widgets in [`ui`], and registers
//! itself with [`set_app_factory`]. The [`engine`] turns each view into a tree,
//! lays it out with Taffy and diffs it against the previous one; the platform
//! renderer applies the resulting operations to native views and reports
//! events back as the application's messages.
//!
//! The guide in `docs/` walks through all of it.

pub mod app;
pub(crate) mod element;
pub mod engine;
// Only the platform renderers drive a session; it stays compiled everywhere
// so its tests run on the host.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
mod host;
mod message;
pub mod navigation;
pub mod presentation;
pub mod style;
pub mod ui;
pub mod widgets;

// Only the iOS renderer evaluates curves from Rust; the module stays compiled
// everywhere so its tests run on the host.
#[cfg_attr(not(target_os = "ios"), allow(dead_code))]
mod animation;

// Only the Android renderer consumes the packed frame arrays; the module stays
// compiled everywhere so its tests run on the host.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod frame_batch;

pub use app::{App, Component, set_app_factory};
// The engine modules keep their short public paths, e.g. `krabik_ui_dsl::runtime`.
pub use engine::{geometry, layout, measure, render, runtime, tree};

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;
