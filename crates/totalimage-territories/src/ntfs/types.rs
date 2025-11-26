//! NTFS-specific types and structures

use chrono::{DateTime, Utc};
use ntfs::NtfsTime;

/// Convert NTFS time to chrono DateTime
pub fn ntfs_time_to_datetime(time: NtfsTime) -> Option<DateTime<Utc>> {
    // NTFS time is 100-nanosecond intervals since January 1, 1601
    // Unix epoch is January 1, 1970
    // Difference is 11644473600 seconds (369 years)
    const UNIX_EPOCH_DIFF: u64 = 11644473600;
    const NANOS_PER_SEC: u64 = 10_000_000; // 100-nanosecond intervals

    let nt_time = time.nt_timestamp();
    if nt_time == 0 {
        return None;
    }

    // Convert to seconds since NTFS epoch
    let seconds_since_ntfs_epoch = nt_time / NANOS_PER_SEC;
    let nanos = ((nt_time % NANOS_PER_SEC) * 100) as u32;

    // Check if time is before Unix epoch
    if seconds_since_ntfs_epoch < UNIX_EPOCH_DIFF {
        return None;
    }

    // Convert to Unix timestamp
    let unix_timestamp = (seconds_since_ntfs_epoch - UNIX_EPOCH_DIFF) as i64;

    DateTime::from_timestamp(unix_timestamp, nanos)
}

/// NTFS file attributes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum NtfsFileAttribute {
    ReadOnly = 0x0001,
    Hidden = 0x0002,
    System = 0x0004,
    Directory = 0x0010,
    Archive = 0x0020,
    Device = 0x0040,
    Normal = 0x0080,
    Temporary = 0x0100,
    SparseFile = 0x0200,
    ReparsePoint = 0x0400,
    Compressed = 0x0800,
    Offline = 0x1000,
    NotContentIndexed = 0x2000,
    Encrypted = 0x4000,
}

impl NtfsFileAttribute {
    /// Create attribute mask from u32
    pub fn from_u32(value: u32) -> Vec<Self> {
        let mut attrs = Vec::new();

        if value & 0x0001 != 0 { attrs.push(Self::ReadOnly); }
        if value & 0x0002 != 0 { attrs.push(Self::Hidden); }
        if value & 0x0004 != 0 { attrs.push(Self::System); }
        if value & 0x0010 != 0 { attrs.push(Self::Directory); }
        if value & 0x0020 != 0 { attrs.push(Self::Archive); }
        if value & 0x0040 != 0 { attrs.push(Self::Device); }
        if value & 0x0080 != 0 { attrs.push(Self::Normal); }
        if value & 0x0100 != 0 { attrs.push(Self::Temporary); }
        if value & 0x0200 != 0 { attrs.push(Self::SparseFile); }
        if value & 0x0400 != 0 { attrs.push(Self::ReparsePoint); }
        if value & 0x0800 != 0 { attrs.push(Self::Compressed); }
        if value & 0x1000 != 0 { attrs.push(Self::Offline); }
        if value & 0x2000 != 0 { attrs.push(Self::NotContentIndexed); }
        if value & 0x4000 != 0 { attrs.push(Self::Encrypted); }

        attrs
    }
}

/// NTFS volume information
#[derive(Debug, Clone)]
pub struct NtfsVolumeInfo {
    /// Volume label (if available)
    pub label: Option<String>,
    /// NTFS version (major)
    pub major_version: u8,
    /// NTFS version (minor)
    pub minor_version: u8,
    /// Total size in bytes
    pub total_size: u64,
    /// Cluster size in bytes
    pub cluster_size: u32,
    /// Sector size in bytes
    pub sector_size: u16,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ntfs_file_attributes() {
        let attrs = NtfsFileAttribute::from_u32(0x0021); // ReadOnly | Archive
        assert!(attrs.contains(&NtfsFileAttribute::ReadOnly));
        assert!(attrs.contains(&NtfsFileAttribute::Archive));
        assert!(!attrs.contains(&NtfsFileAttribute::Hidden));
    }

    #[test]
    fn test_directory_attribute() {
        let attrs = NtfsFileAttribute::from_u32(0x0010);
        assert!(attrs.contains(&NtfsFileAttribute::Directory));
    }
}
