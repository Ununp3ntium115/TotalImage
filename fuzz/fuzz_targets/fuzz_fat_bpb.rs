#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fuzz the FAT BPB (BIOS Parameter Block) parser
    if data.len() >= 512 {
        let _ = totalimage_territories::fat::types::BiosParameterBlock::from_bytes(data);
    }
});
