//! USB drive detection and management
//!
//! Provides platform-specific USB drive detection for Windows, Linux, and macOS.
//! Used for WinPE bootable USB creation.

use crate::error::{AcquireError, Result};
use std::path::PathBuf;

/// Information about a detected USB drive
#[derive(Debug, Clone)]
pub struct UsbDrive {
    /// Device path (e.g., /dev/sdb on Linux, \\.\PhysicalDrive1 on Windows)
    pub device_path: PathBuf,
    /// Total size in bytes
    pub size_bytes: u64,
    /// Vendor name (if available)
    pub vendor: String,
    /// Model name (if available)
    pub model: String,
    /// Whether the drive is removable
    pub is_removable: bool,
    /// Block size in bytes (typically 512)
    pub block_size: u32,
}

impl UsbDrive {
    /// Format size for display
    pub fn size_display(&self) -> String {
        if self.size_bytes >= 1_000_000_000_000 {
            format!("{:.2} TB", self.size_bytes as f64 / 1_000_000_000_000.0)
        } else if self.size_bytes >= 1_000_000_000 {
            format!("{:.2} GB", self.size_bytes as f64 / 1_000_000_000.0)
        } else if self.size_bytes >= 1_000_000 {
            format!("{:.2} MB", self.size_bytes as f64 / 1_000_000.0)
        } else {
            format!("{} bytes", self.size_bytes)
        }
    }

    /// Check if drive is safe to use (removable and reasonable size)
    pub fn is_safe_to_use(&self) -> bool {
        self.is_removable && self.size_bytes > 0 && self.size_bytes < 2_000_000_000_000
        // < 2TB
    }
}

/// Detect all USB drives on the system
///
/// Returns a list of detected USB drives. On some platforms, this may include
/// all removable drives, not just USB devices.
pub fn detect_usb_drives() -> Result<Vec<UsbDrive>> {
    #[cfg(target_os = "windows")]
    {
        detect_usb_windows()
    }

    #[cfg(target_os = "linux")]
    {
        detect_usb_linux()
    }

    #[cfg(target_os = "macos")]
    {
        detect_usb_macos()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        Err(AcquireError::UnsupportedPlatform(
            "USB detection not supported on this platform".to_string(),
        ))
    }
}

#[cfg(target_os = "windows")]
fn detect_usb_windows() -> Result<Vec<UsbDrive>> {
    // Windows implementation using WMI or SetupAPI
    // For now, return a placeholder that indicates this needs implementation
    // TODO: Implement Windows USB detection using:
    // - WMI queries (Win32_DiskDrive, Win32_LogicalDisk)
    // - Or SetupAPI enumeration
    // - Check removable flag and bus type (USB)

    // Placeholder: return empty list with note that this needs implementation
    Ok(vec![])
}

#[cfg(target_os = "linux")]
fn detect_usb_linux() -> Result<Vec<UsbDrive>> {
    use std::fs;
    use std::io;

    let mut drives = Vec::new();

    // Parse /sys/block to find removable devices
    let sys_block = match fs::read_dir("/sys/block") {
        Ok(dir) => dir,
        Err(e) => {
            return Err(AcquireError::ReadError(format!(
                "Failed to read /sys/block: {}",
                e
            )));
        }
    };

    for entry in sys_block {
        let entry = entry.map_err(|e| {
            AcquireError::ReadError(format!("Failed to read /sys/block entry: {}", e))
        })?;

        let device_name = entry.file_name();
        let device_name_str = device_name.to_string_lossy();

        // Skip loop devices and other non-physical devices
        if device_name_str.starts_with("loop") || device_name_str.starts_with("ram") {
            continue;
        }

        let device_path = PathBuf::from(format!("/dev/{}", device_name_str));

        // Check if device is removable
        let removable_path = entry.path().join("removable");
        let is_removable = fs::read_to_string(&removable_path)
            .map(|s| s.trim() == "1")
            .unwrap_or(false);

        // Only include removable devices (USB drives)
        if !is_removable {
            continue;
        }

        // Get size
        let size_path = entry.path().join("size");
        let size_sectors = fs::read_to_string(&size_path)
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        // Get vendor and model
        let vendor = fs::read_to_string(entry.path().join("device/vendor"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let model = fs::read_to_string(entry.path().join("device/model"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        // Block size is typically 512, but check if available
        let block_size = fs::read_to_string(entry.path().join("queue/logical_block_size"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            .unwrap_or(512);

        drives.push(UsbDrive {
            device_path,
            size_bytes: size_sectors * block_size as u64,
            vendor,
            model,
            is_removable: true,
            block_size,
        });
    }

    Ok(drives)
}

#[cfg(target_os = "macos")]
fn detect_usb_macos() -> Result<Vec<UsbDrive>> {
    use std::process::Command;

    // Use diskutil to list disks
    let output = Command::new("diskutil")
        .arg("list")
        .arg("-plist")
        .output()
        .map_err(|e| AcquireError::ReadError(format!("Failed to run diskutil: {}", e)))?;

    if !output.status.success() {
        return Err(AcquireError::ReadError(
            "diskutil command failed".to_string(),
        ));
    }

    // Parse plist output
    // For now, return empty list - full implementation would parse the plist
    // and identify removable/USB drives
    // TODO: Implement plist parsing to extract:
    // - Device identifier (e.g., disk2)
    // - Size
    // - Removable flag
    // - Vendor/Model

    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usb_drive_size_display() {
        let drive = UsbDrive {
            device_path: PathBuf::from("/dev/sdb"),
            size_bytes: 16_000_000_000, // 16 GB
            vendor: "Test".to_string(),
            model: "USB Drive".to_string(),
            is_removable: true,
            block_size: 512,
        };

        assert!(drive.size_display().contains("GB"));
        assert!(drive.is_safe_to_use());
    }

    #[test]
    fn test_usb_drive_safety_check() {
        let safe_drive = UsbDrive {
            device_path: PathBuf::from("/dev/sdb"),
            size_bytes: 8_000_000_000, // 8 GB
            vendor: "Test".to_string(),
            model: "USB Drive".to_string(),
            is_removable: true,
            block_size: 512,
        };

        let unsafe_drive = UsbDrive {
            device_path: PathBuf::from("/dev/sda"),
            size_bytes: 1_000_000_000_000, // 1 TB
            vendor: "Test".to_string(),
            model: "Internal".to_string(),
            is_removable: false,
            block_size: 512,
        };

        assert!(safe_drive.is_safe_to_use());
        assert!(!unsafe_drive.is_safe_to_use());
    }
}
