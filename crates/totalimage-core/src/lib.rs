//! # TotalImage Core
//!
//! Core traits, types, and error handling for the Total Liberation project.
//!
//! This crate provides the foundational abstractions for working with disk images:
//! - **Vaults**: Container formats (Raw, VHD, etc.)
//! - **Territories**: File systems (FAT, ISO, exFAT, etc.)
//! - **Zones**: Partitions (MBR, GPT, etc.)
//! - **DirectoryCells**: Directory navigation
//!
//! ## Anarchist Terminology
//!
//! - **Vault** = Container format (sabotage proprietary formats)
//! - **Territory** = File system (autonomous data domain)
//! - **Zone** = Partition (segregated storage area)
//! - **Cell** = Component/Module
//! - **Collective** = Group of related components
//! - **Action** = Operation/Method
//! - **Manifesto** = Boot sector/specification
//! - **Sabotage** = Read operations
//! - **Liberation** = Data extraction
//! - **Underground Network** = Factory pattern
//! - **Pipeline** = Data stream
//! - **Direct Action** = Memory-mapped I/O
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_core::{Vault, Territory, Result};
//!
//! fn process_vault(mut vault: Box<dyn Vault>) -> Result<()> {
//!     println!("Vault type: {}", vault.identify());
//!     println!("Vault size: {} bytes", vault.length());
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod security;
pub mod traits;
pub mod types;

// Re-export commonly used items
pub use error::{Error, Result};
pub use security::*;
pub use traits::{DirectoryCell, ReadSeek, ReadWriteSeek, Territory, Vault, ZoneTable};
pub use types::{OccupantInfo, Zone};
