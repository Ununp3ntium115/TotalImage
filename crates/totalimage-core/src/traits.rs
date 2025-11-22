//! Core traits for Total Liberation

use crate::{error::Result, types::{OccupantInfo, Zone}};
use std::io::{Read, Seek, Write};

/// Trait for disk image vaults (containers)
pub trait Vault: Send + Sync {
    /// Get a human-readable identifier for this vault type
    fn identify(&self) -> &str;

    /// Get the total size of the vault in bytes
    fn length(&self) -> u64;

    /// Get a readable and seekable stream to the vault content
    fn content(&mut self) -> &mut dyn ReadSeek;
}

/// Trait for partition tables (zone tables)
pub trait ZoneTable: Send + Sync {
    /// Get a human-readable identifier for this zone table type
    fn identify(&self) -> &str;

    /// Get all zones in this partition table
    fn enumerate_zones(&self) -> &[Zone];

    /// Get a specific zone by index
    fn get_zone(&self, index: usize) -> Option<&Zone> {
        self.enumerate_zones().get(index)
    }
}

/// Trait for file systems (territories)
pub trait Territory: Send + Sync {
    /// Get a human-readable identifier for this territory type
    fn identify(&self) -> &str;

    /// Get the volume label (banner)
    fn banner(&self) -> Result<String>;

    /// Set the volume label (not implemented for read-only)
    fn set_banner(&mut self, _label: &str) -> Result<()> {
        Err(crate::error::Error::Unsupported(
            "Setting banner not supported in read-only mode".to_string()
        ))
    }

    /// Get the root directory
    fn headquarters(&self) -> Result<Box<dyn DirectoryCell>>;

    /// Get total size of the territory in bytes
    fn domain_size(&self) -> u64;

    /// Get free space in bytes
    fn liberated_space(&self) -> u64;

    /// Get allocation unit (cluster/block) size in bytes
    fn block_size(&self) -> u64;

    /// Does this territory support subdirectories?
    fn hierarchical(&self) -> bool;

    /// Navigate to a directory by path
    fn navigate_to(&self, path: &str) -> Result<Box<dyn DirectoryCell>>;

    /// Extract a file by path
    fn extract_file(&mut self, path: &str) -> Result<Vec<u8>>;
}

/// Trait for directory operations
pub trait DirectoryCell: Send + Sync {
    /// Get the directory name
    fn name(&self) -> &str;

    /// List all occupants (files and subdirectories) in this directory
    fn list_occupants(&self) -> Result<Vec<OccupantInfo>>;

    /// Enter a subdirectory by name
    fn enter(&self, name: &str) -> Result<Box<dyn DirectoryCell>>;

    /// Check if a file or directory exists
    fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.list_occupants()?.iter().any(|o| o.name == name))
    }

    /// Get info about a specific occupant
    fn get_occupant(&self, name: &str) -> Result<Option<OccupantInfo>> {
        Ok(self.list_occupants()?.into_iter().find(|o| o.name == name))
    }
}

/// Combined trait for Read + Seek
pub trait ReadSeek: Read + Seek + Send {}

/// Blanket implementation for any type that implements Read + Seek
impl<T: Read + Seek + Send> ReadSeek for T {}

/// Combined trait for Read + Write + Seek
pub trait ReadWriteSeek: Read + Write + Seek + Send {}

/// Blanket implementation for any type that implements Read + Write + Seek
impl<T: Read + Write + Seek + Send> ReadWriteSeek for T {}
