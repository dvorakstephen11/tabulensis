#![no_main]

use excel_diff::{parse_m_expression, parse_section_members};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let s = String::from_utf8_lossy(data);
    let expr = s.as_ref().get(..4096).unwrap_or(s.as_ref());
    let section = format!("section Section1;\nshared Foo = {};\n", expr);

    if let Ok(members) = parse_section_members(&section) {
        for m in members {
            let _ = parse_m_expression(&m.expression_m);
        }
    }
});
