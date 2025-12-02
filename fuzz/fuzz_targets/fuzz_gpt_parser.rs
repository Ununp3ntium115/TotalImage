#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Fuzz the GPT parser with arbitrary input
    if data.len() >= 1024 {
        let mut cursor = Cursor::new(data);
        let _ = totalimage_zones::gpt::GptZoneTable::parse(&mut cursor, 512);
    }
});
