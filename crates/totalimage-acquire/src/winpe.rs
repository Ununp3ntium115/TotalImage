//! WinPE (Windows Preinstallation Environment) support
//!
//! Provides functionality to detect WinPE sources, extract WIM files, and configure bootable USB drives.

use crate::error::{AcquireError, Result};
use std::path::{Path, PathBuf};

/// WinPE architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WinpeArchitecture {
    /// x86 (32-bit)
    X86,
    /// x64 (64-bit)
    X64,
}

impl WinpeArchitecture {
    /// Get architecture name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            WinpeArchitecture::X86 => "x86",
            WinpeArchitecture::X64 => "amd64",
        }
    }
}

/// WinPE source information
#[derive(Debug, Clone)]
pub struct WinpeSource {
    /// Path to boot.wim file
    pub boot_wim_path: PathBuf,
    /// Architecture (x86 or amd64)
    pub architecture: WinpeArchitecture,
    /// Path to Windows ADK installation (if detected)
    pub adk_path: Option<PathBuf>,
}

/// Find WinPE source (boot.wim) from Windows ADK installation
///
/// Checks common ADK installation paths on Windows.
/// On non-Windows platforms, returns an error indicating manual path is required.
pub fn find_winpe_source() -> Result<WinpeSource> {
    #[cfg(target_os = "windows")]
    {
        find_winpe_source_windows()
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err(AcquireError::UnsupportedPlatform(
            "WinPE source detection requires Windows. Please specify boot.wim path manually."
                .to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn find_winpe_source_windows() -> Result<WinpeSource> {
    // Common ADK installation paths
    let adk_paths = vec![
        PathBuf::from(
            r"C:\Program Files (x86)\Windows Kits\10\Assessment and Deployment Kit\Windows Preinstallation Environment",
        ),
        PathBuf::from(
            r"C:\Program Files\Windows Kits\10\Assessment and Deployment Kit\Windows Preinstallation Environment",
        ),
    ];

    for adk_path in adk_paths {
        if !adk_path.exists() {
            continue;
        }

        // Check for amd64 (x64) first, then x86
        for (arch, arch_name) in [
            (WinpeArchitecture::X64, "amd64"),
            (WinpeArchitecture::X86, "x86"),
        ] {
            let boot_wim_path = adk_path
                .join(arch_name)
                .join("Media")
                .join("sources")
                .join("boot.wim");

            if boot_wim_path.exists() && boot_wim_path.is_file() {
                return Ok(WinpeSource {
                    boot_wim_path,
                    architecture: arch,
                    adk_path: Some(adk_path),
                });
            }
        }
    }

    Err(AcquireError::SourceNotFound(
        "Windows ADK installation not found. Please install Windows ADK or specify boot.wim path manually.".to_string(),
    ))
}

/// Validate WinPE source (boot.wim file)
///
/// Checks that the file exists and appears to be a valid WIM file.
pub fn validate_winpe_source(boot_wim_path: &Path) -> Result<WinpeArchitecture> {
    if !boot_wim_path.exists() {
        return Err(AcquireError::SourceNotFound(format!(
            "boot.wim not found: {}",
            boot_wim_path.display()
        )));
    }

    if !boot_wim_path.is_file() {
        return Err(AcquireError::SourceNotFound(format!(
            "boot.wim is not a file: {}",
            boot_wim_path.display()
        )));
    }

    // Basic WIM file validation: check for WIM signature
    // WIM files start with "MSWIM" signature at offset 0
    let mut file = std::fs::File::open(boot_wim_path)?;
    let mut header = [0u8; 8];
    std::io::Read::read_exact(&mut file, &mut header)?;

    if &header[0..5] != b"MSWIM" {
        return Err(AcquireError::Internal(format!(
            "Invalid WIM file signature: expected 'MSWIM', got '{}'",
            String::from_utf8_lossy(&header[0..5])
        )));
    }

    // Try to detect architecture from path or WIM metadata
    // For now, default to x64 if path contains "amd64" or "x64", otherwise x86
    let path_str = boot_wim_path.to_string_lossy().to_lowercase();
    let architecture = if path_str.contains("amd64") || path_str.contains("x64") {
        WinpeArchitecture::X64
    } else {
        WinpeArchitecture::X86
    };

    Ok(architecture)
}

/// Extract WIM file to USB drive
///
/// This is a placeholder implementation. Full WIM extraction requires:
/// - WIM format parser
/// - LZX decompression support
/// - File attribute preservation
///
/// # Arguments
///
/// * `wim_path` - Path to boot.wim file
/// * `usb_root` - Root directory of USB drive (FAT32 formatted)
pub fn extract_wim_to_usb(_wim_path: &Path, _usb_root: &Path) -> Result<()> {
    // TODO: Implement full WIM extraction
    // This requires:
    // 1. Parse WIM file structure
    // 2. Extract files from WIM image
    // 3. Decompress LZX-compressed data
    // 4. Preserve file attributes and directory structure
    // 5. Set up boot files (bootmgr, BCD, etc.)

    // For now, return an error indicating this needs implementation
    Err(AcquireError::UnsupportedPlatform(
        "WIM file extraction not yet implemented. Requires WIM format parser and LZX decompression.".to_string(),
    ))
}

/// Create boot configuration (BCD) for WinPE USB
///
/// This is a placeholder implementation. BCD creation requires:
/// - BCD format knowledge
/// - Boot entry configuration
///
/// # Arguments
///
/// * `usb_root` - Root directory of USB drive
/// * `boot_wim_path` - Path to boot.wim relative to USB root
pub fn create_boot_config(_usb_root: &Path, _boot_wim_path: &Path) -> Result<()> {
    // TODO: Implement BCD creation
    // This requires:
    // 1. Create \Boot\BCD file
    // 2. Configure boot entry for boot.wim
    // 3. Set boot parameters
    // 4. Copy bootmgr to USB root

    // For now, return an error indicating this needs implementation
    Err(AcquireError::UnsupportedPlatform(
        "Boot configuration (BCD) creation not yet implemented. Requires BCD format knowledge."
            .to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winpe_architecture_display() {
        assert_eq!(WinpeArchitecture::X64.as_str(), "amd64");
        assert_eq!(WinpeArchitecture::X86.as_str(), "x86");
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_find_winpe_source_non_windows() {
        let result = find_winpe_source();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Windows"));
    }

    #[test]
    fn test_validate_winpe_source_nonexistent() {
        let result = validate_winpe_source(Path::new("/nonexistent/boot.wim"));
        assert!(result.is_err());
    }
}
