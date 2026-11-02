//! Error type shared by the bridge.

use std::fmt;

/// A bridge failure with enough context to identify the failing JNI operation.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    /// Creates an error that describes what failed and why.
    pub fn context(context: impl fmt::Display, source: impl fmt::Display) -> Self {
        Self(format!("{context}: {source}"))
    }

    /// Creates an error from a plain message.
    pub fn message(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<jni::errors::Error> for Error {
    fn from(error: jni::errors::Error) -> Self {
        Self(error.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_prefixes_the_source() {
        let error = Error::context("constructing View", "boom");
        assert_eq!(error.to_string(), "constructing View: boom");
    }

    #[test]
    fn message_keeps_the_supplied_text() {
        assert_eq!(Error::message("plain failure").to_string(), "plain failure");
        assert_eq!(Error::message(String::from("owned")).to_string(), "owned");
    }

    #[test]
    fn errors_expose_no_source() {
        let error = Error::message("leaf");
        assert!(std::error::Error::source(&error).is_none());
    }

    #[test]
    fn jni_errors_convert_into_bridge_errors() {
        let error: Error = jni::errors::Error::ObjectFreed.into();
        assert_eq!(error.to_string(), "Object behind weak reference freed");
    }
}
