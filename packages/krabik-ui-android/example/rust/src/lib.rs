//! Rust-to-Android native UI example.
//!
//! The whole Android view hierarchy is created and configured from Rust through direct JNI calls
//! into the Android framework. The `AndroidBridge` Java class only loads the library, hands the
//! activity to Rust, and forwards clicks back into native code.
//!
//! The Android-specific JNI code lives in the `android` module.

mod android;
