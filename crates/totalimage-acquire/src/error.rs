//! Error types for disk acquisition

use thiserror::Error;

/// Result type for acquisition operations
pub type Result<T> = std::result::Result<T, AcquireError>;

/// Errors that can occur during disk acquisition
#[derive(Error, Debug)]
pub enum AcquireError {
    /// Source device/file not found
    #[error("Source not found: {0}")]
    SourceNotFound(String),

    /// Destination cannot be created
    #[error("Cannot create destination: {0}")]
    DestinationError(String),

    /// Read error from source
    #[error("Read error: {0}")]
    ReadError(String),

    /// Write error to destination
    #[error("Write error: {0}")]
    WriteError(String),

    /// Hash verification failed
    #[error("Hash verification failed: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },

    /// Invalid block size
    #[error("Invalid block size: {0}")]
    InvalidBlockSize(usize),

    /// Acquisition was cancelled
    #[error("Acquisition cancelled")]
    Cancelled,

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Device busy
    #[error("Device busy: {0}")]
    DeviceBusy(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Size mismatch
    #[error("Size mismatch: expected {expected} bytes, got {actual} bytes")]
    SizeMismatch { expected: u64, actual: u64 },

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}
