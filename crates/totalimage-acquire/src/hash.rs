//! Hash computation for forensic verification
//!
//! Supports MD5, SHA1, and SHA256 algorithms for chain of custody.

use md5::{Md5, Digest};
use sha1::Sha1;
use sha2::Sha256;
use std::io::Read;

/// Supported hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// MD5 (128-bit) - fast but cryptographically broken
    Md5,
    /// SHA-1 (160-bit) - legacy support
    Sha1,
    /// SHA-256 (256-bit) - recommended for forensics
    Sha256,
}

impl HashAlgorithm {
    /// Get the output size in bytes
    pub fn output_size(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 16,
            HashAlgorithm::Sha1 => 20,
            HashAlgorithm::Sha256 => 32,
        }
    }

    /// Get the algorithm name
    pub fn name(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA1",
            HashAlgorithm::Sha256 => "SHA256",
        }
    }
}

/// Hash computation result
#[derive(Debug, Clone)]
pub struct HashResult {
    /// Algorithm used
    pub algorithm: HashAlgorithm,
    /// Hash bytes
    pub hash: Vec<u8>,
    /// Hex string representation
    pub hex: String,
}

impl HashResult {
    /// Create a new hash result
    pub fn new(algorithm: HashAlgorithm, hash: Vec<u8>) -> Self {
        let hex = hex::encode(&hash);
        Self { algorithm, hash, hex }
    }

    /// Verify that this hash matches another
    pub fn matches(&self, other: &HashResult) -> bool {
        self.algorithm == other.algorithm && self.hash == other.hash
    }

    /// Verify against a hex string
    pub fn matches_hex(&self, hex: &str) -> bool {
        self.hex.eq_ignore_ascii_case(hex)
    }
}

/// Multi-algorithm hasher for computing hashes during acquisition
pub struct Hasher {
    md5: Option<Md5>,
    sha1: Option<Sha1>,
    sha256: Option<Sha256>,
    bytes_processed: u64,
}

impl Hasher {
    /// Create a new hasher with specified algorithms
    pub fn new(algorithms: &[HashAlgorithm]) -> Self {
        let md5 = if algorithms.contains(&HashAlgorithm::Md5) {
            Some(Md5::new())
        } else {
            None
        };

        let sha1 = if algorithms.contains(&HashAlgorithm::Sha1) {
            Some(Sha1::new())
        } else {
            None
        };

        let sha256 = if algorithms.contains(&HashAlgorithm::Sha256) {
            Some(Sha256::new())
        } else {
            None
        };

        Self {
            md5,
            sha1,
            sha256,
            bytes_processed: 0,
        }
    }

    /// Create a hasher with all algorithms enabled
    pub fn all() -> Self {
        Self::new(&[HashAlgorithm::Md5, HashAlgorithm::Sha1, HashAlgorithm::Sha256])
    }

    /// Update the hasher with data
    pub fn update(&mut self, data: &[u8]) {
        if let Some(ref mut h) = self.md5 {
            h.update(data);
        }
        if let Some(ref mut h) = self.sha1 {
            h.update(data);
        }
        if let Some(ref mut h) = self.sha256 {
            h.update(data);
        }
        self.bytes_processed += data.len() as u64;
    }

    /// Finalize and return all hash results
    pub fn finalize(self) -> Vec<HashResult> {
        let mut results = Vec::new();

        if let Some(h) = self.md5 {
            let hash = h.finalize().to_vec();
            results.push(HashResult::new(HashAlgorithm::Md5, hash));
        }

        if let Some(h) = self.sha1 {
            let hash = h.finalize().to_vec();
            results.push(HashResult::new(HashAlgorithm::Sha1, hash));
        }

        if let Some(h) = self.sha256 {
            let hash = h.finalize().to_vec();
            results.push(HashResult::new(HashAlgorithm::Sha256, hash));
        }

        results
    }

    /// Get bytes processed
    pub fn bytes_processed(&self) -> u64 {
        self.bytes_processed
    }
}

/// Compute hash of a reader
pub fn hash_reader<R: Read>(reader: &mut R, algorithms: &[HashAlgorithm]) -> std::io::Result<Vec<HashResult>> {
    let mut hasher = Hasher::new(algorithms);
    let mut buffer = vec![0u8; 1024 * 1024]; // 1MB buffer

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

/// Compute hash of a file
pub fn hash_file(path: &std::path::Path, algorithms: &[HashAlgorithm]) -> std::io::Result<Vec<HashResult>> {
    let mut file = std::fs::File::open(path)?;
    hash_reader(&mut file, algorithms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_md5_hash() {
        let data = b"Hello, World!";
        let mut reader = Cursor::new(data);
        let results = hash_reader(&mut reader, &[HashAlgorithm::Md5]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].algorithm, HashAlgorithm::Md5);
        // MD5("Hello, World!") = 65a8e27d8879283831b664bd8b7f0ad4
        assert_eq!(results[0].hex, "65a8e27d8879283831b664bd8b7f0ad4");
    }

    #[test]
    fn test_sha256_hash() {
        let data = b"Hello, World!";
        let mut reader = Cursor::new(data);
        let results = hash_reader(&mut reader, &[HashAlgorithm::Sha256]).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].algorithm, HashAlgorithm::Sha256);
        // SHA256("Hello, World!") = dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f
        assert_eq!(results[0].hex, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }

    #[test]
    fn test_multi_hash() {
        let data = b"test";
        let mut reader = Cursor::new(data);
        let results = hash_reader(&mut reader, &[HashAlgorithm::Md5, HashAlgorithm::Sha1, HashAlgorithm::Sha256]).unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_hasher_incremental() {
        let mut hasher = Hasher::new(&[HashAlgorithm::Md5]);
        hasher.update(b"Hello, ");
        hasher.update(b"World!");

        let results = hasher.finalize();
        assert_eq!(results[0].hex, "65a8e27d8879283831b664bd8b7f0ad4");
    }
}
