//! # TotalImage Zones
//!
//! Partition table handlers for the Total Liberation project.
//!
//! This crate provides implementations of various partition table formats:
//! - **MBR**: Master Boot Record (BIOS/legacy partitioning)
//! - **GPT**: GUID Partition Table (UEFI/modern partitioning)
//! - **Direct**: No partition table (entire disk is one zone)
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_zones::{mbr::MbrZoneTable, gpt::GptZoneTable};
//! use totalimage_core::ZoneTable;
//! use std::fs::File;
//!
//! // Parse MBR from a disk image
//! let mut file = File::open("disk.img").unwrap();
//! let table = MbrZoneTable::parse(&mut file, 512).unwrap();
//!
//! println!("Partition table: {}", table.identify());
//! for zone in table.enumerate_zones() {
//!     println!("  {}", zone);
//! }
//! ```

pub mod mbr;
pub mod gpt;

pub use mbr::MbrZoneTable;
pub use gpt::GptZoneTable;
