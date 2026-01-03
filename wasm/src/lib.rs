use std::io::{self, Cursor, Write};

use excel_diff::advanced::{CallbackSink, diff_workbooks_streaming};
use excel_diff::{
    CellValue, DiffConfig, DiffSink, Grid, JsonLinesSink, Sheet, SheetKind, StringPool, Workbook,
};
use js_sys::Function;
use ui_payload::{
    DiffOptions, DiffOutcome, DiffOutcomeConfig, DiffOutcomeMode, DiffPreset, HostCapabilities,
    HostDefaults, SummaryMeta, SummarySink, limits_from_config, summarize_report,
};
use wasm_bindgen::prelude::*;

const WASM_DEFAULT_MAX_MEMORY_MB: u32 = 256;
const JSONL_CHUNK_BYTES: usize = 256 * 1024;

struct JsonlChunkWriter {
    buffer: Vec<u8>,
    on_chunk: Function,
}

impl JsonlChunkWriter {
    fn new(on_chunk: Function) -> Self {
        Self {
            buffer: Vec::with_capacity(JSONL_CHUNK_BYTES),
            on_chunk,
        }
    }

    fn flush_buffer(&mut self) -> io::Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        let text = String::from_utf8_lossy(&self.buffer);
        self.on_chunk
            .call1(&JsValue::NULL, &JsValue::from_str(text.as_ref()))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("chunk callback failed: {e:?}")))?;
        self.buffer.clear();
        Ok(())
    }
}

impl Write for JsonlChunkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(buf);
        if self.buffer.len() >= JSONL_CHUNK_BYTES {
            self.flush_buffer()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.flush_buffer()
    }
}

fn wasm_default_config() -> DiffConfig {
    let mut cfg = DiffConfig::default();
    cfg.hardening.max_memory_mb = Some(WASM_DEFAULT_MAX_MEMORY_MB);
    cfg
}

fn parse_options(options_json: &str) -> Result<DiffOptions, JsValue> {
    if options_json.trim().is_empty() {
        return Ok(DiffOptions::default());
    }
    serde_json::from_str::<DiffOptions>(options_json)
        .map_err(|e| JsValue::from_str(&format!("Invalid options JSON: {e}")))
}

fn outcome_config_from_options(options: &DiffOptions, cfg: &DiffConfig) -> DiffOutcomeConfig {
    let preset = if options.config_json.as_ref().map(|v| v.trim()).unwrap_or("").is_empty() {
        Some(options.preset.unwrap_or(DiffPreset::Balanced))
    } else {
        None
    };
    DiffOutcomeConfig {
        preset,
        limits: Some(limits_from_config(cfg)),
    }
}

fn summary_meta_from_names(old_name: &str, new_name: &str) -> SummaryMeta {
    SummaryMeta {
        old_path: None,
        new_path: None,
        old_name: Some(old_name.to_string()),
        new_name: Some(new_name.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SheetKey {
    name_lower: String,
    kind: SheetKind,
}

fn estimate_diff_cell_volume(old: &Workbook, new: &Workbook) -> u64 {
    excel_diff::with_default_session(|session| {
        let mut max_counts: std::collections::HashMap<SheetKey, u64> =
            std::collections::HashMap::new();
        for sheet in old.sheets.iter().chain(new.sheets.iter()) {
            let name_lower = session.strings.resolve(sheet.name).to_lowercase();
            let key = SheetKey {
                name_lower,
                kind: sheet.kind.clone(),
            };
            let cell_count = sheet.grid.cell_count() as u64;
            max_counts
                .entry(key)
                .and_modify(|v| {
                    if cell_count > *v {
                        *v = cell_count;
                    }
                })
                .or_insert(cell_count);
        }

        max_counts.values().copied().sum()
    })
}

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn diff_files_json(
    old_bytes: Vec<u8>,
    new_bytes: Vec<u8>,
    old_name: &str,
    new_name: &str,
) -> Result<String, JsValue> {
    let kind_old = ui_payload::host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = ui_payload::host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let old_cursor = Cursor::new(old_bytes);
    let new_cursor = Cursor::new(new_bytes);

    let cfg = wasm_default_config();

    let report = match kind_old {
        ui_payload::HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;
            pkg_old.diff(&pkg_new, &cfg)
        }
        ui_payload::HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;
            pkg_old.diff(&pkg_new, &cfg)
        }
    };

    excel_diff::serialize_diff_report(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

#[wasm_bindgen]
pub fn diff_files_with_sheets_json(
    old_bytes: Vec<u8>,
    new_bytes: Vec<u8>,
    old_name: &str,
    new_name: &str,
) -> Result<String, JsValue> {
    let kind_old = ui_payload::host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = ui_payload::host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let old_cursor = Cursor::new(old_bytes);
    let new_cursor = Cursor::new(new_bytes);
    let cfg = wasm_default_config();

    let payload = match kind_old {
        ui_payload::HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;
            ui_payload::build_payload_from_workbooks(&pkg_old, &pkg_new, &cfg)
        }
        ui_payload::HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;
            ui_payload::build_payload_from_pbix(&pkg_old, &pkg_new, &cfg)
        }
    };

    serde_json::to_string(&payload)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
}

#[wasm_bindgen]
pub fn diff_files_outcome_json(
    old_bytes: Vec<u8>,
    new_bytes: Vec<u8>,
    old_name: &str,
    new_name: &str,
    options_json: String,
) -> Result<String, JsValue> {
    let kind_old = ui_payload::host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = ui_payload::host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let options = parse_options(&options_json)?;
    let cfg = options
        .effective_config(wasm_default_config())
        .map_err(|e| JsValue::from_str(&e))?;
    let outcome_config = outcome_config_from_options(&options, &cfg);
    let meta = summary_meta_from_names(old_name, new_name);

    let old_cursor = Cursor::new(old_bytes);
    let new_cursor = Cursor::new(new_bytes);

    let outcome = match kind_old {
        ui_payload::HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;

            let estimated_cells = estimate_diff_cell_volume(&pkg_old.workbook, &pkg_new.workbook);
            let use_large_mode = excel_diff::should_use_large_mode(estimated_cells, &cfg);

            if use_large_mode {
                let mut sink = SummarySink::new();
                let summary = pkg_old
                    .diff_streaming(&pkg_new, &cfg, &mut sink)
                    .map_err(|e| JsValue::from_str(&format!("Streaming diff failed: {}", e)))?;
                let summary = sink.into_summary(summary, meta.clone());
                DiffOutcome {
                    diff_id: None,
                    mode: DiffOutcomeMode::Large,
                    payload: None,
                    summary: Some(summary),
                    config: Some(outcome_config),
                }
            } else {
                let payload = ui_payload::build_payload_from_workbooks(&pkg_old, &pkg_new, &cfg);
                let summary = summarize_report(&payload.report, meta.clone());
                DiffOutcome {
                    diff_id: None,
                    mode: DiffOutcomeMode::Payload,
                    payload: Some(payload),
                    summary: Some(summary),
                    config: Some(outcome_config),
                }
            }
        }
        ui_payload::HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;
            let payload = ui_payload::build_payload_from_pbix(&pkg_old, &pkg_new, &cfg);
            let summary = summarize_report(&payload.report, meta);
            DiffOutcome {
                diff_id: None,
                mode: DiffOutcomeMode::Payload,
                payload: Some(payload),
                summary: Some(summary),
                config: Some(outcome_config),
            }
        }
    };

    serde_json::to_string(&outcome)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize outcome: {}", e)))
}

#[wasm_bindgen]
pub fn diff_files_jsonl_stream(
    old_bytes: Vec<u8>,
    new_bytes: Vec<u8>,
    old_name: &str,
    new_name: &str,
    options_json: String,
    on_chunk: Function,
) -> Result<(), JsValue> {
    let kind_old = ui_payload::host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = ui_payload::host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let options = parse_options(&options_json)?;
    let cfg = options
        .effective_config(wasm_default_config())
        .map_err(|e| JsValue::from_str(&e))?;

    let old_cursor = Cursor::new(old_bytes);
    let new_cursor = Cursor::new(new_bytes);
    let writer = JsonlChunkWriter::new(on_chunk);
    let mut sink = JsonLinesSink::new(writer);

    match kind_old {
        ui_payload::HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;
            pkg_old
                .diff_streaming(&pkg_new, &cfg, &mut sink)
                .map_err(|e| JsValue::from_str(&format!("Streaming diff failed: {}", e)))?;
        }
        ui_payload::HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;
            pkg_old
                .diff_streaming(&pkg_new, &cfg, &mut sink)
                .map_err(|e| JsValue::from_str(&format!("Streaming diff failed: {}", e)))?;
        }
    };

    sink.finish()
        .map_err(|e| JsValue::from_str(&format!("Failed to finalize JSONL: {}", e)))?;

    Ok(())
}

#[wasm_bindgen]
pub fn diff_workbooks_json(old_bytes: Vec<u8>, new_bytes: Vec<u8>) -> Result<String, JsValue> {
    diff_files_json(old_bytes, new_bytes, "old.xlsx", "new.xlsx")
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn get_capabilities() -> Result<String, JsValue> {
    let caps = HostCapabilities::new(get_version()).with_defaults(HostDefaults {
        max_memory_mb: Some(WASM_DEFAULT_MAX_MEMORY_MB),
        large_mode_threshold: excel_diff::AUTO_STREAM_CELL_THRESHOLD,
    });
    serde_json::to_string(&caps)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize capabilities: {}", e)))
}

#[wasm_bindgen]
pub struct DiffSummary {
    pub op_count: usize,
    pub sheets_old: usize,
    pub sheets_new: usize,
}

#[wasm_bindgen]
pub fn diff_summary(old_bytes: Vec<u8>, new_bytes: Vec<u8>) -> Result<DiffSummary, JsValue> {
    let old_cursor = Cursor::new(old_bytes);
    let new_cursor = Cursor::new(new_bytes);

    let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open old file: {}", e)))?;
    let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open new file: {}", e)))?;

    let report = pkg_old.diff(&pkg_new, &wasm_default_config());

    Ok(DiffSummary {
        op_count: report.ops.len(),
        sheets_old: pkg_old.workbook.sheets.len(),
        sheets_new: pkg_new.workbook.sheets.len(),
    })
}

fn create_dense_grid(nrows: u32, ncols: u32, base_value: i64) -> Grid {
    let mut grid = Grid::new(nrows, ncols);
    for row in 0..nrows {
        for col in 0..ncols {
            let value = base_value + row as i64 * 1000 + col as i64;
            grid.insert_cell(row, col, Some(CellValue::Number(value as f64)), None);
        }
    }
    grid
}

fn single_sheet_workbook(pool: &mut StringPool, grid: Grid) -> Workbook {
    let sheet_name = pool.intern("Sheet1");
    Workbook {
        sheets: vec![Sheet {
            name: sheet_name,
            workbook_sheet_id: None,
            kind: SheetKind::Worksheet,
            grid,
        }],
        named_ranges: Vec::new(),
        charts: Vec::new(),
    }
}

#[wasm_bindgen]
pub fn run_memory_benchmark(case_name: &str) -> Result<u32, JsValue> {
    let (old_grid, new_grid) = match case_name {
        "low_similarity" => (
            create_dense_grid(6000, 50, 0),
            create_dense_grid(6000, 50, 100_000_000),
        ),
        "near_identical" => {
            let grid_a = create_dense_grid(6000, 50, 0);
            let mut grid_b = grid_a.clone();
            grid_b.insert_cell(3000, 25, Some(CellValue::Number(999999.0)), None);
            (grid_a, grid_b)
        }
        _ => {
            return Err(JsValue::from_str(
                "Unknown benchmark case (expected 'low_similarity' or 'near_identical')",
            ))
        }
    };

    let mut pool = StringPool::new();
    let old = single_sheet_workbook(&mut pool, old_grid);
    let new = single_sheet_workbook(&mut pool, new_grid);
    let cfg = wasm_default_config();

    let mut sink = CallbackSink::new(|_op| {});
    let summary = diff_workbooks_streaming(&old, &new, &mut pool, &cfg, &mut sink);
    Ok(summary.op_count as u32)
}

#[wasm_bindgen]
pub fn wasm_memory_bytes() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        let pages = core::arch::wasm32::memory_size(0) as u32;
        pages.saturating_mul(65536)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{create_dense_grid, estimate_diff_cell_volume, wasm_default_config};
    use excel_diff::{Sheet, SheetKind, Workbook, AUTO_STREAM_CELL_THRESHOLD, should_use_large_mode, with_default_session};
    use ui_payload::DiffOutcomeMode;

    fn build_workbook(grid: excel_diff::Grid) -> Workbook {
        let name_id = with_default_session(|session| session.strings.intern("Sheet1"));
        Workbook {
            sheets: vec![Sheet {
                name: name_id,
                workbook_sheet_id: None,
                kind: SheetKind::Worksheet,
                grid,
            }],
            named_ranges: Vec::new(),
            charts: Vec::new(),
        }
    }

    #[test]
    fn large_mode_threshold_triggers_in_wasm() {
        let grid = create_dense_grid(1000, 1001, 0);
        let old = build_workbook(grid.clone());
        let new = build_workbook(grid);
        let estimated = estimate_diff_cell_volume(&old, &new);
        assert!(estimated >= AUTO_STREAM_CELL_THRESHOLD);

        let cfg = wasm_default_config();
        let mode = if should_use_large_mode(estimated, &cfg) {
            DiffOutcomeMode::Large
        } else {
            DiffOutcomeMode::Payload
        };
        assert_eq!(mode, DiffOutcomeMode::Large);
    }
}
