//! Error types for the Torry core engine.
//!
//! All errors are modeled as a single enum using `thiserror` for
//! automatic `Display` and `From` implementations. This gives us:
//! - Exhaustive pattern matching at call sites
//! - Zero-cost error conversions with `?` operator
//! - Human-readable error messages for free

use thiserror::Error;

/// All possible errors from the Torry core engine.
#[derive(Error, Debug)]
pub enum TorryError {
    /// A download operation failed.
    #[error("Download failed: {0}")]
    DownloadError(String),

    /// Hash verification did not match the expected value.
    #[error("Hash verification failed: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// An I/O error occurred (file read/write, permissions, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A NAPI-related error occurred.
    #[error("NAPI Error: {0}")]
    Napi(String),

    /// Metadata could not be parsed or is invalid.
    #[error("Invalid metadata: {0}")]
    InvalidMetadata(String),

    /// The provided URL is not valid or not supported.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Convenience Result type for Torry operations.
pub type Result<T> = std::result::Result<T, TorryError>;
