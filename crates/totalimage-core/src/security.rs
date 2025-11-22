//! Security validation constants and helpers
//!
//! This module defines security limits and validation functions to prevent
//! common vulnerabilities in disk image parsing.

use crate::Error;
use std::path::{Path, PathBuf};

/// Maximum sector size we'll accept (4KB - common for advanced format)
pub const MAX_SECTOR_SIZE: u32 = 4096;

/// Maximum allocation size for single buffer (256 MB)
pub const MAX_ALLOCATION_SIZE: usize = 256 * 1024 * 1024;

/// Maximum FAT table size (100 MB - supports very large FAT32)
pub const MAX_FAT_TABLE_SIZE: usize = 100 * 1024 * 1024;

/// Maximum partition count (128 for GPT, padded for safety)
pub const MAX_PARTITION_COUNT: usize = 256;

/// Maximum directory entries to read in one operation
pub const MAX_DIRECTORY_ENTRIES: usize = 10_000;

/// Maximum file size to extract (1 GB)
pub const MAX_FILE_EXTRACT_SIZE: u64 = 1024 * 1024 * 1024;

/// Maximum cluster chain length (prevents infinite loops)
pub const MAX_CLUSTER_CHAIN_LENGTH: usize = 1_000_000;

/// Validate that a size is within allocation limits
///
/// # Security
/// Prevents memory exhaustion attacks from malicious disk images
pub fn validate_allocation_size(size: u64, limit: usize, context: &str) -> crate::Result<usize> {
    if size > limit as u64 {
        return Err(Error::invalid_vault(format!(
            "{} size {} exceeds limit {}",
            context, size, limit
        )));
    }

    size.try_into()
        .map_err(|_| Error::invalid_vault(format!("{} size exceeds platform limits", context)))
}

/// Safely multiply two u64 values with overflow checking
///
/// # Security
/// Prevents integer overflow in size calculations
pub fn checked_multiply_u64(a: u64, b: u64, context: &str) -> crate::Result<u64> {
    a.checked_mul(b)
        .ok_or_else(|| Error::invalid_vault(format!("{}: multiplication overflow", context)))
}

/// Safely multiply u32 values and return u64
pub fn checked_multiply_u32_to_u64(a: u32, b: u32, context: &str) -> crate::Result<u64> {
    (a as u64)
        .checked_mul(b as u64)
        .ok_or_else(|| Error::invalid_vault(format!("{}: multiplication overflow", context)))
}

/// Safely convert u64 to usize with platform checking
///
/// # Security
/// Prevents truncation on 32-bit platforms
pub fn u64_to_usize(value: u64, context: &str) -> crate::Result<usize> {
    value.try_into().map_err(|_| {
        Error::invalid_vault(format!(
            "{}: value {} exceeds platform usize limit",
            context, value
        ))
    })
}

/// Validate sector size is reasonable
pub fn validate_sector_size(sector_size: u32) -> crate::Result<()> {
    if sector_size == 0 || sector_size > MAX_SECTOR_SIZE {
        return Err(Error::invalid_vault(format!(
            "Invalid sector size: {} (must be 1-{})",
            sector_size, MAX_SECTOR_SIZE
        )));
    }

    // Sector size should be power of 2
    if !sector_size.is_power_of_two() {
        return Err(Error::invalid_vault(format!(
            "Sector size {} is not a power of 2",
            sector_size
        )));
    }

    Ok(())
}

/// Sanitize and validate a file path for safe access
///
/// # Security
/// Prevents path traversal attacks in web API
///
/// # Returns
/// Canonical absolute path if valid, error otherwise
pub fn validate_file_path(path: &str) -> crate::Result<PathBuf> {
    // Reject empty paths
    if path.is_empty() {
        return Err(Error::not_found("Empty path".to_string()));
    }

    // Reject paths with null bytes
    if path.contains('\0') {
        return Err(Error::invalid_vault(
            "Path contains null byte".to_string(),
        ));
    }

    let path_obj = Path::new(path);

    // Canonicalize to resolve .. and symlinks
    let canonical = path_obj.canonicalize().map_err(|e| {
        Error::not_found(format!("Path does not exist or is inaccessible: {}", e))
    })?;

    // Ensure it's a file (not a directory or special file)
    if !canonical.is_file() {
        return Err(Error::invalid_vault(format!(
            "Path is not a regular file: {}",
            canonical.display()
        )));
    }

    // Additional security: Could check against allowlist of directories
    // For now, we accept any readable file the process can access

    Ok(canonical)
}

/// Validate partition index is within bounds
pub fn validate_partition_index(index: usize, max: usize) -> crate::Result<()> {
    if index >= max {
        return Err(Error::not_found(format!(
            "Partition index {} out of range (0-{})",
            index,
            max.saturating_sub(1)
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_allocation_size() {
        // Valid size
        assert!(validate_allocation_size(1024, MAX_ALLOCATION_SIZE, "test").is_ok());

        // Too large
        assert!(validate_allocation_size(
            MAX_ALLOCATION_SIZE as u64 + 1,
            MAX_ALLOCATION_SIZE,
            "test"
        )
        .is_err());
    }

    #[test]
    fn test_checked_multiply_u64() {
        // Valid multiplication
        assert_eq!(
            checked_multiply_u64(1000, 512, "test").unwrap(),
            512_000
        );

        // Overflow
        assert!(checked_multiply_u64(u64::MAX, 2, "test").is_err());
    }

    #[test]
    fn test_validate_sector_size() {
        // Valid sizes
        assert!(validate_sector_size(512).is_ok());
        assert!(validate_sector_size(4096).is_ok());

        // Invalid sizes
        assert!(validate_sector_size(0).is_err());
        assert!(validate_sector_size(5000).is_err());
        assert!(validate_sector_size(1000).is_err()); // Not power of 2
    }

    #[test]
    fn test_u64_to_usize() {
        assert_eq!(u64_to_usize(1024, "test").unwrap(), 1024);

        #[cfg(target_pointer_width = "32")]
        {
            // Would overflow on 32-bit
            assert!(u64_to_usize(0xFFFFFFFF + 1, "test").is_err());
        }
    }

    #[test]
    fn test_validate_file_path() {
        // Empty path
        assert!(validate_file_path("").is_err());

        // Null byte
        assert!(validate_file_path("test\0file").is_err());

        // Non-existent path
        assert!(validate_file_path("/nonexistent/file").is_err());
    }
}
