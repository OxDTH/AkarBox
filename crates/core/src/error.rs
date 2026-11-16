//! One error type for the whole crate.
//!
//! Rust has no exceptions: functions that can fail return `Result<T, E>`.
//! `thiserror` generates the boilerplate (`Display`, `std::error::Error`) for us.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unknown or unsupported file format: {0:?}")]
    UnknownFormat(String),

    #[error("no conversion path from {from} to {to}")]
    NoPath { from: String, to: String },

    #[error("`{program}` failed: {stderr}")]
    CommandFailed { program: String, stderr: String },

    #[error("converter finished but did not produce {0}")]
    MissingOutput(PathBuf),

    /// `#[from]` lets the `?` operator turn a `std::io::Error` into ours automatically.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Shorthand so we can write `Result<T>` instead of `Result<T, Error>` everywhere.
pub type Result<T> = std::result::Result<T, Error>;
