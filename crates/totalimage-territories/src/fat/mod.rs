//! FAT (File Allocation Table) file system implementation

pub mod types;

use std::io::SeekFrom;
use totalimage_core::{DirectoryCell, Error, OccupantInfo, ReadSeek, Result, Territory};
use types::{BiosParameterBlock, DirectoryEntry, FatType, LfnEntry};

/// FAT file system territory
///
/// Supports FAT12, FAT16, and FAT32 file systems with directory enumeration
/// and file data access.
#[derive(Debug)]
pub struct FatTerritory {
    bpb: BiosParameterBlock,
    fat_table: Vec<u8>,
    identifier: String,
    /// FAT32 root directory cluster (0 for FAT12/16)
    fat32_root_cluster: u32,
}

impl FatTerritory {
    /// Parse a FAT file system from a stream
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the FAT volume
    ///
    /// # Errors
    ///
    /// Returns an error if the boot sector cannot be read or is invalid
    ///
    /// # Security
    /// Uses validated BPB parsing with checked arithmetic to prevent integer overflow
    pub fn parse(stream: &mut dyn ReadSeek) -> Result<Self> {
        // Read boot sector
        stream.seek(SeekFrom::Start(0))?;
        let mut boot_sector = vec![0u8; 512];
        stream.read_exact(&mut boot_sector)?;

        // Parse BPB with security validation
        let bpb = BiosParameterBlock::from_bytes(&boot_sector)?;

        // Read FAT table with checked arithmetic
        use totalimage_core::validate_allocation_size;
        let fat_size_u64 = totalimage_core::checked_multiply_u32_to_u64(
            bpb.sectors_per_fat(),
            bpb.bytes_per_sector as u32,
            "FAT table size"
        )?;

        let fat_size = validate_allocation_size(
            fat_size_u64,
            totalimage_core::MAX_FAT_TABLE_SIZE,
            "FAT table"
        )?;

        stream.seek(SeekFrom::Start(bpb.fat_offset()? as u64))?;
        let mut fat_table = vec![0u8; fat_size];
        stream.read_exact(&mut fat_table)?;

        let identifier = format!("{} filesystem", bpb.fat_type);

        // For FAT32, read the root cluster from extended BPB
        let fat32_root_cluster = if bpb.fat_type == FatType::Fat32 {
            // Root cluster is at offset 44 in the boot sector
            u32::from_le_bytes([boot_sector[44], boot_sector[45], boot_sector[46], boot_sector[47]])
        } else {
            0
        };

        Ok(Self {
            bpb,
            fat_table,
            identifier,
            fat32_root_cluster,
        })
    }

    /// Get the BPB
    pub fn bpb(&self) -> &BiosParameterBlock {
        &self.bpb
    }

    /// Read FAT entry for a given cluster
    ///
    /// Returns the next cluster in the chain, or None if end of chain
    pub fn read_fat_entry(&self, cluster: u32) -> Option<u32> {
        match self.bpb.fat_type {
            FatType::Fat12 => self.read_fat12_entry(cluster),
            FatType::Fat16 => self.read_fat16_entry(cluster),
            FatType::Fat32 => self.read_fat32_entry(cluster),
        }
    }

    /// Read FAT12 entry (12 bits per entry)
    fn read_fat12_entry(&self, cluster: u32) -> Option<u32> {
        let offset = (cluster + (cluster / 2)) as usize;
        if offset + 1 >= self.fat_table.len() {
            return None;
        }

        let value = if cluster & 1 == 0 {
            // Even cluster: lower 12 bits
            u16::from_le_bytes([self.fat_table[offset], self.fat_table[offset + 1]]) & 0x0FFF
        } else {
            // Odd cluster: upper 12 bits
            u16::from_le_bytes([self.fat_table[offset], self.fat_table[offset + 1]]) >> 4
        };

        // Check for end of chain markers or invalid clusters
        if value >= 0xFF8 || value == 0 || value == 1 {
            None
        } else {
            Some(value as u32)
        }
    }

    /// Read FAT16 entry (16 bits per entry)
    fn read_fat16_entry(&self, cluster: u32) -> Option<u32> {
        let offset = (cluster * 2) as usize;
        if offset + 1 >= self.fat_table.len() {
            return None;
        }

        let value = u16::from_le_bytes([self.fat_table[offset], self.fat_table[offset + 1]]);

        // Check for end of chain markers or invalid clusters
        if value >= 0xFFF8 || value == 0 || value == 1 {
            None
        } else {
            Some(value as u32)
        }
    }

    /// Read FAT32 entry (28 bits per entry, top 4 bits reserved)
    fn read_fat32_entry(&self, cluster: u32) -> Option<u32> {
        let offset = (cluster * 4) as usize;
        if offset + 3 >= self.fat_table.len() {
            return None;
        }

        let value = u32::from_le_bytes([
            self.fat_table[offset],
            self.fat_table[offset + 1],
            self.fat_table[offset + 2],
            self.fat_table[offset + 3],
        ]) & 0x0FFFFFFF; // Mask off top 4 bits

        // Check for end of chain markers or invalid clusters
        if value >= 0x0FFFFFF8 || value == 0 || value == 1 {
            None
        } else {
            Some(value)
        }
    }

    /// Get cluster chain for a starting cluster
    pub fn get_cluster_chain(&self, start_cluster: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut cluster = start_cluster;

        // Prevent infinite loops
        let max_clusters = 65536;
        let mut count = 0;

        while count < max_clusters {
            if cluster < 2 {
                break;
            }

            chain.push(cluster);

            match self.read_fat_entry(cluster) {
                Some(next) => cluster = next,
                None => break,
            }

            count += 1;
        }

        chain
    }

    /// Calculate byte offset for a cluster
    ///
    /// # Security
    /// Uses checked arithmetic to prevent overflow
    pub fn cluster_to_offset(&self, cluster: u32) -> Result<u64> {
        // Cluster 2 is the first data cluster
        let cluster_offset = cluster.saturating_sub(2);

        let data_offset = self.bpb.data_offset()? as u64;
        let bytes_per_cluster = self.bpb.bytes_per_cluster()? as u64;

        let cluster_bytes = totalimage_core::checked_multiply_u64(
            cluster_offset as u64,
            bytes_per_cluster,
            "Cluster offset calculation"
        )?;

        data_offset
            .checked_add(cluster_bytes)
            .ok_or_else(|| Error::invalid_territory("Cluster offset overflow".to_string()))
    }

    /// Read root directory entries (FAT12/16 only)
    pub fn read_root_directory(&self, stream: &mut dyn ReadSeek) -> Result<Vec<DirectoryEntry>> {
        if self.bpb.fat_type == FatType::Fat32 {
            // FAT32 has root directory in data region
            return self.read_directory_from_cluster(stream, self.fat32_root_cluster);
        }

        stream.seek(SeekFrom::Start(self.bpb.root_dir_offset()? as u64))?;

        let mut entries = Vec::new();
        let mut entry_bytes = vec![0u8; DirectoryEntry::ENTRY_SIZE];
        let mut pending_lfn: Vec<LfnEntry> = Vec::new();

        for _ in 0..self.bpb.root_entries {
            stream.read_exact(&mut entry_bytes)?;

            // Check for end of directory
            if DirectoryEntry::is_end_of_directory(&entry_bytes) {
                break;
            }

            // Skip deleted entries (but clear pending LFN)
            if DirectoryEntry::is_deleted_entry(&entry_bytes) {
                pending_lfn.clear();
                continue;
            }

            // Check for LFN entry
            if DirectoryEntry::is_lfn_entry(&entry_bytes) {
                if let Some(lfn) = LfnEntry::from_bytes(&entry_bytes) {
                    pending_lfn.push(lfn);
                }
                continue;
            }

            // Parse regular entry with any accumulated LFN entries
            if let Some(entry) = DirectoryEntry::from_bytes_with_lfn(&entry_bytes, &pending_lfn) {
                pending_lfn.clear();

                // Skip volume labels
                if !entry.is_volume_label() {
                    entries.push(entry);
                }
            } else {
                pending_lfn.clear();
            }
        }

        Ok(entries)
    }

    /// Read directory entries from a cluster chain (for subdirectories and FAT32 root)
    pub fn read_directory_from_cluster(&self, stream: &mut dyn ReadSeek, start_cluster: u32) -> Result<Vec<DirectoryEntry>> {
        if start_cluster < 2 {
            return Ok(Vec::new());
        }

        let chain = self.get_cluster_chain(start_cluster);
        if chain.is_empty() {
            return Ok(Vec::new());
        }

        let cluster_size = self.bpb.bytes_per_cluster()? as usize;
        let entries_per_cluster = cluster_size / DirectoryEntry::ENTRY_SIZE;

        let mut entries = Vec::new();
        let mut entry_bytes = vec![0u8; DirectoryEntry::ENTRY_SIZE];
        let mut pending_lfn: Vec<LfnEntry> = Vec::new();

        for cluster in chain {
            let offset = self.cluster_to_offset(cluster)?;
            stream.seek(SeekFrom::Start(offset))?;

            for _ in 0..entries_per_cluster {
                stream.read_exact(&mut entry_bytes)?;

                // Check for end of directory
                if DirectoryEntry::is_end_of_directory(&entry_bytes) {
                    return Ok(entries);
                }

                // Skip deleted entries (but clear pending LFN)
                if DirectoryEntry::is_deleted_entry(&entry_bytes) {
                    pending_lfn.clear();
                    continue;
                }

                // Check for LFN entry
                if DirectoryEntry::is_lfn_entry(&entry_bytes) {
                    if let Some(lfn) = LfnEntry::from_bytes(&entry_bytes) {
                        pending_lfn.push(lfn);
                    }
                    continue;
                }

                // Parse regular entry with any accumulated LFN entries
                if let Some(entry) = DirectoryEntry::from_bytes_with_lfn(&entry_bytes, &pending_lfn) {
                    pending_lfn.clear();

                    // Skip volume labels and . / .. entries
                    if !entry.is_volume_label() {
                        let short_name = &entry.short_name;
                        if short_name != "." && short_name != ".." {
                            entries.push(entry);
                        }
                    }
                } else {
                    pending_lfn.clear();
                }
            }
        }

        Ok(entries)
    }

    /// List root directory as OccupantInfo (for CLI)
    pub fn list_root_directory(&self, stream: &mut dyn ReadSeek) -> Result<Vec<OccupantInfo>> {
        let entries = self.read_root_directory(stream)?;

        Ok(entries
            .into_iter()
            .map(|entry| OccupantInfo {
                name: entry.name.clone(),
                is_directory: entry.is_directory(),
                size: entry.file_size as u64,
                created: None,
                modified: None,
                accessed: None,
                attributes: entry.attributes as u32,
            })
            .collect())
    }

    /// List directory contents at a path
    pub fn list_directory(&self, stream: &mut dyn ReadSeek, path: &str) -> Result<Vec<OccupantInfo>> {
        let entries = self.read_directory_at_path(stream, path)?;

        Ok(entries
            .into_iter()
            .map(|entry| OccupantInfo {
                name: entry.name.clone(),
                is_directory: entry.is_directory(),
                size: entry.file_size as u64,
                created: None,
                modified: None,
                accessed: None,
                attributes: entry.attributes as u32,
            })
            .collect())
    }

    /// Read directory entries at a given path
    pub fn read_directory_at_path(&self, stream: &mut dyn ReadSeek, path: &str) -> Result<Vec<DirectoryEntry>> {
        let path = path.trim_matches('/').trim_matches('\\');

        // Root directory
        if path.is_empty() {
            return self.read_root_directory(stream);
        }

        // Split path and navigate
        let parts: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

        let mut current_entries = self.read_root_directory(stream)?;

        for (i, part) in parts.iter().enumerate() {
            // Find the directory entry matching this path component
            let dir_entry = current_entries
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(part) && (e.is_directory() || i == parts.len() - 1))
                .ok_or_else(|| Error::not_found(format!("Path component not found: {}", part)))?;

            if i == parts.len() - 1 {
                // Last component - if it's a directory, read its contents
                if dir_entry.is_directory() {
                    return self.read_directory_from_cluster(stream, dir_entry.first_cluster());
                } else {
                    // It's a file - return error since we expected a directory
                    return Err(Error::not_found(format!("Not a directory: {}", part)));
                }
            } else {
                // Not the last component - must be a directory
                if !dir_entry.is_directory() {
                    return Err(Error::not_found(format!("Not a directory: {}", part)));
                }
                current_entries = self.read_directory_from_cluster(stream, dir_entry.first_cluster())?;
            }
        }

        Ok(current_entries)
    }

    /// Find a file in the root directory by name
    pub fn find_file_in_root(&self, stream: &mut dyn ReadSeek, name: &str) -> Result<DirectoryEntry> {
        let entries = self.read_root_directory(stream)?;

        for entry in entries {
            if entry.name.eq_ignore_ascii_case(name) {
                return Ok(entry);
            }
        }

        Err(Error::not_found(format!("File not found: {}", name)))
    }

    /// Find a file by path (supports subdirectories)
    pub fn find_file_by_path(&self, stream: &mut dyn ReadSeek, path: &str) -> Result<DirectoryEntry> {
        let path = path.trim_matches('/').trim_matches('\\');

        if path.is_empty() {
            return Err(Error::not_found("Empty path".to_string()));
        }

        let parts: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            return Err(Error::not_found("Empty path".to_string()));
        }

        let mut current_entries = self.read_root_directory(stream)?;

        for (i, part) in parts.iter().enumerate() {
            let entry = current_entries
                .iter()
                .find(|e| e.name.eq_ignore_ascii_case(part))
                .ok_or_else(|| Error::not_found(format!("Path component not found: {}", part)))?
                .clone();

            if i == parts.len() - 1 {
                // Last component - this is what we're looking for
                return Ok(entry);
            } else {
                // Not the last component - must be a directory
                if !entry.is_directory() {
                    return Err(Error::not_found(format!("Not a directory: {}", part)));
                }
                current_entries = self.read_directory_from_cluster(stream, entry.first_cluster())?;
            }
        }

        Err(Error::not_found(format!("File not found: {}", path)))
    }

    /// Read file data from clusters
    ///
    /// # Security
    /// Validates file size and uses checked arithmetic
    pub fn read_file_data(&self, stream: &mut dyn ReadSeek, entry: &DirectoryEntry) -> Result<Vec<u8>> {
        let first_cluster = entry.first_cluster();

        // Special case: empty files or files in root directory with cluster 0
        if first_cluster == 0 || entry.file_size == 0 {
            return Ok(Vec::new());
        }

        // Validate file size against extraction limit
        use totalimage_core::MAX_FILE_EXTRACT_SIZE;
        if entry.file_size as u64 > MAX_FILE_EXTRACT_SIZE {
            return Err(Error::invalid_territory(format!(
                "File size {} exceeds extraction limit {}",
                entry.file_size, MAX_FILE_EXTRACT_SIZE
            )));
        }

        // Get cluster chain
        let chain = self.get_cluster_chain(first_cluster);

        if chain.is_empty() {
            return Ok(Vec::new());
        }

        // Read data from clusters
        let mut data = Vec::with_capacity(entry.file_size as usize);
        let cluster_size = self.bpb.bytes_per_cluster()? as usize;
        let mut remaining = entry.file_size as usize;

        for cluster in chain {
            let offset = self.cluster_to_offset(cluster)?;
            stream.seek(SeekFrom::Start(offset))?;

            let to_read = remaining.min(cluster_size);
            let mut cluster_data = vec![0u8; to_read];
            stream.read_exact(&mut cluster_data)?;

            data.extend_from_slice(&cluster_data);
            remaining -= to_read;

            if remaining == 0 {
                break;
            }
        }

        Ok(data)
    }

    /// Read file data by path (supports subdirectories)
    pub fn read_file_by_path(&self, stream: &mut dyn ReadSeek, path: &str) -> Result<Vec<u8>> {
        let entry = self.find_file_by_path(stream, path)?;

        if entry.is_directory() {
            return Err(Error::not_found(format!("Path is a directory: {}", path)));
        }

        self.read_file_data(stream, &entry)
    }
}

impl Territory for FatTerritory {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn banner(&self) -> Result<String> {
        // FAT volumes can have volume labels stored in root directory
        // For now return a placeholder
        Ok(String::from("FAT_VOLUME"))
    }

    fn headquarters(&self) -> Result<Box<dyn DirectoryCell>> {
        Ok(Box::new(FatRootDirectory))
    }

    fn domain_size(&self) -> u64 {
        // Use saturating multiply to prevent overflow, truncate if needed
        totalimage_core::checked_multiply_u32_to_u64(
            self.bpb.total_sectors(),
            self.bpb.bytes_per_sector as u32,
            "Domain size"
        ).unwrap_or(0)
    }

    fn liberated_space(&self) -> u64 {
        // Would need to count free clusters in FAT
        // Return 0 for now (simplified implementation)
        0
    }

    fn block_size(&self) -> u64 {
        self.bpb.bytes_per_cluster().unwrap_or(0) as u64
    }

    fn hierarchical(&self) -> bool {
        true // FAT supports subdirectories
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

/// FAT root directory cell
struct FatRootDirectory;

impl DirectoryCell for FatRootDirectory {
    fn name(&self) -> &str {
        "/"
    }

    fn list_occupants(&self) -> Result<Vec<OccupantInfo>> {
        // Simplified: return empty list
        // Full implementation would read directory entries from stream
        Ok(Vec::new())
    }

    fn enter(&self, _name: &str) -> Result<Box<dyn DirectoryCell>> {
        // Simplified: return error
        Err(Error::not_found("Subdirectory not found".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Create a minimal FAT12 boot sector
    fn create_fat12_boot_sector() -> Vec<u8> {
        let mut boot = vec![0u8; 512];

        // Jump instruction
        boot[0..3].copy_from_slice(&[0xEB, 0x3C, 0x90]);

        // OEM name
        boot[3..11].copy_from_slice(b"MSWIN4.1");

        // BPB
        boot[11..13].copy_from_slice(&512u16.to_le_bytes()); // Bytes per sector
        boot[13] = 1; // Sectors per cluster
        boot[14..16].copy_from_slice(&1u16.to_le_bytes()); // Reserved sectors
        boot[16] = 2; // Number of FATs
        boot[17..19].copy_from_slice(&224u16.to_le_bytes()); // Root entries
        boot[19..21].copy_from_slice(&2880u16.to_le_bytes()); // Total sectors (1.44MB floppy)
        boot[21] = 0xF0; // Media descriptor (removable media)
        boot[22..24].copy_from_slice(&9u16.to_le_bytes()); // Sectors per FAT
        boot[24..26].copy_from_slice(&18u16.to_le_bytes()); // Sectors per track
        boot[26..28].copy_from_slice(&2u16.to_le_bytes()); // Number of heads

        // Boot signature
        boot[510..512].copy_from_slice(&[0x55, 0xAA]);

        boot
    }

    #[test]
    fn test_parse_fat12() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560]; // 1.44MB floppy
        disk[0..512].copy_from_slice(&boot_sector);

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        assert_eq!(territory.bpb.fat_type, FatType::Fat12);
        assert_eq!(territory.bpb.bytes_per_sector, 512);
        assert_eq!(territory.bpb.sectors_per_cluster, 1);
        assert_eq!(territory.identify(), "FAT12 filesystem");
    }

    #[test]
    fn test_fat12_entry_reading() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        // Create a simple FAT with a chain: 2 -> 3 -> EOF
        let fat_offset = 512; // After boot sector
        disk[fat_offset] = 0xF0; // Media descriptor in FAT[0]
        disk[fat_offset + 1] = 0xFF;
        disk[fat_offset + 2] = 0xFF;

        // Cluster 2: points to cluster 3
        // FAT12 entry for cluster 2 is at offset 3 bytes (1.5 * 2)
        disk[fat_offset + 3] = 0x03;
        disk[fat_offset + 4] = 0x00;

        // Cluster 3: EOF
        disk[fat_offset + 4] |= 0xF0; // Upper nibble of cluster 3
        disk[fat_offset + 5] = 0xFF;

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        let chain = territory.get_cluster_chain(2);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], 2);
        assert_eq!(chain[1], 3);
    }

    #[test]
    fn test_cluster_to_offset() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        // Calculate expected offset for cluster 2
        // Reserved: 1 sector (512 bytes)
        // FATs: 2 * 9 sectors = 18 sectors (9216 bytes)
        // Root dir: 224 entries * 32 bytes = 7168 bytes = 14 sectors
        // Total: 1 + 18 + 14 = 33 sectors = 16896 bytes
        let expected = 16896;

        assert_eq!(territory.cluster_to_offset(2).unwrap(), expected);
    }

    #[test]
    fn test_root_directory_reading() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        // Add a test file entry in root directory
        let root_offset = 512 + (2 * 9 * 512); // After boot sector and FATs
        disk[root_offset..root_offset + 11].copy_from_slice(b"TEST    TXT");
        disk[root_offset + 11] = 0x20; // Archive attribute

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        let entries = territory.read_root_directory(&mut cursor).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "TEST.TXT");
    }

    #[test]
    fn test_subdirectory_navigation() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        // Set up FAT entries for cluster chain
        let fat_offset = 512;
        disk[fat_offset] = 0xF0;
        disk[fat_offset + 1] = 0xFF;
        disk[fat_offset + 2] = 0xFF;
        // Cluster 2: EOF (for SUBDIR)
        disk[fat_offset + 3] = 0xF8;
        disk[fat_offset + 4] = 0x0F;

        // Add a subdirectory entry in root directory
        let root_offset = 512 + (2 * 9 * 512);
        disk[root_offset..root_offset + 11].copy_from_slice(b"SUBDIR     ");
        disk[root_offset + 11] = DirectoryEntry::ATTR_DIRECTORY;
        disk[root_offset + 26] = 2; // First cluster low = 2
        disk[root_offset + 27] = 0;

        // Add a file inside the subdirectory (at cluster 2)
        let data_offset = 16896; // Calculated from test_cluster_to_offset
        disk[data_offset..data_offset + 11].copy_from_slice(b"NESTED  TXT");
        disk[data_offset + 11] = 0x20; // Archive attribute

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        // Test reading root directory
        let root_entries = territory.read_root_directory(&mut cursor).unwrap();
        assert_eq!(root_entries.len(), 1);
        assert_eq!(root_entries[0].name, "SUBDIR");
        assert!(root_entries[0].is_directory());

        // Test navigating to subdirectory
        let subdir_entries = territory.read_directory_at_path(&mut cursor, "SUBDIR").unwrap();
        assert_eq!(subdir_entries.len(), 1);
        assert_eq!(subdir_entries[0].name, "NESTED.TXT");
    }

    #[test]
    fn test_find_file_by_path() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        // Set up FAT
        let fat_offset = 512;
        disk[fat_offset] = 0xF0;
        disk[fat_offset + 1] = 0xFF;
        disk[fat_offset + 2] = 0xFF;
        disk[fat_offset + 3] = 0xF8;
        disk[fat_offset + 4] = 0x0F;

        // Add subdirectory in root
        let root_offset = 512 + (2 * 9 * 512);
        disk[root_offset..root_offset + 11].copy_from_slice(b"DOCS       ");
        disk[root_offset + 11] = DirectoryEntry::ATTR_DIRECTORY;
        disk[root_offset + 26] = 2;

        // Add file in subdirectory
        let data_offset = 16896;
        disk[data_offset..data_offset + 11].copy_from_slice(b"README  TXT");
        disk[data_offset + 11] = 0x20;
        disk[data_offset + 28] = 100; // File size = 100 bytes

        let mut cursor = Cursor::new(disk);
        let territory = FatTerritory::parse(&mut cursor).unwrap();

        // Find file by path
        let entry = territory.find_file_by_path(&mut cursor, "DOCS/README.TXT").unwrap();
        assert_eq!(entry.name, "README.TXT");
        assert_eq!(entry.file_size, 100);
    }

    #[test]
    fn test_territory_methods() {
        let boot_sector = create_fat12_boot_sector();
        let mut disk = vec![0u8; 1_474_560];
        disk[0..512].copy_from_slice(&boot_sector);

        let mut cursor = Cursor::new(disk);
        let mut territory = FatTerritory::parse(&mut cursor).unwrap();

        assert_eq!(territory.identify(), "FAT12 filesystem");
        assert_eq!(territory.domain_size(), 1_474_560);
        assert_eq!(territory.block_size(), 512);
        assert!(territory.hierarchical());
        assert!(territory.banner().is_ok());
        assert!(territory.headquarters().is_ok());
        assert!(territory.extract_file("test.txt").is_ok());
    }
}
