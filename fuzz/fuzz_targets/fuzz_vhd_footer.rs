#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz the VHD footer parser
    if data.len() >= 512 {
        let _ = totalimage_vaults::vhd::types::VhdFooter::parse(&data[..512]);
    }
});
