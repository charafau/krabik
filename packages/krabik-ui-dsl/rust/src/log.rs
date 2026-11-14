//! Where the framework reports a problem it can continue from.
//!
//! Standard error is not shown anywhere on Android, so messages go to the
//! system log there (`adb logcat -s krabik`) and to standard error elsewhere,
//! which Xcode's console and `cargo test` both show.

/// Reports a problem.
pub(crate) fn error(message: &str) {
    #[cfg(target_os = "android")]
    {
        use std::ffi::{CString, c_char, c_int};

        #[link(name = "log")]
        unsafe extern "C" {
            fn __android_log_write(
                priority: c_int,
                tag: *const c_char,
                text: *const c_char,
            ) -> c_int;
        }
        /// `ANDROID_LOG_ERROR` in `android/log.h`.
        const ERROR: c_int = 6;

        // A message with an interior NUL cannot be passed on; say so instead.
        let text =
            CString::new(message).unwrap_or_else(|_| c"(message contained a NUL byte)".to_owned());
        // SAFETY: both pointers are valid NUL-terminated strings for the
        // duration of the call, which copies them.
        unsafe {
            __android_log_write(ERROR, c"krabik".as_ptr(), text.as_ptr());
        }
    }
    #[cfg(not(target_os = "android"))]
    eprintln!("krabik: {message}");
}
