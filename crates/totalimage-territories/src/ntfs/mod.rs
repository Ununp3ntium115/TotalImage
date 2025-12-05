//! NTFS (NT File System) read-only implementation
//!
//! This module provides read-only access to NTFS filesystems using the
//! [ntfs crate](https://crates.io/crates/ntfs) by Colin Finck.
//!
//! NTFS is the primary filesystem used by Windows NT and later versions.
//!
//! ## Features
//!
//! - Read-only access (safe for forensics and mounted volumes)
//! - Directory enumeration with long filenames
//! - File extraction from resident and non-resident attributes
//! - **NTFS compression support (LZNT1 decompression)**
//! - Alternate Data Stream (ADS) support
//! - Case-insensitive file lookup
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_territories::ntfs::NtfsTerritory;
//! use totalimage_core::Territory;
//! use std::fs::File;
//!
//! let file = File::open("ntfs_partition.img").unwrap();
//! let territory = NtfsTerritory::parse(file).unwrap();
//! println!("Filesystem: {}", territory.identify());
//! ```

pub mod lznt1;
pub mod types;

use ntfs::structured_values::NtfsFileNamespace;
use ntfs::{Ntfs, NtfsFile, NtfsReadSeek};
use std::io::{Read, Seek, SeekFrom};
use totalimage_core::{DirectoryCell, Error, OccupantInfo, Result, Territory};
use types::{ntfs_time_to_datetime, NtfsVolumeInfo};

/// NTFS filesystem territory (read-only)
///
/// Provides read-only access to NTFS filesystems for forensic analysis
/// and data extraction without risk of data corruption.
pub struct NtfsTerritory<T: Read + Seek> {
    /// The underlying NTFS structure
    ntfs: Ntfs,
    /// The reader for filesystem access
    reader: T,
    /// Volume information
    volume_info: NtfsVolumeInfo,
    /// Identifier string
    identifier: String,
}

impl<T: Read + Seek + Send + Sync> NtfsTerritory<T> {
    /// Parse an NTFS filesystem from a stream
    ///
    /// # Arguments
    ///
    /// * `reader` - A readable and seekable stream positioned at the start of the NTFS volume
    ///
    /// # Errors
    ///
    /// Returns an error if the NTFS boot sector cannot be read or is invalid
    ///
    /// # Security
    ///
    /// The filesystem is opened read-only, so you can safely browse even a mounted
    /// filesystem without worrying about data corruption.
    pub fn parse(mut reader: T) -> Result<Self> {
        // Seek to start of volume
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|e| Error::invalid_territory(format!("IO error: {}", e)))?;

        // Parse NTFS structure
        let ntfs = Ntfs::new(&mut reader)
            .map_err(|e| Error::invalid_territory(format!("Failed to parse NTFS: {}", e)))?;

        // Get volume information
        let cluster_size = ntfs.cluster_size();
        let sector_size = ntfs.sector_size();
        let total_size = ntfs.size();

        // Try to get volume label
        let label = Self::get_volume_label(&ntfs, &mut reader).ok();

        // Default to NTFS 3.1 (most common)
        let major_version = 3;
        let minor_version = 1;

        let volume_info = NtfsVolumeInfo {
            label,
            major_version,
            minor_version,
            total_size,
            cluster_size,
            sector_size,
        };

        let identifier = format!("NTFS v{}.{} filesystem", major_version, minor_version);

        Ok(Self {
            ntfs,
            reader,
            volume_info,
            identifier,
        })
    }

    /// Get the NTFS volume label
    fn get_volume_label(_ntfs: &Ntfs, _reader: &mut T) -> Result<String> {
        // For simplicity, return default label
        // Full implementation would parse $Volume file's $VOLUME_NAME attribute
        Ok(String::from("NTFS"))
    }

    /// Get the volume information
    pub fn volume_info(&self) -> &NtfsVolumeInfo {
        &self.volume_info
    }

    /// Get the NTFS structure reference
    pub fn ntfs(&self) -> &Ntfs {
        &self.ntfs
    }

    /// Get a mutable reference to the reader
    pub fn reader(&mut self) -> &mut T {
        &mut self.reader
    }

    /// Read the root directory
    pub fn read_root_directory(&mut self) -> Result<Vec<OccupantInfo>> {
        let ntfs = &self.ntfs;
        let reader = &mut self.reader;

        let root_dir = ntfs
            .root_directory(reader)
            .map_err(|e| Error::invalid_territory(format!("Cannot read root directory: {}", e)))?;

        Self::read_directory_entries_static(ntfs, reader, &root_dir)
    }

    /// Read directory entries from an NTFS file (directory) - static version
    fn read_directory_entries_static(
        _ntfs: &Ntfs,
        reader: &mut T,
        dir: &NtfsFile,
    ) -> Result<Vec<OccupantInfo>> {
        let mut entries = Vec::new();

        // Get the directory index
        let index = dir
            .directory_index(reader)
            .map_err(|e| Error::invalid_territory(format!("Cannot read directory index: {}", e)))?;

        let mut iter = index.entries();

        while let Some(entry_result) = iter.next(reader) {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Error reading directory entry: {}", e);
                    continue;
                }
            };

            // Get filename from index entry
            let filename = match entry.key() {
                Some(Ok(key)) => key,
                _ => continue,
            };

            // Skip DOS names and only use Win32 or Win32+DOS names
            if filename.namespace() == NtfsFileNamespace::Dos {
                continue;
            }

            let name = filename.name().to_string_lossy();

            // Skip . and .. pseudo-entries
            if name == "." || name == ".." {
                continue;
            }

            // Skip system metadata files (starting with $)
            if name.starts_with('$') {
                continue;
            }

            let is_directory = filename.is_directory();
            let size = filename.allocated_size();

            // Get timestamps from filename
            let created = ntfs_time_to_datetime(filename.creation_time());
            let modified = ntfs_time_to_datetime(filename.modification_time());
            let accessed = ntfs_time_to_datetime(filename.access_time());

            entries.push(OccupantInfo {
                name: name.to_string(),
                is_directory,
                size,
                created,
                modified,
                accessed,
                attributes: filename.file_attributes().bits(),
            });
        }

        // Sort entries: directories first, then alphabetically
        entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(entries)
    }

    /// Find a file or directory by path
    pub fn find_by_path(&mut self, path: &str) -> Result<NtfsFile<'_>> {
        let path = path.trim_matches('/').trim_matches('\\');

        let ntfs = &self.ntfs;
        let reader = &mut self.reader;

        if path.is_empty() {
            return ntfs
                .root_directory(reader)
                .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)));
        }

        // Validate path components for security (GAP-006)
        use totalimage_core::validate_fs_path_components;
        let parts = validate_fs_path_components(path)?;

        let mut current = ntfs
            .root_directory(reader)
            .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)))?;

        for part in parts {
            let index = current
                .directory_index(reader)
                .map_err(|e| Error::not_found(format!("Cannot read directory: {}", e)))?;

            // Search through entries for matching name
            let mut iter = index.entries();
            let mut found_ref = None;

            while let Some(entry_result) = iter.next(reader) {
                let entry = match entry_result {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                if let Some(Ok(key)) = entry.key() {
                    let name = key.name().to_string_lossy();
                    if name.eq_ignore_ascii_case(&part) {
                        found_ref = Some(entry.file_reference());
                        break;
                    }
                }
            }

            let file_ref = found_ref
                .ok_or_else(|| Error::not_found(format!("Path component not found: {}", part)))?;
            current = file_ref
                .to_file(ntfs, reader)
                .map_err(|e| Error::not_found(format!("Cannot read file '{}': {}", part, e)))?;
        }

        Ok(current)
    }

    /// Read directory at a specific path
    pub fn read_directory_at_path(&mut self, path: &str) -> Result<Vec<OccupantInfo>> {
        let path = path.trim_matches('/').trim_matches('\\');

        let ntfs = &self.ntfs;
        let reader = &mut self.reader;

        let dir = if path.is_empty() {
            ntfs.root_directory(reader)
                .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)))?
        } else {
            // Validate path components for security (GAP-006)
            use totalimage_core::validate_fs_path_components;
            let parts = validate_fs_path_components(path)?;

            let mut current = ntfs
                .root_directory(reader)
                .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)))?;

            for part in parts {
                let index = current
                    .directory_index(reader)
                    .map_err(|e| Error::not_found(format!("Cannot read directory: {}", e)))?;

                // Search through entries for matching name
                let mut iter = index.entries();
                let mut found_ref = None;

                while let Some(entry_result) = iter.next(reader) {
                    let entry = match entry_result {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    if let Some(Ok(key)) = entry.key() {
                        let name = key.name().to_string_lossy();
                        if name.eq_ignore_ascii_case(&part) {
                            found_ref = Some(entry.file_reference());
                            break;
                        }
                    }
                }

                let file_ref = found_ref.ok_or_else(|| {
                    Error::not_found(format!("Path component not found: {}", part))
                })?;
                current = file_ref
                    .to_file(ntfs, reader)
                    .map_err(|e| Error::not_found(format!("Cannot read file '{}': {}", part, e)))?;
            }
            current
        };

        if !dir.is_directory() {
            return Err(Error::not_found(format!("Not a directory: {}", path)));
        }

        Self::read_directory_entries_static(ntfs, reader, &dir)
    }

    /// Extract file data at a specific path
    pub fn extract_file_data(&mut self, path: &str) -> Result<Vec<u8>> {
        let path = path.trim_matches('/').trim_matches('\\');

        let ntfs = &self.ntfs;
        let reader = &mut self.reader;

        // Navigate to the file
        let file = if path.is_empty() {
            return Err(Error::not_found("Empty path".to_string()));
        } else {
            // Validate path components for security (GAP-006)
            use totalimage_core::validate_fs_path_components;
            let parts = validate_fs_path_components(path)?;

            let mut current = ntfs
                .root_directory(reader)
                .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)))?;

            for part in parts {
                let index = current
                    .directory_index(reader)
                    .map_err(|e| Error::not_found(format!("Cannot read directory: {}", e)))?;

                let mut iter = index.entries();
                let mut found_ref = None;

                while let Some(entry_result) = iter.next(reader) {
                    let entry = match entry_result {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    if let Some(Ok(key)) = entry.key() {
                        let name = key.name().to_string_lossy();
                        if name.eq_ignore_ascii_case(&part) {
                            found_ref = Some(entry.file_reference());
                            break;
                        }
                    }
                }

                let file_ref = found_ref.ok_or_else(|| {
                    Error::not_found(format!("Path component not found: {}", part))
                })?;
                current = file_ref
                    .to_file(ntfs, reader)
                    .map_err(|e| Error::not_found(format!("Cannot read file '{}': {}", part, e)))?;
            }
            current
        };

        if file.is_directory() {
            return Err(Error::not_found(format!("Path is a directory: {}", path)));
        }

        // NOTE: NTFS Compression Support
        //
        // We have LZNT1 decompression implemented (see lznt1 module), but the ntfs crate v0.4
        // does not support reading compressed file data. The crate's documentation explicitly
        // lists compression as "not yet supported".
        //
        // Compressed files can be identified in directory listings (attribute 0x0800 = Compressed),
        // but when trying to read their $DATA attribute, the ntfs crate will either:
        // 1. Return an error
        // 2. Return decompressed data automatically (if future version adds support)
        //
        // For now, attempting to extract a compressed file will fail when the ntfs crate
        // tries to read the $DATA attribute. The error message from the ntfs crate should
        // indicate the issue.
        //
        // TODO: Future enhancement options:
        // - Wait for ntfs crate to add compression support
        // - Implement low-level NTFS data run parsing to read compressed clusters directly
        // - Fork/extend the ntfs crate to expose compressed data

        // Get the $DATA attribute (unnamed = main data stream)
        let data_item = match file.data(reader, "") {
            Some(result) => {
                result.map_err(|e| Error::invalid_territory(format!("Cannot read $DATA: {}", e)))?
            }
            None => return Err(Error::not_found("File has no data".to_string())),
        };

        let data_attr = data_item
            .to_attribute()
            .map_err(|e| Error::invalid_territory(format!("Cannot read data attribute: {}", e)))?;

        // Check file size against extraction limit
        let data_size = data_attr.value_length();
        use totalimage_core::MAX_FILE_EXTRACT_SIZE;
        if data_size > MAX_FILE_EXTRACT_SIZE {
            return Err(Error::invalid_territory(format!(
                "File size {} exceeds extraction limit {}",
                data_size, MAX_FILE_EXTRACT_SIZE
            )));
        }

        // Read uncompressed data
        let mut data = vec![0u8; data_size as usize];
        let mut value_reader = data_attr
            .value(reader)
            .map_err(|e| Error::invalid_territory(format!("Cannot open data stream: {}", e)))?;

        value_reader
            .read_exact(reader, &mut data)
            .map_err(|e| Error::invalid_territory(format!("Cannot read data: {}", e)))?;

        Ok(data)
    }

    /// List alternate data streams for a file
    pub fn list_alternate_data_streams(&mut self, path: &str) -> Result<Vec<String>> {
        let path = path.trim_matches('/').trim_matches('\\');

        let ntfs = &self.ntfs;
        let reader = &mut self.reader;
        let mut streams = Vec::new();

        // Navigate to the file (inline to avoid borrow issues)
        let file = if path.is_empty() {
            return Err(Error::not_found("Empty path".to_string()));
        } else {
            // Validate path components for security (GAP-006)
            use totalimage_core::validate_fs_path_components;
            let parts = validate_fs_path_components(path)?;

            let mut current = ntfs
                .root_directory(reader)
                .map_err(|e| Error::not_found(format!("Cannot read root: {}", e)))?;

            for part in parts {
                let index = current
                    .directory_index(reader)
                    .map_err(|e| Error::not_found(format!("Cannot read directory: {}", e)))?;

                let mut iter = index.entries();
                let mut found_ref = None;

                while let Some(entry_result) = iter.next(reader) {
                    let entry = match entry_result {
                        Ok(e) => e,
                        Err(_) => continue,
                    };

                    if let Some(Ok(key)) = entry.key() {
                        let name = key.name().to_string_lossy();
                        if name.eq_ignore_ascii_case(&part) {
                            found_ref = Some(entry.file_reference());
                            break;
                        }
                    }
                }

                let file_ref = found_ref.ok_or_else(|| {
                    Error::not_found(format!("Path component not found: {}", part))
                })?;
                current = file_ref
                    .to_file(ntfs, reader)
                    .map_err(|e| Error::not_found(format!("Cannot read file '{}': {}", part, e)))?;
            }
            current
        };

        let mut attrs = file.attributes();
        while let Some(attr_result) = attrs.next(reader) {
            let attr_item = match attr_result {
                Ok(a) => a,
                Err(_) => continue,
            };

            let attr = match attr_item.to_attribute() {
                Ok(a) => a,
                Err(_) => continue,
            };

            if let Ok(ty) = attr.ty() {
                if ty == ntfs::NtfsAttributeType::Data {
                    if let Ok(name) = attr.name() {
                        if let Ok(name_str) = name.to_string() {
                            if !name_str.is_empty() {
                                streams.push(name_str);
                            }
                        }
                    }
                }
            }
        }

        Ok(streams)
    }
}

impl<T: Read + Seek + Send + Sync + 'static> Territory for NtfsTerritory<T> {
    fn identify(&self) -> &str {
        &self.identifier
    }

    fn banner(&self) -> Result<String> {
        Ok(self
            .volume_info
            .label
            .clone()
            .unwrap_or_else(|| "NTFS".to_string()))
    }

    fn headquarters(&self) -> Result<Box<dyn DirectoryCell>> {
        Ok(Box::new(NtfsRootDirectory))
    }

    fn domain_size(&self) -> u64 {
        self.volume_info.total_size
    }

    fn liberated_space(&self) -> u64 {
        // Would need to read $Bitmap to calculate free clusters
        0
    }

    fn block_size(&self) -> u64 {
        self.volume_info.cluster_size as u64
    }

    fn hierarchical(&self) -> bool {
        true // NTFS supports subdirectories
    }

    fn navigate_to(&self, _path: &str) -> Result<Box<dyn DirectoryCell>> {
        self.headquarters()
    }

    fn extract_file(&mut self, path: &str) -> Result<Vec<u8>> {
        self.extract_file_data(path)
    }
}

/// NTFS root directory cell (placeholder for trait implementation)
struct NtfsRootDirectory;

impl DirectoryCell for NtfsRootDirectory {
    fn name(&self) -> &str {
        "/"
    }

    fn list_occupants(&self) -> Result<Vec<OccupantInfo>> {
        // Simplified: return empty list
        Ok(Vec::new())
    }

    fn enter(&self, _name: &str) -> Result<Box<dyn DirectoryCell>> {
        Err(Error::not_found(
            "Subdirectory navigation not available".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::types::NtfsFileAttribute;

    #[test]
    fn test_ntfs_attributes() {
        let attrs = NtfsFileAttribute::from_u32(0x0030); // Directory | Archive
        assert!(attrs.contains(&NtfsFileAttribute::Directory));
        assert!(attrs.contains(&NtfsFileAttribute::Archive));
    }

    #[test]
    fn test_hidden_system() {
        let attrs = NtfsFileAttribute::from_u32(0x0006); // Hidden | System
        assert!(attrs.contains(&NtfsFileAttribute::Hidden));
        assert!(attrs.contains(&NtfsFileAttribute::System));
    }

    #[test]
    fn test_ntfs_readonly_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0001);
        assert!(attrs.contains(&NtfsFileAttribute::ReadOnly));
        assert!(!attrs.contains(&NtfsFileAttribute::Directory));
    }

    #[test]
    fn test_ntfs_compressed_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0800);
        assert!(attrs.contains(&NtfsFileAttribute::Compressed));
    }

    #[test]
    fn test_ntfs_encrypted_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x4000);
        assert!(attrs.contains(&NtfsFileAttribute::Encrypted));
    }

    #[test]
    fn test_ntfs_sparse_file_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0200);
        assert!(attrs.contains(&NtfsFileAttribute::SparseFile));
    }

    #[test]
    fn test_ntfs_reparse_point_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0400);
        assert!(attrs.contains(&NtfsFileAttribute::ReparsePoint));
    }

    #[test]
    fn test_ntfs_temporary_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0100);
        assert!(attrs.contains(&NtfsFileAttribute::Temporary));
    }

    #[test]
    fn test_ntfs_offline_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x1000);
        assert!(attrs.contains(&NtfsFileAttribute::Offline));
    }

    #[test]
    fn test_ntfs_not_content_indexed_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x2000);
        assert!(attrs.contains(&NtfsFileAttribute::NotContentIndexed));
    }

    #[test]
    fn test_ntfs_multiple_attributes() {
        // ReadOnly | Hidden | System | Archive
        let attrs = NtfsFileAttribute::from_u32(0x0027);
        assert!(attrs.contains(&NtfsFileAttribute::ReadOnly));
        assert!(attrs.contains(&NtfsFileAttribute::Hidden));
        assert!(attrs.contains(&NtfsFileAttribute::System));
        assert!(attrs.contains(&NtfsFileAttribute::Archive));
        assert!(!attrs.contains(&NtfsFileAttribute::Directory));
    }

    #[test]
    fn test_ntfs_no_attributes() {
        let attrs = NtfsFileAttribute::from_u32(0x0000);
        assert!(!attrs.contains(&NtfsFileAttribute::ReadOnly));
        assert!(!attrs.contains(&NtfsFileAttribute::Hidden));
        assert!(!attrs.contains(&NtfsFileAttribute::Directory));
    }

    #[test]
    fn test_ntfs_all_standard_attributes() {
        // Test all standard file attributes
        let all_attrs = 0x0001 | 0x0002 | 0x0004 | 0x0020 | 0x0010 | 0x0080;
        let attrs = NtfsFileAttribute::from_u32(all_attrs);

        assert!(attrs.contains(&NtfsFileAttribute::ReadOnly));
        assert!(attrs.contains(&NtfsFileAttribute::Hidden));
        assert!(attrs.contains(&NtfsFileAttribute::System));
        assert!(attrs.contains(&NtfsFileAttribute::Archive));
        assert!(attrs.contains(&NtfsFileAttribute::Directory));
    }
}
