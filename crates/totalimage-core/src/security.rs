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

/// Maximum file size for memory mapping (16 GB - practical limit for most systems)
pub const MAX_MMAP_SIZE: u64 = 16 * 1024 * 1024 * 1024;

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

    // Defense in depth: Reject obvious traversal attempts before canonicalization
    // This catches cases where canonicalization might behave unexpectedly
    if path.contains("..") {
        return Err(Error::invalid_path(
            "Path traversal sequences not allowed".to_string(),
        ));
    }

    // Reject control characters and other suspicious sequences
    if path.chars().any(|c| c.is_control() && c != '\t') {
        return Err(Error::invalid_path(
            "Path contains invalid control characters".to_string(),
        ));
    }

    let path_obj = Path::new(path);

    // Canonicalize to resolve symlinks
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

    Ok(canonical)
}

/// Validate a file path against an allowed directory whitelist
///
/// # Security
/// Ensures file access is restricted to specific directories
///
/// # Arguments
/// * `path` - The path to validate
/// * `allowed_dirs` - List of directories that are allowed
///
/// # Returns
/// Canonical absolute path if within allowed directories, error otherwise
pub fn validate_file_path_in_dirs(path: &str, allowed_dirs: &[&Path]) -> crate::Result<PathBuf> {
    // First do basic validation
    let canonical = validate_file_path(path)?;

    // Check if the canonical path is within any of the allowed directories
    let in_allowed_dir = allowed_dirs.iter().any(|allowed| {
        if let Ok(allowed_canonical) = allowed.canonicalize() {
            canonical.starts_with(&allowed_canonical)
        } else {
            false
        }
    });

    if !in_allowed_dir {
        return Err(Error::permission_denied(format!(
            "Access denied: path '{}' is outside allowed directories",
            path
        )));
    }

    Ok(canonical)
}

/// Sanitize a filename extracted from a disk image
///
/// # Security
/// Prevents malicious filenames from causing path traversal or other issues
///
/// # Returns
/// Sanitized filename safe for use in file operations
pub fn sanitize_extracted_filename(filename: &str) -> String {
    filename
        .chars()
        // Remove path separators
        .filter(|&c| c != '/' && c != '\\')
        // Remove null bytes and control characters
        .filter(|&c| !c.is_control())
        // Limit length
        .take(255)
        .collect::<String>()
        // Remove leading/trailing dots and spaces
        .trim_start_matches(|c| c == '.' || c == ' ')
        .trim_end_matches(|c| c == '.' || c == ' ')
        .to_string()
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
