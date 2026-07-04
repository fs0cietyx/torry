//! Download engine — chunk management and parallel transfers.
//!
//! This module will contain the core download logic including:
//! - HTTP range request handling for parallel chunk downloads
//! - Automatic resume on connection failure
//! - Progress reporting via callbacks
//! - Chunk reassembly and file writing

pub mod disk;
pub mod manager;

/// Configuration for a download task.
#[derive(Debug, Clone)]
pub struct DownloadOptions {
    /// The URL to download from.
    pub url: String,
    /// The local file path to write to.
    pub output_path: String,
    /// Size of each download chunk in bytes.
    pub chunk_size: usize,
}

impl Default for DownloadOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            output_path: String::new(),
            chunk_size: 1024 * 1024, // 1 MB
        }
    }
}

/// Adds two numbers.
///
/// This is a **pipeline verification function** used to prove the full chain:
/// `TypeScript → NAPI binding → torry-core (here) → NAPI binding → TypeScript`
///
/// It will be replaced with actual download logic in a later step.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
        assert_eq!(add(i32::MAX, 0), i32::MAX);
    }

    #[test]
    fn test_default_options() {
        let opts = DownloadOptions::default();
        assert_eq!(opts.chunk_size, 1024 * 1024);
        assert!(opts.url.is_empty());
        assert!(opts.output_path.is_empty());
    }
}
