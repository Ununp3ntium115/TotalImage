//! Vault factory for automatic format detection
//!
//! This module provides automatic detection and opening of disk image formats.

use crate::{Aff4Vault, E01Vault, RawVault, VaultConfig, VhdVault};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use totalimage_core::{Result, Vault};

/// Detected vault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultType {
    /// Raw sector image (.img, .dsk, .iso, etc.)
    Raw,
    /// Microsoft VHD format
    Vhd,
    /// EnCase E01 forensic format
    E01,
    /// Advanced Forensic Format 4
    Aff4,
    /// Unknown format
    Unknown,
}

impl VaultType {
    /// Get a human-readable name for this vault type
    pub fn name(&self) -> &'static str {
        match self {
            VaultType::Raw => "Raw Sector Image",
            VaultType::Vhd => "Microsoft VHD",
            VaultType::E01 => "EnCase E01",
            VaultType::Aff4 => "AFF4 Container",
            VaultType::Unknown => "Unknown",
        }
    }
}

/// Magic bytes for various formats
const VHD_MAGIC: &[u8] = b"conectix";
const E01_MAGIC: &[u8] = b"EVF\x09\x0d\x0a\xff\x00";
const ZIP_MAGIC: &[u8] = &[0x50, 0x4b, 0x03, 0x04]; // AFF4 is ZIP-based

/// Detect the vault type from a file path
///
/// Uses magic bytes for detection, falling back to file extension.
pub fn detect_vault_type(path: &Path) -> Result<VaultType> {
    // Try to read magic bytes
    let mut file = File::open(path)?;

    let mut magic = [0u8; 16];
    let bytes_read = file.read(&mut magic).unwrap_or(0);

    if bytes_read >= 8 {
        // Check VHD magic at start or end (footer can be at start for dynamic VHD)
        if &magic[0..8] == VHD_MAGIC {
            return Ok(VaultType::Vhd);
        }

        // Check E01 magic
        if &magic[0..8] == E01_MAGIC {
            return Ok(VaultType::E01);
        }

        // Check ZIP magic (potential AFF4)
        if &magic[0..4] == ZIP_MAGIC {
            // Further check for AFF4 by looking for container.description
            if is_aff4_container(path) {
                return Ok(VaultType::Aff4);
            }
        }
    }

    // Check VHD footer at end of file
    if let Ok(metadata) = file.metadata() {
        let file_size = metadata.len();
        if file_size >= 512 {
            if file.seek(SeekFrom::End(-512)).is_ok() {
                let mut footer = [0u8; 8];
                if file.read_exact(&mut footer).is_ok() && &footer == VHD_MAGIC {
                    return Ok(VaultType::Vhd);
                }
            }
        }
    }

    // Fall back to extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "vhd" | "vhdx" => return Ok(VaultType::Vhd),
            "e01" | "ex01" | "s01" | "l01" => return Ok(VaultType::E01),
            "aff4" | "af4" => return Ok(VaultType::Aff4),
            "img" | "ima" | "flp" | "vfd" | "dsk" | "iso" | "bin" | "raw" | "dd" => {
                return Ok(VaultType::Raw)
            }
            _ => {}
        }
    }

    // Default to raw if we can't determine the type
    Ok(VaultType::Raw)
}

/// Check if a ZIP file is an AFF4 container
fn is_aff4_container(path: &Path) -> bool {
    // Try to open as ZIP and look for AFF4 markers
    if let Ok(file) = File::open(path) {
        if let Ok(mut archive) = zip::ZipArchive::new(file) {
            // AFF4 containers have container.description or .turtle files
            for i in 0..archive.len() {
                if let Ok(entry) = archive.by_index(i) {
                    let name = entry.name();
                    if name.ends_with(".turtle")
                        || name.ends_with(".description")
                        || name.contains("container.description")
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Open any supported vault format with automatic detection
///
/// This function detects the vault type and opens the appropriate handler.
///
/// # Arguments
///
/// * `path` - Path to the disk image file
/// * `config` - Vault configuration options
///
/// # Returns
///
/// A boxed trait object implementing `Vault`
///
/// # Example
///
/// ```rust,no_run
/// use totalimage_vaults::factory::open_vault;
/// use totalimage_vaults::VaultConfig;
/// use std::path::Path;
///
/// let mut vault = open_vault(Path::new("disk.vhd"), VaultConfig::default()).unwrap();
/// println!("Vault type: {}", vault.identify());
/// println!("Size: {} bytes", vault.length());
/// ```
pub fn open_vault(path: &Path, config: VaultConfig) -> Result<Box<dyn Vault>> {
    let vault_type = detect_vault_type(path)?;
    open_vault_as(path, vault_type, config)
}

/// Open a vault with a specific type (skip auto-detection)
///
/// Use this when you know the vault type or want to force a specific handler.
pub fn open_vault_as(path: &Path, vault_type: VaultType, config: VaultConfig) -> Result<Box<dyn Vault>> {
    match vault_type {
        VaultType::Raw => {
            let vault = RawVault::open(path, config)?;
            Ok(Box::new(vault))
        }
        VaultType::Vhd => {
            let vault = VhdVault::open(path, config)?;
            Ok(Box::new(vault))
        }
        VaultType::E01 => {
            let vault = E01Vault::open(path)?;
            Ok(Box::new(vault))
        }
        VaultType::Aff4 => {
            let vault = Aff4Vault::open(path)?;
            Ok(Box::new(vault))
        }
        VaultType::Unknown => {
            // Try raw as fallback
            let vault = RawVault::open(path, config)?;
            Ok(Box::new(vault))
        }
    }
}

/// Get information about supported vault types
pub fn supported_formats() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        ("Raw Sector Image", &["img", "ima", "flp", "vfd", "dsk", "iso", "bin", "raw", "dd"]),
        ("Microsoft VHD", &["vhd"]),
        ("EnCase E01", &["e01", "ex01", "s01", "l01"]),
        ("AFF4 Container", &["aff4", "af4"]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_raw_by_extension() {
        let temp = NamedTempFile::with_suffix(".img").unwrap();
        let result = detect_vault_type(temp.path()).unwrap();
        assert_eq!(result, VaultType::Raw);
    }

    #[test]
    fn test_detect_vhd_by_extension() {
        let temp = NamedTempFile::with_suffix(".vhd").unwrap();
        let result = detect_vault_type(temp.path()).unwrap();
        assert_eq!(result, VaultType::Vhd);
    }

    #[test]
    fn test_detect_e01_by_extension() {
        let temp = NamedTempFile::with_suffix(".e01").unwrap();
        let result = detect_vault_type(temp.path()).unwrap();
        assert_eq!(result, VaultType::E01);
    }

    #[test]
    fn test_detect_vhd_by_magic() {
        let mut temp = NamedTempFile::with_suffix(".dat").unwrap();
        // Write VHD magic at start
        temp.write_all(b"conectix").unwrap();
        temp.write_all(&[0u8; 504]).unwrap(); // Pad to 512 bytes
        temp.flush().unwrap();

        let result = detect_vault_type(temp.path()).unwrap();
        assert_eq!(result, VaultType::Vhd);
    }

    #[test]
    fn test_detect_e01_by_magic() {
        let mut temp = NamedTempFile::with_suffix(".dat").unwrap();
        // Write E01 magic
        temp.write_all(b"EVF\x09\x0d\x0a\xff\x00").unwrap();
        temp.write_all(&[0u8; 504]).unwrap();
        temp.flush().unwrap();

        let result = detect_vault_type(temp.path()).unwrap();
        assert_eq!(result, VaultType::E01);
    }

    #[test]
    fn test_vault_type_name() {
        assert_eq!(VaultType::Raw.name(), "Raw Sector Image");
        assert_eq!(VaultType::Vhd.name(), "Microsoft VHD");
        assert_eq!(VaultType::E01.name(), "EnCase E01");
        assert_eq!(VaultType::Aff4.name(), "AFF4 Container");
    }

    #[test]
    fn test_supported_formats() {
        let formats = supported_formats();
        assert!(!formats.is_empty());
        assert!(formats.iter().any(|(name, _)| *name == "Microsoft VHD"));
    }
}
