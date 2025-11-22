//! # TotalImage Vaults
//!
//! Container format handlers for the Total Liberation project.
//!
//! This crate provides implementations of various disk image container formats:
//! - **RawVault**: Plain sector images (.img, .ima, .flp, .vfd, .dsk, .iso)
//! - **MicrosoftVault**: VHD format (coming soon)
//! - **NhdVault**, **ImzVault**, etc. (future)
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_vaults::{RawVault, VaultConfig};
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
//! ```

pub mod raw;

pub use raw::{RawVault, VaultConfig};
