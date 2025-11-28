//! Metadata caching layer using redb
//!
//! Provides persistent caching of vault metadata with TTL-based expiration
//! and LRU eviction to prevent unbounded growth.

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// TTL for cache entries (30 days)
const CACHE_TTL_SECS: u64 = 30 * 24 * 60 * 60;

/// Maximum cache size in bytes (100 MB)
const MAX_CACHE_SIZE: u64 = 100 * 1024 * 1024;

/// Cache maintenance interval (1 hour)
const MAINTENANCE_INTERVAL_SECS: u64 = 60 * 60;

/// Table definitions
const VAULT_INFO_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("vault_info");
const ZONE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("zone_tables");
const DIR_LISTINGS: TableDefinition<&str, &[u8]> = TableDefinition::new("dir_listings");

/// Cached entry with timestamp
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CacheEntry<T> {
    timestamp: u64,
    data: T,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        Self { timestamp, data }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs();
        now - self.timestamp > CACHE_TTL_SECS
    }
}

/// Thread-safe metadata cache
pub struct MetadataCache {
    db: Arc<Mutex<Database>>,
}

impl MetadataCache {
    /// Create a new metadata cache at the specified path
    pub fn new(cache_path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        // Ensure parent directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = Database::create(&cache_path)?;

        // Initialize tables
        {
            let write_txn = db.begin_write()?;
            {
                let _ = write_txn.open_table(VAULT_INFO_TABLE)?;
                let _ = write_txn.open_table(ZONE_TABLE)?;
                let _ = write_txn.open_table(DIR_LISTINGS)?;
            }
            write_txn.commit()?;
        }

        Ok(Self {
            db: Arc::new(Mutex::new(db)),
        })
    }

    /// Get vault info from cache
    pub fn get_vault_info<T>(&self, path: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(VAULT_INFO_TABLE)?;

        if let Some(value) = table.get(path)? {
            let entry: CacheEntry<T> = bincode::deserialize(value.value())?;
            if !entry.is_expired() {
                tracing::debug!("Cache HIT for vault_info: {}", path);
                return Ok(Some(entry.data));
            } else {
                tracing::debug!("Cache EXPIRED for vault_info: {}", path);
            }
        } else {
            tracing::debug!("Cache MISS for vault_info: {}", path);
        }

        Ok(None)
    }

    /// Set vault info in cache
    pub fn set_vault_info<T>(&self, path: &str, info: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
    {
        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(VAULT_INFO_TABLE)?;
            let entry = CacheEntry::new(info);
            let encoded = bincode::serialize(&entry)?;
            table.insert(path, encoded.as_slice())?;
        }
        write_txn.commit()?;

        tracing::debug!("Cached vault_info: {}", path);

        Ok(())
    }

    /// Get zone table from cache
    pub fn get_zones<T>(&self, path: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(ZONE_TABLE)?;

        if let Some(value) = table.get(path)? {
            let entry: CacheEntry<T> = bincode::deserialize(value.value())?;
            if !entry.is_expired() {
                tracing::debug!("Cache HIT for zones: {}", path);
                return Ok(Some(entry.data));
            } else {
                tracing::debug!("Cache EXPIRED for zones: {}", path);
            }
        } else {
            tracing::debug!("Cache MISS for zones: {}", path);
        }

        Ok(None)
    }

    /// Set zone table in cache
    pub fn set_zones<T>(&self, path: &str, zones: &T) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
    {
        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(ZONE_TABLE)?;
            let entry = CacheEntry::new(zones);
            let encoded = bincode::serialize(&entry)?;
            table.insert(path, encoded.as_slice())?;
        }
        write_txn.commit()?;

        tracing::debug!("Cached zones: {}", path);

        Ok(())
    }

    /// Get directory listing from cache
    #[allow(dead_code)] // Reserved for future directory caching feature
    pub fn get_dir_listing<T>(&self, path: &str) -> Result<Option<T>, Box<dyn std::error::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(DIR_LISTINGS)?;

        if let Some(value) = table.get(path)? {
            let entry: CacheEntry<T> = bincode::deserialize(value.value())?;
            if !entry.is_expired() {
                tracing::debug!("Cache HIT for directory: {}", path);
                return Ok(Some(entry.data));
            } else {
                tracing::debug!("Cache EXPIRED for directory: {}", path);
            }
        } else {
            tracing::debug!("Cache MISS for directory: {}", path);
        }

        Ok(None)
    }

    /// Set directory listing in cache
    #[allow(dead_code)] // Reserved for future directory caching feature
    pub fn set_dir_listing<T>(
        &self,
        path: &str,
        listing: &T,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: Serialize,
    {
        let db = self.db.lock().unwrap();
        let write_txn = db.begin_write()?;
        {
            let mut table = write_txn.open_table(DIR_LISTINGS)?;
            let entry = CacheEntry::new(listing);
            let encoded = bincode::serialize(&entry)?;
            table.insert(path, encoded.as_slice())?;
        }
        write_txn.commit()?;

        tracing::debug!("Cached directory listing: {}", path);

        Ok(())
    }

    /// Clean up expired entries from all tables
    pub fn cleanup_expired(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let db = self.db.lock().unwrap();
        let mut removed_count = 0;

        // Clean vault_info table
        removed_count += self.cleanup_table(&db, VAULT_INFO_TABLE)?;

        // Clean zone_table
        removed_count += self.cleanup_table(&db, ZONE_TABLE)?;

        // Clean dir_listings table
        removed_count += self.cleanup_table(&db, DIR_LISTINGS)?;

        if removed_count > 0 {
            tracing::info!("Cleaned up {} expired cache entries", removed_count);
        }

        Ok(removed_count)
    }

    /// Helper to clean up a specific table
    fn cleanup_table(
        &self,
        db: &Database,
        table_def: TableDefinition<&str, &[u8]>,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        let mut removed_count = 0;
        let mut keys_to_remove = Vec::new();

        // First, collect expired keys
        {
            let read_txn = db.begin_read()?;
            let table = read_txn.open_table(table_def)?;

            for entry in table.iter()? {
                let (key, value) = entry?;
                // Extract timestamp from raw bytes (first 8 bytes)
                let bytes = value.value();
                if bytes.len() >= 8 {
                    let timestamp = u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                        bytes[4], bytes[5], bytes[6], bytes[7],
                    ]);
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or(Duration::ZERO)
                        .as_secs();
                    if now - timestamp > CACHE_TTL_SECS {
                        keys_to_remove.push(key.value().to_string());
                    }
                }
            }
        }

        // Then remove them
        if !keys_to_remove.is_empty() {
            let write_txn = db.begin_write()?;
            {
                let mut table = write_txn.open_table(table_def)?;
                for key in &keys_to_remove {
                    table.remove(key.as_str())?;
                    removed_count += 1;
                }
            }
            write_txn.commit()?;
        }

        Ok(removed_count)
    }

    /// Get approximate cache size (requires database to be already locked)
    fn cache_size_with_db(&self, db: &Database) -> Result<u64, Box<dyn std::error::Error>> {
        // Get the database file size as a proxy for cache size
        let read_txn = db.begin_read()?;

        // Sum up the stored bytes from all tables
        let mut total_bytes = 0u64;

        // Estimate based on number of entries
        let vault_table = read_txn.open_table(VAULT_INFO_TABLE)?;
        total_bytes += vault_table.len()? * 1024; // Estimate 1KB per entry

        let zone_table = read_txn.open_table(ZONE_TABLE)?;
        total_bytes += zone_table.len()? * 2048; // Estimate 2KB per entry

        let dir_table = read_txn.open_table(DIR_LISTINGS)?;
        total_bytes += dir_table.len()? * 512; // Estimate 512B per entry

        Ok(total_bytes)
    }

    /// Evict oldest entries if cache is too large (LRU eviction)
    pub fn evict_if_needed(&self) -> Result<(), Box<dyn std::error::Error>> {
        let db = self.db.lock().unwrap();
        let size = self.cache_size_with_db(&db)?;
        drop(db); // Release lock before evicting

        if size > MAX_CACHE_SIZE {
            tracing::warn!(
                "Cache size ({} bytes) exceeds limit ({} bytes), performing LRU eviction",
                size,
                MAX_CACHE_SIZE
            );

            // Simple LRU: remove oldest 10% of entries
            self.evict_oldest(0.1)?;
        }

        Ok(())
    }

    /// Evict oldest entries by percentage
    fn evict_oldest(&self, percentage: f64) -> Result<usize, Box<dyn std::error::Error>> {
        let db = self.db.lock().unwrap();
        let mut all_entries = Vec::new();

        // Collect all entries with timestamps
        {
            let read_txn = db.begin_read()?;

            // Collect from vault_info
            let table = read_txn.open_table(VAULT_INFO_TABLE)?;
            for entry in table.iter()? {
                let (key, value) = entry?;
                // Extract timestamp from raw bytes (first 8 bytes are the u64 timestamp)
                let bytes = value.value();
                if bytes.len() >= 8 {
                    let timestamp = u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                        bytes[4], bytes[5], bytes[6], bytes[7],
                    ]);
                    all_entries.push((
                        "vault_info".to_string(),
                        key.value().to_string(),
                        timestamp,
                    ));
                }
            }

            // Collect from zone_table
            let table = read_txn.open_table(ZONE_TABLE)?;
            for entry in table.iter()? {
                let (key, value) = entry?;
                let bytes = value.value();
                if bytes.len() >= 8 {
                    let timestamp = u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                        bytes[4], bytes[5], bytes[6], bytes[7],
                    ]);
                    all_entries.push((
                        "zone_table".to_string(),
                        key.value().to_string(),
                        timestamp,
                    ));
                }
            }

            // Collect from dir_listings
            let table = read_txn.open_table(DIR_LISTINGS)?;
            for entry in table.iter()? {
                let (key, value) = entry?;
                let bytes = value.value();
                if bytes.len() >= 8 {
                    let timestamp = u64::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                        bytes[4], bytes[5], bytes[6], bytes[7],
                    ]);
                    all_entries.push((
                        "dir_listings".to_string(),
                        key.value().to_string(),
                        timestamp,
                    ));
                }
            }
        }

        // Sort by timestamp (oldest first)
        all_entries.sort_by_key(|(_, _, ts)| *ts);

        // Calculate how many to remove
        let to_remove = ((all_entries.len() as f64) * percentage).ceil() as usize;
        let entries_to_remove: Vec<_> = all_entries.into_iter().take(to_remove).collect();

        // Remove entries
        if !entries_to_remove.is_empty() {
            let write_txn = db.begin_write()?;
            {
                let mut vault_table = write_txn.open_table(VAULT_INFO_TABLE)?;
                let mut zone_table = write_txn.open_table(ZONE_TABLE)?;
                let mut dir_table = write_txn.open_table(DIR_LISTINGS)?;

                for (table_name, key, _) in &entries_to_remove {
                    match table_name.as_str() {
                        "vault_info" => { vault_table.remove(key.as_str())?; }
                        "zone_table" => { zone_table.remove(key.as_str())?; }
                        "dir_listings" => { dir_table.remove(key.as_str())?; }
                        _ => {}
                    }
                }
            }
            write_txn.commit()?;

            tracing::info!("Evicted {} oldest cache entries", entries_to_remove.len());
        }

        Ok(entries_to_remove.len())
    }

    /// Get cache statistics
    pub fn stats(&self) -> Result<CacheStats, Box<dyn std::error::Error>> {
        let db = self.db.lock().unwrap();
        let read_txn = db.begin_read()?;

        let vault_table = read_txn.open_table(VAULT_INFO_TABLE)?;
        let zone_table = read_txn.open_table(ZONE_TABLE)?;
        let dir_table = read_txn.open_table(DIR_LISTINGS)?;

        let estimated_size_bytes = self.cache_size_with_db(&db)?;

        Ok(CacheStats {
            vault_info_count: vault_table.len()?,
            zone_table_count: zone_table.len()?,
            dir_listings_count: dir_table.len()?,
            estimated_size_bytes,
        })
    }

    /// Spawn a background task for automatic cache maintenance
    ///
    /// This task runs periodically to:
    /// - Clean up expired entries
    /// - Evict oldest entries if cache exceeds size limit
    ///
    /// The task runs every MAINTENANCE_INTERVAL_SECS (1 hour by default)
    pub fn spawn_maintenance_task(cache: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(MAINTENANCE_INTERVAL_SECS));

            loop {
                interval.tick().await;

                tracing::debug!("Running cache maintenance...");

                // Clean expired entries
                match cache.cleanup_expired() {
                    Ok(removed) => {
                        if removed > 0 {
                            tracing::info!("Cache maintenance: removed {} expired entries", removed);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Cache cleanup failed: {}", e);
                    }
                }

                // Check size and evict if needed
                if let Err(e) = cache.evict_if_needed() {
                    tracing::warn!("Cache eviction check failed: {}", e);
                }
            }
        });
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub vault_info_count: u64,
    pub zone_table_count: u64,
    pub dir_listings_count: u64,
    pub estimated_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct TestData {
        name: String,
        value: u64,
    }

    fn create_test_cache() -> (MetadataCache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().join("test_cache.redb");
        let cache = MetadataCache::new(cache_path).unwrap();
        (cache, temp_dir)
    }

    #[test]
    fn test_vault_info_cache() {
        let (cache, _temp) = create_test_cache();

        let test_data = TestData {
            name: "test_vault".to_string(),
            value: 12345,
        };

        // Initially empty
        assert!(cache.get_vault_info::<TestData>("test.img").unwrap().is_none());

        // Set data
        cache.set_vault_info("test.img", &test_data).unwrap();

        // Retrieve data
        let retrieved: TestData = cache.get_vault_info("test.img").unwrap().unwrap();
        assert_eq!(retrieved, test_data);
    }

    #[test]
    fn test_zones_cache() {
        let (cache, _temp) = create_test_cache();

        let test_data = TestData {
            name: "test_zones".to_string(),
            value: 67890,
        };

        // Initially empty
        assert!(cache.get_zones::<TestData>("test.img").unwrap().is_none());

        // Set data
        cache.set_zones("test.img", &test_data).unwrap();

        // Retrieve data
        let retrieved: TestData = cache.get_zones("test.img").unwrap().unwrap();
        assert_eq!(retrieved, test_data);
    }

    #[test]
    fn test_dir_listing_cache() {
        let (cache, _temp) = create_test_cache();

        let test_data = TestData {
            name: "test_dir".to_string(),
            value: 11111,
        };

        // Initially empty
        assert!(cache.get_dir_listing::<TestData>("/test/path").unwrap().is_none());

        // Set data
        cache.set_dir_listing("/test/path", &test_data).unwrap();

        // Retrieve data
        let retrieved: TestData = cache.get_dir_listing("/test/path").unwrap().unwrap();
        assert_eq!(retrieved, test_data);
    }

    #[test]
    fn test_multiple_entries() {
        let (cache, _temp) = create_test_cache();

        // Add multiple entries
        for i in 0..10 {
            let data = TestData {
                name: format!("vault_{}", i),
                value: i as u64,
            };
            cache.set_vault_info(&format!("vault_{}.img", i), &data).unwrap();
        }

        // Retrieve and verify
        for i in 0..10 {
            let retrieved: TestData = cache
                .get_vault_info(&format!("vault_{}.img", i))
                .unwrap()
                .unwrap();
            assert_eq!(retrieved.name, format!("vault_{}", i));
            assert_eq!(retrieved.value, i as u64);
        }
    }

    #[test]
    fn test_cache_stats() {
        let (cache, _temp) = create_test_cache();

        // Add some entries
        let data = TestData {
            name: "test".to_string(),
            value: 123,
        };

        cache.set_vault_info("test1.img", &data).unwrap();
        cache.set_zones("test2.img", &data).unwrap();
        cache.set_dir_listing("/test/dir", &data).unwrap();

        let stats = cache.stats().unwrap();
        assert_eq!(stats.vault_info_count, 1);
        assert_eq!(stats.zone_table_count, 1);
        assert_eq!(stats.dir_listings_count, 1);
        assert!(stats.estimated_size_bytes > 0);
    }

    #[test]
    fn test_cleanup_expired() {
        let (cache, _temp) = create_test_cache();

        let data = TestData {
            name: "test".to_string(),
            value: 123,
        };

        cache.set_vault_info("test.img", &data).unwrap();

        // Cleanup shouldn't remove non-expired entries
        let removed = cache.cleanup_expired().unwrap();
        assert_eq!(removed, 0);

        // Entry should still be there
        assert!(cache.get_vault_info::<TestData>("test.img").unwrap().is_some());
    }

    #[test]
    fn test_entry_expiration() {
        // Test that entries with old timestamps are considered expired
        let old_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - (CACHE_TTL_SECS + 1000);

        let entry = CacheEntry {
            timestamp: old_timestamp,
            data: TestData {
                name: "test".to_string(),
                value: 123,
            },
        };

        assert!(entry.is_expired());
    }

    #[test]
    fn test_evict_oldest() {
        let (cache, _temp) = create_test_cache();

        // Add multiple entries
        for i in 0..20 {
            let data = TestData {
                name: format!("vault_{}", i),
                value: i as u64,
            };
            cache.set_vault_info(&format!("vault_{}.img", i), &data).unwrap();

            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Evict 10% (2 entries)
        let evicted = cache.evict_oldest(0.1).unwrap();
        assert_eq!(evicted, 2);

        // Oldest entries should be gone
        assert!(cache.get_vault_info::<TestData>("vault_0.img").unwrap().is_none());
        assert!(cache.get_vault_info::<TestData>("vault_1.img").unwrap().is_none());

        // Newer entries should still be present
        assert!(cache.get_vault_info::<TestData>("vault_19.img").unwrap().is_some());
    }
}
