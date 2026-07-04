//! Torry Core — Pure Rust download engine.
//!
//! This crate contains all download logic, hash verification, and chunk
//! management. It has zero knowledge of Node.js or NAPI — those concerns
//! live in the `torry-binding` crate.
//!
//! # Architecture
//!
//! ```text
//! torry-core (this crate)
//!   ├── download  — Chunk management, parallel transfers, resume
//!   ├── hash      — Cryptographic verification (SHA-256, BLAKE3)
//!   └── error     — Error types used across all modules
//! ```

pub mod db;
pub mod download;
pub mod error;
pub mod hash;
pub mod magnet;
pub mod metadata;
pub mod peer;
pub mod profile;
pub mod session;
pub mod state;
pub mod tracker;

pub use error::TorryError;

/// Returns the version of the torry-core library.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_not_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_version_matches_cargo() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }
}
