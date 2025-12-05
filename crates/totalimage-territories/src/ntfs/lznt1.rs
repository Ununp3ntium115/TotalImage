//! LZNT1 Decompression for NTFS Compressed Files
//!
//! LZNT1 is the compression algorithm used by NTFS for compressed files.
//! This module implements decompression according to the Microsoft specification:
//! https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-xca/5655f4a3-6ba4-489b-959f-e1f407c52f15
//!
//! ## Algorithm Overview
//!
//! LZNT1 uses a sliding window compression similar to LZ77:
//! - Compressed data is divided into 4KB chunks
//! - Each chunk begins with a 2-byte header
//! - Compressed symbols are either literals (1 byte) or back-references (2 bytes)
//! - Back-references encode (offset, length) pairs pointing to previous data
//!
//! ## Format Details
//!
//! **Chunk Header (2 bytes):**
//! - Bits 0-11: Chunk size (size of compressed data + header - 3)
//! - Bit 12-14: Signature (0b000)
//! - Bit 15: Compression flag (1 = compressed, 0 = uncompressed)
//!
//! **Compressed Data:**
//! - 1-byte flags byte (8 bits, read right-to-left)
//! - Each bit indicates: 0 = literal byte, 1 = back-reference (2 bytes)
//!
//! **Back-reference format:**
//! - Variable bit layout depending on position in decompressed buffer
//! - Encodes (offset, length) where offset points backwards in output

use std::io::{self, Read};

/// Maximum size of a single decompressed chunk (4 KB)
const MAX_CHUNK_SIZE: usize = 4096;

/// LZNT1 compression signature in chunk header (bits 12-14 must be 0)
const COMPRESSION_SIGNATURE: u16 = 0b000;

/// Decompression error
#[derive(Debug, thiserror::Error)]
pub enum Lznt1Error {
    #[error("Invalid chunk header")]
    InvalidChunkHeader,

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid back-reference")]
    InvalidBackReference,

    #[error("Output buffer overflow")]
    BufferOverflow,

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Lznt1Error>;

/// Decompress LZNT1-compressed data
///
/// # Arguments
///
/// * `input` - Compressed data stream
/// * `uncompressed_size` - Expected size of decompressed data
///
/// # Returns
///
/// Decompressed data as a Vec<u8>
///
/// # Errors
///
/// Returns an error if:
/// - Chunk headers are invalid
/// - Back-references are out of bounds
/// - Compressed data is truncated
pub fn decompress<R: Read>(mut input: R, uncompressed_size: usize) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(uncompressed_size);

    // Process chunks until we've decompressed the expected amount
    while output.len() < uncompressed_size {
        let chunk = decompress_chunk(&mut input)?;

        if chunk.is_empty() {
            break; // End of compressed data
        }

        output.extend_from_slice(&chunk);
    }

    // Truncate to exact size if we decompressed more than expected
    output.truncate(uncompressed_size);

    Ok(output)
}

/// Decompress a single LZNT1 chunk (up to 4KB)
fn decompress_chunk<R: Read>(input: &mut R) -> Result<Vec<u8>> {
    // Read 2-byte chunk header
    let mut header_bytes = [0u8; 2];
    match input.read_exact(&mut header_bytes) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
            // End of data
            return Ok(Vec::new());
        }
        Err(e) => return Err(e.into()),
    }

    let header = u16::from_le_bytes(header_bytes);

    // Parse header
    let header_chunk_size = (header & 0x0FFF) as usize; // Bits 0-11
    let signature = (header >> 12) & 0x07; // Bits 12-14
    let is_compressed = (header >> 15) & 0x01 == 1; // Bit 15

    // Validate signature
    if signature != COMPRESSION_SIGNATURE {
        return Err(Lznt1Error::InvalidChunkHeader);
    }

    // The header value represents (chunk_size_in_bytes - 3)
    // So actual chunk size = header_value + 3
    // This includes the 2-byte header we already read
    // Data size = (header_value + 3) - 2 = header_value + 1
    let data_size = header_chunk_size + 1;
    let mut chunk_data = vec![0u8; data_size];
    input.read_exact(&mut chunk_data)?;

    if !is_compressed {
        // Uncompressed chunk: return data as-is
        return Ok(chunk_data);
    }

    // Decompress chunk
    decompress_chunk_data(&chunk_data)
}

/// Decompress the data portion of a compressed chunk
fn decompress_chunk_data(input: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::with_capacity(MAX_CHUNK_SIZE);
    let mut input_pos = 0;

    while input_pos < input.len() {
        // Read flags byte
        if input_pos >= input.len() {
            break;
        }
        let flags = input[input_pos];
        input_pos += 1;

        // Process 8 symbols (bits 0-7)
        for bit_index in 0..8 {
            if input_pos >= input.len() {
                break;
            }

            let is_reference = (flags >> bit_index) & 1 == 1;

            if is_reference {
                // Back-reference (2 bytes)
                if input_pos + 1 >= input.len() {
                    return Err(Lznt1Error::UnexpectedEof);
                }

                let token = u16::from_le_bytes([input[input_pos], input[input_pos + 1]]);
                input_pos += 2;

                // Decode back-reference based on current output position
                let (offset, length) = decode_back_reference(token, output.len())?;

                // Copy data from earlier in output buffer
                if offset > output.len() {
                    return Err(Lznt1Error::InvalidBackReference);
                }

                let copy_start = output.len() - offset;

                // Handle overlapping copies (e.g., repeating patterns)
                for _ in 0..length {
                    if copy_start + (output.len() - copy_start) < output.len() {
                        return Err(Lznt1Error::InvalidBackReference);
                    }
                    let byte = output[copy_start + (output.len() - copy_start - offset)];
                    output.push(byte);

                    if output.len() >= MAX_CHUNK_SIZE {
                        return Ok(output); // Chunk complete
                    }
                }
            } else {
                // Literal byte
                output.push(input[input_pos]);
                input_pos += 1;

                if output.len() >= MAX_CHUNK_SIZE {
                    return Ok(output); // Chunk complete
                }
            }
        }
    }

    Ok(output)
}

/// Decode a back-reference token into (offset, length)
///
/// The bit layout varies based on the current position in the output buffer:
/// - Larger offsets available as we decompress more data
/// - Length is encoded in the remaining bits
fn decode_back_reference(token: u16, output_pos: usize) -> Result<(usize, usize)> {
    // Calculate the number of bits needed for offset based on output position
    let mut offset_bits = 0;
    let mut temp_pos = output_pos;

    while temp_pos >= 0x10 {
        offset_bits += 1;
        temp_pos >>= 1;
    }

    offset_bits = offset_bits.max(4); // Minimum 4 bits for offset
    let length_bits = 16 - offset_bits;

    // Extract offset and length from token
    let offset_mask = (1u16 << offset_bits) - 1;
    let length_mask = (1u16 << length_bits) - 1;

    let offset = ((token >> length_bits) & offset_mask) as usize;
    let length = (token & length_mask) as usize;

    // Offset is relative to current position
    let actual_offset = offset + 1;
    let actual_length = length + 3; // Minimum match length is 3

    Ok((actual_offset, actual_length))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_uncompressed_chunk() {
        // Chunk header: bits 0-11 = 0x009 (9)
        // Actual chunk size = 9 + 3 = 12 bytes (2 byte header + 10 byte data)
        // Data size = 10 bytes
        let mut data = vec![
            0x09, 0x00, // Header: chunk_size=12 (stored as 9), uncompressed
        ];

        // Add 10 bytes of data
        data.extend_from_slice(b"HelloWorld");

        let mut cursor = Cursor::new(data);
        let result = decompress_chunk(&mut cursor).unwrap();

        assert_eq!(result, b"HelloWorld");
    }

    #[test]
    fn test_compressed_chunk_literals_only() {
        // Chunk with all literals (flags = 0x00 for all literal bytes)
        // Header: bits 0-11 = 0x005 (5), bit 15 = 1 (compressed)
        // Actual chunk size = 5 + 3 = 8 bytes (2 byte header + 6 byte data)
        let data = vec![
            0x05, 0x80, // Header: chunk_size=8 (stored as 5), compressed
            0x00, // Flags: all literals
            b'H', b'e', b'l', b'l', b'o',
        ];

        let mut cursor = Cursor::new(data);
        let result = decompress_chunk(&mut cursor).unwrap();

        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_invalid_chunk_header_signature() {
        // Header with invalid signature (bits 12-14 should be 0)
        let data = vec![
            0x0A, 0x10, // Invalid signature
        ];

        let mut cursor = Cursor::new(data);
        let result = decompress_chunk(&mut cursor);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            Lznt1Error::InvalidChunkHeader
        ));
    }

    #[test]
    fn test_back_reference_decode() {
        // Test back-reference decoding at various output positions

        // At position 16 (0x10), we need 4 bits for offset, 12 bits for length
        let (offset, length) = decode_back_reference(0x1000, 16).unwrap();
        // offset = 0x1 = 1 -> actual = 2
        // length = 0x000 = 0 -> actual = 3
        assert_eq!(offset, 2);
        assert_eq!(length, 3);
    }

    #[test]
    fn test_empty_input() {
        let data = vec![];
        let mut cursor = Cursor::new(data);
        let result = decompress_chunk(&mut cursor).unwrap();

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_decompress_small_data() {
        // Create a simple compressed stream with one chunk
        // Header: bits 0-11 = 0x004 (4), bit 15 = 1 (compressed)
        // Actual chunk size = 4 + 3 = 7 bytes (2 byte header + 5 byte data)
        let data = vec![
            0x04, 0x80, // Header: chunk_size=7 (stored as 4), compressed
            0x00, // Flags: all literals
            b'T', b'e', b's', b't',
        ];

        let result = decompress(Cursor::new(data), 4).unwrap();
        assert_eq!(result, b"Test");
    }

    #[test]
    fn test_decompress_truncates_to_expected_size() {
        // Decompress more data than requested
        // Header: bits 0-11 = 0x008 (8), bit 15 = 1 (compressed)
        // Actual chunk size = 8 + 3 = 11 bytes (2 byte header + 9 byte data)
        let data = vec![
            0x08, 0x80, // Header: chunk_size=11 (stored as 8), compressed
            0x00, // Flags: all literals
            b'H', b'e', b'l', b'l', b'o', b'X', b'X', b'X',
        ];

        let result = decompress(Cursor::new(data), 5).unwrap();
        assert_eq!(result, b"Hello");
    }

    #[test]
    fn test_max_chunk_size() {
        assert_eq!(MAX_CHUNK_SIZE, 4096);
    }

    #[test]
    fn test_compression_signature() {
        assert_eq!(COMPRESSION_SIGNATURE, 0b000);
    }
}
