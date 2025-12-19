#![no_main]

use libfuzzer_sys::fuzz_target;

use excel_diff::{parse_data_mashup, build_data_mashup};

fuzz_target!(|data: &[u8]| {
    if let Ok(raw) = parse_data_mashup(data) {
        let _ = build_data_mashup(&raw);
    }
});

