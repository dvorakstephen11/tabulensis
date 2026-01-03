#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

use excel_diff::{ContainerLimits, PbixPackage};

fuzz_target!(|data: &[u8]| {
    let limits = ContainerLimits {
        max_entries: 2000,
        max_part_uncompressed_bytes: 5 * 1024 * 1024,
        max_total_uncompressed_bytes: 50 * 1024 * 1024,
    };

    let cursor = Cursor::new(data);
    let _ = PbixPackage::open_with_limits(cursor, limits);
});
