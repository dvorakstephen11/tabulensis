#![cfg(feature = "perf-metrics")]

use excel_diff::perf::{DiffMetrics, Phase};
use excel_diff::read_datamashup_text;

const SYNTHETIC_DECODED_BYTES: usize = 8 * 1024 * 1024;
const SYNTHETIC_LINE_LEN: usize = 76;

fn make_deterministic_bytes(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut x: u32 = 0x1234_5678;
    for _ in 0..len {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        out.push((x >> 24) as u8);
    }
    out
}

fn encode_base64_standard(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(((data.len() + 2) / 3) * 4);
    let mut i = 0usize;
    while i < data.len() {
        let b0 = data[i];
        let b1 = if i + 1 < data.len() { data[i + 1] } else { 0 };
        let b2 = if i + 2 < data.len() { data[i + 2] } else { 0 };

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        if i + 1 < data.len() {
            out.push(TABLE[((n >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < data.len() {
            out.push(TABLE[(n & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }

        i += 3;
    }
    out
}

fn add_base64_whitespace(b64: &str) -> String {
    let mut out = String::with_capacity(b64.len() + b64.len() / SYNTHETIC_LINE_LEN * 3 + 2);
    let mut col = 0usize;
    for ch in b64.chars() {
        if col == 0 {
            out.push(' ');
            out.push(' ');
        }
        out.push(ch);
        col += 1;
        if col == SYNTHETIC_LINE_LEN {
            out.push('\n');
            col = 0;
        }
    }
    if col != 0 {
        out.push('\n');
    }
    out
}

fn make_synthetic_payload(decoded_len: usize) -> String {
    let bytes = make_deterministic_bytes(decoded_len);
    let b64 = encode_base64_standard(&bytes);
    add_base64_whitespace(&b64)
}

fn iterations_for_payload(payload_len: usize) -> usize {
    let target_bytes: u64 = 50 * 1024 * 1024;
    let size = payload_len.max(1) as u64;
    let mut iterations = (target_bytes / size).max(1) as usize;
    if iterations > 50_000 {
        iterations = 50_000;
    }
    iterations
}

#[test]
#[ignore = "Long-running test: run with `cargo test -p excel_diff --features perf-metrics --test e2e_perf_datamashup_text -- --ignored --nocapture` to execute"]
fn e2e_perf_datamashup_text_extract() {
    let payload = make_synthetic_payload(SYNTHETIC_DECODED_BYTES);
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><root xmlns:dm="http://schemas.microsoft.com/DataMashup"><dm:DataMashup>{}</dm:DataMashup></root>"#,
        payload
    );
    let xml_bytes = xml.as_bytes();
    let iterations = iterations_for_payload(payload.len());

    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Parse);
    for _ in 0..iterations {
        let text = read_datamashup_text(xml_bytes)
            .expect("XML parse should succeed")
            .expect("DataMashup element should be found");
        std::hint::black_box(text);
    }
    metrics.end_phase(Phase::Parse);
    metrics.end_phase(Phase::Total);

    println!(
        "PERF_METRIC datamashup_text_extract iterations={} payload_chars={} parse_time_ms={} total_time_ms={} peak_memory_bytes={}",
        iterations,
        payload.len(),
        metrics.parse_time_ms,
        metrics.total_time_ms,
        metrics.peak_memory_bytes
    );
}
