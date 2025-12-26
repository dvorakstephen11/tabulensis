use std::collections::HashSet;
use std::io::Cursor;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostKind {
    Workbook,
    Pbix,
}

#[derive(Serialize)]
struct SheetCell {
    row: u32,
    col: u32,
    value: Option<String>,
    formula: Option<String>,
}

#[derive(Serialize)]
struct SheetSnapshot {
    name: String,
    nrows: u32,
    ncols: u32,
    cells: Vec<SheetCell>,
}

#[derive(Serialize)]
struct WorkbookSnapshot {
    sheets: Vec<SheetSnapshot>,
}

#[derive(Serialize)]
struct SheetPairSnapshot {
    old: WorkbookSnapshot,
    new: WorkbookSnapshot,
}

#[derive(Serialize)]
struct DiffWithSheets {
    report: excel_diff::DiffReport,
    sheets: SheetPairSnapshot,
}

fn host_kind_from_name(name: &str) -> Option<HostKind> {
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "xlsx" | "xlsm" | "xltx" | "xltm" => Some(HostKind::Workbook),
        "pbix" | "pbit" => Some(HostKind::Pbix),
        _ => None,
    }
}

fn collect_sheet_ids(ops: &[excel_diff::DiffOp]) -> HashSet<excel_diff::StringId> {
    let mut sheets = HashSet::new();
    for op in ops {
        let sheet = match op {
            excel_diff::DiffOp::SheetAdded { sheet }
            | excel_diff::DiffOp::SheetRemoved { sheet }
            | excel_diff::DiffOp::RowAdded { sheet, .. }
            | excel_diff::DiffOp::RowRemoved { sheet, .. }
            | excel_diff::DiffOp::RowReplaced { sheet, .. }
            | excel_diff::DiffOp::ColumnAdded { sheet, .. }
            | excel_diff::DiffOp::ColumnRemoved { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRows { sheet, .. }
            | excel_diff::DiffOp::BlockMovedColumns { sheet, .. }
            | excel_diff::DiffOp::BlockMovedRect { sheet, .. }
            | excel_diff::DiffOp::RectReplaced { sheet, .. }
            | excel_diff::DiffOp::CellEdited { sheet, .. } => Some(*sheet),
            _ => None,
        };
        if let Some(sheet_id) = sheet {
            sheets.insert(sheet_id);
        }
    }
    sheets
}

fn render_cell_value(
    pool: &excel_diff::StringPool,
    value: &Option<excel_diff::CellValue>,
) -> Option<String> {
    match value {
        None => None,
        Some(excel_diff::CellValue::Blank) => Some(String::new()),
        Some(excel_diff::CellValue::Number(n)) => Some(n.to_string()),
        Some(excel_diff::CellValue::Text(id)) => Some(pool.resolve(*id).to_string()),
        Some(excel_diff::CellValue::Bool(b)) => {
            Some(if *b { "TRUE".to_string() } else { "FALSE".to_string() })
        }
        Some(excel_diff::CellValue::Error(id)) => Some(pool.resolve(*id).to_string()),
    }
}

fn snapshot_sheet(
    sheet: &excel_diff::Sheet,
    pool: &excel_diff::StringPool,
) -> SheetSnapshot {
    let name = pool.resolve(sheet.name).to_string();
    let mut cells = Vec::with_capacity(sheet.grid.cell_count());
    for ((row, col), cell) in sheet.grid.iter_cells() {
        let value = render_cell_value(pool, &cell.value);
        let formula = cell
            .formula
            .map(|id| format!("={}", pool.resolve(id)));
        cells.push(SheetCell {
            row,
            col,
            value,
            formula,
        });
    }

    SheetSnapshot {
        name,
        nrows: sheet.grid.nrows,
        ncols: sheet.grid.ncols,
        cells,
    }
}

fn snapshot_workbook(
    workbook: &excel_diff::Workbook,
    sheet_ids: &HashSet<excel_diff::StringId>,
    pool: &excel_diff::StringPool,
) -> WorkbookSnapshot {
    if sheet_ids.is_empty() {
        return WorkbookSnapshot { sheets: Vec::new() };
    }

    let mut sheets = Vec::new();
    for sheet in &workbook.sheets {
        if !sheet_ids.contains(&sheet.name) {
            continue;
        }
        sheets.push(snapshot_sheet(sheet, pool));
    }

    WorkbookSnapshot { sheets }
}

#[wasm_bindgen]
pub fn diff_files_json(
    old_bytes: &[u8],
    new_bytes: &[u8],
    old_name: &str,
    new_name: &str,
) -> Result<String, JsValue> {
    let kind_old = host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());

    let cfg = excel_diff::DiffConfig::default();

    let report = match kind_old {
        HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;
            pkg_old.diff(&pkg_new, &cfg)
        }
        HostKind::Pbix => {
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
    old_bytes: &[u8],
    new_bytes: &[u8],
    old_name: &str,
    new_name: &str,
) -> Result<String, JsValue> {
    let kind_old = host_kind_from_name(old_name)
        .ok_or_else(|| JsValue::from_str("Unsupported old file extension"))?;
    let kind_new = host_kind_from_name(new_name)
        .ok_or_else(|| JsValue::from_str("Unsupported new file extension"))?;

    if kind_old != kind_new {
        return Err(JsValue::from_str("Old/new files must be the same type"));
    }

    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());
    let cfg = excel_diff::DiffConfig::default();

    match kind_old {
        HostKind::Workbook => {
            let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old workbook: {}", e)))?;
            let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new workbook: {}", e)))?;

            let report = pkg_old.diff(&pkg_new, &cfg);
            let sheet_ids = collect_sheet_ids(&report.ops);

            let sheets = excel_diff::with_default_session(|session| SheetPairSnapshot {
                old: snapshot_workbook(&pkg_old.workbook, &sheet_ids, &session.strings),
                new: snapshot_workbook(&pkg_new.workbook, &sheet_ids, &session.strings),
            });

            let payload = DiffWithSheets { report, sheets };
            serde_json::to_string(&payload)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
        }
        HostKind::Pbix => {
            let pkg_old = excel_diff::PbixPackage::open(old_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open old PBIX/PBIT: {}", e)))?;
            let pkg_new = excel_diff::PbixPackage::open(new_cursor)
                .map_err(|e| JsValue::from_str(&format!("Failed to open new PBIX/PBIT: {}", e)))?;

            let report = pkg_old.diff(&pkg_new, &cfg);
            let empty = WorkbookSnapshot { sheets: Vec::new() };
            let payload = DiffWithSheets {
                report,
                sheets: SheetPairSnapshot {
                    old: empty,
                    new: WorkbookSnapshot { sheets: Vec::new() },
                },
            };
            serde_json::to_string(&payload)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
        }
    }
}

#[wasm_bindgen]
pub fn diff_workbooks_json(old_bytes: &[u8], new_bytes: &[u8]) -> Result<String, JsValue> {
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
pub fn diff_summary(old_bytes: &[u8], new_bytes: &[u8]) -> Result<DiffSummary, JsValue> {
    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());

    let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open old file: {}", e)))?;
    let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open new file: {}", e)))?;

    let report = pkg_old.diff(&pkg_new, &excel_diff::DiffConfig::default());

    Ok(DiffSummary {
        op_count: report.ops.len(),
        sheets_old: pkg_old.workbook.sheets.len(),
        sheets_new: pkg_new.workbook.sheets.len(),
    })
}

