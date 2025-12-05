//! Rock Ridge Interchange Protocol (RRIP) support for ISO-9660
//!
//! Rock Ridge is an extension to ISO-9660 that adds POSIX filesystem features:
//! - Long filenames (NM - Alternate Name)
//! - Unix permissions and ownership (PX - POSIX File Attributes)
//! - Symbolic links (SL - Symbolic Link)
//! - Device nodes (PN - POSIX Device Number)
//! - Extended timestamps (TF - Time Stamp)
//! - Deeper directory hierarchies (CL/PL/RE - Child/Parent/Relocated)
//!
//! ## Implementation Status
//!
//! Currently implemented:
//! - ✅ NM (Alternate Name) - Long filenames
//! - ✅ PX (POSIX Attributes) - File mode, links, uid, gid
//! - ✅ TF (Timestamps) - Creation, modification, access times
//!
//! Not yet implemented:
//! - ❌ SL (Symbolic Links)
//! - ❌ PN (Device Numbers)
//! - ❌ CL/PL/RE (Relocated Directories)
//!
//! ## Specification
//!
//! Rock Ridge uses System Use Sharing Protocol (SUSP) fields in the
//! System Use area at the end of ISO-9660 directory records.
//!
//! Reference: IEEE P1282 - Rock Ridge Interchange Protocol

use chrono::{DateTime, TimeZone, Utc};

/// Rock Ridge System Use entry signature (2 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RockRidgeSignature {
    /// SP - System Use Sharing Protocol indicator
    SP,
    /// CE - Continuation Area
    CE,
    /// PX - POSIX File Attributes
    PX,
    /// PN - POSIX Device Number
    PN,
    /// SL - Symbolic Link
    SL,
    /// NM - Alternate Name (long filename)
    NM,
    /// CL - Child Link
    CL,
    /// PL - Parent Link
    PL,
    /// RE - Relocated Directory
    RE,
    /// TF - Time Stamp
    TF,
    /// SF - Sparse File
    SF,
}

impl RockRidgeSignature {
    /// Parse signature from 2-byte array
    pub fn from_bytes(bytes: &[u8; 2]) -> Option<Self> {
        match bytes {
            b"SP" => Some(Self::SP),
            b"CE" => Some(Self::CE),
            b"PX" => Some(Self::PX),
            b"PN" => Some(Self::PN),
            b"SL" => Some(Self::SL),
            b"NM" => Some(Self::NM),
            b"CL" => Some(Self::CL),
            b"PL" => Some(Self::PL),
            b"RE" => Some(Self::RE),
            b"TF" => Some(Self::TF),
            b"SF" => Some(Self::SF),
            _ => None,
        }
    }
}

/// Rock Ridge extensions parsed from a directory record
#[derive(Debug, Clone, Default)]
pub struct RockRidgeExtensions {
    /// Alternate name (long filename)
    pub alternate_name: Option<String>,
    /// POSIX file attributes
    pub posix_attrs: Option<PosixAttributes>,
    /// Extended timestamps
    pub timestamps: Option<RockRidgeTimestamps>,
}

/// POSIX file attributes (PX entry)
#[derive(Debug, Clone)]
pub struct PosixAttributes {
    /// File mode (permissions and type)
    pub mode: u32,
    /// Number of hard links
    pub links: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

/// Rock Ridge timestamps (TF entry)
#[derive(Debug, Clone)]
pub struct RockRidgeTimestamps {
    /// File creation time
    pub creation: Option<DateTime<Utc>>,
    /// Last modification time
    pub modification: Option<DateTime<Utc>>,
    /// Last access time
    pub access: Option<DateTime<Utc>>,
    /// Attribute change time
    pub attribute_change: Option<DateTime<Utc>>,
}

/// Parse Rock Ridge extensions from System Use area
///
/// The System Use area appears at the end of an ISO-9660 directory record,
/// after the file identifier and padding.
///
/// # Arguments
///
/// * `system_use_data` - Raw bytes from the System Use area
///
/// # Returns
///
/// Parsed Rock Ridge extensions, or None if no valid Rock Ridge data found
pub fn parse_rock_ridge(system_use_data: &[u8]) -> Option<RockRidgeExtensions> {
    if system_use_data.len() < 4 {
        return None;
    }

    let mut extensions = RockRidgeExtensions::default();
    let mut pos = 0;
    let mut found_any = false;

    while pos + 4 <= system_use_data.len() {
        // Read entry header: [signature (2 bytes)][length (1 byte)][version (1 byte)]
        let sig_bytes = [system_use_data[pos], system_use_data[pos + 1]];
        let length = system_use_data[pos + 2] as usize;
        let _version = system_use_data[pos + 3];

        // Validate length
        if length < 4 || pos + length > system_use_data.len() {
            break;
        }

        let entry_data = &system_use_data[pos..pos + length];

        // Parse based on signature
        if let Some(sig) = RockRidgeSignature::from_bytes(&sig_bytes) {
            found_any = true;

            match sig {
                RockRidgeSignature::NM => {
                    // Alternate Name entry
                    if let Some(name) = parse_nm_entry(entry_data) {
                        // Append to existing name (NM entries can be continued)
                        if let Some(existing) = &mut extensions.alternate_name {
                            existing.push_str(&name);
                        } else {
                            extensions.alternate_name = Some(name);
                        }
                    }
                }
                RockRidgeSignature::PX => {
                    // POSIX Attributes entry
                    extensions.posix_attrs = parse_px_entry(entry_data);
                }
                RockRidgeSignature::TF => {
                    // Timestamp entry
                    extensions.timestamps = parse_tf_entry(entry_data);
                }
                _ => {
                    // Other Rock Ridge entries not yet implemented
                }
            }
        }

        pos += length;
    }

    if found_any {
        Some(extensions)
    } else {
        None
    }
}

/// Parse NM (Alternate Name) entry
fn parse_nm_entry(data: &[u8]) -> Option<String> {
    // NM entry format:
    // [0-1]: Signature "NM"
    // [2]: Length
    // [3]: Version
    // [4]: Flags
    // [5..]: Name characters

    if data.len() < 6 {
        return None;
    }

    let flags = data[4];
    let _continue_flag = (flags & 0x01) != 0; // Name continues in next NM entry
    let _current_flag = (flags & 0x02) != 0; // Name refers to current directory (".")
    let _parent_flag = (flags & 0x04) != 0; // Name refers to parent directory ("..")

    // Skip special directory names
    if _current_flag || _parent_flag {
        return None;
    }

    let name_bytes = &data[5..];
    String::from_utf8(name_bytes.to_vec()).ok()
}

/// Parse PX (POSIX Attributes) entry
fn parse_px_entry(data: &[u8]) -> Option<PosixAttributes> {
    // PX entry format (version 1, 36 bytes minimum):
    // [0-1]: Signature "PX"
    // [2]: Length
    // [3]: Version
    // [4-7]: File mode (both-endian)
    // [12-15]: Links (both-endian)
    // [20-23]: User ID (both-endian)
    // [28-31]: Group ID (both-endian)

    if data.len() < 36 {
        return None;
    }

    let mode = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let links = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
    let uid = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
    let gid = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);

    Some(PosixAttributes {
        mode,
        links,
        uid,
        gid,
    })
}

/// Parse TF (Timestamp) entry
fn parse_tf_entry(data: &[u8]) -> Option<RockRidgeTimestamps> {
    // TF entry format:
    // [0-1]: Signature "TF"
    // [2]: Length
    // [3]: Version
    // [4]: Flags (which timestamps are present)
    // [5..]: Timestamp data (17 bytes each for long format, 7 bytes for short)

    if data.len() < 5 {
        return None;
    }

    let flags = data[4];
    let _creation = (flags & 0x01) != 0;
    let _modify = (flags & 0x02) != 0;
    let _access = (flags & 0x04) != 0;
    let _attributes = (flags & 0x08) != 0;
    let long_form = (flags & 0x80) != 0; // 0 = short (7 bytes), 1 = long (17 bytes)

    let timestamp_size = if long_form { 17 } else { 7 };
    let mut pos = 5;

    let mut timestamps = RockRidgeTimestamps {
        creation: None,
        modification: None,
        access: None,
        attribute_change: None,
    };

    // Parse timestamps in order based on flags
    if _creation && pos + timestamp_size <= data.len() {
        timestamps.creation = parse_iso_timestamp(&data[pos..pos + timestamp_size], long_form);
        pos += timestamp_size;
    }
    if _modify && pos + timestamp_size <= data.len() {
        timestamps.modification = parse_iso_timestamp(&data[pos..pos + timestamp_size], long_form);
        pos += timestamp_size;
    }
    if _access && pos + timestamp_size <= data.len() {
        timestamps.access = parse_iso_timestamp(&data[pos..pos + timestamp_size], long_form);
        pos += timestamp_size;
    }
    if _attributes && pos + timestamp_size <= data.len() {
        timestamps.attribute_change =
            parse_iso_timestamp(&data[pos..pos + timestamp_size], long_form);
    }

    Some(timestamps)
}

/// Parse ISO-9660 timestamp (short 7-byte or long 17-byte format)
fn parse_iso_timestamp(data: &[u8], long_form: bool) -> Option<DateTime<Utc>> {
    if long_form {
        // Long form: 17 bytes (ASCII decimal + timezone)
        // Format: YYYYMMDDHHmmsscc (year, month, day, hour, minute, second, centiseconds)
        if data.len() < 17 {
            return None;
        }
        // For simplicity, return None for now (full implementation would parse ASCII)
        None
    } else {
        // Short form: 7 bytes (binary)
        // [0]: Years since 1900
        // [1]: Month (1-12)
        // [2]: Day (1-31)
        // [3]: Hour (0-23)
        // [4]: Minute (0-59)
        // [5]: Second (0-59)
        // [6]: GMT offset in 15-minute intervals

        if data.len() < 7 {
            return None;
        }

        let year = 1900 + data[0] as i32;
        let month = data[1] as u32;
        let day = data[2] as u32;
        let hour = data[3] as u32;
        let minute = data[4] as u32;
        let second = data[5] as u32;

        // Create UTC datetime (ignoring timezone offset for now)
        chrono::Utc
            .with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};

    #[test]
    fn test_signature_parsing() {
        assert_eq!(
            RockRidgeSignature::from_bytes(b"SP"),
            Some(RockRidgeSignature::SP)
        );
        assert_eq!(
            RockRidgeSignature::from_bytes(b"NM"),
            Some(RockRidgeSignature::NM)
        );
        assert_eq!(
            RockRidgeSignature::from_bytes(b"PX"),
            Some(RockRidgeSignature::PX)
        );
        assert_eq!(
            RockRidgeSignature::from_bytes(b"TF"),
            Some(RockRidgeSignature::TF)
        );
        assert_eq!(RockRidgeSignature::from_bytes(b"XX"), None);
    }

    #[test]
    fn test_nm_entry_basic() {
        // NM entry: "NM" + length(11) + version(1) + flags(0) + "test.txt"
        let data = b"NM\x0B\x01\x00test.txt";
        let name = parse_nm_entry(data).unwrap();
        assert_eq!(name, "test.txt");
    }

    #[test]
    fn test_px_entry() {
        // PX entry with mode=0755, links=1, uid=1000, gid=1000
        let mut data = vec![b'P', b'X', 36, 1]; // Signature, length, version

        // Mode: 0755 (octal) = 493 (decimal) = 0x01ED
        data.extend_from_slice(&493u32.to_le_bytes());
        data.extend_from_slice(&493u32.to_be_bytes());

        // Links: 1
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&1u32.to_be_bytes());

        // UID: 1000
        data.extend_from_slice(&1000u32.to_le_bytes());
        data.extend_from_slice(&1000u32.to_be_bytes());

        // GID: 1000
        data.extend_from_slice(&1000u32.to_le_bytes());
        data.extend_from_slice(&1000u32.to_be_bytes());

        let attrs = parse_px_entry(&data).unwrap();
        assert_eq!(attrs.mode, 493);
        assert_eq!(attrs.links, 1);
        assert_eq!(attrs.uid, 1000);
        assert_eq!(attrs.gid, 1000);
    }

    #[test]
    fn test_empty_system_use() {
        let data = b"";
        assert!(parse_rock_ridge(data).is_none());
    }

    #[test]
    fn test_rock_ridge_with_nm() {
        // System Use area with NM entry for "longfilename.txt"
        let mut data = Vec::new();

        // NM entry
        data.extend_from_slice(b"NM");
        data.push(21); // Length = 4 (header) + 1 (flags) + 16 (name)
        data.push(1); // Version
        data.push(0); // Flags
        data.extend_from_slice(b"longfilename.txt");

        let ext = parse_rock_ridge(&data).unwrap();
        assert_eq!(ext.alternate_name.as_deref(), Some("longfilename.txt"));
    }

    #[test]
    fn test_short_timestamp_parsing() {
        // Timestamp for 2024-01-15 12:30:45 UTC
        // Years since 1900 = 124 (2024 - 1900)
        let data = [124, 1, 15, 12, 30, 45, 0];

        let dt = parse_iso_timestamp(&data, false).unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 1);
        assert_eq!(dt.day(), 15);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
        assert_eq!(dt.second(), 45);
    }
}
