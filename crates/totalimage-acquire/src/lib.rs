//! Disk image acquisition and creation crate
//!
//! Provides functionality for:
//! - Creating raw disk images (dd equivalent)
//! - Creating VHD images (Fixed and Dynamic)
//! - Hash verification (MD5, SHA1, SHA256)
//! - Progress tracking during acquisition
//!
//! This crate implements the "write" side of TotalImage for FTK Imager replacement.

pub mod e01_writer;
pub mod error;
pub mod format;
pub mod hash;
pub mod partition;
pub mod progress;
pub mod raw;
pub mod usb;
pub mod vhd;
pub mod winpe;

pub use e01_writer::{E01Writer, E01WriterConfig};
pub use error::{AcquireError, Result};
pub use format::Fat32Formatter;
pub use hash::{hash_reader, HashAlgorithm, HashResult, Hasher};
pub use partition::{PartitionTableBuilder, PartitionTableType};
pub use progress::{AcquireProgress, ProgressCallback};
pub use raw::{AcquireOptions, RawAcquirer};
pub use usb::{detect_usb_drives, UsbDrive};
pub use vhd::{VhdCreationResult, VhdCreator, VhdOptions, VhdOutputType};
pub use winpe::{
    create_boot_config, extract_wim_to_usb, find_winpe_source, validate_winpe_source,
    WinpeArchitecture, WinpeSource,
};
