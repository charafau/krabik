//! A declarative, platform-independent UI runtime.
//!
//! Applications describe their interface with [`ui`], the [`runtime`] reconciles
//! the description into a minimal batch of operations, [`layout`] calculates
//! positions with Taffy, and the platform renderer maps the operations onto
//! native widgets.

pub mod app;
pub(crate) mod element;
pub mod engine;
// Only the platform renderers drive a session; it stays compiled everywhere
// so its tests run on the host.
pub mod gallery;
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
mod host;
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

pub use app::{App, create_app, set_app_factory};
// The engine modules keep their short public paths, e.g. `krabik_ui_dsl::runtime`.
pub use engine::{geometry, layout, measure, render, runtime, tree};
pub use gallery::GalleryApp;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;
