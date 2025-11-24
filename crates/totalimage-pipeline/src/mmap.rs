//! Memory-mapped pipeline for direct action (high-performance I/O)

use memmap2::Mmap;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// A pipeline backed by a memory-mapped file for high-performance access.
///
/// This provides "direct action" - immediate access to file contents without
/// system call overhead for each read.
///
/// # Example
///
/// ```rust,no_run
/// use totalimage_pipeline::MmapPipeline;
/// use std::path::Path;
///
/// let mut pipeline = MmapPipeline::open(Path::new("disk.img")).unwrap();
/// ```
pub struct MmapPipeline {
    mmap: Mmap,
    position: u64,
}

impl MmapPipeline {
    /// Open a file with memory mapping
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to open
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or mapped
    ///
    /// # Security
    ///
    /// Validates file before mapping:
    /// - Ensures file is a regular file (not device, pipe, etc.)
    /// - Checks file size is within reasonable limits
    /// - Uses read-only mapping to prevent accidental writes
    ///
    /// # Safety
    ///
    /// Uses `unsafe` for memory mapping because:
    /// - The OS guarantees memory safety for valid file descriptors
    /// - We validate the file is a regular file before mapping
    /// - We use MAP_PRIVATE (read-only) to prevent modification
    /// - File must not be truncated during access (caller responsibility)
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        // Validate file is a regular file (not device, pipe, directory, etc.)
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Only regular files can be memory-mapped"
            ));
        }

        // Check file size is within reasonable limits for memory mapping
        use totalimage_core::MAX_MMAP_SIZE;
        if metadata.len() > MAX_MMAP_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "File size {} exceeds memory mapping limit {} (16 GB)",
                    metadata.len(),
                    MAX_MMAP_SIZE
                )
            ));
        }

        // SAFETY: We've validated:
        // 1. File is a regular file (not a device or pipe)
        // 2. File size is reasonable and won't exhaust memory
        // 3. File descriptor is valid (File::open succeeded)
        // 4. Mmap uses MAP_PRIVATE by default (read-only, no write-through)
        let mmap = unsafe { Mmap::map(&file)? };

        Ok(Self { mmap, position: 0 })
    }

    /// Create a memory-mapped pipeline from an existing file
    ///
    /// # Security
    ///
    /// Validates file before mapping (same checks as `open()`)
    ///
    /// # Safety
    ///
    /// See `open()` for safety documentation
    pub fn from_file(file: &File) -> io::Result<Self> {
        let metadata = file.metadata()?;

        // Validate file is a regular file
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Only regular files can be memory-mapped"
            ));
        }

        // Check file size limit
        use totalimage_core::MAX_MMAP_SIZE;
        if metadata.len() > MAX_MMAP_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "File size {} exceeds memory mapping limit {} (16 GB)",
                    metadata.len(),
                    MAX_MMAP_SIZE
                )
            ));
        }

        // SAFETY: Same guarantees as open()
        let mmap = unsafe { Mmap::map(file)? };
        Ok(Self { mmap, position: 0 })
    }

    /// Get the length of the mapped region
    pub fn len(&self) -> u64 {
        self.mmap.len() as u64
    }

    /// Check if the mapped region is empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Get the current position
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get remaining bytes from current position
    pub fn remaining(&self) -> u64 {
        self.len().saturating_sub(self.position)
    }

    /// Get a slice of the mapped data at the current position
    pub fn as_slice(&self) -> &[u8] {
        &self.mmap[self.position as usize..]
    }

    /// Get a slice of the entire mapped data
    pub fn as_full_slice(&self) -> &[u8] {
        &self.mmap
    }
}

impl Read for MmapPipeline {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let remaining = self.remaining() as usize;
        if remaining == 0 {
            return Ok(0); // EOF
        }

        let to_read = buf.len().min(remaining);
        let start = self.position as usize;
        let end = start + to_read;

        buf[..to_read].copy_from_slice(&self.mmap[start..end]);
        self.position += to_read as u64;

        Ok(to_read)
    }
}

impl Seek for MmapPipeline {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.len() as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before beginning of file",
            ));
        }

        let new_pos = new_pos as u64;
        if new_pos > self.len() {
            // Allow seeking past EOF (standard behavior)
            self.position = new_pos;
        } else {
            self.position = new_pos;
        }

        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_pipeline_basic() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0..100).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let pipeline = MmapPipeline::open(tmpfile.path()).unwrap();

        assert_eq!(pipeline.len(), 100);
        assert_eq!(pipeline.position(), 0);
        assert_eq!(pipeline.remaining(), 100);
        assert!(!pipeline.is_empty());
    }

    #[test]
    fn test_mmap_pipeline_read() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0..100).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let mut pipeline = MmapPipeline::open(tmpfile.path()).unwrap();
        let mut buf = [0u8; 10];

        // Read first 10 bytes
        let n = pipeline.read(&mut buf).unwrap();
        assert_eq!(n, 10);
        assert_eq!(&buf, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(pipeline.position(), 10);
    }

    #[test]
    fn test_mmap_pipeline_seek() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0..100).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let mut pipeline = MmapPipeline::open(tmpfile.path()).unwrap();

        // Seek to position 50
        pipeline.seek(SeekFrom::Start(50)).unwrap();
        assert_eq!(pipeline.position(), 50);

        let mut buf = [0u8; 5];
        pipeline.read(&mut buf).unwrap();
        assert_eq!(&buf, &[50, 51, 52, 53, 54]);
    }

    #[test]
    fn test_mmap_pipeline_as_slice() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        let data: Vec<u8> = (0..100).collect();
        tmpfile.write_all(&data).unwrap();
        tmpfile.flush().unwrap();

        let mut pipeline = MmapPipeline::open(tmpfile.path()).unwrap();

        // Get full slice
        assert_eq!(pipeline.as_full_slice().len(), 100);
        assert_eq!(pipeline.as_full_slice()[0], 0);
        assert_eq!(pipeline.as_full_slice()[99], 99);

        // Seek and get slice from current position
        pipeline.seek(SeekFrom::Start(50)).unwrap();
        let slice = pipeline.as_slice();
        assert_eq!(slice.len(), 50);
        assert_eq!(slice[0], 50);
    }
}
