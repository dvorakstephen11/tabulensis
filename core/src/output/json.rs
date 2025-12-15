#[cfg(feature = "excel-open-xml")]
use crate::config::DiffConfig;
use crate::diff::DiffReport;
#[cfg(feature = "excel-open-xml")]
use crate::engine::diff_workbooks as compute_diff;
#[cfg(feature = "excel-open-xml")]
use crate::excel_open_xml::{ExcelOpenError, open_workbook};
use crate::session::DiffSession;
use crate::string_pool::StringId;
use serde::Serialize;
use serde::ser::Error as SerdeError;
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
    if contains_non_finite_numbers(report) {
        return Err(SerdeError::custom(
            "non-finite numbers (NaN or infinity) are not supported in DiffReport JSON output",
        ));
    }
    serde_json::to_string(report)
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<DiffReport, ExcelOpenError> {
    let mut session = DiffSession::new();
    let wb_a = open_workbook(path_a, session.strings_mut())?;
    let wb_b = open_workbook(path_b, session.strings_mut())?;
    Ok(compute_diff(
        &wb_a,
        &wb_b,
        session.strings_mut(),
        config,
    ))
}

#[cfg(feature = "excel-open-xml")]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<String, ExcelOpenError> {
    let report = diff_workbooks(path_a, path_b, config)?;
    serialize_diff_report(&report).map_err(|e| ExcelOpenError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn resolve_string<'a>(report: &'a DiffReport, id: StringId) -> Option<&'a str> {
        report.strings.get(id.0 as usize).map(|s| s.as_str())
    }

    fn render_value(report: &DiffReport, value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(id)) => resolve_string(report, *id).map(|s| s.to_string()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            Some(CellValue::Error(id)) => resolve_string(report, *id).map(|s| s.to_string()),
            Some(CellValue::Blank) => Some(String::new()),
            None => None,
        }
    }

    report
        .ops
        .iter()
        .filter_map(|op| {
            if let DiffOp::CellEdited { addr, from, to, .. } = op {
                if from == to {
                    return None;
                }
                Some(CellDiff {
                    coords: addr.to_a1(),
                    value_file1: render_value(report, &from.value),
                    value_file2: render_value(report, &to.value),
                })
            } else {
                None
            }
        })
        .collect()
}

fn contains_non_finite_numbers(report: &DiffReport) -> bool {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    report.ops.iter().any(|op| match op {
        DiffOp::CellEdited { from, to, .. } => {
            matches!(from.value, Some(CellValue::Number(n)) if !n.is_finite())
                || matches!(to.value, Some(CellValue::Number(n)) if !n.is_finite())
        }
        _ => false,
    })
}
