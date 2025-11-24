//! Tool cache for MCP server
//!
//! Provides caching functionality for tool results using redb database.
//! Shared with totalimage-web for consistent caching behavior.

use anyhow::Result;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const TOOL_RESULTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tool_results");
const CACHE_TTL_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days

/// Cache entry with metadata
#[derive(serde::Serialize, serde::Deserialize)]
struct CacheEntry<T> {
    data: T,
    created_at: u64,
    tool: String,
    version: String,
}

impl<T> CacheEntry<T> {
    fn new(data: T, tool: &str, version: &str) -> Self {
        Self {
            data,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            tool: tool.to_string(),
            version: version.to_string(),
        }
    }

    fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now - self.created_at > CACHE_TTL_SECONDS
    }
}

/// Tool cache for MCP server results
pub struct ToolCache {
    db: Arc<Mutex<Database>>,
    tool_name: String,
    version: String,
}

impl ToolCache {
    /// Create a new tool cache
    pub fn new(cache_path: PathBuf, tool_name: impl Into<String>, version: impl Into<String>) -> Result<Self> {
        std::fs::create_dir_all(cache_path.parent().unwrap_or(cache_path.as_path()))?;

        let db = Database::create(&cache_path)?;

        // Initialize table
        let write_txn = db.begin_write()?;
        {
            write_txn.open_table(TOOL_RESULTS_TABLE)?;
        }
        write_txn.commit()?;

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
            tool_name: tool_name.into(),
            version: version.into(),
        })
    }

    /// Get a cached result
    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(TOOL_RESULTS_TABLE)?;

        match table.get(key)? {
            Some(value) => {
                let entry: CacheEntry<T> = bincode::deserialize(value.value())?;

                // Check expiration
                if entry.is_expired() {
                    drop(read_txn);
                    // Entry expired, remove it
                    let write_txn = db.begin_write()?;
                    {
                        let mut table = write_txn.open_table(TOOL_RESULTS_TABLE)?;
                        table.remove(key)?;
                    }
                    write_txn.commit()?;
                    return Ok(None);
                }

                Ok(Some(entry.data))
            }
            None => Ok(None),
        }
    }

    /// Set a cached result
    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let entry = CacheEntry::new(value, &self.tool_name, &self.version);
        let encoded = bincode::serialize(&entry)?;

        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(TOOL_RESULTS_TABLE)?;
            table.insert(key, encoded.as_slice())?;
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Clear all cached results
    pub fn clear(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(TOOL_RESULTS_TABLE)?;
            // Remove all entries
            let keys: Vec<String> = table
                .iter()?
                .filter_map(|r| r.ok())
                .map(|(k, _)| k.value().to_string())
                .collect();
            for key in keys {
                table.remove(key.as_str())?;
            }
        }
        write_txn.commit()?;

        Ok(())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats> {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(TOOL_RESULTS_TABLE)?;

        let entry_count = table.len()?;

        // Estimate size based on entry count (average ~10KB per entry)
        let size_bytes = entry_count * 10_000;

        Ok(CacheStats {
            entry_count,
            size_bytes,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStats {
    pub entry_count: u64,
    pub size_bytes: u64,
}
