//! Declarative Android view bindings created through direct JNI.
//!
//! This crate turns a plain Rust [`ui::Element`] tree into real Android framework views. It has no
//! application screens, navigation, or JNI entry points of its own: callers build a tree, hand it
//! to [`Renderer`] with an `Activity` and a `View.OnClickListener`, and map the click tags written
//! by [`ui::Button`] back to their own actions.
//!
//! ```text
//! Rust screen description (ui::Element)
//!             │
//!             ▼
//!       render::Renderer ── JNI ──► Android framework views
//! ```
//!
//! # Layout
//!
//! - [`mod@constants`]: framework class names, resources, and Android constants.
//! - [`mod@env`]: fallible JNI helpers and Logcat output.
//! - [`mod@error`]: the error type shared by the bindings.
//! - [`mod@ui`]: layout primitives, widget components, and the declarative element tree.
//! - [`mod@material`]: Material dialogs, snackbars, pickers, badges, tooltips, and dynamic colours.
//! - [`mod@views`]: construction and configuration of individual Android views, including
//!   AndroidX layout managers and the [`views::ItemAdapterFactory`] used by `RecyclerView`,
//!   `ViewPager2`, and the Material carousel.
//! - [`mod@render`]: translates an element tree into a live view hierarchy.
//!
//! No XML layouts, Compose screens, WebViews, or custom renderers are involved.

pub mod constants;
pub mod env;
pub mod error;
pub mod material;
pub mod render;
pub mod ui;
pub mod views;

pub use error::{Error, Result};
pub use render::Renderer;
