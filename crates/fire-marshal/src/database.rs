//! Shared redb database for cross-tool caching
//!
//! Provides unified caching across all PYRO Platform tools with:
//! - TTL-based expiration
//! - Tool version tracking for cache invalidation
//! - Multiple table support

use crate::{Error, Result};
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// Table definitions
const TOOL_REGISTRY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tool_registry");
const EXECUTION_LOG_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("execution_log");
const CACHE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("cache");

/// Cache entry with metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CacheEntry<T> {
    /// The cached data
    pub data: T,
    /// Unix timestamp when entry was created
    pub created_at: u64,
    /// Which tool created this entry
    pub tool: String,
    /// Tool version for cache invalidation
    pub version: String,
}

impl<T: Serialize> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(data: T, tool: &str, version: &str) -> Self {
        Self {
            data,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            tool: tool.to_string(),
            version: version.to_string(),
        }
    }
}

impl<T> CacheEntry<T> {
    /// Check if entry is expired
    pub fn is_expired(&self, ttl_seconds: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.created_at) > ttl_seconds
    }
}

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// TTL for cache entries in seconds (default: 30 days)
    pub ttl_seconds: u64,
    /// Maximum cache size in bytes (default: 100 MB)
    pub max_size_bytes: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 30 * 24 * 60 * 60, // 30 days
            max_size_bytes: 100 * 1024 * 1024, // 100 MB
        }
    }
}

/// Platform database for shared caching
pub struct PlatformDatabase {
    db: Arc<Mutex<Database>>,
    config: DatabaseConfig,
}

impl PlatformDatabase {
    /// Create or open database at given path
    pub fn new(db_path: &Path, config: DatabaseConfig) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(db_path)?;

        // Initialize tables
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(TOOL_REGISTRY_TABLE)?;
            let _ = write_txn.open_table(EXECUTION_LOG_TABLE)?;
            let _ = write_txn.open_table(CACHE_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            config,
        })
    }

    /// Store a value in the cache
    pub fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        tool: &str,
        version: &str,
    ) -> Result<()> {
        let entry = CacheEntry::new(value, tool, version);
        let encoded = bincode::serialize(&entry)?;

        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(CACHE_TABLE)?;
            table.insert(key, encoded.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Get a value from the cache
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let expired = {
            let db = self.db.lock().map_err(|_| {
                Error::Database(redb::Error::Io(std::io::Error::other(
                    "Lock poisoned",
                )))
            })?;
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(CACHE_TABLE)?;

            match table.get(key)? {
                Some(value) => {
                    let entry: CacheEntry<T> = bincode::deserialize(value.value())?;

                    // Check expiration
                    if entry.is_expired(self.config.ttl_seconds) {
                        true // Signal to remove
                    } else {
                        return Ok(Some(entry.data));
                    }
                }
                None => return Ok(None), // Key not found
            }
        }; // Mutex released here

        // Handle expired entry removal outside the lock
        if expired {
            self.remove(key)?;
        }

        Ok(None)
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &str) -> Result<()> {
        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(CACHE_TABLE)?;
            let _ = table.remove(key)?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Register a tool in the database
    pub fn register_tool(&self, tool_info: &crate::ToolInfo) -> Result<()> {
        let encoded = bincode::serialize(tool_info)?;

        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(TOOL_REGISTRY_TABLE)?;
            table.insert(tool_info.name.as_str(), encoded.as_slice())?;
        }
        write_txn.commit()?;

        tracing::info!("Registered tool in database: {}", tool_info.name);
        Ok(())
    }

    /// Get all registered tools from database
    pub fn get_registered_tools(&self) -> Result<Vec<crate::ToolInfo>> {
        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(TOOL_REGISTRY_TABLE)?;

        let mut tools = Vec::new();
        for result in table.iter()? {
            let (_, value) = result?;
            let tool_info: crate::ToolInfo = bincode::deserialize(value.value())?;
            tools.push(tool_info);
        }

        Ok(tools)
    }

    /// Log a tool execution
    pub fn log_execution(
        &self,
        tool_name: &str,
        method: &str,
        success: bool,
        duration_ms: u64,
    ) -> Result<()> {
        let log_entry = ExecutionLog {
            tool_name: tool_name.to_string(),
            method: method.to_string(),
            success,
            duration_ms,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let key = format!(
            "{}:{}:{}",
            log_entry.timestamp, tool_name, method
        );
        let encoded = bincode::serialize(&log_entry)?;

        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(EXECUTION_LOG_TABLE)?;
            table.insert(key.as_str(), encoded.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<DatabaseStats> {
        let db = self.db.lock().map_err(|_| {
            Error::Database(redb::Error::Io(std::io::Error::other(
                "Lock poisoned",
            )))
        })?;
        let read_txn = db.begin_read()?;

        let tool_count = read_txn.open_table(TOOL_REGISTRY_TABLE)?.len()?;
        let cache_count = read_txn.open_table(CACHE_TABLE)?.len()?;
        let log_count = read_txn.open_table(EXECUTION_LOG_TABLE)?.len()?;

        Ok(DatabaseStats {
            registered_tools: tool_count,
            cache_entries: cache_count,
            execution_logs: log_count,
        })
    }
}

/// Execution log entry
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionLog {
    pub tool_name: String,
    pub method: String,
    pub success: bool,
    pub duration_ms: u64,
    pub timestamp: u64,
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub registered_tools: u64,
    pub cache_entries: u64,
    pub execution_logs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_set_get() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let db = PlatformDatabase::new(&db_path, DatabaseConfig::default()).unwrap();

        db.set("test_key", &"test_value".to_string(), "test_tool", "1.0")
            .unwrap();

        let result: Option<String> = db.get("test_key").unwrap();
        assert_eq!(result, Some("test_value".to_string()));
    }

    #[test]
    fn test_cache_remove() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let db = PlatformDatabase::new(&db_path, DatabaseConfig::default()).unwrap();

        db.set("test_key", &"test_value".to_string(), "test_tool", "1.0")
            .unwrap();
        db.remove("test_key").unwrap();

        let result: Option<String> = db.get("test_key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_cache_expiration() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        let config = DatabaseConfig {
            ttl_seconds: 0, // Immediate expiration
            ..Default::default()
        };
        let db = PlatformDatabase::new(&db_path, config).unwrap();

        db.set("test_key", &"test_value".to_string(), "test_tool", "1.0")
            .unwrap();

        // Wait for expiration (TTL=0 means expires after 1 second)
        std::thread::sleep(std::time::Duration::from_millis(1100));

        let result: Option<String> = db.get("test_key").unwrap();
        assert_eq!(result, None);
    }
}
