//! A declarative, platform-independent UI runtime.
//!
//! Applications describe their interface with [`ui`], the [`runtime`] reconciles
//! the description into a minimal batch of operations, [`layout`] calculates
//! positions with Taffy, and the platform renderer maps the operations onto
//! native widgets.

uniffi::setup_scaffolding!();

// The Kotlin JNI scaffolding is generated for Android only. On iOS the
// renderer uses a plain C ABI instead, and the runtime crate is not linked.
#[cfg(not(target_os = "ios"))]
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

// Only the Android renderer consumes the packed frame arrays; the module stays
// compiled everywhere so its tests run on the host.
#[cfg_attr(not(target_os = "android"), allow(dead_code))]
mod frame_batch;

pub use app::{App, create_app, set_app_factory};
pub use gallery::GalleryApp;

#[cfg(target_os = "android")]
mod android;

#[cfg(target_os = "ios")]
mod ios;

/// Returns a greeting.
///
/// Kept as the smoke-test export of the generated bindings pipeline.
#[uniffi::export]
pub fn get_greeting() -> String {
    "Hello from Rust!".to_string()
}
