//! A declarative, platform-independent UI runtime.
//!
//! Applications describe their interface with [`ui`], the [`runtime`] reconciles
//! the description into a minimal batch of operations, [`layout`] calculates
//! positions with Taffy, and the platform renderer maps the operations onto
//! native widgets.

uniffi::setup_scaffolding!();

include!(concat!(
    env!("OUT_DIR"),
    "/uniffi_bindgen_kotlin_jni.uniffi.rs"
));

pub mod app;
pub mod gallery;
pub mod geometry;
pub mod layout;
pub mod measure;
pub mod render;
pub mod runtime;
pub mod tree;
pub mod ui;

pub use app::{create_app, set_app_factory, App};
pub use gallery::GalleryApp;

#[cfg(target_os = "android")]
mod android;

/// Returns a greeting.
///
/// Kept as the smoke-test export of the generated bindings pipeline.
#[uniffi::export]
pub fn get_greeting() -> String {
    "Hello from Rust!".to_string()
}
