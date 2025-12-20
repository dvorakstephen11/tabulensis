#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if data.len() > 1_000_000 {
        return;
    }

    let cursor = std::io::Cursor::new(data.to_vec());
    let _ = excel_diff::WorkbookPackage::open(cursor);
});

