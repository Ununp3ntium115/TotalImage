//! VHD (Virtual Hard Disk) vault implementation
//!
//! This module implements support for Microsoft VHD disk image format.
//!
//! ## Supported Formats
//!
//! - **Fixed VHD**: Simple format where data is stored contiguously with a footer at the end
//! - **Dynamic VHD**: Sparse format using a Block Allocation Table (BAT) for space efficiency
//!
//! ## Format Overview
//!
//! VHD files have a 512-byte footer at the end containing metadata.
//! - Fixed VHDs: Data from byte 0 to (file_size - 512)
//! - Dynamic VHDs: Data stored in blocks referenced by BAT

pub mod types;

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use totalimage_core::{ReadSeek, Result, Vault};
use totalimage_pipeline::{MmapPipeline, PartialPipeline};
use types::{BlockAllocationTable, ParentLocatorEntry, VhdDynamicHeader, VhdFooter, VhdType};

use crate::VaultConfig;

/// VHD vault - Microsoft Virtual Hard Disk container
pub struct VhdVault {
    pipeline: Box<dyn ReadSeek>,
    footer: VhdFooter,
    dynamic_header: Option<VhdDynamicHeader>,
    bat: Option<BlockAllocationTable>,
}

impl VhdVault {
    /// Open a VHD vault from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the VHD file
    /// * `config` - Configuration for opening the vault
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be opened
    /// - The VHD footer is invalid or corrupted
    /// - The dynamic header or BAT is invalid (for dynamic VHDs)
    pub fn open(path: &Path, config: VaultConfig) -> Result<Self> {
        let mut file = File::open(path)?;
        let file_len = file.metadata()?.len();

        if file_len < VhdFooter::SIZE as u64 {
            return Err(totalimage_core::Error::invalid_vault(
                "File too small to be a VHD",
            ));
        }

        // Read footer from last 512 bytes
        file.seek(SeekFrom::End(-(VhdFooter::SIZE as i64)))?;
        let mut footer_bytes = [0u8; VhdFooter::SIZE];
        file.read_exact(&mut footer_bytes)?;

        let footer = VhdFooter::parse(&footer_bytes)?;

        // Verify footer checksum
        if !footer.verify_checksum() {
            return Err(totalimage_core::Error::invalid_vault(
                "VHD footer checksum verification failed",
            ));
        }

        // Handle different VHD types
        match footer.disk_type {
            VhdType::Fixed => {
                // Fixed VHD: content is everything except the footer
                let file = File::open(path)?;
                let base: Box<dyn ReadSeek> = if config.use_mmap {
                    Box::new(MmapPipeline::from_file(&file)?)
                } else {
                    Box::new(file)
                };

                let content_len = file_len - VhdFooter::SIZE as u64;
                let pipeline = Box::new(PartialPipeline::new(base, 0, content_len)?);

                Ok(Self {
                    pipeline,
                    footer,
                    dynamic_header: None,
                    bat: None,
                })
            }
            VhdType::Dynamic | VhdType::Differencing => {
                // Dynamic VHD: read dynamic header and BAT
                if footer.data_offset == 0xFFFFFFFFFFFFFFFF {
                    return Err(totalimage_core::Error::invalid_vault(
                        "Dynamic VHD has invalid data offset",
                    ));
                }

                // Read dynamic header
                file.seek(SeekFrom::Start(footer.data_offset))?;
                let mut dyn_header_bytes = [0u8; VhdDynamicHeader::SIZE];
                file.read_exact(&mut dyn_header_bytes)?;

                let dynamic_header = VhdDynamicHeader::parse(&dyn_header_bytes)?;

                // Verify dynamic header checksum
                if !dynamic_header.verify_checksum() {
                    return Err(totalimage_core::Error::invalid_vault(
                        "VHD dynamic header checksum verification failed",
                    ));
                }

                // Read Block Allocation Table
                file.seek(SeekFrom::Start(dynamic_header.table_offset))?;
                let bat_size = dynamic_header.max_table_entries as usize * 4;
                let mut bat_bytes = vec![0u8; bat_size];
                file.read_exact(&mut bat_bytes)?;

                let bat = BlockAllocationTable::parse(&bat_bytes, dynamic_header.block_size)?;

                // Create dynamic pipeline
                let file = File::open(path)?;
                let base: Box<dyn ReadSeek> = if config.use_mmap {
                    Box::new(MmapPipeline::from_file(&file)?)
                } else {
                    Box::new(file)
                };

                let pipeline = Box::new(VhdDynamicPipeline::new(
                    base,
                    bat.clone(),
                    footer.current_size,
                )?);

                Ok(Self {
                    pipeline,
                    footer,
                    dynamic_header: Some(dynamic_header),
                    bat: Some(bat),
                })
            }
            _ => Err(totalimage_core::Error::invalid_vault(format!(
                "Unsupported VHD type: {:?}",
                footer.disk_type
            ))),
        }
    }

    /// Get the VHD footer
    pub fn footer(&self) -> &VhdFooter {
        &self.footer
    }

    /// Get the dynamic header (if this is a dynamic/differencing VHD)
    pub fn dynamic_header(&self) -> Option<&VhdDynamicHeader> {
        self.dynamic_header.as_ref()
    }

    /// Get the block allocation table (if this is a dynamic/differencing VHD)
    pub fn bat(&self) -> Option<&BlockAllocationTable> {
        self.bat.as_ref()
    }

    /// Check if this is a dynamic VHD
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self.footer.disk_type,
            VhdType::Dynamic | VhdType::Differencing
        )
    }

    /// Check if this is a differencing VHD
    pub fn is_differencing(&self) -> bool {
        self.footer.disk_type == VhdType::Differencing
    }

    /// Get parent UUID (for differencing VHDs)
    pub fn parent_uuid(&self) -> Option<[u8; 16]> {
        if self.is_differencing() {
            self.dynamic_header.as_ref().map(|h| h.parent_uuid)
        } else {
            None
        }
    }

    /// Get parent name from the dynamic header (for differencing VHDs)
    pub fn parent_name(&self) -> Option<String> {
        if self.is_differencing() {
            self.dynamic_header.as_ref().and_then(|h| h.parent_name())
        } else {
            None
        }
    }

    /// Get parent locator entries (for differencing VHDs)
    pub fn parent_locators(&self) -> Vec<ParentLocatorEntry> {
        if self.is_differencing() {
            self.dynamic_header
                .as_ref()
                .map(|h| h.parent_locators())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Read the parent path from a locator entry
    pub fn read_parent_path(&mut self, locator: &ParentLocatorEntry) -> Result<Option<String>> {
        if locator.platform_data_length == 0 {
            return Ok(None);
        }

        // Seek to the locator data
        self.pipeline.seek(SeekFrom::Start(locator.platform_data_offset))?;

        let mut data = vec![0u8; locator.platform_data_length as usize];
        self.pipeline.read_exact(&mut data)?;

        if locator.is_windows_unicode() {
            // UTF-16LE encoded path
            let utf16: Vec<u16> = data
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect();

            // Trim null terminator
            let end = utf16.iter().position(|&c| c == 0).unwrap_or(utf16.len());
            String::from_utf16(&utf16[..end])
                .map(Some)
                .map_err(|_| totalimage_core::Error::invalid_vault("Invalid UTF-16 path"))
        } else if locator.is_windows_ansi() {
            // ANSI path (assuming UTF-8 or ASCII)
            let end = data.iter().position(|&c| c == 0).unwrap_or(data.len());
            Ok(Some(String::from_utf8_lossy(&data[..end]).to_string()))
        } else {
            // Unknown platform - try as UTF-8
            let end = data.iter().position(|&c| c == 0).unwrap_or(data.len());
            Ok(Some(String::from_utf8_lossy(&data[..end]).to_string()))
        }
    }

    /// Resolve the parent VHD path relative to this VHD's location
    pub fn resolve_parent_path(&self, vhd_path: &Path, parent_path: &str) -> PathBuf {
        let parent = PathBuf::from(parent_path);

        // If the parent path is absolute, use it directly
        if parent.is_absolute() {
            return parent;
        }

        // Otherwise, resolve relative to the child VHD's directory
        if let Some(parent_dir) = vhd_path.parent() {
            parent_dir.join(&parent)
        } else {
            parent
        }
    }

    /// Open a differencing VHD with automatic parent chain resolution
    ///
    /// This opens the differencing VHD and all its parent VHDs, returning
    /// a vault that reads from the complete chain.
    pub fn open_with_parents(path: &Path, config: VaultConfig) -> Result<VhdChainVault> {
        VhdChainVault::open(path, config)
    }
}

/// VHD Chain Vault - Handles differencing VHDs with parent chains
///
/// This vault resolves the parent chain and reads data from the
/// appropriate VHD in the chain (child VHD for modified blocks,
/// parent VHD for unmodified blocks).
pub struct VhdChainVault {
    /// VHD chain from child (index 0) to root parent (last index)
    chain: Vec<VhdVault>,
    /// Virtual disk size
    virtual_size: u64,
    /// Block size (from child VHD)
    block_size: u32,
    /// Current read position
    position: u64,
}

impl VhdChainVault {
    /// Open a VHD chain starting from a differencing VHD
    pub fn open(path: &Path, config: VaultConfig) -> Result<Self> {
        let mut chain = Vec::new();
        let mut current_path = path.to_path_buf();

        // Follow the parent chain until we reach a non-differencing VHD
        loop {
            let mut vault = VhdVault::open(&current_path, config.clone())?;

            if vault.is_differencing() {
                // Find parent path from locators
                let parent_path = Self::find_parent_path(&mut vault, &current_path)?;
                chain.push(vault);

                if let Some(parent) = parent_path {
                    current_path = parent;
                } else {
                    return Err(totalimage_core::Error::invalid_vault(
                        "Differencing VHD has no valid parent locator",
                    ));
                }
            } else {
                // Non-differencing VHD (root of chain)
                chain.push(vault);
                break;
            }

            // Prevent infinite loops (max chain depth)
            if chain.len() > 256 {
                return Err(totalimage_core::Error::invalid_vault(
                    "VHD parent chain exceeds maximum depth",
                ));
            }
        }

        // Get virtual size and block size from child VHD
        let virtual_size = chain[0].footer.current_size;
        let block_size = chain[0]
            .dynamic_header
            .as_ref()
            .map(|h| h.block_size)
            .unwrap_or(2 * 1024 * 1024); // Default 2MB

        Ok(Self {
            chain,
            virtual_size,
            block_size,
            position: 0,
        })
    }

    /// Find the parent path from a differencing VHD
    fn find_parent_path(vault: &mut VhdVault, child_path: &Path) -> Result<Option<PathBuf>> {
        // First try the parent locators
        for locator in vault.parent_locators() {
            if let Ok(Some(path_str)) = vault.read_parent_path(&locator) {
                let resolved = vault.resolve_parent_path(child_path, &path_str);
                if resolved.exists() {
                    return Ok(Some(resolved));
                }
            }
        }

        // Fall back to parent name from dynamic header
        if let Some(parent_name) = vault.parent_name() {
            let resolved = vault.resolve_parent_path(child_path, &parent_name);
            if resolved.exists() {
                return Ok(Some(resolved));
            }
        }

        Ok(None)
    }

    /// Get the number of VHDs in the chain
    pub fn chain_depth(&self) -> usize {
        self.chain.len()
    }

    /// Check if a block is allocated in a specific VHD in the chain
    fn is_block_allocated(&self, chain_index: usize, block_index: usize) -> bool {
        self.chain
            .get(chain_index)
            .and_then(|v| v.bat.as_ref())
            .and_then(|bat| bat.get_block_offset(block_index))
            .is_some()
    }

    /// Find which VHD in the chain has a block allocated
    fn find_block_owner(&self, block_index: usize) -> usize {
        for i in 0..self.chain.len() {
            if self.is_block_allocated(i, block_index) {
                return i;
            }
        }
        // If no VHD has it allocated, use the root parent
        self.chain.len() - 1
    }

    /// Read data from a specific VHD at a given offset
    fn read_from_chain(&mut self, chain_index: usize, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        if chain_index >= self.chain.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid chain index",
            ));
        }

        let vault = &mut self.chain[chain_index];
        vault.pipeline.seek(SeekFrom::Start(offset))?;
        vault.pipeline.read(buf)
    }
}

impl Read for VhdChainVault {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.virtual_size {
            return Ok(0);
        }

        let remaining = (self.virtual_size - self.position) as usize;
        let to_read = buf.len().min(remaining);

        let mut total_read = 0;

        while total_read < to_read {
            let current_offset = self.position + total_read as u64;
            let block_index = (current_offset / self.block_size as u64) as usize;
            let block_offset = current_offset % self.block_size as u64;

            // Find which VHD owns this block
            let owner = self.find_block_owner(block_index);

            // Calculate how much we can read from this block
            let remaining_in_block = self.block_size as u64 - block_offset;
            let chunk_size = ((to_read - total_read) as u64).min(remaining_in_block) as usize;

            // Calculate the virtual offset for reading
            let virtual_offset = block_index as u64 * self.block_size as u64 + block_offset;

            // Read from the appropriate VHD
            let bytes_read = self.read_from_chain(
                owner,
                virtual_offset,
                &mut buf[total_read..total_read + chunk_size],
            )?;

            if bytes_read == 0 {
                break;
            }

            total_read += bytes_read;
        }

        self.position += total_read as u64;
        Ok(total_read)
    }
}

impl Seek for VhdChainVault {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.virtual_size as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before beginning of VHD chain",
            ));
        }

        self.position = (new_pos as u64).min(self.virtual_size);
        Ok(self.position)
    }
}

impl Vault for VhdChainVault {
    fn identify(&self) -> &str {
        "Microsoft VHD (Differencing Chain)"
    }

    fn length(&self) -> u64 {
        self.virtual_size
    }

    fn content(&mut self) -> &mut dyn ReadSeek {
        self
    }
}

// SAFETY: VhdChainVault is safe to Send and Sync because:
// - `chain` (Vec<VhdVault>): Each VhdVault contains file handles (Send+Sync on all platforms)
//   and plain data structures (footer, dynamic_header, bat, block_data)
// - `virtual_size`, `block_size`, `position`: Plain numeric types
//
// Concurrent access requires external synchronization (e.g., Mutex) because:
// - position tracking requires exclusive access for sequential reads
// - VHD chain traversal modifies internal state during reads
unsafe impl Send for VhdChainVault {}
unsafe impl Sync for VhdChainVault {}

impl Vault for VhdVault {
    fn identify(&self) -> &str {
        match self.footer.disk_type {
            VhdType::Fixed => "Microsoft VHD (Fixed)",
            VhdType::Dynamic => "Microsoft VHD (Dynamic)",
            VhdType::Differencing => "Microsoft VHD (Differencing)",
            _ => "Microsoft VHD",
        }
    }

    fn length(&self) -> u64 {
        self.footer.current_size
    }

    fn content(&mut self) -> &mut dyn ReadSeek {
        &mut *self.pipeline
    }
}

/// Pipeline for dynamic VHD files
///
/// This pipeline translates virtual offsets to physical offsets using the BAT.
struct VhdDynamicPipeline<R: Read + Seek> {
    base: R,
    bat: BlockAllocationTable,
    virtual_size: u64,
    position: u64,
}

impl<R: Read + Seek> VhdDynamicPipeline<R> {
    fn new(base: R, bat: BlockAllocationTable, virtual_size: u64) -> Result<Self> {
        Ok(Self {
            base,
            bat,
            virtual_size,
            position: 0,
        })
    }
}

impl<R: Read + Seek> Read for VhdDynamicPipeline<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.position >= self.virtual_size {
            return Ok(0); // EOF
        }

        // Calculate how much we can read
        let remaining = (self.virtual_size - self.position) as usize;
        let to_read = buf.len().min(remaining);

        let mut total_read = 0;

        while total_read < to_read {
            let current_offset = self.position + total_read as u64;

            // Get block index and offset within block
            let block_index = self.bat.offset_to_block(current_offset);
            let block_offset = self.bat.offset_within_block(current_offset);

            // Calculate how much we can read from this block
            let remaining_in_block = self.bat.block_size as u64 - block_offset;
            let chunk_size = ((to_read - total_read) as u64).min(remaining_in_block) as usize;

            // Check if block is allocated
            if let Some(physical_offset) = self.bat.get_block_offset(block_index) {
                // Block is allocated: read from physical location
                // Note: Each block has a 512-byte bitmap at the start
                let bitmap_size = 512u64;
                let physical_pos = physical_offset + bitmap_size + block_offset;

                self.base.seek(SeekFrom::Start(physical_pos))?;
                let bytes_read = self.base.read(&mut buf[total_read..total_read + chunk_size])?;

                if bytes_read == 0 {
                    break; // Unexpected EOF
                }

                total_read += bytes_read;
            } else {
                // Block is not allocated (sparse): return zeros
                for i in 0..chunk_size {
                    buf[total_read + i] = 0;
                }
                total_read += chunk_size;
            }
        }

        self.position += total_read as u64;
        Ok(total_read)
    }
}

impl<R: Read + Seek> Seek for VhdDynamicPipeline<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.virtual_size as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before beginning of VHD",
            ));
        }

        let new_pos = new_pos as u64;
        if new_pos > self.virtual_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek beyond end of VHD",
            ));
        }

        self.position = new_pos;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use types::DiskGeometry;

    /// Create a synthetic fixed VHD for testing
    fn create_test_fixed_vhd(data_size: usize) -> Vec<u8> {
        let mut vhd = Vec::new();

        // Add data
        let data: Vec<u8> = (0..data_size).map(|i| (i % 256) as u8).collect();
        vhd.extend_from_slice(&data);

        // Create footer
        let footer = create_test_footer(data_size as u64, VhdType::Fixed);
        let mut footer_bytes = [0u8; VhdFooter::SIZE];
        footer.serialize(&mut footer_bytes);
        vhd.extend_from_slice(&footer_bytes);

        vhd
    }

    /// Create a test footer with valid checksum
    fn create_test_footer(size: u64, disk_type: VhdType) -> VhdFooter {
        let geometry = DiskGeometry {
            cylinders: 1024,
            heads: 16,
            sectors: 63,
        };

        let mut footer = VhdFooter {
            cookie: *VhdFooter::COOKIE,
            features: 0x00000002,
            version: 0x00010000,
            data_offset: if disk_type == VhdType::Fixed {
                0xFFFFFFFFFFFFFFFF
            } else {
                512
            },
            timestamp: 0,
            creator_app: *b"test",
            creator_version: 0x00010000,
            creator_os: 0x5769326B, // Wi2k
            original_size: size,
            current_size: size,
            geometry,
            disk_type,
            checksum: 0,
            uuid: [0u8; 16],
            saved_state: 0,
            reserved: [0u8; 427],
        };

        // Calculate checksum
        let mut bytes = [0u8; VhdFooter::SIZE];
        footer.serialize(&mut bytes);

        let mut sum: u32 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            if i >= 64 && i < 68 {
                continue;
            }
            sum = sum.wrapping_add(byte as u32);
        }
        footer.checksum = !sum;

        footer
    }

    #[test]
    fn test_vhd_vault_fixed_open() {
        let vhd_data = create_test_fixed_vhd(1024);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        assert_eq!(vault.identify(), "Microsoft VHD (Fixed)");
        assert_eq!(vault.length(), 1024);
        assert!(!vault.is_dynamic());
    }

    #[test]
    fn test_vhd_vault_fixed_content_read() {
        let vhd_data = create_test_fixed_vhd(1024);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Read first 10 bytes
        let mut buf = [0u8; 10];
        vault.content().read(&mut buf).unwrap();

        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_vhd_vault_fixed_content_seek() {
        let vhd_data = create_test_fixed_vhd(1024);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Seek to offset 100
        vault.content().seek(SeekFrom::Start(100)).unwrap();

        let mut buf = [0u8; 5];
        vault.content().read(&mut buf).unwrap();

        assert_eq!(&buf, &[100, 101, 102, 103, 104]);
    }

    #[test]
    fn test_vhd_vault_invalid_footer() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let mut data = vec![0u8; 1024];

        // Write invalid footer
        data[512..520].copy_from_slice(b"notvalid");
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let result = VhdVault::open(tmpfile.path(), VaultConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_vhd_vault_footer_checksum_fail() {
        let mut vhd_data = create_test_fixed_vhd(1024);

        // Corrupt the checksum
        let checksum_offset = 1024 + 64; // data size + offset to checksum in footer
        vhd_data[checksum_offset] ^= 0xFF;

        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let result = VhdVault::open(tmpfile.path(), VaultConfig::default());
        assert!(result.is_err());
        if let Err(totalimage_core::Error::InvalidVault(msg)) = result {
            assert!(msg.contains("checksum"));
        }
    }

    #[test]
    fn test_vhd_vault_file_too_small() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&[0u8; 100]).unwrap();
        tmpfile.flush().unwrap();

        let result = VhdVault::open(tmpfile.path(), VaultConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_vhd_footer_accessor() {
        let vhd_data = create_test_fixed_vhd(1024);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        let footer = vault.footer();
        assert_eq!(footer.current_size, 1024);
        assert_eq!(footer.disk_type, VhdType::Fixed);
    }

    #[test]
    fn test_vhd_dynamic_header_none_for_fixed() {
        let vhd_data = create_test_fixed_vhd(1024);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        assert!(vault.dynamic_header().is_none());
        assert!(vault.bat().is_none());
    }

    #[test]
    fn test_vhd_vault_content_full_read() {
        let data_size = 512;
        let vhd_data = create_test_fixed_vhd(data_size);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Read all content
        let mut buf = vec![0u8; data_size];
        vault.content().read_exact(&mut buf).unwrap();

        // Verify data
        for (i, &byte) in buf.iter().enumerate() {
            assert_eq!(byte, (i % 256) as u8);
        }
    }

    /// Create a synthetic dynamic VHD for testing
    fn create_test_dynamic_vhd(virtual_size: u64, block_size: u32, allocated_blocks: &[usize]) -> Vec<u8> {
        let mut vhd = Vec::new();

        // Calculate number of blocks needed
        let block_count = ((virtual_size + block_size as u64 - 1) / block_size as u64) as u32;

        // Create footer (at beginning for dynamic VHD)
        let footer = create_test_footer(virtual_size, VhdType::Dynamic);
        let mut footer_bytes = [0u8; VhdFooter::SIZE];
        footer.serialize(&mut footer_bytes);
        vhd.extend_from_slice(&footer_bytes);

        // Create dynamic header
        let dyn_header = create_test_dynamic_header(block_count, block_size);
        let mut dyn_header_bytes = [0u8; VhdDynamicHeader::SIZE];
        dyn_header.serialize(&mut dyn_header_bytes);
        vhd.extend_from_slice(&dyn_header_bytes);

        // Calculate BAT offset (right after dynamic header)
        let bat_offset = VhdFooter::SIZE + VhdDynamicHeader::SIZE;

        // Create BAT
        let mut bat_entries = vec![0xFFFFFFFFu32; block_count as usize]; // All unallocated by default
        let mut next_sector = ((bat_offset + block_count as usize * 4 + 511) / 512) as u32; // Round up to next sector

        for &block_idx in allocated_blocks {
            if block_idx < block_count as usize {
                bat_entries[block_idx] = next_sector;
                // Each block has: 512-byte bitmap + block_size data
                let block_total_size = 512 + block_size;
                next_sector += (block_total_size + 511) / 512; // Round up to sectors
            }
        }

        // Write BAT
        for &entry in &bat_entries {
            vhd.extend_from_slice(&entry.to_be_bytes());
        }

        // Pad to sector boundary
        while vhd.len() % 512 != 0 {
            vhd.push(0);
        }

        // Write allocated blocks
        for &block_idx in allocated_blocks {
            if block_idx < block_count as usize {
                // Block bitmap (512 bytes, all bits set for simplicity)
                vhd.extend_from_slice(&[0xFFu8; 512]);

                // Block data
                for i in 0..block_size {
                    let virtual_offset = block_idx as u64 * block_size as u64 + i as u64;
                    vhd.push((virtual_offset % 256) as u8);
                }

                // Pad to sector boundary
                while vhd.len() % 512 != 0 {
                    vhd.push(0);
                }
            }
        }

        // Add footer at the end
        vhd.extend_from_slice(&footer_bytes);

        vhd
    }

    /// Create a test dynamic header with valid checksum
    fn create_test_dynamic_header(max_table_entries: u32, block_size: u32) -> VhdDynamicHeader {
        let bat_offset = VhdFooter::SIZE + VhdDynamicHeader::SIZE;

        let mut header = VhdDynamicHeader {
            cookie: *VhdDynamicHeader::COOKIE,
            data_offset: 0xFFFFFFFFFFFFFFFF,
            table_offset: bat_offset as u64,
            header_version: 0x00010000,
            max_table_entries,
            block_size,
            checksum: 0,
            parent_uuid: [0u8; 16],
            parent_timestamp: 0,
            reserved1: 0,
            parent_unicode_name: [0u16; 256],
            parent_locator_entries: [[0u8; 24]; 8],
            reserved2: [0u8; 256],
        };

        // Calculate checksum
        let mut bytes = [0u8; VhdDynamicHeader::SIZE];
        header.serialize(&mut bytes);

        let mut sum: u32 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            if i >= 36 && i < 40 {
                continue;
            }
            sum = sum.wrapping_add(byte as u32);
        }
        header.checksum = !sum;

        header
    }

    #[test]
    fn test_vhd_vault_dynamic_open() {
        let block_size = 2 * 1024 * 1024; // 2 MB blocks
        let virtual_size = 10 * 1024 * 1024; // 10 MB virtual disk
        let allocated_blocks = vec![0, 2, 4]; // Allocate blocks 0, 2, and 4

        let vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        assert_eq!(vault.identify(), "Microsoft VHD (Dynamic)");
        assert_eq!(vault.length(), virtual_size);
        assert!(vault.is_dynamic());
        assert!(vault.dynamic_header().is_some());
        assert!(vault.bat().is_some());
    }

    #[test]
    fn test_vhd_vault_dynamic_read_allocated_block() {
        let block_size = 4096; // Small blocks for testing
        let virtual_size = 16384; // 4 blocks total
        let allocated_blocks = vec![0, 2]; // Allocate blocks 0 and 2

        let vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Read from allocated block 0
        vault.content().seek(SeekFrom::Start(0)).unwrap();
        let mut buf = [0u8; 10];
        vault.content().read(&mut buf).unwrap();
        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_vhd_vault_dynamic_read_sparse_block() {
        let block_size = 4096; // Small blocks for testing
        let virtual_size = 16384; // 4 blocks total
        let allocated_blocks = vec![0]; // Only allocate block 0

        let vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Read from sparse block 1 (should return zeros)
        vault.content().seek(SeekFrom::Start(block_size as u64)).unwrap();
        let mut buf = [0u8; 100];
        vault.content().read(&mut buf).unwrap();

        // All zeros for sparse block
        assert_eq!(&buf[..], &[0u8; 100]);
    }

    #[test]
    fn test_vhd_vault_dynamic_cross_block_read() {
        let block_size = 4096; // Small blocks for testing
        let virtual_size = 16384; // 4 blocks total
        let allocated_blocks = vec![0, 1]; // Allocate blocks 0 and 1

        let vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Read across block boundary
        vault.content().seek(SeekFrom::Start(4090)).unwrap();
        let mut buf = [0u8; 12]; // Read 6 bytes from block 0, 6 from block 1
        vault.content().read(&mut buf).unwrap();

        // Verify data from both blocks
        let expected: Vec<u8> = (4090u64..4102u64).map(|i| (i % 256) as u8).collect();
        assert_eq!(&buf[..], &expected[..]);
    }

    #[test]
    fn test_vhd_vault_dynamic_header_checksum() {
        let block_size = 2 * 1024 * 1024;
        let virtual_size = 10 * 1024 * 1024;
        let allocated_blocks = vec![0];

        let mut vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);

        // Corrupt dynamic header checksum
        let checksum_offset = VhdFooter::SIZE + 36;
        vhd_data[checksum_offset] ^= 0xFF;

        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let result = VhdVault::open(tmpfile.path(), VaultConfig::default());
        assert!(result.is_err());
        if let Err(totalimage_core::Error::InvalidVault(msg)) = result {
            assert!(msg.contains("checksum"));
        }
    }

    #[test]
    fn test_vhd_dynamic_pipeline_seek() {
        let block_size = 4096;
        let virtual_size = 16384;
        let allocated_blocks = vec![0, 1, 2, 3]; // Allocate all blocks for this test

        let vhd_data = create_test_dynamic_vhd(virtual_size, block_size, &allocated_blocks);
        let mut tmpfile = NamedTempFile::new().unwrap();
        tmpfile.write_all(&vhd_data).unwrap();
        tmpfile.flush().unwrap();

        let mut vault = VhdVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        // Test seeking to various positions
        vault.content().seek(SeekFrom::Start(100)).unwrap();
        let mut buf = [0u8; 5];
        vault.content().read(&mut buf).unwrap();
        assert_eq!(&buf, &[100, 101, 102, 103, 104]);

        // Seek from current
        vault.content().seek(SeekFrom::Current(10)).unwrap();
        vault.content().read(&mut buf).unwrap();
        assert_eq!(&buf, &[115, 116, 117, 118, 119]);

        // Seek from end
        vault.content().seek(SeekFrom::End(-10)).unwrap();
        vault.content().read(&mut buf).unwrap();
        let expected: Vec<u8> = ((virtual_size - 10)..(virtual_size - 5))
            .map(|i| (i % 256) as u8)
            .collect();
        assert_eq!(&buf[..], &expected[..]);
    }
}
