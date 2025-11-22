//! Raw vault - Direct sector image container
//!
//! This module implements the simplest vault type: a raw sector image with no
//! container metadata. Common file extensions: .img, .ima, .flp, .vfd, .dsk, .iso

use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;
use totalimage_core::{Result, Vault, ReadSeek};
use totalimage_pipeline::MmapPipeline;

/// Configuration for opening a vault
#[derive(Debug, Clone)]
pub struct VaultConfig {
    /// Use memory mapping for direct action (high performance)
    pub use_mmap: bool,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self { use_mmap: true }
    }
}

/// Raw vault - a simple passthrough to the underlying file
///
/// This is the most common and simplest vault type. It provides direct access
/// to disk images without any container metadata.
///
/// # Example
///
/// ```rust,no_run
/// use totalimage_vaults::{RawVault, VaultConfig};
/// use totalimage_core::Vault;
/// use std::path::Path;
///
/// let vault = RawVault::open(Path::new("disk.img"), VaultConfig::default()).unwrap();
/// println!("Vault type: {}", vault.identify());
/// println!("Size: {} bytes", vault.length());
/// ```
pub struct RawVault {
    pipeline: Box<dyn ReadSeek>,
    length: u64,
}

impl RawVault {
    /// Open a raw vault from a file path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the disk image file
    /// * `config` - Configuration for opening the vault
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or accessed
    pub fn open(path: &Path, config: VaultConfig) -> Result<Self> {
        let file = File::open(path)?;
        let length = file.metadata()?.len();

        let pipeline: Box<dyn ReadSeek> = if config.use_mmap {
            // Direct action: memory-mapped file
            Box::new(MmapPipeline::from_file(&file)?)
        } else {
            // Standard file stream
            Box::new(file)
        };

        Ok(Self { pipeline, length })
    }

    /// Create a new raw vault from any readable and seekable stream
    ///
    /// # Arguments
    ///
    /// * `stream` - Any stream that implements Read + Seek
    /// * `length` - The length of the stream in bytes
    pub fn from_stream<R: Read + Seek + Send + Sync + 'static>(stream: R, length: u64) -> Self {
        Self {
            pipeline: Box::new(stream),
            length,
        }
    }

    /// Manufacture a new blank raw vault (for image creation)
    ///
    /// Creates a new in-memory raw vault filled with zeros.
    ///
    /// # Arguments
    ///
    /// * `size` - Size of the vault in bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use totalimage_vaults::RawVault;
    /// use totalimage_core::Vault;
    ///
    /// // Create a blank 1.44MB floppy image
    /// let vault = RawVault::manufacture(1_474_560);
    /// assert_eq!(vault.length(), 1_474_560);
    /// ```
    pub fn manufacture(size: u64) -> Self {
        use std::io::Cursor;

        let buffer = vec![0u8; size as usize];
        let cursor = Cursor::new(buffer);

        Self {
            pipeline: Box::new(cursor),
            length: size,
        }
    }
}

impl Vault for RawVault {
    fn identify(&self) -> &str {
        "Raw sector image"
    }

    fn length(&self) -> u64 {
        self.length
    }

    fn content(&mut self) -> &mut dyn ReadSeek {
        &mut *self.pipeline
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::NamedTempFile;

    #[test]
    fn test_raw_vault_from_stream() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data.clone());

        let vault = RawVault::from_stream(cursor, 100);

        assert_eq!(vault.identify(), "Raw sector image");
        assert_eq!(vault.length(), 100);
    }

    #[test]
    fn test_raw_vault_manufacture() {
        let vault = RawVault::manufacture(1024);

        assert_eq!(vault.identify(), "Raw sector image");
        assert_eq!(vault.length(), 1024);
    }

    #[test]
    fn test_raw_vault_content_read() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data.clone());

        let mut vault = RawVault::from_stream(cursor, 100);

        let mut buf = [0u8; 10];
        vault.content().read(&mut buf).unwrap();

        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_raw_vault_content_seek() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data.clone());

        let mut vault = RawVault::from_stream(cursor, 100);

        use std::io::SeekFrom;
        vault.content().seek(SeekFrom::Start(50)).unwrap();

        let mut buf = [0u8; 5];
        vault.content().read(&mut buf).unwrap();

        assert_eq!(&buf, &[50, 51, 52, 53, 54]);
    }

    #[test]
    fn test_raw_vault_open_file() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0..100).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let vault = RawVault::open(tmpfile.path(), VaultConfig::default()).unwrap();

        assert_eq!(vault.identify(), "Raw sector image");
        assert_eq!(vault.length(), 100);
    }

    #[test]
    fn test_raw_vault_open_with_mmap() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let config = VaultConfig { use_mmap: true };
        let mut vault = RawVault::open(tmpfile.path(), config).unwrap();

        let mut buf = [0u8; 10];
        vault.content().read(&mut buf).unwrap();
        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn test_raw_vault_open_without_mmap() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0u8..=255).cycle().take(1000).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let config = VaultConfig { use_mmap: false };
        let mut vault = RawVault::open(tmpfile.path(), config).unwrap();

        let mut buf = [0u8; 10];
        vault.content().read(&mut buf).unwrap();
        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }
}
