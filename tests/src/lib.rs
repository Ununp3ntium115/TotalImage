//! Integration test library for TotalImage
//!
//! This crate provides shared utilities and helpers for integration tests.

/// Test utilities and helpers
pub mod utils {
    use std::path::Path;

    /// Check if test fixtures are available
    pub fn fixtures_available() -> bool {
        Path::new("tests/fixtures").exists()
    }
}
