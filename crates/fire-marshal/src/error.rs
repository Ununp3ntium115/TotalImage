//! Error types for Fire Marshal

use thiserror::Error;

/// Fire Marshal error type
#[derive(Error, Debug)]
pub enum Error {
    /// Tool not found in registry
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool already registered
    #[error("Tool already registered: {0}")]
    ToolAlreadyRegistered(String),

    /// Invalid tool manifest
    #[error("Invalid tool manifest: {0}")]
    InvalidManifest(String),

    /// Tool execution failed
    #[error("Tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] redb::Error),

    /// Database creation error
    #[error("Database creation error: {0}")]
    DatabaseCreation(#[from] redb::DatabaseError),

    /// Database transaction error
    #[error("Database transaction error: {0}")]
    DatabaseTransaction(#[from] redb::TransactionError),

    /// Database table error
    #[error("Database table error: {0}")]
    DatabaseTable(#[from] redb::TableError),

    /// Database storage error
    #[error("Database storage error: {0}")]
    DatabaseStorage(#[from] redb::StorageError),

    /// Database commit error
    #[error("Database commit error: {0}")]
    DatabaseCommit(#[from] redb::CommitError),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Rate limited
    #[error("Rate limited: too many requests")]
    RateLimited,

    /// Timeout
    #[error("Request timed out")]
    Timeout,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

/// Result type alias for Fire Marshal
pub type Result<T> = std::result::Result<T, Error>;
