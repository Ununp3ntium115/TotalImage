//! # TotalImage Vaults
//!
//! Container format handlers for the Total Liberation project.
//!
//! This crate provides implementations of various disk image container formats:
//! - **RawVault**: Plain sector images (.img, .ima, .flp, .vfd, .dsk, .iso)
//! - **VhdVault**: Microsoft VHD format (Fixed and Dynamic)
//! - **E01Vault**: EnCase forensic format
//! - **Aff4Vault**: Advanced Forensic Format 4
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_vaults::factory::open_vault;
//! use totalimage_vaults::VaultConfig;
//! use std::path::Path;
//!
//! // Open any supported format with auto-detection
//! let mut vault = open_vault(Path::new("disk.vhd"), VaultConfig::default()).unwrap();
//!
//! println!("Type: {}", vault.identify());
//! println!("Size: {} bytes", vault.length());
//! ```

pub mod aff4;
pub mod e01;
pub mod factory;
pub mod raw;
pub mod vhd;

pub use aff4::Aff4Vault;
pub use e01::E01Vault;
pub use factory::{detect_vault_type, open_vault, open_vault_as, supported_formats, VaultType};
pub use raw::{RawVault, VaultConfig};
pub use vhd::{VhdChainVault, VhdVault};
