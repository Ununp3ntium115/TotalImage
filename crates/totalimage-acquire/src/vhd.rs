//! VHD image creation
//!
//! Creates Microsoft VHD (Virtual Hard Disk) images.
//! Supports Fixed and Dynamic VHD formats.

use crate::error::{AcquireError, Result};
use crate::hash::{HashAlgorithm, HashResult, Hasher};
use crate::progress::AcquireProgress;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// VHD type to create
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VhdOutputType {
    /// Fixed VHD - data followed by footer
    Fixed,
    /// Dynamic VHD - sparse with BAT
    Dynamic,
}

/// Options for VHD creation
#[derive(Debug, Clone)]
pub struct VhdOptions {
    /// Type of VHD to create
    pub vhd_type: VhdOutputType,
    /// Block size for dynamic VHD (default: 2 MB)
    pub block_size: u32,
    /// Hash algorithms to use during creation
    pub hash_algorithms: Vec<HashAlgorithm>,
    /// Creator application identifier (4 bytes)
    pub creator_app: [u8; 4],
}

impl Default for VhdOptions {
    fn default() -> Self {
        Self {
            vhd_type: VhdOutputType::Fixed,
            block_size: 2 * 1024 * 1024, // 2 MB
            hash_algorithms: vec![HashAlgorithm::Md5, HashAlgorithm::Sha256],
            creator_app: *b"tim\x00", // TotalImage
        }
    }
}

/// VHD image creator
pub struct VhdCreator {
    options: VhdOptions,
    cancel_flag: Arc<AtomicBool>,
}

impl VhdCreator {
    /// Create a new VHD creator with options
    pub fn new(options: VhdOptions) -> Self {
        Self {
            options,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get a cancel flag that can be used to abort creation
    pub fn cancel_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel_flag)
    }

    /// Create a fixed VHD from a reader
    pub fn create_fixed<R, W, F>(
        &self,
        source: &mut R,
        source_size: u64,
        dest: &mut W,
        progress_callback: Option<F>,
    ) -> Result<VhdCreationResult>
    where
        R: Read,
        W: Write + Seek,
        F: FnMut(&AcquireProgress),
    {
        let start_time = Instant::now();
        let mut hasher = Hasher::new(&self.options.hash_algorithms);
        let mut bytes_written = 0u64;
        let mut callback = progress_callback;

        // Buffer for copying
        let buffer_size = 1024 * 1024; // 1 MB
        let mut buffer = vec![0u8; buffer_size];

        // Copy source data
        loop {
            if self.cancel_flag.load(Ordering::Relaxed) {
                return Err(AcquireError::Cancelled);
            }

            let bytes_read = source.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }

            dest.write_all(&buffer[..bytes_read])?;
            hasher.update(&buffer[..bytes_read]);
            bytes_written += bytes_read as u64;

            if let Some(ref mut cb) = callback {
                let progress = AcquireProgress::calculate(
                    Some(source_size),
                    bytes_written,
                    start_time,
                    "Creating VHD",
                );
                cb(&progress);
            }
        }

        // Pad to source_size if needed
        if bytes_written < source_size {
            let padding = source_size - bytes_written;
            let zeros = vec![0u8; buffer_size.min(padding as usize)];
            let mut remaining = padding;
            while remaining > 0 {
                let to_write = (remaining as usize).min(zeros.len());
                dest.write_all(&zeros[..to_write])?;
                hasher.update(&zeros[..to_write]);
                remaining -= to_write as u64;
                bytes_written += to_write as u64;
            }
        }

        // Create and write footer
        let footer = create_vhd_footer(source_size, VhdType::Fixed, &self.options.creator_app);
        let footer_bytes = serialize_footer(&footer);
        dest.write_all(&footer_bytes)?;

        let elapsed = start_time.elapsed();
        let hashes = hasher.finalize();

        Ok(VhdCreationResult {
            bytes_written: bytes_written + 512, // Include footer
            source_size,
            elapsed,
            hashes,
            vhd_type: VhdOutputType::Fixed,
        })
    }

    /// Create a dynamic VHD from a reader
    pub fn create_dynamic<R, W, F>(
        &self,
        source: &mut R,
        source_size: u64,
        dest: &mut W,
        progress_callback: Option<F>,
    ) -> Result<VhdCreationResult>
    where
        R: Read + Seek,
        W: Write + Seek,
        F: FnMut(&AcquireProgress),
    {
        let start_time = Instant::now();
        let mut hasher = Hasher::new(&self.options.hash_algorithms);
        let mut callback = progress_callback;
        let block_size = self.options.block_size as u64;

        // Calculate number of blocks
        let num_blocks = (source_size + block_size - 1) / block_size;
        let bat_size = ((num_blocks * 4 + 511) / 512) * 512; // Round up to sector

        // Footer at start (copy)
        let footer = create_vhd_footer(source_size, VhdType::Dynamic, &self.options.creator_app);
        let footer_bytes = serialize_footer(&footer);
        dest.write_all(&footer_bytes)?;

        // Dynamic header at offset 512
        let header = create_dynamic_header(num_blocks as u32, self.options.block_size);
        let header_bytes = serialize_dynamic_header(&header);
        dest.write_all(&header_bytes)?;

        // BAT at offset 512 + 1024 = 1536
        // Initialize BAT with 0xFFFFFFFF (unused entries)
        let mut bat = vec![0xFFFFFFFFu32; num_blocks as usize];

        // Calculate where data blocks start
        let data_start = 512 + 1024 + bat_size;
        let mut current_block_offset = data_start;

        // Scan source for non-zero blocks and build BAT
        let mut buffer = vec![0u8; block_size as usize];
        let mut bytes_read_total = 0u64;
        let mut _blocks_written = 0u32;

        source.seek(SeekFrom::Start(0))?;

        for block_idx in 0..num_blocks {
            if self.cancel_flag.load(Ordering::Relaxed) {
                return Err(AcquireError::Cancelled);
            }

            // Read block from source
            let to_read = block_size.min(source_size - block_idx * block_size) as usize;
            buffer.fill(0);
            source.read_exact(&mut buffer[..to_read])?;
            hasher.update(&buffer[..to_read]);
            bytes_read_total += to_read as u64;

            // Check if block is all zeros
            let is_zero = buffer[..to_read].iter().all(|&b| b == 0);

            if !is_zero {
                // Record BAT entry (sector number)
                bat[block_idx as usize] = (current_block_offset / 512) as u32;
                current_block_offset += block_size + 512; // Block + sector bitmap
                _blocks_written += 1;
            }

            if let Some(ref mut cb) = callback {
                let progress = AcquireProgress::calculate(
                    Some(source_size),
                    bytes_read_total,
                    start_time,
                    "Scanning blocks",
                );
                cb(&progress);
            }
        }

        // Write BAT
        for entry in &bat {
            dest.write_all(&entry.to_be_bytes())?;
        }

        // Pad BAT to sector boundary
        let bat_written = num_blocks * 4;
        let bat_padding = bat_size - bat_written;
        if bat_padding > 0 {
            dest.write_all(&vec![0u8; bat_padding as usize])?;
        }

        // Second pass: write non-zero blocks
        source.seek(SeekFrom::Start(0))?;
        let mut bytes_written = 512 + 1024 + bat_size; // Footer + header + BAT

        for block_idx in 0..num_blocks {
            if self.cancel_flag.load(Ordering::Relaxed) {
                return Err(AcquireError::Cancelled);
            }

            let to_read = block_size.min(source_size - block_idx * block_size) as usize;
            buffer.fill(0);
            source.read_exact(&mut buffer[..to_read])?;

            let is_zero = buffer[..to_read].iter().all(|&b| b == 0);

            if !is_zero {
                // Write sector bitmap (all sectors present)
                let bitmap_size = ((block_size / 512 + 7) / 8) as usize;
                let bitmap_padded = ((bitmap_size + 511) / 512) * 512;
                let bitmap = vec![0xFFu8; bitmap_padded];
                dest.write_all(&bitmap)?;
                bytes_written += bitmap_padded as u64;

                // Write block data
                dest.write_all(&buffer)?;
                bytes_written += block_size;
            }
        }

        // Write footer at end
        dest.write_all(&footer_bytes)?;
        bytes_written += 512;

        let elapsed = start_time.elapsed();
        let hashes = hasher.finalize();

        Ok(VhdCreationResult {
            bytes_written,
            source_size,
            elapsed,
            hashes,
            vhd_type: VhdOutputType::Dynamic,
        })
    }
}

/// Result of VHD creation
#[derive(Debug)]
pub struct VhdCreationResult {
    /// Total bytes written to destination
    pub bytes_written: u64,
    /// Original source size
    pub source_size: u64,
    /// Time taken for creation
    pub elapsed: std::time::Duration,
    /// Hash results of source data
    pub hashes: Vec<HashResult>,
    /// Type of VHD created
    pub vhd_type: VhdOutputType,
}

impl VhdCreationResult {
    /// Get compression ratio (for dynamic VHD)
    pub fn compression_ratio(&self) -> f64 {
        if self.source_size == 0 {
            return 1.0;
        }
        self.bytes_written as f64 / self.source_size as f64
    }
}

/// VHD disk types
#[derive(Debug, Clone, Copy)]
enum VhdType {
    Fixed = 2,
    Dynamic = 3,
}

/// VHD footer structure
struct VhdFooterData {
    cookie: [u8; 8],
    features: u32,
    version: u32,
    data_offset: u64,
    timestamp: u32,
    creator_app: [u8; 4],
    creator_version: u32,
    creator_os: u32,
    original_size: u64,
    current_size: u64,
    cylinders: u16,
    heads: u8,
    sectors: u8,
    disk_type: u32,
    uuid: [u8; 16],
}

/// Create a VHD footer
fn create_vhd_footer(size: u64, vhd_type: VhdType, creator_app: &[u8; 4]) -> VhdFooterData {
    // Calculate CHS geometry
    let (cylinders, heads, sectors) = calculate_chs(size);

    // Generate UUID
    let uuid = generate_uuid();

    // VHD epoch is January 1, 2000 00:00:00
    let vhd_epoch = 946684800u64; // Unix timestamp
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let timestamp = (now.saturating_sub(vhd_epoch)) as u32;

    let data_offset = match vhd_type {
        VhdType::Fixed => 0xFFFFFFFFFFFFFFFFu64, // No dynamic header
        VhdType::Dynamic => 512, // Dynamic header follows footer
    };

    VhdFooterData {
        cookie: *b"conectix",
        features: 2, // Reserved (no features)
        version: 0x00010000, // Version 1.0
        data_offset,
        timestamp,
        creator_app: *creator_app,
        creator_version: 0x00010000,
        creator_os: 0x5769326B, // "Wi2k" (Windows)
        original_size: size,
        current_size: size,
        cylinders,
        heads,
        sectors,
        disk_type: vhd_type as u32,
        uuid,
    }
}

/// Serialize footer to bytes with checksum
fn serialize_footer(footer: &VhdFooterData) -> [u8; 512] {
    let mut bytes = [0u8; 512];

    bytes[0..8].copy_from_slice(&footer.cookie);
    bytes[8..12].copy_from_slice(&footer.features.to_be_bytes());
    bytes[12..16].copy_from_slice(&footer.version.to_be_bytes());
    bytes[16..24].copy_from_slice(&footer.data_offset.to_be_bytes());
    bytes[24..28].copy_from_slice(&footer.timestamp.to_be_bytes());
    bytes[28..32].copy_from_slice(&footer.creator_app);
    bytes[32..36].copy_from_slice(&footer.creator_version.to_be_bytes());
    bytes[36..40].copy_from_slice(&footer.creator_os.to_be_bytes());
    bytes[40..48].copy_from_slice(&footer.original_size.to_be_bytes());
    bytes[48..56].copy_from_slice(&footer.current_size.to_be_bytes());

    // Geometry
    bytes[56..58].copy_from_slice(&footer.cylinders.to_be_bytes());
    bytes[58] = footer.heads;
    bytes[59] = footer.sectors;

    bytes[60..64].copy_from_slice(&footer.disk_type.to_be_bytes());
    // Checksum at 64..68 - calculated below
    bytes[68..84].copy_from_slice(&footer.uuid);
    bytes[84] = 0; // saved_state
    // bytes[85..512] = reserved (zeros)

    // Calculate checksum
    let mut sum: u32 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if i >= 64 && i < 68 {
            continue; // Skip checksum field
        }
        sum = sum.wrapping_add(byte as u32);
    }
    let checksum = !sum;
    bytes[64..68].copy_from_slice(&checksum.to_be_bytes());

    bytes
}

/// VHD dynamic header structure
struct VhdDynamicHeaderData {
    cookie: [u8; 8],
    data_offset: u64,
    table_offset: u64,
    header_version: u32,
    max_table_entries: u32,
    block_size: u32,
    parent_uuid: [u8; 16],
    parent_timestamp: u32,
}

/// Create a dynamic header
fn create_dynamic_header(num_blocks: u32, block_size: u32) -> VhdDynamicHeaderData {
    VhdDynamicHeaderData {
        cookie: *b"cxsparse",
        data_offset: 0xFFFFFFFFFFFFFFFFu64, // Unused
        table_offset: 512 + 1024, // After footer + header
        header_version: 0x00010000,
        max_table_entries: num_blocks,
        block_size,
        parent_uuid: [0u8; 16],
        parent_timestamp: 0,
    }
}

/// Serialize dynamic header to bytes with checksum
fn serialize_dynamic_header(header: &VhdDynamicHeaderData) -> [u8; 1024] {
    let mut bytes = [0u8; 1024];

    bytes[0..8].copy_from_slice(&header.cookie);
    bytes[8..16].copy_from_slice(&header.data_offset.to_be_bytes());
    bytes[16..24].copy_from_slice(&header.table_offset.to_be_bytes());
    bytes[24..28].copy_from_slice(&header.header_version.to_be_bytes());
    bytes[28..32].copy_from_slice(&header.max_table_entries.to_be_bytes());
    bytes[32..36].copy_from_slice(&header.block_size.to_be_bytes());
    // Checksum at 36..40 - calculated below
    bytes[40..56].copy_from_slice(&header.parent_uuid);
    bytes[56..60].copy_from_slice(&header.parent_timestamp.to_be_bytes());
    // bytes[60..64] = reserved
    // bytes[64..576] = parent unicode name (zeros)
    // bytes[576..768] = parent locator entries (zeros)
    // bytes[768..1024] = reserved (zeros)

    // Calculate checksum
    let mut sum: u32 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if i >= 36 && i < 40 {
            continue; // Skip checksum field
        }
        sum = sum.wrapping_add(byte as u32);
    }
    let checksum = !sum;
    bytes[36..40].copy_from_slice(&checksum.to_be_bytes());

    bytes
}

/// Calculate CHS geometry from size
fn calculate_chs(size: u64) -> (u16, u8, u8) {
    let total_sectors = size / 512;

    // VHD geometry calculation algorithm
    let (cylinders, heads, sectors) = if total_sectors > 65535 * 16 * 255 {
        (65535, 16, 255)
    } else if total_sectors >= 65535 * 16 * 63 {
        let heads = 16u8;
        let sectors = 255u8;
        let cylinders = (total_sectors / (heads as u64 * sectors as u64)) as u16;
        (cylinders.min(65535), heads, sectors)
    } else {
        let sectors = 17u8;
        let mut cyl_times_heads = total_sectors / sectors as u64;

        let heads = if cyl_times_heads >= 1024 * 16 {
            16u8
        } else if cyl_times_heads >= 1024 * 8 {
            ((cyl_times_heads + 1023) / 1024) as u8
        } else if cyl_times_heads >= 1024 * 4 {
            ((cyl_times_heads + 1023) / 1024) as u8
        } else if cyl_times_heads >= 1024 * 2 {
            ((cyl_times_heads + 1023) / 1024) as u8
        } else {
            ((cyl_times_heads + 1023) / 1024).max(1) as u8
        };

        let heads = heads.max(4);
        cyl_times_heads = total_sectors / (sectors as u64);
        let cylinders = (cyl_times_heads / heads as u64) as u16;
        (cylinders.min(65535), heads.min(16), sectors)
    };

    (cylinders, heads, sectors)
}

/// Generate a random UUID
fn generate_uuid() -> [u8; 16] {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher as StdHasher};

    let mut uuid = [0u8; 16];
    let hasher = RandomState::new();

    for chunk in uuid.chunks_mut(8) {
        let mut h = hasher.build_hasher();
        h.write_u64(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64);
        let hash = h.finish();
        let bytes = hash.to_le_bytes();
        let len = chunk.len().min(8);
        chunk[..len].copy_from_slice(&bytes[..len]);
    }

    // Set version (4) and variant (RFC 4122)
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;

    uuid
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_create_fixed_vhd() {
        let source_data = vec![0xABu8; 1024 * 1024]; // 1 MB
        let mut source = Cursor::new(&source_data);
        let mut dest = Cursor::new(Vec::new());

        let creator = VhdCreator::new(VhdOptions::default());
        let result = creator
            .create_fixed::<_, _, fn(&AcquireProgress)>(
                &mut source,
                source_data.len() as u64,
                &mut dest,
                None,
            )
            .unwrap();

        // Check size: data + 512 byte footer
        assert_eq!(result.bytes_written, source_data.len() as u64 + 512);
        assert_eq!(result.source_size, source_data.len() as u64);
        assert!(!result.hashes.is_empty());

        // Verify footer cookie at end
        let output = dest.into_inner();
        assert_eq!(&output[output.len() - 512..output.len() - 504], b"conectix");
    }

    #[test]
    fn test_create_dynamic_vhd_sparse() {
        // Create sparse source: mostly zeros with some data
        let mut source_data = vec![0u8; 4 * 1024 * 1024]; // 4 MB
        source_data[0..1024].fill(0xAB); // First KB has data
        source_data[2 * 1024 * 1024..2 * 1024 * 1024 + 1024].fill(0xCD); // Some data in middle

        let mut source = Cursor::new(&source_data);
        let mut dest = Cursor::new(Vec::new());

        let mut options = VhdOptions::default();
        options.vhd_type = VhdOutputType::Dynamic;
        options.block_size = 1024 * 1024; // 1 MB blocks

        let creator = VhdCreator::new(options);
        let result = creator
            .create_dynamic::<_, _, fn(&AcquireProgress)>(
                &mut source,
                source_data.len() as u64,
                &mut dest,
                None,
            )
            .unwrap();

        // Dynamic VHD should be smaller than source (sparse)
        assert!(result.bytes_written < source_data.len() as u64);
        assert!(result.compression_ratio() < 1.0);

        // Verify cookies
        let output = dest.into_inner();
        assert_eq!(&output[0..8], b"conectix"); // Footer copy at start
        assert_eq!(&output[512..520], b"cxsparse"); // Dynamic header
        assert_eq!(&output[output.len() - 512..output.len() - 504], b"conectix"); // Footer at end
    }

    #[test]
    fn test_chs_geometry() {
        // Small disk
        let (c, h, s) = calculate_chs(100 * 1024 * 1024); // 100 MB
        assert!(c > 0 && h > 0 && s > 0);

        // Large disk
        let (c, h, s) = calculate_chs(100 * 1024 * 1024 * 1024); // 100 GB
        assert!(c > 0); // cylinders should be set
        assert!(h > 0 && h <= 16); // heads: 1-16
        assert!(s > 0); // sectors should be set
    }

    #[test]
    fn test_footer_checksum() {
        let footer = create_vhd_footer(1024 * 1024 * 1024, VhdType::Fixed, b"test");
        let bytes = serialize_footer(&footer);

        // Verify checksum calculation
        let mut sum: u32 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            if i >= 64 && i < 68 {
                continue;
            }
            sum = sum.wrapping_add(byte as u32);
        }
        let stored_checksum = u32::from_be_bytes([bytes[64], bytes[65], bytes[66], bytes[67]]);
        assert_eq!(!sum, stored_checksum);
    }
}
