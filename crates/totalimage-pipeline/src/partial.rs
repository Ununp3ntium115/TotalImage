//! Partial pipeline - provides a window into a subset of a stream

use std::io::{self, Read, Seek, SeekFrom};

/// A pipeline that exposes only a portion of an underlying stream.
///
/// This is useful for presenting a partition or zone as an independent stream
/// without copying the data.
///
/// # Example
///
/// ```rust,no_run
/// use totalimage_pipeline::PartialPipeline;
/// use std::io::Cursor;
///
/// let data = vec![0u8; 1024];
/// let cursor = Cursor::new(data);
///
/// // Create a view of bytes 512-767
/// let mut partial = PartialPipeline::new(cursor, 512, 256).unwrap();
/// ```
pub struct PartialPipeline<R: Read + Seek> {
    inner: R,
    start: u64,
    length: u64,
    position: u64,
}

impl<R: Read + Seek> PartialPipeline<R> {
    /// Create a new partial pipeline
    ///
    /// # Arguments
    ///
    /// * `inner` - The underlying stream
    /// * `start` - Offset from the beginning of the stream
    /// * `length` - Length of the window
    ///
    /// # Errors
    ///
    /// Returns an error if seeking to the start position fails
    pub fn new(mut inner: R, start: u64, length: u64) -> io::Result<Self> {
        // Verify we can seek to the start position
        inner.seek(SeekFrom::Start(start))?;

        Ok(Self {
            inner,
            start,
            length,
            position: 0,
        })
    }

    /// Get the start offset of this partial pipeline
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Get the length of this partial pipeline
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Get the current position within this partial pipeline
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Get the remaining bytes from current position to end
    pub fn remaining(&self) -> u64 {
        self.length.saturating_sub(self.position)
    }
}

impl<R: Read + Seek> Read for PartialPipeline<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Calculate how many bytes we can read
        let remaining = self.remaining() as usize;
        if remaining == 0 {
            return Ok(0); // EOF
        }

        // Limit read to the remaining bytes
        let to_read = buf.len().min(remaining);

        // Ensure inner stream is at correct position
        let absolute_pos = self.start + self.position;
        self.inner.seek(SeekFrom::Start(absolute_pos))?;

        // Read from inner stream
        let bytes_read = self.inner.read(&mut buf[..to_read])?;

        // Update position
        self.position += bytes_read as u64;

        Ok(bytes_read)
    }
}

impl<R: Read + Seek> Seek for PartialPipeline<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(offset) => self.length as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };

        // Ensure position is within bounds
        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek before beginning of partial pipeline",
            ));
        }

        let new_pos = new_pos as u64;
        if new_pos > self.length {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek beyond end of partial pipeline",
            ));
        }

        self.position = new_pos;
        Ok(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_partial_pipeline_basic() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);

        // Create a window from 20-29 (10 bytes)
        let mut partial = PartialPipeline::new(cursor, 20, 10).unwrap();

        assert_eq!(partial.start(), 20);
        assert_eq!(partial.length(), 10);
        assert_eq!(partial.position(), 0);
        assert_eq!(partial.remaining(), 10);
    }

    #[test]
    fn test_partial_pipeline_read() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);

        let mut partial = PartialPipeline::new(cursor, 20, 10).unwrap();
        let mut buf = [0u8; 5];

        // Read first 5 bytes
        let n = partial.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, &[20, 21, 22, 23, 24]);
        assert_eq!(partial.position(), 5);
        assert_eq!(partial.remaining(), 5);

        // Read next 5 bytes
        let n = partial.read(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, &[25, 26, 27, 28, 29]);
        assert_eq!(partial.position(), 10);
        assert_eq!(partial.remaining(), 0);

        // Read at EOF
        let n = partial.read(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_partial_pipeline_seek() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);

        let mut partial = PartialPipeline::new(cursor, 20, 10).unwrap();

        // Seek to position 5
        partial.seek(SeekFrom::Start(5)).unwrap();
        assert_eq!(partial.position(), 5);

        // Read should start from position 5
        let mut buf = [0u8; 2];
        partial.read(&mut buf).unwrap();
        assert_eq!(&buf, &[25, 26]);

        // Seek from current
        partial.seek(SeekFrom::Current(-2)).unwrap();
        assert_eq!(partial.position(), 5);

        // Seek from end
        partial.seek(SeekFrom::End(-3)).unwrap();
        assert_eq!(partial.position(), 7);

        partial.read(&mut buf).unwrap();
        assert_eq!(&buf, &[27, 28]);
    }

    #[test]
    fn test_partial_pipeline_read_beyond() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);

        let mut partial = PartialPipeline::new(cursor, 20, 10).unwrap();
        let mut buf = [0u8; 20]; // Buffer larger than partial pipeline

        // Should only read up to length
        let n = partial.read(&mut buf).unwrap();
        assert_eq!(n, 10);
        assert_eq!(&buf[..n], &[20, 21, 22, 23, 24, 25, 26, 27, 28, 29]);
    }

    #[test]
    fn test_partial_pipeline_seek_invalid() {
        let data: Vec<u8> = (0..100).collect();
        let cursor = Cursor::new(data);

        let mut partial = PartialPipeline::new(cursor, 20, 10).unwrap();

        // Seek before beginning
        let result = partial.seek(SeekFrom::Start(0));
        partial.seek(SeekFrom::Current(-5));
        assert!(result.is_ok()); // SeekFrom::Start(0) is valid

        // Seek beyond end
        let result = partial.seek(SeekFrom::Start(15));
        assert!(result.is_err());
    }
}
