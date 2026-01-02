#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
use crate::config::DiffConfig;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
use crate::datamashup::build_data_mashup;
use crate::diff::{DiffReport, DiffSummary};
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
use crate::excel_open_xml::{PackageError, open_data_mashup, open_vba_modules, open_workbook};
#[allow(unused_imports)]
use crate::session::DiffSession;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
use crate::sink::VecSink;
use serde::Serialize;
use serde::ser::Error as SerdeError;
#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
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

#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
pub fn diff_workbooks(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<DiffReport, PackageError> {
    let path_a = path_a.as_ref();
    let path_b = path_b.as_ref();

    let mut session = DiffSession::new();

    let wb_a = open_workbook(path_a, session.strings_mut())?;
    let wb_b = open_workbook(path_b, session.strings_mut())?;

    let dm_a = open_data_mashup(path_a)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;
    let dm_b = open_data_mashup(path_b)?
        .map(|raw| build_data_mashup(&raw))
        .transpose()?;

    let vba_a = open_vba_modules(path_a, session.strings_mut())?;
    let vba_b = open_vba_modules(path_b, session.strings_mut())?;

    let mut sink = VecSink::new();
    let grid_result = crate::engine::try_diff_workbooks_streaming(
        &wb_a,
        &wb_b,
        session.strings_mut(),
        config,
        &mut sink,
    );

    let (mut ops, summary) = match grid_result {
        Ok(summary) => (sink.into_ops(), summary),
        Err(err) => (
            Vec::new(),
            DiffSummary {
                complete: false,
                warnings: vec![err.to_string()],
                op_count: 0,
                #[cfg(feature = "perf-metrics")]
                metrics: None,
            },
        ),
    };

    let mut object_ops = crate::object_diff::diff_named_ranges(&wb_a, &wb_b, session.strings());
    object_ops.extend(crate::object_diff::diff_charts(
        &wb_a,
        &wb_b,
        session.strings(),
    ));
    object_ops.extend(crate::object_diff::diff_vba_modules(
        vba_a.as_deref(),
        vba_b.as_deref(),
        session.strings(),
    ));
    ops.extend(object_ops);

    let m_ops = crate::m_diff::diff_m_ops_for_packages(&dm_a, &dm_b, session.strings_mut(), config);

    ops.extend(m_ops);

    let strings = session.strings.into_strings();
    Ok(DiffReport::from_ops_and_summary(ops, summary, strings))
}

#[cfg(all(feature = "excel-open-xml", feature = "std-fs"))]
pub fn diff_workbooks_to_json(
    path_a: impl AsRef<Path>,
    path_b: impl AsRef<Path>,
    config: &DiffConfig,
) -> Result<String, PackageError> {
    let report = diff_workbooks(path_a, path_b, config)?;
    serialize_diff_report(&report).map_err(|e| PackageError::SerializationError(e.to_string()))
}

pub fn diff_report_to_cell_diffs(report: &DiffReport) -> Vec<CellDiff> {
    use crate::diff::DiffOp;
    use crate::workbook::CellValue;

    fn render_value(report: &DiffReport, value: &Option<CellValue>) -> Option<String> {
        match value {
            Some(CellValue::Number(n)) => Some(n.to_string()),
            Some(CellValue::Text(id)) => report.resolve(*id).map(|s| s.to_string()),
            Some(CellValue::Bool(b)) => Some(b.to_string()),
            Some(CellValue::Error(id)) => report.resolve(*id).map(|s| s.to_string()),
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
