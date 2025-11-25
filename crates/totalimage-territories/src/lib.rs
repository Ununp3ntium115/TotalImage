//! # TotalImage Territories
//!
//! File system implementations for the Total Liberation project.
//!
//! This crate provides Territory implementations for various file systems:
//! - **FAT**: FAT12, FAT16, and FAT32 file systems
//! - **ISO 9660**: CD-ROM file system (read-only)
//! - **exFAT**: Extended FAT file system for flash media
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_territories::fat::FatTerritory;
//! use totalimage_core::Territory;
//! use std::fs::File;
//!
//! // Parse FAT filesystem from a partition
//! let mut file = File::open("partition.img").unwrap();
//! let territory = FatTerritory::parse(&mut file).unwrap();
//! println!("Filesystem: {}", territory.identify());
//! ```

pub mod exfat;
pub mod fat;
pub mod iso;

pub use exfat::ExfatTerritory;
pub use fat::FatTerritory;
pub use iso::IsoTerritory;
