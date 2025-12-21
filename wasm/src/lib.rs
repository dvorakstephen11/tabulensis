use wasm_bindgen::prelude::*;
use std::io::Cursor;

#[wasm_bindgen]
pub fn diff_workbooks_json(old_bytes: &[u8], new_bytes: &[u8]) -> Result<String, JsValue> {
    let old_cursor = Cursor::new(old_bytes.to_vec());
    let new_cursor = Cursor::new(new_bytes.to_vec());

    let pkg_old = excel_diff::WorkbookPackage::open(old_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open old file: {}", e)))?;
    let pkg_new = excel_diff::WorkbookPackage::open(new_cursor)
        .map_err(|e| JsValue::from_str(&format!("Failed to open new file: {}", e)))?;

    let report = pkg_old.diff(&pkg_new, &excel_diff::DiffConfig::default());

    excel_diff::serialize_diff_report(&report)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize report: {}", e)))
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

