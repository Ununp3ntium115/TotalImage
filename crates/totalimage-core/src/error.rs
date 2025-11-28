//! Liberation error types

use thiserror::Error;

/// The main error type for Total Liberation operations
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error during pipeline operations
    #[error("Pipeline I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid vault format or corrupted data
    #[error("Invalid vault format: {0}")]
    InvalidVault(String),

    /// Invalid zone table or partition structure
    #[error("Invalid zone table: {0}")]
    InvalidZoneTable(String),

    /// Invalid territory (file system) structure
    #[error("Invalid territory: {0}")]
    InvalidTerritory(String),

    /// Vault signature verification failed
    #[error("Vault signature verification failed: {0}")]
    SignatureVerification(String),

    /// Checksum verification failed
    #[error("Checksum verification failed: {0}")]
    ChecksumVerification(String),

    /// Unsupported format or feature
    #[error("Unsupported: {0}")]
    Unsupported(String),

    /// File or directory not found in territory
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid path or file name
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Resource already exists
    #[error("Already exists: {0}")]
    AlreadyExists(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Invalid operation or state
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Generic error with custom message
    #[error("{0}")]
    Custom(String),
}

/// Result type alias for Total Liberation operations
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create a custom error from a string
    pub fn custom(msg: impl Into<String>) -> Self {
        Error::Custom(msg.into())
    }

    /// Create an invalid vault error
    pub fn invalid_vault(msg: impl Into<String>) -> Self {
        Error::InvalidVault(msg.into())
    }

    /// Create an invalid zone table error
    pub fn invalid_zone_table(msg: impl Into<String>) -> Self {
        Error::InvalidZoneTable(msg.into())
    }

    /// Create an invalid territory error
    pub fn invalid_territory(msg: impl Into<String>) -> Self {
        Error::InvalidTerritory(msg.into())
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        Error::NotFound(msg.into())
    }

    /// Create an unsupported error
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Error::Unsupported(msg.into())
    }

    /// Create an invalid path error
    pub fn invalid_path(msg: impl Into<String>) -> Self {
        Error::InvalidPath(msg.into())
    }

    /// Create a permission denied error
    pub fn permission_denied(msg: impl Into<String>) -> Self {
        Error::PermissionDenied(msg.into())
    }

    /// Create an invalid operation error
    pub fn invalid_operation(msg: impl Into<String>) -> Self {
        Error::InvalidOperation(msg.into())
    }
}
