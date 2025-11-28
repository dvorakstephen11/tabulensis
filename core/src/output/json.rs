#[cfg(feature = "excel-open-xml")]
use crate::addressing::index_to_address;
#[cfg(feature = "excel-open-xml")]
use crate::workbook::{Cell, CellValue, Sheet, Workbook};
use serde::Serialize;
#[cfg(feature = "excel-open-xml")]
use std::collections::HashMap;
#[cfg(feature = "excel-open-xml")]
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CellDiff {
    #[serde(rename = "coords")]
    pub coords: String,
    #[serde(rename = "value_file1")]
    pub value_file1: Option<String>,
    #[serde(rename = "value_file2")]
    pub value_file2: Option<String>,
}

pub fn serialize_cell_diffs(diffs: &[CellDiff]) -> serde_json::Result<String> {
    serde_json::to_string(diffs)
}

#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<Vec<CellDiff>, ExcelOpenError> {
    let wb_a = open_workbook(path_a)?;
    let wb_b = open_workbook(path_b)?;
    Ok(compute_cell_diffs(&wb_a, &wb_b))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<String, ExcelOpenError> {
    let diffs = diff_workbooks(path_a, path_b)?;
    serialize_cell_diffs(&diffs).map_err(|e| ExcelOpenError::XmlParseError(e.to_string()))
}

#[cfg(feature = "excel-open-xml")]
fn compute_cell_diffs(a: &Workbook, b: &Workbook) -> Vec<CellDiff> {
    let map_a = sheet_map(a);
    let map_b = sheet_map(b);

    let mut names: Vec<&str> = map_a.keys().chain(map_b.keys()).copied().collect();
    names.sort_unstable();
    names.dedup();

    let mut diffs = Vec::new();
    for name in names {
        let sheet_a = map_a.get(name);
        let sheet_b = map_b.get(name);
        let dims = sheet_dims(sheet_a);
        let other_dims = sheet_dims(sheet_b);
        let nrows = dims.0.max(other_dims.0);
        let ncols = dims.1.max(other_dims.1);

        for r in 0..nrows {
            for c in 0..ncols {
                let cell_a = sheet_a
                    .and_then(|s| s.grid.rows.get(r as usize))
                    .and_then(|row| row.cells.get(c as usize));
                let cell_b = sheet_b
                    .and_then(|s| s.grid.rows.get(r as usize))
                    .and_then(|row| row.cells.get(c as usize));

                let value_a = cell_a.and_then(render_cell_value);
                let value_b = cell_b.and_then(render_cell_value);

                if value_a != value_b {
                    let coords = index_to_address(r, c);
                    diffs.push(CellDiff {
                        coords,
                        value_file1: value_a,
                        value_file2: value_b,
                    });
                }
            }
        }
    }

    diffs
}

#[cfg(feature = "excel-open-xml")]
fn sheet_map(workbook: &Workbook) -> HashMap<&str, &Sheet> {
    workbook
        .sheets
        .iter()
        .map(|s| (s.name.as_str(), s))
        .collect()
}

#[cfg(feature = "excel-open-xml")]
fn sheet_dims(sheet: Option<&&Sheet>) -> (u32, u32) {
    sheet
        .map(|s| (s.grid.nrows, s.grid.ncols))
        .unwrap_or((0, 0))
}

#[cfg(feature = "excel-open-xml")]
fn render_cell_value(cell: &Cell) -> Option<String> {
    match &cell.value {
        Some(CellValue::Number(n)) => Some(n.to_string()),
        Some(CellValue::Text(s)) => Some(s.clone()),
        Some(CellValue::Bool(b)) => Some(b.to_string()),
        None => None,
    }
}
