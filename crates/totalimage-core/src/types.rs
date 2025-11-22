//! Core types for Total Liberation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Information about a file or directory occupant in a territory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OccupantInfo {
    /// Name of the file or directory
    pub name: String,

    /// True if this is a directory, false if it's a file
    pub is_directory: bool,

    /// Size in bytes (0 for directories)
    pub size: u64,

    /// Creation timestamp
    pub created: Option<DateTime<Utc>>,

    /// Last modified timestamp
    pub modified: Option<DateTime<Utc>>,

    /// Last accessed timestamp
    pub accessed: Option<DateTime<Utc>>,

    /// File attributes (platform-specific)
    pub attributes: u32,
}

impl OccupantInfo {
    /// Create a new file occupant
    pub fn file(name: String, size: u64) -> Self {
        Self {
            name,
            is_directory: false,
            size,
            created: None,
            modified: None,
            accessed: None,
            attributes: 0,
        }
    }

    /// Create a new directory occupant
    pub fn directory(name: String) -> Self {
        Self {
            name,
            is_directory: true,
            size: 0,
            created: None,
            modified: None,
            accessed: None,
            attributes: 0,
        }
    }

    /// Set creation timestamp
    pub fn with_created(mut self, created: DateTime<Utc>) -> Self {
        self.created = Some(created);
        self
    }

    /// Set modified timestamp
    pub fn with_modified(mut self, modified: DateTime<Utc>) -> Self {
        self.modified = Some(modified);
        self
    }

    /// Set accessed timestamp
    pub fn with_accessed(mut self, accessed: DateTime<Utc>) -> Self {
        self.accessed = Some(accessed);
        self
    }

    /// Set attributes
    pub fn with_attributes(mut self, attributes: u32) -> Self {
        self.attributes = attributes;
        self
    }
}

impl fmt::Display for OccupantInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_char = if self.is_directory { "d" } else { "f" };
        write!(
            f,
            "{} {:>12} {}",
            type_char,
            if self.is_directory {
                "<DIR>".to_string()
            } else {
                format_size(self.size)
            },
            self.name
        )
    }
}

/// Format size in human-readable format
fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    if unit_idx == 0 {
        format!("{} {}", size as u64, UNITS[unit_idx])
    } else {
        format!("{:.2} {}", size, UNITS[unit_idx])
    }
}

/// A zone (partition) within a vault
#[derive(Debug, Clone)]
pub struct Zone {
    /// Index of this zone
    pub index: usize,

    /// Offset from start of vault in bytes
    pub offset: u64,

    /// Length of zone in bytes
    pub length: u64,

    /// Type of zone (e.g., "FAT32", "NTFS", "Linux")
    pub zone_type: String,

    /// Detected territory type (if known)
    pub territory_type: Option<String>,
}

impl Zone {
    /// Create a new zone
    pub fn new(index: usize, offset: u64, length: u64, zone_type: String) -> Self {
        Self {
            index,
            offset,
            length,
            zone_type,
            territory_type: None,
        }
    }

    /// Set the detected territory type
    pub fn with_territory_type(mut self, territory_type: String) -> Self {
        self.territory_type = Some(territory_type);
        self
    }
}

impl fmt::Display for Zone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Zone {} [{} @ 0x{:08X}, {} bytes]",
            self.index,
            self.zone_type,
            self.offset,
            self.length
        )?;
        if let Some(ref territory) = self.territory_type {
            write!(f, " -> {}", territory)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_occupant_info_file() {
        let file = OccupantInfo::file("test.txt".to_string(), 1024);
        assert_eq!(file.name, "test.txt");
        assert!(!file.is_directory);
        assert_eq!(file.size, 1024);
    }

    #[test]
    fn test_occupant_info_directory() {
        let dir = OccupantInfo::directory("test_dir".to_string());
        assert_eq!(dir.name, "test_dir");
        assert!(dir.is_directory);
        assert_eq!(dir.size, 0);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1536 * 1024), "1.50 MB");
    }

    #[test]
    fn test_zone_creation() {
        let zone = Zone::new(0, 0x1000, 0x10000, "FAT32".to_string());
        assert_eq!(zone.index, 0);
        assert_eq!(zone.offset, 0x1000);
        assert_eq!(zone.length, 0x10000);
        assert_eq!(zone.zone_type, "FAT32");
        assert!(zone.territory_type.is_none());
    }
}
