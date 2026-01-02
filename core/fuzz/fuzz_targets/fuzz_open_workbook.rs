#![no_main]

use libfuzzer_sys::fuzz_target;
use std::io::Cursor;

use excel_diff::{ContainerLimits, DiffSession, OpcContainer, WorkbookPackage, with_default_session};

fuzz_target!(|data: &[u8]| {
    with_default_session(|session| *session = DiffSession::new());
    let limits = ContainerLimits {
        max_entries: 100,
        max_part_uncompressed_bytes: 1024 * 1024,
        max_total_uncompressed_bytes: 10 * 1024 * 1024,
    };

    let cursor = Cursor::new(data);
    let _ = OpcContainer::open_from_reader_with_limits(cursor, limits);

    let cursor = Cursor::new(data);
    let _ = WorkbookPackage::open_with_limits(cursor, limits);
});

