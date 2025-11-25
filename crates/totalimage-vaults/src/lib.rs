//! # TotalImage Vaults
//!
//! Container format handlers for the Total Liberation project.
//!
//! This crate provides implementations of various disk image container formats:
//! - **RawVault**: Plain sector images (.img, .ima, .flp, .vfd, .dsk, .iso)
//! - **VhdVault**: Microsoft VHD format (Fixed and Dynamic)
//! - **NhdVault**, **ImzVault**, etc. (future)
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_vaults::{RawVault, VhdVault, VaultConfig};
//! use totalimage_core::Vault;
//! use std::path::Path;
//!
//! // Open a raw disk image
//! let mut vault = RawVault::open(
//!     Path::new("disk.img"),
//!     VaultConfig::default()
//! ).unwrap();
//!
//! println!("Type: {}", vault.identify());
//! println!("Size: {} bytes", vault.length());
//!
//! // Open a VHD file
//! let mut vhd = VhdVault::open(
//!     Path::new("disk.vhd"),
//!     VaultConfig::default()
//! ).unwrap();
//!
//! println!("Type: {}", vhd.identify());
//! println!("Size: {} bytes", vhd.length());
//! ```

pub mod e01;
pub mod raw;
pub mod vhd;

pub use e01::E01Vault;
pub use raw::{RawVault, VaultConfig};
pub use vhd::{VhdChainVault, VhdVault};
