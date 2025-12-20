#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = excel_diff::parse_data_mashup(data);

    let limits = excel_diff::DataMashupLimits {
        max_inner_entries: 256,
        max_inner_part_bytes: 256 * 1024,
        max_inner_total_bytes: 2 * 1024 * 1024,
    };
    let _ = excel_diff::parse_package_parts_with_limits(data, limits);
});

