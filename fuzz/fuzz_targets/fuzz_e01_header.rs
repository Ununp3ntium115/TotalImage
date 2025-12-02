#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz the E01 file header parser
    if data.len() >= 13 {
        let _ = totalimage_vaults::e01::types::E01FileHeader::parse(data);
    }
});
