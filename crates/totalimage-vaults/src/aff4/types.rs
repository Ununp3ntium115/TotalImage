//! AFF4 (Advanced Forensic Format 4) type definitions
//!
//! AFF4 is a forensic disk image format that uses:
//! - ZIP container for storage
//! - RDF/Turtle metadata for semantic information
//! - Compression (deflate, snappy, lz4)
//! - Chunked image streams

use totalimage_core::{Error, Result};

/// AFF4 namespace URIs
pub mod namespace {
    pub const AFF4: &str = "http://aff4.org/Schema#";
    pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
    pub const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
}

/// AFF4 object types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aff4ObjectType {
    /// Image stream (disk image data)
    ImageStream,
    /// Map stream (sparse image)
    Map,
    /// Zip volume container
    ZipVolume,
    /// Directory
    Directory,
    /// Unknown type
    Unknown,
}

impl Aff4ObjectType {
    /// Parse from RDF type URI
    pub fn from_uri(uri: &str) -> Self {
        if uri.contains("ImageStream") {
            Self::ImageStream
        } else if uri.contains("Map") {
            Self::Map
        } else if uri.contains("ZipVolume") {
            Self::ZipVolume
        } else if uri.contains("Directory") {
            Self::Directory
        } else {
            Self::Unknown
        }
    }
}

/// AFF4 compression method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aff4Compression {
    /// No compression (stored)
    None,
    /// Deflate (zlib)
    Deflate,
    /// Snappy compression
    Snappy,
    /// LZ4 compression
    Lz4,
    /// Unknown compression
    Unknown(u8),
}

impl Aff4Compression {
    /// Parse from compression URI
    pub fn from_uri(uri: &str) -> Self {
        if uri.contains("NullCompressor") || uri.contains("stored") {
            Self::None
        } else if uri.contains("DeflateCompressor") || uri.contains("deflate") {
            Self::Deflate
        } else if uri.contains("SnappyCompressor") || uri.contains("snappy") {
            Self::Snappy
        } else if uri.contains("Lz4Compressor") || uri.contains("lz4") {
            Self::Lz4
        } else {
            Self::Unknown(0)
        }
    }
}

/// AFF4 image stream metadata
#[derive(Debug, Clone)]
pub struct Aff4ImageStream {
    /// URN of this stream
    pub urn: String,
    /// Size of the image in bytes
    pub size: u64,
    /// Chunk size in bytes
    pub chunk_size: u32,
    /// Chunks per segment
    pub chunks_per_segment: u32,
    /// Compression method
    pub compression: Aff4Compression,
    /// Data stream path within ZIP
    pub data_path: Option<String>,
    /// Index path within ZIP
    pub index_path: Option<String>,
}

impl Default for Aff4ImageStream {
    fn default() -> Self {
        Self {
            urn: String::new(),
            size: 0,
            chunk_size: 32768, // Default 32KB chunks
            chunks_per_segment: 2048,
            compression: Aff4Compression::Deflate,
            data_path: None,
            index_path: None,
        }
    }
}

/// AFF4 volume metadata
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct Aff4Volume {
    /// Volume URN
    pub urn: String,
    /// Creation time
    pub creation_time: Option<String>,
    /// Tool that created this volume
    pub tool: Option<String>,
    /// Tool version
    pub tool_version: Option<String>,
    /// Image streams in this volume
    pub streams: Vec<Aff4ImageStream>,
}


/// AFF4 container information
#[derive(Debug, Clone)]
pub struct Aff4Container {
    /// Main volume
    pub volume: Aff4Volume,
    /// All RDF statements (simplified)
    pub statements: Vec<Aff4Statement>,
}

/// Simple RDF statement (subject, predicate, object)
#[derive(Debug, Clone)]
pub struct Aff4Statement {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

/// AFF4 bevy index entry
#[derive(Debug, Clone, Copy)]
pub struct Aff4BevyIndexEntry {
    /// Offset to chunk data
    pub offset: u64,
    /// Length of chunk data
    pub length: u32,
}

impl Aff4BevyIndexEntry {
    /// Size of an index entry in bytes
    pub const SIZE: usize = 12;

    /// Parse from bytes (little-endian)
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < Self::SIZE {
            return Err(Error::invalid_vault("AFF4 bevy index entry too small"));
        }

        let offset = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]);
        let length = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);

        Ok(Self { offset, length })
    }
}

/// Simple Turtle RDF parser for AFF4 metadata
pub struct TurtleParser;

impl TurtleParser {
    /// Parse Turtle RDF content into statements
    pub fn parse(content: &str) -> Vec<Aff4Statement> {
        let mut statements = Vec::new();
        let mut prefixes: Vec<(String, String)> = Vec::new();
        let mut current_subject = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse prefix declarations
            if line.starts_with("@prefix") {
                if let Some((prefix, uri)) = Self::parse_prefix(line) {
                    prefixes.push((prefix, uri));
                }
                continue;
            }

            // Parse statements
            if let Some(stmt) = Self::parse_statement(line, &prefixes, &current_subject) {
                if !stmt.subject.is_empty() {
                    current_subject = stmt.subject.clone();
                }
                statements.push(stmt);
            }
        }

        statements
    }

    /// Parse a prefix declaration
    fn parse_prefix(line: &str) -> Option<(String, String)> {
        // Format: @prefix prefix: <uri> .
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let prefix = parts[1].trim_end_matches(':').to_string();
            let uri = parts[2].trim_start_matches('<').trim_end_matches('>').to_string();
            return Some((prefix, uri));
        }
        None
    }

    /// Parse a statement
    fn parse_statement(
        line: &str,
        prefixes: &[(String, String)],
        current_subject: &str,
    ) -> Option<Aff4Statement> {
        let line = line.trim_end_matches('.');
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() {
            return None;
        }

        // Determine subject, predicate, object
        let (subject, pred_idx) = if parts[0].starts_with('<') || parts[0].contains(':') {
            (Self::expand_uri(parts[0], prefixes), 1)
        } else {
            (current_subject.to_string(), 0)
        };

        if parts.len() <= pred_idx + 1 {
            return None;
        }

        let predicate = Self::expand_uri(parts[pred_idx], prefixes);
        let object = parts[pred_idx + 1..].join(" ");
        let object = Self::expand_uri(&object, prefixes);

        Some(Aff4Statement {
            subject,
            predicate,
            object,
        })
    }

    /// Expand prefixed URI
    fn expand_uri(uri: &str, prefixes: &[(String, String)]) -> String {
        // Handle <uri> format
        if uri.starts_with('<') && uri.ends_with('>') {
            return uri[1..uri.len() - 1].to_string();
        }

        // Handle quoted strings
        if let Some(stripped) = uri.strip_prefix('"') {
            let end = stripped.find('"').unwrap_or(stripped.len());
            return stripped[..end].to_string();
        }

        // Handle prefixed URIs (prefix:localname)
        if let Some(colon_pos) = uri.find(':') {
            let prefix = &uri[..colon_pos];
            let local = &uri[colon_pos + 1..];

            for (p, u) in prefixes {
                if p == prefix {
                    return format!("{}{}", u, local);
                }
            }
        }

        uri.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aff4_object_type_from_uri() {
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#ImageStream"),
            Aff4ObjectType::ImageStream
        );
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#Map"),
            Aff4ObjectType::Map
        );
        assert_eq!(
            Aff4ObjectType::from_uri("http://aff4.org/Schema#ZipVolume"),
            Aff4ObjectType::ZipVolume
        );
    }

    #[test]
    fn test_aff4_compression_from_uri() {
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#DeflateCompressor"),
            Aff4Compression::Deflate
        );
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#SnappyCompressor"),
            Aff4Compression::Snappy
        );
        assert_eq!(
            Aff4Compression::from_uri("http://aff4.org/Schema#NullCompressor"),
            Aff4Compression::None
        );
    }

    #[test]
    fn test_bevy_index_entry_parse() {
        let mut bytes = [0u8; 12];
        bytes[0..8].copy_from_slice(&1000u64.to_le_bytes());
        bytes[8..12].copy_from_slice(&500u32.to_le_bytes());

        let entry = Aff4BevyIndexEntry::parse(&bytes).unwrap();
        assert_eq!(entry.offset, 1000);
        assert_eq!(entry.length, 500);
    }

    #[test]
    fn test_turtle_parser_prefix() {
        let content = r#"
@prefix aff4: <http://aff4.org/Schema#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

<aff4://test> rdf:type aff4:ImageStream .
<aff4://test> aff4:size "1024" .
"#;

        let statements = TurtleParser::parse(content);
        assert!(statements.len() >= 2);
    }

    #[test]
    fn test_turtle_expand_uri() {
        let prefixes = vec![
            ("aff4".to_string(), "http://aff4.org/Schema#".to_string()),
        ];

        let expanded = TurtleParser::expand_uri("aff4:ImageStream", &prefixes);
        assert_eq!(expanded, "http://aff4.org/Schema#ImageStream");

        let quoted = TurtleParser::expand_uri("\"hello world\"", &prefixes);
        assert_eq!(quoted, "hello world");
    }
}
