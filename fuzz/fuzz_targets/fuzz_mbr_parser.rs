#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    // Fuzz the MBR parser with arbitrary input
    if data.len() >= 512 {
        let mut cursor = Cursor::new(data);
        let _ = totalimage_zones::mbr::MbrZoneTable::parse(&mut cursor, 512);
    }
});
