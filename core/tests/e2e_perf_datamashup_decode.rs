#![cfg(feature = "perf-metrics")]

use excel_diff::perf::{DiffMetrics, Phase};
use excel_diff::decode_datamashup_base64;
use quick_xml::{Reader, events::Event};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const SYNTHETIC_DECODED_BYTES: usize = 8 * 1024 * 1024;
const MIN_DECODED_BYTES: usize = 2 * 1024 * 1024;
const SYNTHETIC_LINE_LEN: usize = 76;

fn fixtures_root() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path
}

fn extract_datamashup_base64(xml: &[u8]) -> Option<String> {
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut in_datamashup = false;
    let mut content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if is_datamashup_element(e.name().as_ref()) => {
                if in_datamashup {
                    return None;
                }
                in_datamashup = true;
                content.clear();
            }
            Ok(Event::Text(t)) if in_datamashup => {
                let text = t.unescape().ok()?;
                content.push_str(text.as_ref());
            }
            Ok(Event::CData(t)) if in_datamashup => {
                content.push_str(&String::from_utf8_lossy(&t.into_inner()));
            }
            Ok(Event::End(e)) if is_datamashup_element(e.name().as_ref()) => {
                if !in_datamashup {
                    return None;
                }
                return Some(std::mem::take(&mut content));
            }
            Ok(Event::Eof) => return None,
            Err(_) => return None,
            _ => {}
        }
        buf.clear();
    }
}

fn is_datamashup_element(name: &[u8]) -> bool {
    match name.iter().rposition(|&b| b == b':') {
        Some(idx) => name.get(idx + 1..) == Some(b"DataMashup".as_slice()),
        None => name == b"DataMashup",
    }
}

fn read_datamashup_payload(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).ok()?;
        let name = entry.name().to_string();
        if !name.starts_with("customXml/") || !name.ends_with(".xml") {
            continue;
        }
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).ok()?;
        if let Some(text) = extract_datamashup_base64(&buf) {
            return Some(text);
        }
    }
    None
}

fn pick_largest_datamashup_payload() -> (String, String) {
    let root = fixtures_root();
    let entries = fs::read_dir(&root).unwrap_or_else(|e| {
        panic!("failed to read fixtures directory {}: {e}", root.display());
    });

    let mut best: Option<(String, String)> = None;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("xlsx") {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if let Some(payload) = read_datamashup_payload(&path) {
            let replace = best
                .as_ref()
                .map(|(_, current)| payload.len() > current.len())
                .unwrap_or(true);
            if replace {
                best = Some((name, payload));
            }
        }
    }

    best.unwrap_or_else(|| {
        panic!(
            "no DataMashup fixtures found in {}. Run `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean` (see fixtures/README.md).",
            root.display()
        )
    })
}

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

fn select_perf_payload() -> (String, String) {
    let (fixture_name, payload) = pick_largest_datamashup_payload();
    let decoded_len = decode_datamashup_base64(&payload)
        .ok()
        .map(|bytes| bytes.len())
        .unwrap_or(0);

    if decoded_len < MIN_DECODED_BYTES {
        return (
            format!("synthetic_datamashup_{}mib", SYNTHETIC_DECODED_BYTES / (1024 * 1024)),
            make_synthetic_payload(SYNTHETIC_DECODED_BYTES),
        );
    }

    (fixture_name, payload)
}

fn iterations_for_payload(decoded_len: usize) -> usize {
    let target_bytes: u64 = 50 * 1024 * 1024;
    let decoded = decoded_len.max(1) as u64;
    let mut iterations = (target_bytes / decoded).max(1) as usize;
    if iterations > 50_000 {
        iterations = 50_000;
    }
    iterations
}

#[test]
#[ignore = "Long-running test: run with `cargo test -p excel_diff --features perf-metrics -- --ignored --nocapture` to execute"]
fn e2e_perf_datamashup_decode() {
    let (fixture_name, payload) = select_perf_payload();
    let decoded_len = decode_datamashup_base64(&payload)
        .expect("payload should decode")
        .len();
    let iterations = iterations_for_payload(decoded_len);

    let mut metrics = DiffMetrics::default();
    metrics.start_phase(Phase::Total);
    metrics.start_phase(Phase::Parse);
    for _ in 0..iterations {
        let out = decode_datamashup_base64(&payload).expect("payload should decode");
        std::hint::black_box(out);
    }
    metrics.end_phase(Phase::Parse);
    metrics.end_phase(Phase::Total);

    assert!(metrics.parse_time_ms > 0, "parse_time_ms should be non-zero");
    assert!(
        metrics.total_time_ms >= metrics.parse_time_ms,
        "total_time_ms should include parse_time_ms"
    );

    println!(
        "PERF_METRIC datamashup_decode fixture={} iterations={} payload_chars={} decoded_bytes={} parse_time_ms={} total_time_ms={} peak_memory_bytes={}",
        fixture_name,
        iterations,
        payload.len(),
        decoded_len,
        metrics.parse_time_ms,
        metrics.total_time_ms,
        metrics.peak_memory_bytes
    );
}
