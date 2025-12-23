//! Property-based testing utilities for TotalImage
//!
//! This module provides common strategies and helpers for property-based testing
//! across all TotalImage crates using the proptest framework.

#[cfg(any(test, feature = "proptest"))]
pub use proptest::prelude::*;

/// Strategy for generating valid sector sizes
#[cfg(any(test, feature = "proptest"))]
pub fn sector_size_strategy() -> impl proptest::strategy::Strategy<Value = u32> {
    prop_oneof![
        Just(512u32),   // Most common
        Just(4096u32),  // 4K sectors
        (256u32..=8192u32).prop_filter("Must be power of 2", |&s| s.is_power_of_two()),
    ]
}

/// Strategy for generating valid disk sizes (in bytes)
#[cfg(any(test, feature = "proptest"))]
pub fn disk_size_strategy() -> impl proptest::strategy::Strategy<Value = u64> {
    // Generate sector counts and multiply by 512 to ensure multiples of 512
    (1u64..=3_906_250_000u64) // 1 sector to ~2TB (in sectors)
        .prop_map(|sectors| sectors * 512)
}

/// Strategy for generating valid VHD disk types
#[cfg(any(test, feature = "proptest"))]
pub fn vhd_disk_type_strategy() -> impl proptest::strategy::Strategy<Value = u32> {
    prop_oneof![
        Just(2u32),  // Fixed
        Just(3u32),  // Dynamic
        Just(4u32),  // Differencing
    ]
}

/// Strategy for generating valid partition counts
#[cfg(any(test, feature = "proptest"))]
pub fn partition_count_strategy() -> impl proptest::strategy::Strategy<Value = u32> {
    0u32..=128u32 // GPT supports up to 128 partitions
}

/// Strategy for generating valid FAT types
#[cfg(any(test, feature = "proptest"))]
pub fn fat_type_strategy() -> impl proptest::strategy::Strategy<Value = u8> {
    prop_oneof![
        Just(1u8),   // FAT12
        Just(6u8),   // FAT16
        Just(11u8),  // FAT32
        Just(12u8),  // FAT32 (LBA)
    ]
}

/// Helper to create proptest configuration
#[cfg(any(test, feature = "proptest"))]
pub fn proptest_config() -> proptest::test_runner::Config {
    proptest::test_runner::Config::with_cases(1000) // Run 1000 test cases
}
