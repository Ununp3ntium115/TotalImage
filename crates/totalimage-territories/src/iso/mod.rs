//! ISO-9660 (CD-ROM) file system implementation

pub mod types;

use std::io::SeekFrom;
use totalimage_core::{DirectoryCell, Error, OccupantInfo, ReadSeek, Result, Territory};
use types::{
    DirectoryRecord, PrimaryVolumeDescriptor, VolumeDescriptorType, SECTOR_SIZE,
    VOLUME_DESCRIPTOR_START,
};

/// ISO-9660 file system territory
///
/// Supports basic ISO-9660 (CD-ROM) file systems with directory enumeration
/// and file data access. Read-only by design.
#[derive(Debug)]
pub struct IsoTerritory {
    primary_descriptor: PrimaryVolumeDescriptor,
    root_directory: DirectoryRecord,
    identifier: String,
}

impl IsoTerritory {
    /// Parse an ISO-9660 file system from a stream
    ///
    /// # Arguments
    ///
    /// * `stream` - A stream positioned at the start of the ISO volume
    ///
    /// # Errors
    ///
    /// Returns an error if volume descriptors cannot be read or are invalid
    pub fn parse(stream: &mut dyn ReadSeek) -> Result<Self> {
        // Seek to volume descriptor set (sector 16)
        stream.seek(SeekFrom::Start(VOLUME_DESCRIPTOR_START))?;

        let mut primary_descriptor: Option<PrimaryVolumeDescriptor> = None;

        // Read volume descriptors until we find terminator
        loop {
            let mut sector = vec![0u8; SECTOR_SIZE];
            stream.read_exact(&mut sector)?;

            let descriptor_type = sector[0];
            let identifier = &sector[1..6];

            // Check for valid ISO-9660 identifier
            if identifier != b"CD001" {
                return Err(Error::invalid_territory(format!(
                    "Invalid ISO-9660 identifier: {:?}",
                    identifier
                )));
            }

            // Parse based on type
            match VolumeDescriptorType::from_u8(descriptor_type) {
                Some(VolumeDescriptorType::PrimaryVolumeDescriptor) => {
                    primary_descriptor = Some(
                        PrimaryVolumeDescriptor::from_bytes(&sector).ok_or_else(|| {
                            Error::invalid_territory("Failed to parse primary volume descriptor".to_string())
                        })?,
                    );
                }
                Some(VolumeDescriptorType::VolumeDescriptorSetTerminator) => {
                    // End of volume descriptor set
                    break;
                }
                Some(VolumeDescriptorType::SupplementaryVolumeDescriptor)
                | Some(VolumeDescriptorType::BootRecord)
                | Some(VolumeDescriptorType::VolumePartitionDescriptor) => {
                    // Skip these for now (could handle Joliet, El Torito, etc.)
                }
                None => {
                    return Err(Error::invalid_territory(format!(
                        "Unknown volume descriptor type: {}",
                        descriptor_type
                    )));
                }
            }
        }

        // Must have a primary volume descriptor
        let primary = primary_descriptor
            .ok_or_else(|| Error::invalid_territory("No primary volume descriptor found".to_string()))?;

        // Get root directory record from primary descriptor
        let root_directory = primary.root_directory_record.clone();

        let identifier = "ISO-9660 filesystem".to_string();

        Ok(Self {
            primary_descriptor: primary,
            root_directory,
            identifier,
        })
    }

    /// Get the primary volume descriptor
    pub fn primary_descriptor(&self) -> &PrimaryVolumeDescriptor {
        &self.primary_descriptor
    }

    /// Read directory entries from a directory record
    pub fn read_directory(
        &self,
        stream: &mut dyn ReadSeek,
        directory: &DirectoryRecord,
    ) -> Result<Vec<DirectoryRecord>> {
        if !directory.is_directory() {
            return Err(Error::invalid_territory("Not a directory".to_string()));
        }

        let extent_lba = directory.extent_location.get();
        let data_length = directory.data_length.get();

        // Seek to directory extent
        let offset = extent_lba as u64 * SECTOR_SIZE as u64;
        stream.seek(SeekFrom::Start(offset))?;

        // Read directory data
        let mut data = vec![0u8; data_length as usize];
        stream.read_exact(&mut data)?;

        // Parse directory records
        let mut entries = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            // Check for end of directory or padding
            if data[pos] == 0 {
                break;
            }

            let record_length = data[pos] as usize;
            if record_length == 0 || pos + record_length > data.len() {
                break;
            }

            // Parse directory record
            if let Some(record) = DirectoryRecord::from_bytes(&data[pos..pos + record_length]) {
                // Skip "." and ".." entries
                let name = record.file_name();
                if name != "." && name != ".." {
                    entries.push(record);
                }
            }

            pos += record_length;

            // Skip padding to align to even byte boundary
            if pos % 2 != 0 {
                pos += 1;
            }
        }

        Ok(entries)
    }

    /// Read file data from a file record
    pub fn read_file(
        &self,
        stream: &mut dyn ReadSeek,
        file: &DirectoryRecord,
    ) -> Result<Vec<u8>> {
        if file.is_directory() {
            return Err(Error::invalid_territory("Cannot read directory as file".to_string()));
        }

        let extent_lba = file.extent_location.get();
        let data_length = file.data_length.get();

        // Seek to file extent
        let offset = extent_lba as u64 * SECTOR_SIZE as u64;
        stream.seek(SeekFrom::Start(offset))?;

        // Read file data
        let mut data = vec![0u8; data_length as usize];
        stream.read_exact(&mut data)?;

        Ok(data)
    }
}

impl Territory for IsoTerritory {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn banner(&self) -> Result<String> {
        Ok(self.primary_descriptor.volume_label())
    }

    fn set_banner(&mut self, _label: &str) -> Result<()> {
        Err(Error::unsupported("ISO-9660 is read-only".to_string()))
    }

    fn headquarters(&self) -> Result<Box<dyn DirectoryCell>> {
        Ok(Box::new(IsoRootDirectory {
            root: self.root_directory.clone(),
        }))
    }

    fn domain_size(&self) -> u64 {
        let block_count = self.primary_descriptor.volume_space_size.get() as u64;
        let block_size = self.primary_descriptor.logical_block_size.get() as u64;
        block_count * block_size
    }

    fn liberated_space(&self) -> u64 {
        // ISO-9660 is read-only, no concept of free space
        0
    }

    fn block_size(&self) -> u64 {
        self.primary_descriptor.logical_block_size.get() as u64
    }

    fn hierarchical(&self) -> bool {
        true // ISO-9660 supports subdirectories
    }

    fn navigate_to(&self, _path: &str) -> Result<Box<dyn DirectoryCell>> {
        // Simplified: always return root directory
        // Full implementation would parse path and traverse directories
        self.headquarters()
    }

    fn extract_file(&mut self, _path: &str) -> Result<Vec<u8>> {
        // Simplified: return empty
        // Full implementation would parse path, find file, read data
        Ok(Vec::new())
    }
}

/// ISO-9660 root directory cell
struct IsoRootDirectory {
    root: DirectoryRecord,
}

impl DirectoryCell for IsoRootDirectory {
    fn name(&self) -> &str {
        "/"
    }

    fn list_occupants(&self) -> Result<Vec<OccupantInfo>> {
        // Simplified: return empty list
        // Full implementation would need access to the stream to read directory entries
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

    /// Create a minimal ISO-9660 volume with primary descriptor and terminator
    fn create_minimal_iso() -> Vec<u8> {
        let mut iso = vec![0u8; 64 * 1024]; // 32 sectors minimum

        // Sector 16: Primary Volume Descriptor
        let pvd_offset = VOLUME_DESCRIPTOR_START as usize;

        // Type: Primary (1)
        iso[pvd_offset] = 1;

        // Identifier: "CD001"
        iso[pvd_offset + 1..pvd_offset + 6].copy_from_slice(b"CD001");

        // Version: 1
        iso[pvd_offset + 6] = 1;

        // System Identifier (32 bytes, padded with spaces)
        let system_id = b"LINUX                           ";
        iso[pvd_offset + 8..pvd_offset + 40].copy_from_slice(system_id);

        // Volume Identifier (32 bytes, padded with spaces)
        let volume_id = b"TEST_ISO                        ";
        iso[pvd_offset + 40..pvd_offset + 72].copy_from_slice(volume_id);

        // Volume Space Size (both-endian u32): 32 blocks
        let volume_size = 32u32;
        iso[pvd_offset + 80..pvd_offset + 84].copy_from_slice(&volume_size.to_le_bytes());
        iso[pvd_offset + 84..pvd_offset + 88].copy_from_slice(&volume_size.to_be_bytes());

        // Volume Set Size (both-endian u16): 1
        iso[pvd_offset + 120..pvd_offset + 122].copy_from_slice(&1u16.to_le_bytes());
        iso[pvd_offset + 122..pvd_offset + 124].copy_from_slice(&1u16.to_be_bytes());

        // Volume Sequence Number (both-endian u16): 1
        iso[pvd_offset + 124..pvd_offset + 126].copy_from_slice(&1u16.to_le_bytes());
        iso[pvd_offset + 126..pvd_offset + 128].copy_from_slice(&1u16.to_be_bytes());

        // Logical Block Size (both-endian u16): 2048
        let block_size = 2048u16;
        iso[pvd_offset + 128..pvd_offset + 130].copy_from_slice(&block_size.to_le_bytes());
        iso[pvd_offset + 130..pvd_offset + 132].copy_from_slice(&block_size.to_be_bytes());

        // Path Table Size (both-endian u32): 0 (minimal)
        iso[pvd_offset + 132..pvd_offset + 136].copy_from_slice(&0u32.to_le_bytes());
        iso[pvd_offset + 136..pvd_offset + 140].copy_from_slice(&0u32.to_be_bytes());

        // L Path Table location: 19
        iso[pvd_offset + 140..pvd_offset + 144].copy_from_slice(&19u32.to_le_bytes());

        // M Path Table location: 20
        iso[pvd_offset + 151..pvd_offset + 155].copy_from_slice(&20u32.to_be_bytes());

        // Root Directory Record (34 bytes at offset 156)
        let root_offset = pvd_offset + 156;
        iso[root_offset] = 34; // Length
        iso[root_offset + 1] = 0; // Extended attribute length

        // Extent location (both-endian): sector 18
        let root_extent = 18u32;
        iso[root_offset + 2..root_offset + 6].copy_from_slice(&root_extent.to_le_bytes());
        iso[root_offset + 6..root_offset + 10].copy_from_slice(&root_extent.to_be_bytes());

        // Data length (both-endian): 2048 bytes
        iso[root_offset + 10..root_offset + 14].copy_from_slice(&2048u32.to_le_bytes());
        iso[root_offset + 14..root_offset + 18].copy_from_slice(&2048u32.to_be_bytes());

        // Recording date: 2024-01-15 12:00:00
        iso[root_offset + 18] = 124; // Year (1900 + 124 = 2024)
        iso[root_offset + 19] = 1; // Month
        iso[root_offset + 20] = 15; // Day
        iso[root_offset + 21] = 12; // Hour
        iso[root_offset + 22] = 0; // Minute
        iso[root_offset + 23] = 0; // Second
        iso[root_offset + 24] = 0; // GMT offset

        // File flags: Directory
        iso[root_offset + 25] = DirectoryRecord::FLAG_DIRECTORY;

        // Volume sequence number (both-endian): 1
        iso[root_offset + 28..root_offset + 30].copy_from_slice(&1u16.to_le_bytes());
        iso[root_offset + 30..root_offset + 32].copy_from_slice(&1u16.to_be_bytes());

        // File identifier length: 1 (root is special)
        iso[root_offset + 32] = 1;

        // File identifier: 0x00 (current directory)
        iso[root_offset + 33] = 0x00;

        // Fill in creation/modification dates (17 bytes each at various offsets)
        // Using simplified zeroed dates for this test
        for i in 0..17 {
            iso[pvd_offset + 813 + i] = b'0'; // Creation date
            iso[pvd_offset + 830 + i] = b'0'; // Modification date
            iso[pvd_offset + 847 + i] = b'0'; // Expiration date
            iso[pvd_offset + 864 + i] = b'0'; // Effective date
        }

        // File structure version
        iso[pvd_offset + 881] = 1;

        // Sector 17: Volume Descriptor Set Terminator
        let term_offset = pvd_offset + SECTOR_SIZE;
        iso[term_offset] = 255; // Type: Terminator
        iso[term_offset + 1..term_offset + 6].copy_from_slice(b"CD001");
        iso[term_offset + 6] = 1;

        iso
    }

    #[test]
    fn test_parse_iso() {
        let iso_data = create_minimal_iso();
        let mut cursor = Cursor::new(iso_data);
        let territory = IsoTerritory::parse(&mut cursor).unwrap();

        assert_eq!(territory.identify(), "ISO-9660 filesystem");
        assert_eq!(territory.primary_descriptor().logical_block_size.get(), 2048);
        assert_eq!(territory.primary_descriptor().volume_space_size.get(), 32);
    }

    #[test]
    fn test_iso_volume_label() {
        let iso_data = create_minimal_iso();
        let mut cursor = Cursor::new(iso_data);
        let territory = IsoTerritory::parse(&mut cursor).unwrap();

        let label = territory.banner().unwrap();
        assert_eq!(label, "TEST_ISO");
    }

    #[test]
    fn test_iso_territory_methods() {
        let iso_data = create_minimal_iso();
        let mut cursor = Cursor::new(iso_data);
        let mut territory = IsoTerritory::parse(&mut cursor).unwrap();

        assert_eq!(territory.identify(), "ISO-9660 filesystem");
        assert_eq!(territory.domain_size(), 32 * 2048);
        assert_eq!(territory.block_size(), 2048);
        assert_eq!(territory.liberated_space(), 0); // Read-only
        assert!(territory.hierarchical());
        assert!(territory.headquarters().is_ok());

        // Test read-only enforcement
        assert!(territory.set_banner("NEW_LABEL").is_err());
    }

    #[test]
    fn test_invalid_iso_identifier() {
        let mut iso_data = vec![0u8; 64 * 1024];
        let pvd_offset = VOLUME_DESCRIPTOR_START as usize;

        iso_data[pvd_offset] = 1; // Type: Primary
        iso_data[pvd_offset + 1..pvd_offset + 6].copy_from_slice(b"XXXXX"); // Invalid identifier

        let mut cursor = Cursor::new(iso_data);
        let result = IsoTerritory::parse(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_no_primary_descriptor() {
        let mut iso_data = vec![0u8; 64 * 1024];
        let pvd_offset = VOLUME_DESCRIPTOR_START as usize;

        // Only terminator, no primary descriptor
        iso_data[pvd_offset] = 255; // Type: Terminator
        iso_data[pvd_offset + 1..pvd_offset + 6].copy_from_slice(b"CD001");
        iso_data[pvd_offset + 6] = 1;

        let mut cursor = Cursor::new(iso_data);
        let result = IsoTerritory::parse(&mut cursor);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_directory() {
        let iso_data = create_minimal_iso();
        let mut cursor = Cursor::new(iso_data);
        let territory = IsoTerritory::parse(&mut cursor).unwrap();

        // Create a test directory record
        let root = &territory.root_directory;

        // Reading directory should not fail, but return empty (no files in minimal ISO)
        let entries = territory.read_directory(&mut cursor, root).unwrap();
        assert_eq!(entries.len(), 0); // Empty root directory in minimal ISO
    }

    #[test]
    fn test_directory_record_parsing() {
        // Test that we can parse the root directory record from our minimal ISO
        let iso_data = create_minimal_iso();
        let mut cursor = Cursor::new(iso_data);
        let territory = IsoTerritory::parse(&mut cursor).unwrap();

        let root = &territory.root_directory;
        assert!(root.is_directory());
        assert_eq!(root.extent_location.get(), 18);
        assert_eq!(root.data_length.get(), 2048);
    }
}
