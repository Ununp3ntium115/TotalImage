//! # TotalImage Pipeline
//!
//! I/O abstractions and pipeline utilities for the Total Liberation project.
//!
//! This crate provides various stream wrappers for efficient data access:
//! - **PartialPipeline**: Window into a subset of a stream (for partitions)
//! - **MmapPipeline**: Memory-mapped file access for direct action
//!
//! ## Example
//!
//! ```rust,no_run
//! use totalimage_pipeline::{PartialPipeline, MmapPipeline};
//! use std::path::Path;
//! use std::io::{Read, Seek, SeekFrom};
//!
//! // Open a file with memory mapping
//! let mut mmap = MmapPipeline::open(Path::new("disk.img")).unwrap();
//!
//! // Create a partial view (e.g., for a partition)
//! let mut partial = PartialPipeline::new(mmap, 0x8000, 0x100000).unwrap();
//!
//! // Read from the partial view
//! let mut buf = [0u8; 512];
//! partial.read(&mut buf).unwrap();
//! ```

pub mod mmap;
pub mod partial;

pub use mmap::MmapPipeline;
pub use partial::PartialPipeline;
