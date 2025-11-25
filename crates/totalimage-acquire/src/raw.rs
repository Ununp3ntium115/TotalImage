//! Raw disk image acquisition (dd equivalent)
//!
//! Creates raw sector-by-sector copies of disks or partitions.

use crate::error::{AcquireError, Result};
use crate::hash::{HashAlgorithm, HashResult, Hasher};
use crate::progress::{AcquireProgress, ProgressCallback};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Options for raw acquisition
#[derive(Debug, Clone)]
pub struct AcquireOptions {
    /// Block size for I/O operations (default: 64KB)
    pub block_size: usize,
    /// Hash algorithms to compute during acquisition
    pub hash_algorithms: Vec<HashAlgorithm>,
    /// Skip bad blocks instead of failing
    pub skip_bad_blocks: bool,
    /// Verify after acquisition by re-reading
    pub verify_after: bool,
    /// Sync after each write
    pub sync_writes: bool,
    /// Number of bytes to acquire (None = entire source)
    pub count: Option<u64>,
    /// Offset to start reading from
    pub skip: u64,
}

impl Default for AcquireOptions {
    fn default() -> Self {
        Self {
            block_size: 64 * 1024, // 64KB
            hash_algorithms: vec![HashAlgorithm::Md5, HashAlgorithm::Sha256],
            skip_bad_blocks: false,
            verify_after: true,
            sync_writes: false,
            count: None,
            skip: 0,
        }
    }
}

/// Result of a raw acquisition operation
#[derive(Debug)]
pub struct AcquireResult {
    /// Total bytes acquired
    pub bytes_acquired: u64,
    /// Hash results for the acquired data
    pub hashes: Vec<HashResult>,
    /// Time elapsed
    pub elapsed: std::time::Duration,
    /// Average transfer rate in bytes/second
    pub bytes_per_second: f64,
    /// Number of bad blocks encountered
    pub bad_blocks: u64,
    /// Verification passed (if verify_after was enabled)
    pub verified: Option<bool>,
}

/// Raw disk image acquirer
pub struct RawAcquirer {
    options: AcquireOptions,
    cancel_flag: Arc<AtomicBool>,
}

impl RawAcquirer {
    /// Create a new acquirer with default options
    pub fn new() -> Self {
        Self {
            options: AcquireOptions::default(),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create with custom options
    pub fn with_options(options: AcquireOptions) -> Self {
        Self {
            options,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a cancel flag that can be used to cancel the operation
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }

    /// Acquire from a file/device to a raw image file
    pub fn acquire_to_file(
        &self,
        source_path: &Path,
        dest_path: &Path,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<AcquireResult> {
        // Open source
        let mut source = File::open(source_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                AcquireError::SourceNotFound(source_path.display().to_string())
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                AcquireError::PermissionDenied(source_path.display().to_string())
            } else {
                AcquireError::IoError(e)
            }
        })?;

        // Get source size
        let source_size = source.seek(SeekFrom::End(0))?;
        source.seek(SeekFrom::Start(self.options.skip))?;

        // Calculate total bytes to acquire
        let total_bytes = if let Some(count) = self.options.count {
            count.min(source_size.saturating_sub(self.options.skip))
        } else {
            source_size.saturating_sub(self.options.skip)
        };

        // Create destination file
        let mut dest = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(dest_path)
            .map_err(|e| AcquireError::DestinationError(e.to_string()))?;

        // Perform acquisition
        let result = self.acquire_stream(&mut source, &mut dest, Some(total_bytes), progress_callback)?;

        // Verify if requested
        let verified = if self.options.verify_after && !result.hashes.is_empty() {
            let verify_result = self.verify_file(dest_path, &result.hashes)?;
            Some(verify_result)
        } else {
            None
        };

        Ok(AcquireResult {
            bytes_acquired: result.bytes_acquired,
            hashes: result.hashes,
            elapsed: result.elapsed,
            bytes_per_second: result.bytes_per_second,
            bad_blocks: result.bad_blocks,
            verified,
        })
    }

    /// Acquire from any reader to any writer
    pub fn acquire_stream<R: Read, W: Write>(
        &self,
        source: &mut R,
        dest: &mut W,
        total_bytes: Option<u64>,
        progress_callback: Option<ProgressCallback>,
    ) -> Result<AcquireResult> {
        let start_time = Instant::now();
        let mut hasher = Hasher::new(&self.options.hash_algorithms);
        let mut buffer = vec![0u8; self.options.block_size];
        let mut bytes_acquired: u64 = 0;
        let mut bad_blocks: u64 = 0;
        let remaining = total_bytes;

        loop {
            // Check for cancellation
            if self.cancel_flag.load(Ordering::Relaxed) {
                return Err(AcquireError::Cancelled);
            }

            // Calculate how much to read
            let to_read = if let Some(remaining) = remaining {
                let left = remaining.saturating_sub(bytes_acquired);
                if left == 0 {
                    break;
                }
                (left as usize).min(buffer.len())
            } else {
                buffer.len()
            };

            // Read from source
            let bytes_read = match source.read(&mut buffer[..to_read]) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(e) => {
                    if self.options.skip_bad_blocks {
                        bad_blocks += 1;
                        // Fill with zeros for bad block
                        buffer[..to_read].fill(0);
                        to_read
                    } else {
                        return Err(AcquireError::ReadError(e.to_string()));
                    }
                }
            };

            // Update hash
            hasher.update(&buffer[..bytes_read]);

            // Write to destination
            dest.write_all(&buffer[..bytes_read])
                .map_err(|e| AcquireError::WriteError(e.to_string()))?;

            if self.options.sync_writes {
                dest.flush().map_err(|e| AcquireError::WriteError(e.to_string()))?;
            }

            bytes_acquired += bytes_read as u64;

            // Report progress
            if let Some(ref callback) = progress_callback {
                let progress = AcquireProgress::calculate(
                    total_bytes,
                    bytes_acquired,
                    start_time,
                    "Acquiring",
                );
                callback(&progress);
            }
        }

        // Final flush
        dest.flush().map_err(|e| AcquireError::WriteError(e.to_string()))?;

        let elapsed = start_time.elapsed();
        let bytes_per_second = if elapsed.as_secs_f64() > 0.0 {
            bytes_acquired as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        Ok(AcquireResult {
            bytes_acquired,
            hashes: hasher.finalize(),
            elapsed,
            bytes_per_second,
            bad_blocks,
            verified: None,
        })
    }

    /// Verify a file against expected hashes
    pub fn verify_file(&self, path: &Path, expected_hashes: &[HashResult]) -> Result<bool> {
        let mut file = File::open(path)?;
        let algorithms: Vec<_> = expected_hashes.iter().map(|h| h.algorithm).collect();

        let actual_hashes = crate::hash::hash_reader(&mut file, &algorithms)?;

        for expected in expected_hashes {
            let actual = actual_hashes.iter().find(|h| h.algorithm == expected.algorithm);
            if let Some(actual) = actual {
                if !actual.matches(expected) {
                    return Err(AcquireError::HashMismatch {
                        expected: expected.hex.clone(),
                        actual: actual.hex.clone(),
                    });
                }
            }
        }

        Ok(true)
    }
}

impl Default for RawAcquirer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;

    #[test]
    fn test_acquire_stream() {
        let source_data = b"Hello, World! This is test data for acquisition.";
        let mut source = Cursor::new(source_data);
        let mut dest = Vec::new();

        let acquirer = RawAcquirer::new();
        let result = acquirer.acquire_stream(
            &mut source,
            &mut dest,
            Some(source_data.len() as u64),
            None,
        ).unwrap();

        assert_eq!(result.bytes_acquired, source_data.len() as u64);
        assert_eq!(dest, source_data);
        assert!(!result.hashes.is_empty());
    }

    #[test]
    fn test_acquire_to_file() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source.bin");
        let dest_path = dir.path().join("dest.img");

        // Create source file
        let source_data = vec![0xABu8; 1024];
        std::fs::write(&source_path, &source_data).unwrap();

        // Acquire
        let acquirer = RawAcquirer::with_options(AcquireOptions {
            verify_after: true,
            ..Default::default()
        });

        let result = acquirer.acquire_to_file(&source_path, &dest_path, None).unwrap();

        assert_eq!(result.bytes_acquired, 1024);
        assert_eq!(result.verified, Some(true));

        // Verify destination content
        let dest_data = std::fs::read(&dest_path).unwrap();
        assert_eq!(dest_data, source_data);
    }

    #[test]
    fn test_acquire_partial() {
        let dir = tempdir().unwrap();
        let source_path = dir.path().join("source.bin");
        let dest_path = dir.path().join("dest.img");

        // Create source file
        std::fs::write(&source_path, vec![0u8; 1000]).unwrap();

        // Acquire only first 500 bytes
        let acquirer = RawAcquirer::with_options(AcquireOptions {
            count: Some(500),
            verify_after: false,
            ..Default::default()
        });

        let result = acquirer.acquire_to_file(&source_path, &dest_path, None).unwrap();

        assert_eq!(result.bytes_acquired, 500);
    }

    #[test]
    fn test_cancel_acquisition() {
        let source_data = vec![0u8; 1024 * 1024]; // 1MB
        let mut source = Cursor::new(&source_data);
        let mut dest = Vec::new();

        let acquirer = RawAcquirer::with_options(AcquireOptions {
            block_size: 1024,
            ..Default::default()
        });

        // Set cancel flag
        acquirer.cancel_flag().store(true, Ordering::Relaxed);

        let result = acquirer.acquire_stream(
            &mut source,
            &mut dest,
            Some(source_data.len() as u64),
            None,
        );

        assert!(matches!(result, Err(AcquireError::Cancelled)));
    }
}
