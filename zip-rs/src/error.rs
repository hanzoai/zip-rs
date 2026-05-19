//! Error type for handlers.
//!
//! `zip_rs::Result<T>` is `core::result::Result<T, zip_rs::Error>`. `Error`
//! is a thin wrapper around a `String` so any `Into<String>` value (a
//! `&str`, a formatted string, an `anyhow::Error`'s message) can be
//! returned with the `?` operator or `into()`.

use alloc::string::String;
use core::fmt;

/// Handler error. Wraps a message that gets serialized into the JSON
/// response under the `error` field.
#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::new(alloc::format!("json: {}", e))
    }
}

/// Handler result alias. Errors are serialized to
/// `{"ok":false,"error":"<message>"}` by the macro-generated wrapper.
pub type Result<T> = core::result::Result<T, Error>;
