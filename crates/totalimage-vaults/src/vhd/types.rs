//! VHD (Virtual Hard Disk) type definitions
//!
//! This module contains the core data structures for parsing Microsoft VHD files.

use totalimage_core::Result;

/// VHD disk type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VhdType {
    None = 0,
    Reserved1 = 1,
    Fixed = 2,
    Dynamic = 3,
    Differencing = 4,
    Reserved5 = 5,
    Reserved6 = 6,
}

impl VhdType {
    /// Parse VHD type from a u32 value
    pub fn from_u32(value: u32) -> Result<Self> {
        match value {
            0 => Ok(VhdType::None),
            1 => Ok(VhdType::Reserved1),
            2 => Ok(VhdType::Fixed),
            3 => Ok(VhdType::Dynamic),
            4 => Ok(VhdType::Differencing),
            5 => Ok(VhdType::Reserved5),
            6 => Ok(VhdType::Reserved6),
            _ => Err(totalimage_core::Error::invalid_vault(format!(
                "Invalid VHD disk type: {}",
                value
            ))),
        }
    }
}

/// Disk geometry (CHS addressing)
#[derive(Debug, Clone, Copy)]
pub struct DiskGeometry {
    pub cylinders: u16,
    pub heads: u8,
    pub sectors: u8,
}

impl DiskGeometry {
    /// Parse disk geometry from bytes
    pub fn parse(bytes: &[u8]) -> Self {
        Self {
            cylinders: u16::from_be_bytes([bytes[0], bytes[1]]),
            heads: bytes[2],
            sectors: bytes[3],
        }
    }

    /// Convert geometry to bytes
    pub fn to_bytes(&self) -> [u8; 4] {
        let cyl_bytes = self.cylinders.to_be_bytes();
        [cyl_bytes[0], cyl_bytes[1], self.heads, self.sectors]
    }
}

/// VHD Footer structure (512 bytes)
///
/// The footer appears at the end of all VHD files. For fixed VHDs, it only
/// appears at the end. For dynamic/differencing VHDs, a copy also appears at
/// the beginning.
#[derive(Debug, Clone)]
pub struct VhdFooter {
    pub cookie: [u8; 8],           // "conectix"
    pub features: u32,
    pub version: u32,
    pub data_offset: u64,
    pub timestamp: u32,
    pub creator_app: [u8; 4],
    pub creator_version: u32,
    pub creator_os: u32,
    pub original_size: u64,
    pub current_size: u64,
    pub geometry: DiskGeometry,
    pub disk_type: VhdType,
    pub checksum: u32,
    pub uuid: [u8; 16],
    pub saved_state: u8,
    pub reserved: [u8; 427],
}

impl VhdFooter {
    /// VHD footer cookie value "conectix"
    pub const COOKIE: &'static [u8; 8] = b"conectix";

    /// Size of the VHD footer in bytes
    pub const SIZE: usize = 512;

    /// Parse VHD footer from raw bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_vault(
                "VHD footer too small",
            ));
        }

        // Parse cookie
        let mut cookie = [0u8; 8];
        cookie.copy_from_slice(&bytes[0..8]);

        // Verify cookie
        if &cookie != Self::COOKIE {
            return Err(totalimage_core::Error::invalid_vault(format!(
                "Invalid VHD footer cookie: expected 'conectix', got '{}'",
                String::from_utf8_lossy(&cookie)
            )));
        }

        // Parse fields (all big-endian)
        let features = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
        let version = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
        let data_offset = u64::from_be_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19],
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        let timestamp = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        let mut creator_app = [0u8; 4];
        creator_app.copy_from_slice(&bytes[28..32]);

        let creator_version = u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        let creator_os = u32::from_be_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);
        let original_size = u64::from_be_bytes([
            bytes[40], bytes[41], bytes[42], bytes[43],
            bytes[44], bytes[45], bytes[46], bytes[47],
        ]);
        let current_size = u64::from_be_bytes([
            bytes[48], bytes[49], bytes[50], bytes[51],
            bytes[52], bytes[53], bytes[54], bytes[55],
        ]);

        let geometry = DiskGeometry::parse(&bytes[56..60]);

        let disk_type_raw = u32::from_be_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);
        let disk_type = VhdType::from_u32(disk_type_raw)?;

        let checksum = u32::from_be_bytes([bytes[64], bytes[65], bytes[66], bytes[67]]);

        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&bytes[68..84]);

        let saved_state = bytes[84];

        let mut reserved = [0u8; 427];
        reserved.copy_from_slice(&bytes[85..512]);

        Ok(Self {
            cookie,
            features,
            version,
            data_offset,
            timestamp,
            creator_app,
            creator_version,
            creator_os,
            original_size,
            current_size,
            geometry,
            disk_type,
            checksum,
            uuid,
            saved_state,
            reserved,
        })
    }

    /// Verify the footer checksum
    ///
    /// The checksum is the one's complement of the sum of all bytes in the
    /// footer, with the checksum field itself set to zero during calculation.
    pub fn verify_checksum(&self) -> bool {
        // Serialize footer back to bytes
        let mut bytes = [0u8; Self::SIZE];
        self.serialize(&mut bytes);

        // Calculate checksum with checksum field zeroed
        let mut sum: u32 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            // Skip checksum field (bytes 64-67)
            if i >= 64 && i < 68 {
                continue;
            }
            sum = sum.wrapping_add(byte as u32);
        }

        // One's complement
        let calculated = !sum;

        calculated == self.checksum
    }

    /// Serialize footer to bytes
    pub fn serialize(&self, bytes: &mut [u8; Self::SIZE]) {
        bytes[0..8].copy_from_slice(&self.cookie);
        bytes[8..12].copy_from_slice(&self.features.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.version.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.data_offset.to_be_bytes());
        bytes[24..28].copy_from_slice(&self.timestamp.to_be_bytes());
        bytes[28..32].copy_from_slice(&self.creator_app);
        bytes[32..36].copy_from_slice(&self.creator_version.to_be_bytes());
        bytes[36..40].copy_from_slice(&self.creator_os.to_be_bytes());
        bytes[40..48].copy_from_slice(&self.original_size.to_be_bytes());
        bytes[48..56].copy_from_slice(&self.current_size.to_be_bytes());
        bytes[56..60].copy_from_slice(&self.geometry.to_bytes());
        bytes[60..64].copy_from_slice(&(self.disk_type as u32).to_be_bytes());
        bytes[64..68].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[68..84].copy_from_slice(&self.uuid);
        bytes[84] = self.saved_state;
        bytes[85..512].copy_from_slice(&self.reserved);
    }
}

/// VHD Dynamic Header structure (1024 bytes)
///
/// This header appears only in dynamic and differencing VHDs, located at the
/// offset specified in the footer's data_offset field.
#[derive(Debug, Clone)]
pub struct VhdDynamicHeader {
    pub cookie: [u8; 8],              // "cxsparse"
    pub data_offset: u64,
    pub table_offset: u64,
    pub header_version: u32,
    pub max_table_entries: u32,
    pub block_size: u32,
    pub checksum: u32,
    pub parent_uuid: [u8; 16],
    pub parent_timestamp: u32,
    pub reserved1: u32,
    pub parent_unicode_name: [u16; 256],
    pub parent_locator_entries: [[u8; 24]; 8],
    pub reserved2: [u8; 256],
}

impl VhdDynamicHeader {
    /// VHD dynamic header cookie value "cxsparse"
    pub const COOKIE: &'static [u8; 8] = b"cxsparse";

    /// Size of the VHD dynamic header in bytes
    pub const SIZE: usize = 1024;

    /// Parse VHD dynamic header from raw bytes
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(totalimage_core::Error::invalid_vault(
                "VHD dynamic header too small",
            ));
        }

        // Parse cookie
        let mut cookie = [0u8; 8];
        cookie.copy_from_slice(&bytes[0..8]);

        // Verify cookie
        if &cookie != Self::COOKIE {
            return Err(totalimage_core::Error::invalid_vault(format!(
                "Invalid VHD dynamic header cookie: expected 'cxsparse', got '{}'",
                String::from_utf8_lossy(&cookie)
            )));
        }

        // Parse fields (all big-endian)
        let data_offset = u64::from_be_bytes([
            bytes[8], bytes[9], bytes[10], bytes[11],
            bytes[12], bytes[13], bytes[14], bytes[15],
        ]);
        let table_offset = u64::from_be_bytes([
            bytes[16], bytes[17], bytes[18], bytes[19],
            bytes[20], bytes[21], bytes[22], bytes[23],
        ]);
        let header_version = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let max_table_entries = u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
        let block_size = u32::from_be_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
        let checksum = u32::from_be_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);

        let mut parent_uuid = [0u8; 16];
        parent_uuid.copy_from_slice(&bytes[40..56]);

        let parent_timestamp = u32::from_be_bytes([bytes[56], bytes[57], bytes[58], bytes[59]]);
        let reserved1 = u32::from_be_bytes([bytes[60], bytes[61], bytes[62], bytes[63]]);

        // Parse parent unicode name (256 UTF-16 BE characters)
        let mut parent_unicode_name = [0u16; 256];
        for i in 0..256 {
            let offset = 64 + i * 2;
            parent_unicode_name[i] = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
        }

        // Parse parent locator entries (8 entries of 24 bytes each)
        let mut parent_locator_entries = [[0u8; 24]; 8];
        for i in 0..8 {
            let offset = 576 + i * 24;
            parent_locator_entries[i].copy_from_slice(&bytes[offset..offset + 24]);
        }

        // Parse remaining reserved bytes
        let mut reserved2 = [0u8; 256];
        reserved2.copy_from_slice(&bytes[768..1024]);

        Ok(Self {
            cookie,
            data_offset,
            table_offset,
            header_version,
            max_table_entries,
            block_size,
            checksum,
            parent_uuid,
            parent_timestamp,
            reserved1,
            parent_unicode_name,
            parent_locator_entries,
            reserved2,
        })
    }

    /// Verify the dynamic header checksum
    pub fn verify_checksum(&self) -> bool {
        // Serialize header back to bytes
        let mut bytes = [0u8; Self::SIZE];
        self.serialize(&mut bytes);

        // Calculate checksum with checksum field zeroed
        let mut sum: u32 = 0;
        for (i, &byte) in bytes.iter().enumerate() {
            // Skip checksum field (bytes 36-39)
            if i >= 36 && i < 40 {
                continue;
            }
            sum = sum.wrapping_add(byte as u32);
        }

        // One's complement
        let calculated = !sum;

        calculated == self.checksum
    }

    /// Serialize dynamic header to bytes
    pub fn serialize(&self, bytes: &mut [u8; Self::SIZE]) {
        bytes[0..8].copy_from_slice(&self.cookie);
        bytes[8..16].copy_from_slice(&self.data_offset.to_be_bytes());
        bytes[16..24].copy_from_slice(&self.table_offset.to_be_bytes());
        bytes[24..28].copy_from_slice(&self.header_version.to_be_bytes());
        bytes[28..32].copy_from_slice(&self.max_table_entries.to_be_bytes());
        bytes[32..36].copy_from_slice(&self.block_size.to_be_bytes());
        bytes[36..40].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[40..56].copy_from_slice(&self.parent_uuid);
        bytes[56..60].copy_from_slice(&self.parent_timestamp.to_be_bytes());
        bytes[60..64].copy_from_slice(&self.reserved1.to_be_bytes());

        // Serialize parent unicode name
        for i in 0..256 {
            let offset = 64 + i * 2;
            bytes[offset..offset + 2].copy_from_slice(&self.parent_unicode_name[i].to_be_bytes());
        }

        // Serialize parent locator entries
        for i in 0..8 {
            let offset = 576 + i * 24;
            bytes[offset..offset + 24].copy_from_slice(&self.parent_locator_entries[i]);
        }

        // Serialize reserved bytes
        bytes[768..1024].copy_from_slice(&self.reserved2);
    }
}

/// Block Allocation Table for dynamic VHDs
///
/// The BAT maps virtual blocks to physical sectors in the VHD file.
/// Each entry is a 4-byte sector offset (512-byte sectors).
#[derive(Clone)]
pub struct BlockAllocationTable {
    pub entries: Vec<u32>,
    pub block_size: u32,
}

impl BlockAllocationTable {
    /// Parse BAT from raw bytes
    pub fn parse(bytes: &[u8], block_size: u32) -> Result<Self> {
        if bytes.len() % 4 != 0 {
            return Err(totalimage_core::Error::invalid_vault(
                "BAT size must be multiple of 4",
            ));
        }

        let entry_count = bytes.len() / 4;
        let mut entries = Vec::with_capacity(entry_count);

        for i in 0..entry_count {
            let offset = i * 4;
            let entry = u32::from_be_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]);
            entries.push(entry);
        }

        Ok(Self { entries, block_size })
    }

    /// Get the sector offset for a block index
    ///
    /// Returns None if the block is not allocated (sparse)
    pub fn get_block_offset(&self, block_index: usize) -> Option<u64> {
        if block_index >= self.entries.len() {
            return None;
        }

        let entry = self.entries[block_index];
        if entry == 0xFFFFFFFF {
            // Unallocated block (sparse)
            None
        } else {
            // Convert sector offset to byte offset
            Some((entry as u64) * 512)
        }
    }

    /// Calculate the block index for a virtual offset
    pub fn offset_to_block(&self, offset: u64) -> usize {
        (offset / self.block_size as u64) as usize
    }

    /// Calculate the offset within a block
    pub fn offset_within_block(&self, offset: u64) -> u64 {
        offset % self.block_size as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vhd_type_from_u32() {
        assert!(matches!(VhdType::from_u32(0).unwrap(), VhdType::None));
        assert!(matches!(VhdType::from_u32(2).unwrap(), VhdType::Fixed));
        assert!(matches!(VhdType::from_u32(3).unwrap(), VhdType::Dynamic));
        assert!(matches!(VhdType::from_u32(4).unwrap(), VhdType::Differencing));
        assert!(VhdType::from_u32(99).is_err());
    }

    #[test]
    fn test_disk_geometry_parse() {
        let bytes = [0x01, 0x23, 0x45, 0x67];
        let geom = DiskGeometry::parse(&bytes);
        assert_eq!(geom.cylinders, 0x0123);
        assert_eq!(geom.heads, 0x45);
        assert_eq!(geom.sectors, 0x67);
    }

    #[test]
    fn test_disk_geometry_round_trip() {
        let geom = DiskGeometry {
            cylinders: 1024,
            heads: 16,
            sectors: 63,
        };
        let bytes = geom.to_bytes();
        let parsed = DiskGeometry::parse(&bytes);
        assert_eq!(parsed.cylinders, geom.cylinders);
        assert_eq!(parsed.heads, geom.heads);
        assert_eq!(parsed.sectors, geom.sectors);
    }

    #[test]
    fn test_vhd_footer_invalid_cookie() {
        let mut bytes = [0u8; 512];
        bytes[0..8].copy_from_slice(b"notvalid");
        assert!(VhdFooter::parse(&bytes).is_err());
    }

    #[test]
    fn test_vhd_footer_too_small() {
        let bytes = [0u8; 100];
        assert!(VhdFooter::parse(&bytes).is_err());
    }

    #[test]
    fn test_vhd_dynamic_header_invalid_cookie() {
        let mut bytes = [0u8; 1024];
        bytes[0..8].copy_from_slice(b"notvalid");
        assert!(VhdDynamicHeader::parse(&bytes).is_err());
    }

    #[test]
    fn test_bat_parse() {
        // Create a simple BAT with 3 entries
        let mut bytes = vec![0u8; 12];
        bytes[0..4].copy_from_slice(&0x00001000u32.to_be_bytes());
        bytes[4..8].copy_from_slice(&0xFFFFFFFFu32.to_be_bytes());
        bytes[8..12].copy_from_slice(&0x00002000u32.to_be_bytes());

        let bat = BlockAllocationTable::parse(&bytes, 2 * 1024 * 1024).unwrap();

        assert_eq!(bat.entries.len(), 3);
        assert_eq!(bat.get_block_offset(0), Some(0x1000 * 512));
        assert_eq!(bat.get_block_offset(1), None); // Sparse
        assert_eq!(bat.get_block_offset(2), Some(0x2000 * 512));
    }

    #[test]
    fn test_bat_offset_calculations() {
        let bat = BlockAllocationTable {
            entries: vec![0x1000, 0x2000],
            block_size: 2 * 1024 * 1024, // 2 MB
        };

        // Test block index calculation
        assert_eq!(bat.offset_to_block(0), 0);
        assert_eq!(bat.offset_to_block(1024), 0);
        assert_eq!(bat.offset_to_block(2 * 1024 * 1024), 1);
        assert_eq!(bat.offset_to_block(2 * 1024 * 1024 + 500), 1);

        // Test offset within block
        assert_eq!(bat.offset_within_block(0), 0);
        assert_eq!(bat.offset_within_block(1024), 1024);
        assert_eq!(bat.offset_within_block(2 * 1024 * 1024 + 500), 500);
    }
}
