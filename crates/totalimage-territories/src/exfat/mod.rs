//! exFAT filesystem implementation
//!
//! exFAT (Extended File Allocation Table) is a filesystem optimized for
//! flash memory such as USB drives and SD cards. It supports files larger
//! than 4GB and has improved space efficiency compared to FAT32.
//!
//! # Structure
//!
//! ```text
//! ┌──────────────────────────┐
//! │   Main Boot Region       │  Sectors 0-11
//! ├──────────────────────────┤
//! │   Backup Boot Region     │  Sectors 12-23
//! ├──────────────────────────┤
//! │   FAT Region             │  Allocation table
//! ├──────────────────────────┤
//! │   Cluster Heap           │  Data clusters
//! │   (incl. Root Directory) │
//! └──────────────────────────┘
//! ```

pub mod types;

use std::io::{Read, Seek, SeekFrom};
use totalimage_core::{DirectoryCell, OccupantInfo, Result, Territory};

pub use types::*;

/// exFAT Territory implementation
#[derive(Debug)]
pub struct ExfatTerritory {
    /// Identifier string
    identifier: String,
    /// Boot sector information
    boot_sector: ExfatBootSector,
    /// Volume label (if found)
    volume_label: Option<String>,
    /// Bytes per sector
    bytes_per_sector: u32,
    /// Bytes per cluster
    bytes_per_cluster: u32,
    /// Cluster heap offset in bytes
    cluster_heap_offset: u64,
    /// Total cluster count
    cluster_count: u32,
    /// Root directory first cluster
    root_dir_cluster: u32,
    /// Volume length in bytes
    volume_length: u64,
}

impl ExfatTerritory {
    /// Parse exFAT filesystem from a reader
    pub fn parse<R: Read + Seek>(reader: &mut R) -> Result<Self> {
        // Read boot sector
        let mut boot_bytes = [0u8; 512];
        reader.read_exact(&mut boot_bytes)?;
        let boot_sector = ExfatBootSector::parse(&boot_bytes)?;

        let bytes_per_sector = boot_sector.bytes_per_sector();
        let bytes_per_cluster = boot_sector.bytes_per_cluster();
        let cluster_heap_offset = boot_sector.cluster_heap_offset as u64 * bytes_per_sector as u64;
        let volume_length = boot_sector.volume_length * bytes_per_sector as u64;

        let identifier = format!(
            "exFAT {} clusters, {} bytes/cluster",
            boot_sector.cluster_count, bytes_per_cluster
        );

        Ok(Self {
            identifier,
            boot_sector: boot_sector.clone(),
            volume_label: None,
            bytes_per_sector,
            bytes_per_cluster,
            cluster_heap_offset,
            cluster_count: boot_sector.cluster_count,
            root_dir_cluster: boot_sector.root_dir_cluster,
            volume_length,
        })
    }

    /// Get the boot sector
    pub fn boot_sector(&self) -> &ExfatBootSector {
        &self.boot_sector
    }

    /// Calculate byte offset for a cluster
    fn cluster_offset(&self, cluster: u32) -> u64 {
        if cluster < 2 {
            return self.cluster_heap_offset;
        }
        self.cluster_heap_offset + (cluster - 2) as u64 * self.bytes_per_cluster as u64
    }

    /// Read FAT entry for a cluster
    fn read_fat_entry<R: Read + Seek>(&self, reader: &mut R, cluster: u32) -> Result<u32> {
        let fat_offset = self.boot_sector.fat_offset as u64 * self.bytes_per_sector as u64;
        let entry_offset = fat_offset + cluster as u64 * 4;

        reader.seek(SeekFrom::Start(entry_offset))?;

        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        Ok(u32::from_le_bytes(buf))
    }

    /// Read cluster chain into a buffer
    pub fn read_cluster_chain<R: Read + Seek>(
        &self,
        reader: &mut R,
        start_cluster: u32,
        max_bytes: Option<u64>,
    ) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut current_cluster = start_cluster;
        let mut bytes_read = 0u64;
        let max = max_bytes.unwrap_or(u64::MAX);

        // Circular reference protection
        let max_clusters = self.cluster_count + 10;
        let mut clusters_visited = 0u32;

        while !cluster::is_end(current_cluster) && clusters_visited < max_clusters {
            if current_cluster < 2 || current_cluster >= self.cluster_count + 2 {
                break;
            }

            let offset = self.cluster_offset(current_cluster);
            reader.seek(SeekFrom::Start(offset))?;

            let to_read = (self.bytes_per_cluster as u64).min(max - bytes_read);
            let mut cluster_data = vec![0u8; to_read as usize];
            reader.read_exact(&mut cluster_data)?;
            data.extend_from_slice(&cluster_data);

            bytes_read += to_read;
            if bytes_read >= max {
                break;
            }

            current_cluster = self.read_fat_entry(reader, current_cluster)?;
            clusters_visited += 1;
        }

        Ok(data)
    }

    /// Read contiguous clusters (no FAT chain needed)
    pub fn read_contiguous_clusters<R: Read + Seek>(
        &self,
        reader: &mut R,
        start_cluster: u32,
        size: u64,
    ) -> Result<Vec<u8>> {
        let offset = self.cluster_offset(start_cluster);
        reader.seek(SeekFrom::Start(offset))?;

        let mut data = vec![0u8; size as usize];
        reader.read_exact(&mut data)?;

        Ok(data)
    }

    /// Read root directory entries
    pub fn read_root_directory<R: Read + Seek>(&self, reader: &mut R) -> Result<Vec<ExfatDirectoryEntry>> {
        self.read_directory_from_cluster(reader, self.root_dir_cluster)
    }

    /// Read directory from a cluster
    pub fn read_directory_from_cluster<R: Read + Seek>(
        &self,
        reader: &mut R,
        start_cluster: u32,
    ) -> Result<Vec<ExfatDirectoryEntry>> {
        // Read directory cluster chain
        let dir_data = self.read_cluster_chain(reader, start_cluster, None)?;

        let mut entries = Vec::new();
        let mut i = 0;

        while i + 32 <= dir_data.len() {
            let entry_type = EntryType::from_byte(dir_data[i]);

            match entry_type {
                EntryType::EndOfDirectory => break,
                EntryType::FileEntry => {
                    // Parse file directory entry
                    let file_entry = FileDirectoryEntry::parse(&dir_data[i..i + 32])?;
                    let secondary_count = file_entry.secondary_count as usize;

                    // Need at least stream extension + file name entries
                    if secondary_count < 2 || i + 32 * (secondary_count + 1) > dir_data.len() {
                        i += 32;
                        continue;
                    }

                    // Parse stream extension (second entry)
                    let stream_offset = i + 32;
                    if dir_data[stream_offset] != 0xC0 {
                        i += 32;
                        continue;
                    }
                    let stream_entry = StreamExtensionEntry::parse(&dir_data[stream_offset..stream_offset + 32])?;

                    // Parse file name entries
                    let mut name = String::new();
                    let name_length = stream_entry.name_length as usize;
                    let mut chars_collected = 0;

                    for j in 2..=secondary_count {
                        let name_offset = i + 32 * j;
                        if dir_data[name_offset] != 0xC1 {
                            break;
                        }
                        let name_entry = FileNameEntry::parse(&dir_data[name_offset..name_offset + 32])?;

                        for &ch in &name_entry.file_name {
                            if ch == 0 || chars_collected >= name_length {
                                break;
                            }
                            if let Some(c) = char::from_u32(ch as u32) {
                                name.push(c);
                                chars_collected += 1;
                            }
                        }
                    }

                    entries.push(ExfatDirectoryEntry {
                        name,
                        attributes: file_entry.attributes,
                        size: stream_entry.data_length,
                        first_cluster: stream_entry.first_cluster,
                        created: file_entry.create_timestamp,
                        modified: file_entry.modify_timestamp,
                        accessed: file_entry.access_timestamp,
                        is_contiguous: stream_entry.is_contiguous(),
                    });

                    // Skip all secondary entries
                    i += 32 * (secondary_count + 1);
                }
                _ => {
                    i += 32;
                }
            }
        }

        Ok(entries)
    }

    /// Read file contents
    pub fn read_file<R: Read + Seek>(&self, reader: &mut R, entry: &ExfatDirectoryEntry) -> Result<Vec<u8>> {
        if entry.is_directory() {
            return Err(totalimage_core::Error::invalid_territory(
                "Cannot read directory as file",
            ));
        }

        if entry.is_contiguous {
            self.read_contiguous_clusters(reader, entry.first_cluster, entry.size)
        } else {
            self.read_cluster_chain(reader, entry.first_cluster, Some(entry.size))
        }
    }

    /// Read subdirectory contents
    pub fn read_subdirectory<R: Read + Seek>(
        &self,
        reader: &mut R,
        entry: &ExfatDirectoryEntry,
    ) -> Result<Vec<ExfatDirectoryEntry>> {
        if !entry.is_directory() {
            return Err(totalimage_core::Error::invalid_territory(
                "Not a directory",
            ));
        }

        self.read_directory_from_cluster(reader, entry.first_cluster)
    }

    /// Navigate to a path and return the entry
    pub fn find_entry_by_path<R: Read + Seek>(
        &self,
        reader: &mut R,
        path: &str,
    ) -> Result<ExfatDirectoryEntry> {
        let components: Vec<&str> = path
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();

        if components.is_empty() {
            return Err(totalimage_core::Error::invalid_territory("Empty path"));
        }

        let mut current_entries = self.read_root_directory(reader)?;

        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let upper_component = component.to_uppercase();

            let found = current_entries
                .iter()
                .find(|e| e.name.to_uppercase() == upper_component)
                .cloned();

            match found {
                Some(entry) => {
                    if is_last {
                        return Ok(entry);
                    }
                    if !entry.is_directory() {
                        return Err(totalimage_core::Error::invalid_territory(format!(
                            "'{}' is not a directory",
                            component
                        )));
                    }
                    current_entries = self.read_subdirectory(reader, &entry)?;
                }
                None => {
                    return Err(totalimage_core::Error::invalid_territory(format!(
                        "Path component '{}' not found",
                        component
                    )));
                }
            }
        }

        Err(totalimage_core::Error::invalid_territory("Path not found"))
    }
}

impl Territory for ExfatTerritory {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn banner(&self) -> Result<String> {
        Ok(self.volume_label.clone().unwrap_or_else(|| "EXFAT".to_string()))
    }

    fn headquarters(&self) -> Result<Box<dyn DirectoryCell>> {
        Ok(Box::new(ExfatRootDirectory))
    }

    fn domain_size(&self) -> u64 {
        self.volume_length
    }

    fn liberated_space(&self) -> u64 {
        // Would need to count free clusters - return 0 for now
        0
    }

    fn block_size(&self) -> u64 {
        self.bytes_per_cluster as u64
    }

    fn hierarchical(&self) -> bool {
        true // exFAT supports subdirectories
    }

    fn navigate_to(&self, _path: &str) -> Result<Box<dyn DirectoryCell>> {
        // Simplified: always return root directory
        self.headquarters()
    }

    fn extract_file(&mut self, _path: &str) -> Result<Vec<u8>> {
        // Simplified: return empty
        // Full implementation would parse path, find file, read clusters
        Ok(Vec::new())
    }
}

/// exFAT root directory cell (placeholder for DirectoryCell trait)
#[derive(Debug)]
struct ExfatRootDirectory;

impl DirectoryCell for ExfatRootDirectory {
    fn name(&self) -> &str {
        "/"
    }

    fn list_occupants(&self) -> Result<Vec<OccupantInfo>> {
        // Simplified implementation - would need reader access
        Ok(Vec::new())
    }

    fn enter(&self, _name: &str) -> Result<Box<dyn DirectoryCell>> {
        Err(totalimage_core::Error::invalid_territory(
            "Directory navigation not implemented in simplified mode",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_offset_calculation() {
        let boot_sector = ExfatBootSector {
            jump_boot: [0xEB, 0x76, 0x90],
            fs_name: *b"EXFAT   ",
            partition_offset: 0,
            volume_length: 1000000,
            fat_offset: 128,
            fat_length: 256,
            cluster_heap_offset: 512,
            cluster_count: 10000,
            root_dir_cluster: 4,
            volume_serial: 0x12345678,
            fs_revision: 0x0100,
            volume_flags: 0,
            bytes_per_sector_shift: 9,
            sectors_per_cluster_shift: 3,
            number_of_fats: 1,
            drive_select: 0x80,
            percent_in_use: 0,
        };

        let territory = ExfatTerritory {
            identifier: "test".to_string(),
            boot_sector: boot_sector.clone(),
            volume_label: None,
            bytes_per_sector: 512,
            bytes_per_cluster: 4096,
            cluster_heap_offset: 512 * 512,
            cluster_count: 10000,
            root_dir_cluster: 4,
            volume_length: 512 * 1000000,
        };

        // Cluster 2 should be at heap offset
        assert_eq!(territory.cluster_offset(2), 512 * 512);

        // Cluster 3 should be one cluster after
        assert_eq!(territory.cluster_offset(3), 512 * 512 + 4096);

        // Cluster 4 should be two clusters after
        assert_eq!(territory.cluster_offset(4), 512 * 512 + 8192);
    }

    #[test]
    fn test_bytes_per_cluster() {
        let boot_sector = ExfatBootSector {
            jump_boot: [0xEB, 0x76, 0x90],
            fs_name: *b"EXFAT   ",
            partition_offset: 0,
            volume_length: 1000000,
            fat_offset: 128,
            fat_length: 256,
            cluster_heap_offset: 512,
            cluster_count: 10000,
            root_dir_cluster: 4,
            volume_serial: 0x12345678,
            fs_revision: 0x0100,
            volume_flags: 0,
            bytes_per_sector_shift: 9,  // 512 bytes/sector
            sectors_per_cluster_shift: 3, // 8 sectors/cluster
            number_of_fats: 1,
            drive_select: 0x80,
            percent_in_use: 0,
        };

        assert_eq!(boot_sector.bytes_per_sector(), 512);
        assert_eq!(boot_sector.sectors_per_cluster(), 8);
        assert_eq!(boot_sector.bytes_per_cluster(), 4096);
    }

    #[test]
    fn test_territory_identify() {
        let boot_sector = ExfatBootSector {
            jump_boot: [0xEB, 0x76, 0x90],
            fs_name: *b"EXFAT   ",
            partition_offset: 0,
            volume_length: 1000000,
            fat_offset: 128,
            fat_length: 256,
            cluster_heap_offset: 512,
            cluster_count: 10000,
            root_dir_cluster: 4,
            volume_serial: 0x12345678,
            fs_revision: 0x0100,
            volume_flags: 0,
            bytes_per_sector_shift: 9,
            sectors_per_cluster_shift: 3,
            number_of_fats: 1,
            drive_select: 0x80,
            percent_in_use: 0,
        };

        let territory = ExfatTerritory {
            identifier: "exFAT 10000 clusters, 4096 bytes/cluster".to_string(),
            boot_sector,
            volume_label: Some("MY_USB".to_string()),
            bytes_per_sector: 512,
            bytes_per_cluster: 4096,
            cluster_heap_offset: 512 * 512,
            cluster_count: 10000,
            root_dir_cluster: 4,
            volume_length: 512 * 1000000,
        };

        assert!(territory.identify().contains("exFAT"));
        assert_eq!(territory.block_size(), 4096);
        assert!(territory.hierarchical());
    }
}
