use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::engine::diff_workbooks as compute_diff;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};
use serde::Serialize;
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

pub fn serialize_diff_report(report: &DiffReport) -> serde_json::Result<String> {
    serde_json::to_string(report)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<DiffReport, ExcelOpenError> {
    let wb_a = open_workbook(path_a)?;
    let wb_b = open_workbook(path_b)?;
    Ok(compute_diff(&wb_a, &wb_b))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
) -> Result<String, ExcelOpenError> {
    let report = diff_workbooks(path_a, path_b)?;
    serialize_diff_report(&report).map_err(|e| ExcelOpenError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(s)) => Some(s.clone()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(&from.value),
                    value_file2: render_value(&to.value),
                })
            } else {
                None
            }
        })
        .collect()
}
