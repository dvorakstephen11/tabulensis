use std::io::Cursor;

use excel_diff::advanced::{CallbackSink, diff_workbooks_streaming};
use excel_diff::{CellValue, DiffConfig, Grid, Sheet, SheetKind, StringPool, Workbook};
use wasm_bindgen::prelude::*;

const WASM_DEFAULT_MAX_MEMORY_MB: u32 = 256;

fn wasm_default_config() -> DiffConfig {
    let mut cfg = DiffConfig::default();
    cfg.hardening.max_memory_mb = Some(WASM_DEFAULT_MAX_MEMORY_MB);
    cfg
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
pub fn diff_workbooks_json(old_bytes: Vec<u8>, new_bytes: Vec<u8>) -> Result<String, JsValue> {
    diff_files_json(old_bytes, new_bytes, "old.xlsx", "new.xlsx")
}

#[wasm_bindgen]
pub fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
