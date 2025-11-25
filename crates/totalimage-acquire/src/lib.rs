//! Disk image acquisition and creation crate
//!
//! Provides functionality for:
//! - Creating raw disk images (dd equivalent)
//! - Creating VHD images
//! - Hash verification (MD5, SHA1, SHA256)
//! - Progress tracking during acquisition
//!
//! This crate implements the "write" side of TotalImage for FTK Imager replacement.

pub mod error;
pub mod hash;
pub mod progress;
pub mod raw;

pub use error::{AcquireError, Result};
pub use hash::{HashAlgorithm, HashResult, Hasher};
pub use progress::{AcquireProgress, ProgressCallback};
pub use raw::{RawAcquirer, AcquireOptions};
