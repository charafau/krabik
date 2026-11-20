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

// Lets `view!`, which names this crate by its full path, work inside it too.
extern crate self as krabik_ui_dsl;

pub mod app;
pub mod command;
pub(crate) mod element;
pub mod engine;
#[cfg(feature = "hot-reload")]
pub mod hot;
// Only the platform renderers drive a session; it stays compiled everywhere
// so its tests run on the host.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
mod host;
mod log;
mod message;
pub mod navigation;
pub mod presentation;
pub mod scaffold;
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
#[cfg(feature = "svg")]
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
mod svg;

pub use app::{App, Component, Environment, set_app_factory};
pub use krabik_ui_dsl_macros::view;
// The engine modules keep their short public paths, e.g. `krabik_ui_dsl::runtime`.
pub use engine::{geometry, layout, measure, render, runtime, tree};

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

/// Compiles the complete Rust listings in `docs/` as doc-tests, so the
/// documentation cannot drift from the API without `cargo test` noticing.
///
/// Only blocks fenced as plain `rust` are compiled; excerpts are fenced
/// `rust,ignore`.
#[cfg(doctest)]
mod documentation {
    #[doc = include_str!("../../docs/internals/09-app-navigation-scaffold-presentation.md")]
    pub struct Internals09AppNavigationScaffoldPresentation;
    #[doc = include_str!("../../docs/recipes/debug-a-render.md")]
    pub struct RecipesDebugARender;
    #[doc = include_str!("../../docs/recipes/write-tests.md")]
    pub struct RecipesWriteTests;
    #[doc = include_str!("../../docs/tutorial/01-your-first-app.md")]
    pub struct Tutorial01YourFirstApp;
    #[doc = include_str!("../../docs/tutorial/02-widgets-and-layout.md")]
    pub struct Tutorial02WidgetsAndLayout;
    #[doc = include_str!("../../docs/tutorial/03-lists-and-state.md")]
    pub struct Tutorial03ListsAndState;
    #[doc = include_str!("../../docs/tutorial/04-components.md")]
    pub struct Tutorial04Components;
    #[doc = include_str!("../../docs/tutorial/05-navigation-and-scaffold.md")]
    pub struct Tutorial05NavigationAndScaffold;
    #[doc = include_str!("../../docs/tutorial/06-dialogs-sheets-and-more-chrome.md")]
    pub struct Tutorial06DialogsSheetsAndMoreChrome;
    #[doc = include_str!("../../docs/tutorial/07-colours-and-dark-mode.md")]
    pub struct Tutorial07ColoursAndDarkMode;
    #[doc = include_str!("../../docs/tutorial/08-the-view-macro.md")]
    pub struct Tutorial08TheViewMacro;
    #[doc = include_str!("../../docs/tutorial/11-background-work-and-timers.md")]
    pub struct Tutorial11BackgroundWorkAndTimers;
}
