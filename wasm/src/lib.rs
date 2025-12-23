use std::io::Cursor;
use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum HostKind {
    Workbook,
    Pbix,
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

