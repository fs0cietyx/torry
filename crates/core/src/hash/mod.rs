//! Hash verification for downloaded files.
//!
//! Supports multiple hash algorithms for verifying download integrity.
//! This is the core of Torry's "verification-first" design — every
//! download can be cryptographically verified before being trusted.

use crate::error::Result;

/// Supported hash algorithms for download verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256 — widely used, good compatibility with existing checksums.
    Sha256,
    /// BLAKE3 — faster than SHA-256, modern cryptographic design.
    Blake3,
}

impl std::fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "SHA-256"),
            HashAlgorithm::Blake3 => write!(f, "BLAKE3"),
        }
    }
}

/// Verify a file's hash matches the expected value.
///
/// # Arguments
/// * `path` — Path to the file to verify.
/// * `expected` — Expected hex-encoded hash string.
/// * `algorithm` — Hash algorithm to use for computation.
///
/// # Returns
/// `Ok(true)` if the computed hash matches `expected`, `Ok(false)` otherwise.
pub fn verify_hash(
    _path: &str,
    _expected: &str,
    _algorithm: HashAlgorithm,
) -> Result<bool> {
    // TODO: Implement actual hash computation
    // Will use: sha2 crate (SHA-256) or blake3 crate (BLAKE3)
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_algorithm_display() {
        assert_eq!(format!("{}", HashAlgorithm::Sha256), "SHA-256");
        assert_eq!(format!("{}", HashAlgorithm::Blake3), "BLAKE3");
    }

    #[test]
    fn test_hash_algorithm_equality() {
        assert_eq!(HashAlgorithm::Sha256, HashAlgorithm::Sha256);
        assert_ne!(HashAlgorithm::Sha256, HashAlgorithm::Blake3);
    }

    #[test]
    fn test_verify_hash_placeholder() {
        let result = verify_hash("test.txt", "abc123", HashAlgorithm::Sha256);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
