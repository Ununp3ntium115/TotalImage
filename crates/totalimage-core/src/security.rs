//! Security validation constants and helpers
//!
//! This module defines security limits and validation functions to prevent
//! common vulnerabilities in disk image parsing.

use crate::Error;
use std::{env, io};
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

/// Default environment variable for sandboxing host file access.
pub const DEFAULT_ALLOWED_ROOT_ENV: &str = "TOTALIMAGE_ALLOWED_ROOT";

/// Load allowed filesystem roots from the default environment variable.
pub fn allowed_roots_from_env() -> crate::Result<Vec<PathBuf>> {
    allowed_roots_from_env_var(DEFAULT_ALLOWED_ROOT_ENV)
}

/// Load allowed filesystem roots from a specific environment variable.
///
/// The variable should contain a platform-specific path list (e.g. `:/data:/images`
/// on Unix, `C:\\Images;D:\\Archives` on Windows). Each entry must exist and be a directory.
pub fn allowed_roots_from_env_var(var_name: &str) -> crate::Result<Vec<PathBuf>> {
    let value = env::var(var_name).map_err(|_| {
        Error::invalid_vault(format!(
            "{} must be set to directories TotalImage can access",
            var_name
        ))
    })?;

    let mut roots = Vec::new();
    for raw in env::split_paths(&value) {
        if raw.as_os_str().is_empty() {
            continue;
        }

        let canonical = raw.canonicalize().map_err(|e| {
            Error::invalid_vault(format!(
                "Allowed root {} is invalid: {}",
                raw.display(),
                e
            ))
        })?;

        if !canonical.is_dir() {
            return Err(Error::invalid_vault(format!(
                "Allowed root {} is not a directory",
                canonical.display()
            )));
        }

        roots.push(canonical);
    }

    if roots.is_empty() {
        return Err(Error::invalid_vault(format!(
            "{} must contain at least one directory",
            var_name
        )));
    }

    Ok(roots)
}

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

/// Sanitize and validate a file path for safe access against an allowed root list.
///
/// # Security
/// Prevents path traversal attacks in services that expose host files.
///
/// # Returns
/// Canonical absolute path if valid, error otherwise
pub fn validate_file_path(path: &str, allowed_roots: &[PathBuf]) -> crate::Result<PathBuf> {
    if path.trim().is_empty() {
        return Err(Error::not_found("Empty path".to_string()));
    }

    if path.contains('\0') {
        return Err(Error::invalid_vault(
            "Path contains null byte".to_string(),
        ));
    }

    if allowed_roots.is_empty() {
        return Err(Error::invalid_vault(
            "No allowed directories configured for file validation".to_string(),
        ));
    }

    let path_obj = Path::new(path);
    let candidates: Vec<PathBuf> = if path_obj.is_absolute() {
        vec![path_obj.to_path_buf()]
    } else {
        allowed_roots.iter().map(|root| root.join(path_obj)).collect()
    };

    let mut saw_not_found = false;
    let mut last_error: Option<Error> = None;

    for candidate in candidates {
        match candidate.canonicalize() {
            Ok(canonical) => {
                if !canonical.is_file() {
                    last_error = Some(Error::invalid_vault(format!(
                        "Path is not a regular file: {}",
                        canonical.display()
                    )));
                    continue;
                }

                if allowed_roots.iter().any(|root| canonical.starts_with(root)) {
                    return Ok(canonical);
                }

                last_error = Some(Error::invalid_vault(format!(
                    "Path {} is outside allowed directories",
                    canonical.display()
                )));
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    saw_not_found = true;
                } else {
                    last_error = Some(Error::invalid_vault(format!(
                        "Failed to access {}: {}",
                        candidate.display(),
                        e
                    )));
                }
            }
        }
    }

    if saw_not_found {
        return Err(Error::not_found(format!(
            "Path does not exist or is inaccessible: {}",
            path
        )));
    }

    Err(last_error.unwrap_or_else(|| {
        Error::invalid_vault(format!(
            "Unable to validate {} against allowed directories",
            path
        ))
    }))
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
        .trim_start_matches(['.', ' '])
        .trim_end_matches(['.', ' '])
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

/// Validate filesystem path components to prevent directory traversal (GAP-006).
///
/// This function validates paths within forensic disk images to prevent attackers
/// from crafting malicious filesystems with ".." or absolute paths that could
/// traverse outside the intended directory structure.
///
/// # Security
///
/// Prevents path traversal attacks in filesystem implementations (FAT, exFAT, ISO, NTFS).
/// Without this validation, a malicious disk image could contain directory entries
/// with names like "../../../etc/passwd" that could trick filesystem parsers into
/// accessing unintended locations.
///
/// # Validation Rules
///
/// - Rejects empty paths
/// - Rejects paths containing ".." (parent directory)
/// - Rejects paths containing "." (current directory)
/// - Rejects absolute paths (starting with / or \)
/// - Rejects paths with null bytes
/// - Splits on both / and \ (cross-platform)
///
/// # Arguments
///
/// * `path` - The filesystem path to validate (e.g., "dir/subdir/file.txt")
///
/// # Returns
///
/// Vector of validated path components if valid, error otherwise
///
/// # Examples
///
/// ```
/// # use totalimage_core::validate_fs_path_components;
/// // Valid paths
/// assert!(validate_fs_path_components("dir/file.txt").is_ok());
/// assert!(validate_fs_path_components("subdir\\another\\file.dat").is_ok());
///
/// // Invalid paths (rejected)
/// assert!(validate_fs_path_components("../etc/passwd").is_err());
/// assert!(validate_fs_path_components("/absolute/path").is_err());
/// assert!(validate_fs_path_components("dir/./file").is_err());
/// ```
pub fn validate_fs_path_components(path: &str) -> crate::Result<Vec<String>> {
    // Reject empty paths
    if path.is_empty() {
        return Err(Error::invalid_vault(
            "Empty filesystem path".to_string(),
        ));
    }

    // Reject paths with null bytes
    if path.contains('\0') {
        return Err(Error::invalid_vault(
            "Filesystem path contains null byte".to_string(),
        ));
    }

    // Reject absolute paths
    if path.starts_with('/') || path.starts_with('\\') {
        return Err(Error::invalid_vault(format!(
            "Absolute filesystem paths not allowed: {}",
            path
        )));
    }

    let path = path.trim_matches('/').trim_matches('\\');
    
    // Split path on / or \
    let parts: Vec<String> = path
        .split(|c| c == '/' || c == '\\')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Validate each component
    for part in &parts {
        // Reject ".." (parent directory traversal)
        if part == ".." {
            return Err(Error::invalid_vault(format!(
                "Path traversal detected (..): {}",
                path
            )));
        }

        // Reject "." (current directory - unnecessary and suspicious)
        if part == "." {
            return Err(Error::invalid_vault(format!(
                "Current directory reference (.): {}",
                path
            )));
        }

        // Additional check: Reject any component containing null bytes
        if part.contains('\0') {
            return Err(Error::invalid_vault(
                "Path component contains null byte".to_string(),
            ));
        }
    }

    if parts.is_empty() {
        return Err(Error::invalid_vault(
            "No valid path components found".to_string(),
        ));
    }

    Ok(parts)
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
        let roots = vec![std::env::temp_dir()];

        // Empty path
        assert!(validate_file_path("", &roots).is_err());

        // Null byte
        assert!(validate_file_path("test\0file", &roots).is_err());

        // Non-existent path
        assert!(validate_file_path("/nonexistent/file", &roots).is_err());
    }

    #[test]
    fn test_validate_fs_path_components_valid() {
        // Valid simple path
        let result = validate_fs_path_components("dir/file.txt");
        assert!(result.is_ok());
        let parts = result.unwrap();
        assert_eq!(parts, vec!["dir", "file.txt"]);

        // Valid nested path
        let result = validate_fs_path_components("dir1/dir2/dir3/file.dat");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 4);

        // Valid Windows-style path
        let result = validate_fs_path_components("folder\\subfolder\\file.exe");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["folder", "subfolder", "file.exe"]);

        // Valid mixed separators
        let result = validate_fs_path_components("dir1/dir2\\file.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["dir1", "dir2", "file.txt"]);
    }

    #[test]
    fn test_validate_fs_path_components_parent_traversal() {
        // Parent directory traversal
        assert!(validate_fs_path_components("../etc/passwd").is_err());
        assert!(validate_fs_path_components("dir/../file").is_err());
        assert!(validate_fs_path_components("../../root").is_err());
        assert!(validate_fs_path_components("dir\\..\\file").is_err());
    }

    #[test]
    fn test_validate_fs_path_components_absolute_paths() {
        // Unix absolute path
        assert!(validate_fs_path_components("/etc/passwd").is_err());
        assert!(validate_fs_path_components("/absolute/path").is_err());

        // Windows absolute path
        assert!(validate_fs_path_components("\\Windows\\System32").is_err());
    }

    #[test]
    fn test_validate_fs_path_components_current_directory() {
        // Current directory reference
        assert!(validate_fs_path_components("./file").is_err());
        assert!(validate_fs_path_components("dir/./file").is_err());
        assert!(validate_fs_path_components(".").is_err());
    }

    #[test]
    fn test_validate_fs_path_components_edge_cases() {
        // Empty path
        assert!(validate_fs_path_components("").is_err());

        // Null byte
        assert!(validate_fs_path_components("file\0name").is_err());

        // Only separators
        assert!(validate_fs_path_components("///").is_err());
        assert!(validate_fs_path_components("\\\\\\").is_err());

        // Trailing separators (should be handled)
        let result = validate_fs_path_components("dir/file/");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["dir", "file"]);
    }

    #[test]
    fn test_checked_multiply_u32_to_u64_valid() {
        // Valid multiplication
        assert_eq!(
            checked_multiply_u32_to_u64(1000, 512, "test").unwrap(),
            512_000
        );

        // Large but valid multiplication
        assert_eq!(
            checked_multiply_u32_to_u64(65536, 32768, "test").unwrap(),
            2_147_483_648
        );

        // Maximum u32 values that don't overflow
        assert_eq!(
            checked_multiply_u32_to_u64(1, u32::MAX, "test").unwrap(),
            u32::MAX as u64
        );
    }

    #[test]
    fn test_checked_multiply_u32_to_u64_no_overflow() {
        // Note: u32 * u32 will never overflow u64 (max result is (2^32-1)^2 < 2^64-1)
        // This test verifies the maximum possible u32 multiplication
        let result = checked_multiply_u32_to_u64(u32::MAX, u32::MAX, "test");
        assert!(result.is_ok());
        // u32::MAX * u32::MAX = 0xFFFFFFFE00000001
        assert_eq!(result.unwrap(), 18446744065119617025);

        // Zero edge case
        assert_eq!(checked_multiply_u32_to_u64(0, u32::MAX, "test").unwrap(), 0);
        assert_eq!(checked_multiply_u32_to_u64(u32::MAX, 0, "test").unwrap(), 0);
    }

    #[test]
    fn test_validate_partition_index_valid() {
        // Valid indices
        assert!(validate_partition_index(0, 10).is_ok());
        assert!(validate_partition_index(5, 10).is_ok());
        assert!(validate_partition_index(9, 10).is_ok());
    }

    #[test]
    fn test_validate_partition_index_out_of_bounds() {
        // Index equals max (out of bounds)
        assert!(validate_partition_index(10, 10).is_err());

        // Index exceeds max
        assert!(validate_partition_index(15, 10).is_err());

        // Edge case: empty partition table
        assert!(validate_partition_index(0, 0).is_err());
    }

    #[test]
    fn test_validate_allocation_size_edge_cases() {
        // Zero allocation (valid)
        assert!(validate_allocation_size(0, MAX_ALLOCATION_SIZE, "test").is_ok());

        // Exactly at limit (valid)
        assert!(validate_allocation_size(
            MAX_ALLOCATION_SIZE as u64,
            MAX_ALLOCATION_SIZE,
            "test"
        )
        .is_ok());

        // Just over limit (invalid)
        assert!(validate_allocation_size(
            (MAX_ALLOCATION_SIZE as u64) + 1,
            MAX_ALLOCATION_SIZE,
            "test"
        )
        .is_err());

        // Very large allocation (invalid)
        assert!(validate_allocation_size(u64::MAX, MAX_ALLOCATION_SIZE, "test").is_err());
    }

    #[test]
    fn test_checked_multiply_u64_edge_cases() {
        // Zero multiplication
        assert_eq!(checked_multiply_u64(0, 1000, "test").unwrap(), 0);
        assert_eq!(checked_multiply_u64(1000, 0, "test").unwrap(), 0);

        // Multiplication by 1
        assert_eq!(checked_multiply_u64(12345, 1, "test").unwrap(), 12345);

        // Large but valid multiplication
        assert_eq!(
            checked_multiply_u64(1_000_000, 1_000_000, "test").unwrap(),
            1_000_000_000_000
        );

        // Maximum value that doesn't overflow
        assert_eq!(checked_multiply_u64(u64::MAX, 1, "test").unwrap(), u64::MAX);
    }

    #[test]
    fn test_validate_sector_size_all_valid_sizes() {
        // Test all valid power-of-2 sector sizes
        assert!(validate_sector_size(1).is_ok());
        assert!(validate_sector_size(2).is_ok());
        assert!(validate_sector_size(4).is_ok());
        assert!(validate_sector_size(8).is_ok());
        assert!(validate_sector_size(16).is_ok());
        assert!(validate_sector_size(32).is_ok());
        assert!(validate_sector_size(64).is_ok());
        assert!(validate_sector_size(128).is_ok());
        assert!(validate_sector_size(256).is_ok());
        assert!(validate_sector_size(512).is_ok());
        assert!(validate_sector_size(1024).is_ok());
        assert!(validate_sector_size(2048).is_ok());
        assert!(validate_sector_size(4096).is_ok());
    }
}
