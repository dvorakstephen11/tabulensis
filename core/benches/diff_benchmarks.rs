use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use excel_diff::{
    CellValue, DiffConfig, DiffSession, Grid, Sheet, SheetKind, Workbook,
    decode_datamashup_base64,
    try_diff_workbooks_with_pool,
};
use quick_xml::{Reader, events::Event};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipArchive;

const MAX_BENCH_TIME_SECS: u64 = 30;
const WARMUP_SECS: u64 = 3;
const SAMPLE_SIZE: usize = 10;
const SYNTHETIC_DECODED_BYTES: usize = 8 * 1024 * 1024;
const MIN_DECODED_BYTES: usize = 2 * 1024 * 1024;
const SYNTHETIC_LINE_LEN: usize = 76;

fn create_large_grid(nrows: u32, ncols: u32, base_value: i32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number(
                    (base_value as i64 + row as i64 * 1000 + col as i64) as f64,
                )),
                None,
            );
        }
    }
    grid
}

fn create_repetitive_grid(nrows: u32, ncols: u32, pattern_length: u32) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        let pattern_idx = row % pattern_length;
        for col in 0..ncols {
            grid.insert_cell(
                row,
                col,
                Some(CellValue::Number((pattern_idx * 1000 + col) as f64)),
                None,
            );
        }
    }
    grid
}

fn create_sparse_grid(nrows: u32, ncols: u32, fill_percent: u32, seed: u64) -> Grid {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            let mut hasher = DefaultHasher::new();
            (row, col, seed).hash(&mut hasher);
            let hash = hasher.finish();
            if (hash % 100) < fill_percent as u64 {
                grid.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row * 1000 + col) as f64)),
                    None,
                );
            }
        }
    }
    grid
}

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

fn collect_datamashup_payloads() -> Vec<(String, String)> {
    let root = fixtures_root();
    let entries = fs::read_dir(&root).unwrap_or_else(|e| {
        panic!("failed to read fixtures directory {}: {e}", root.display());
    });
    let mut payloads = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("xlsx") {
            continue;
        }
        let file_name = match path.file_name().and_then(|s| s.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };
        if let Some(payload) = read_datamashup_payload(&path) {
            payloads.push((file_name, payload));
        }
    }

    if payloads.is_empty() {
        panic!(
            "no DataMashup fixtures found in {}. Run `generate-fixtures --manifest fixtures/manifest_cli_tests.yaml --force --clean` (see fixtures/README.md).",
            root.display()
        );
    }

    payloads
}

fn select_payloads(mut payloads: Vec<(String, String)>) -> Vec<(String, String)> {
    payloads.sort_by_key(|(_, text)| text.len());
    if payloads.len() <= 1 {
        return payloads;
    }

    let smallest = payloads.first().cloned().expect("non-empty payloads");
    let largest = payloads.last().cloned().expect("non-empty payloads");
    if smallest.0 == largest.0 {
        vec![smallest]
    } else {
        vec![smallest, largest]
    }
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

fn should_add_synthetic_payload(payloads: &[(String, String)]) -> bool {
    let largest = payloads
        .iter()
        .max_by_key(|(_, payload)| payload.len())
        .map(|(_, payload)| payload);

    match largest.and_then(|payload| decode_datamashup_base64(payload).ok()) {
        Some(decoded) => decoded.len() < MIN_DECODED_BYTES,
        None => true,
    }
}

fn single_sheet_workbook(session: &mut DiffSession, name: &str, grid: Grid) -> Workbook {
    let sheet_name = session.strings.intern(name);
    Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    }
}

fn bench_identical_grids(c: &mut Criterion) {
    let mut group = c.benchmark_group("identical_grids");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 0);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

fn bench_single_cell_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_cell_edit");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let mut grid_b = create_large_grid(*size, 50, 0);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

fn bench_all_rows_different(c: &mut Criterion) {
    let mut group = c.benchmark_group("all_rows_different");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let grid_b = create_large_grid(*size, 50, 1);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

fn bench_adversarial_repetitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("adversarial_repetitive");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_repetitive_grid(*size, 50, 100);
        let mut grid_b = create_repetitive_grid(*size, 50, 100);
        grid_b.insert_cell(size / 2, 25, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

fn bench_sparse_grid(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_grid_1pct");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000, 5000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_sparse_grid(*size, 100, 1, 12345);
        let mut grid_b = create_sparse_grid(*size, 100, 1, 12345);
        grid_b.insert_cell(size / 2, 50, Some(CellValue::Number(999999.0)), None);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 100));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

fn bench_datamashup_base64_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("datamashup_base64_decode");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    let mut payloads = select_payloads(collect_datamashup_payloads());
    if should_add_synthetic_payload(&payloads) {
        payloads.push((
            format!("synthetic_datamashup_{}mib", SYNTHETIC_DECODED_BYTES / (1024 * 1024)),
            make_synthetic_payload(SYNTHETIC_DECODED_BYTES),
        ));
    }
    for (name, payload) in payloads {
        let decoded_len = decode_datamashup_base64(&payload)
            .expect("fixture payload should decode")
            .len();
        group.throughput(Throughput::Bytes(decoded_len as u64));
        group.bench_with_input(BenchmarkId::new("fixture", name), &payload, |b, text| {
            b.iter(|| {
                let out = decode_datamashup_base64(text).expect("payload should decode");
                criterion::black_box(out);
            });
        });
    }

    group.finish();
}

fn bench_row_insertion(c: &mut Criterion) {
    let mut group = c.benchmark_group("row_insertion");
    group.measurement_time(Duration::from_secs(MAX_BENCH_TIME_SECS));
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    for size in [500u32, 1000, 2000].iter() {
        let mut session = DiffSession::new();
        let grid_a = create_large_grid(*size, 50, 0);
        let mut grid_b = Grid::new(size + 100, 50);
        for row in 0..(size / 2) {
            for col in 0..50 {
                grid_b.insert_cell(
                    row,
                    col,
                    Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    None,
                );
            }
        }
        for col in 0..50 {
            for i in 0..100 {
                let row = size / 2 + i;
                let marker = 1_000_000.0 + i as f64 * 10.0 + col as f64;
                grid_b.insert_cell(row, col, Some(CellValue::Number(marker)), None);
            }
        }
        for row in (size / 2)..*size {
            for col in 0..50 {
                let new_row = row + 100;
                grid_b.insert_cell(
                    new_row,
                    col,
                    Some(CellValue::Number((row as i64 * 1000 + col as i64) as f64)),
                    None,
                );
            }
        }
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);
        let config = DiffConfig::default();

        group.throughput(Throughput::Elements(*size as u64 * 50));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

#[cfg(all(feature = "model-diff", feature = "excel-open-xml"))]
fn bench_pbit_model_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("pbit_model_diff");
    group.warm_up_time(Duration::from_secs(WARMUP_SECS));
    group.sample_size(SAMPLE_SIZE);

    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/generated");
    let path_a = base.join("pbit_model_a.pbit");
    let path_b = base.join("pbit_model_b.pbit");

    let bytes_a = std::fs::read(&path_a).expect("read pbit_model_a.pbit");
    let bytes_b = std::fs::read(&path_b).expect("read pbit_model_b.pbit");
    let config = DiffConfig::default();

    group.bench_function("open_parse_diff", |b| {
        b.iter(|| {
            let cursor_a = std::io::Cursor::new(bytes_a.clone());
            let cursor_b = std::io::Cursor::new(bytes_b.clone());
            let pkg_a = excel_diff::PbixPackage::open(cursor_a).expect("open pbit a");
            let pkg_b = excel_diff::PbixPackage::open(cursor_b).expect("open pbit b");
            let report = pkg_a.diff(&pkg_b, &config);
            criterion::black_box(report);
        });
    });

    group.finish();
}

#[cfg(not(all(feature = "model-diff", feature = "excel-open-xml")))]
fn bench_pbit_model_diff(_c: &mut Criterion) {}

fn create_grid_with_block_move(nrows: u32, ncols: u32, move_start: u32, move_size: u32) -> (Grid, Grid) {
    let mut grid_a = Grid::new(nrows, ncols);
    let mut grid_b = Grid::new(nrows, ncols);

    for row in 0..nrows {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            grid_a.insert_cell(row, col, Some(CellValue::Number(value)), None);
        }
    }

    let move_end = move_start + move_size;
    let dest_start = nrows - move_size - 100;

    for row in 0..move_start {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            grid_b.insert_cell(row, col, Some(CellValue::Number(value)), None);
        }
    }

    for row in move_end..nrows {
        for col in 0..ncols {
            let value = (row * 1000 + col) as f64;
            let new_row = row - move_size + (dest_start - move_start + move_size);
            if new_row < nrows && new_row != dest_start && (new_row < dest_start || new_row >= dest_start + move_size) {
                grid_b.insert_cell(row - move_size, col, Some(CellValue::Number(value)), None);
            }
        }
    }

    for i in 0..move_size {
        for col in 0..ncols {
            let value = ((move_start + i) * 1000 + col) as f64;
            grid_b.insert_cell(dest_start + i, col, Some(CellValue::Number(value)), None);
        }
    }

    (grid_a, grid_b)
}

fn bench_block_move_alignment(c: &mut Criterion) {
    let mut group = c.benchmark_group("block_move_alignment");
    group.measurement_time(Duration::from_secs(60));
    group.warm_up_time(Duration::from_secs(5));
    group.sample_size(10);

    for size in [5000u32, 10000].iter() {
        let mut session = DiffSession::new();
        let (grid_a, grid_b) = create_grid_with_block_move(*size, 20, 100, 50);
        let wb_a = single_sheet_workbook(&mut session, "Bench", grid_a);
        let wb_b = single_sheet_workbook(&mut session, "Bench", grid_b);

        let config = DiffConfig::builder()
            .preflight_min_rows(u32::MAX)
            .max_move_detection_rows(20000)
            .build()
            .expect("valid config");

        group.throughput(Throughput::Elements(*size as u64 * 20));
        group.bench_with_input(BenchmarkId::new("rows", size), size, move |b, _| {
            b.iter(|| {
                let _ = try_diff_workbooks_with_pool(&wb_a, &wb_b, &mut session.strings, &config)
                    .expect("diff should succeed");
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_identical_grids,
    bench_single_cell_edit,
    bench_all_rows_different,
    bench_adversarial_repetitive,
    bench_sparse_grid,
    bench_datamashup_base64_decode,
    bench_row_insertion,
    bench_pbit_model_diff,
);

criterion_group!(
    name = alignment_benches;
    config = Criterion::default().sample_size(10);
    targets = bench_block_move_alignment,
);

criterion_main!(benches, alignment_benches);
