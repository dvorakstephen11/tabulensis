//! Common test utilities shared across integration tests.

#![allow(dead_code)]

use excel_diff::{
    CellSnapshot, CellValue, DiffConfig, DiffOp, DiffReport, DiffSession, DiffSummary,
    ExtractedColumnTypeChanges, ExtractedRenamePairs, ExtractedString, ExtractedStringList, Grid,
    QuerySemanticDetail, RenamePair, Sheet, SheetKind, StepChange, StepDiff, StepParams,
    StepSnapshot, StringId, Workbook, WorkbookPackage, with_default_session,
};
use serde::Deserialize;
use std::fs::File;
use std::path::PathBuf;

pub fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../fixtures/generated");
    path.push(filename);
    path
}

pub fn open_fixture_pkg(name: &str) -> WorkbookPackage {
    let path = fixture_path(name);
    let file = File::open(&path).unwrap_or_else(|e| {
        panic!("failed to open fixture {}: {e}", path.display());
    });
    WorkbookPackage::open(file).unwrap_or_else(|e| {
        panic!("failed to parse fixture {}: {e}", path.display());
    })
}

pub fn open_fixture_workbook(name: &str) -> Workbook {
    open_fixture_pkg(name).workbook
}

pub fn diff_fixture_pkgs(a: &str, b: &str, config: &DiffConfig) -> DiffReport {
    let pkg_a = open_fixture_pkg(a);
    let pkg_b = open_fixture_pkg(b);
    pkg_a.diff(&pkg_b, config)
}

pub fn grid_from_numbers(values: &[&[i32]]) -> Grid {
    let nrows = values.len() as u32;
    let ncols = if nrows == 0 {
        0
    } else {
        values[0].len() as u32
    };

    let mut grid = Grid::new(nrows, ncols);
    for (r, row_vals) in values.iter().enumerate() {
        for (c, v) in row_vals.iter().enumerate() {
            grid.insert_cell(r as u32, c as u32, Some(CellValue::Number(*v as f64)), None);
        }
    }

    grid
}

pub fn sid(s: &str) -> StringId {
    with_default_session(|session| session.strings.intern(s))
}

pub fn single_sheet_workbook(name: &str, grid: Grid) -> Workbook {
    with_default_session(|session| Workbook {
        sheets: vec![Sheet {
            name: session.strings.intern(name),
            kind: SheetKind::Worksheet,
            grid,
        }],
        ..Default::default()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredOutput {
    pub ops: Vec<DiffOp>,
    pub summary: DiffSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonlOutput {
    pub strings: Vec<String>,
    pub ops: Vec<DiffOp>,
}

pub fn assert_structured_determinism_with_fresh_sessions<F>(runs: usize, mut f: F)
where
    F: FnMut(&mut DiffSession) -> StructuredOutput,
{
    let mut baseline: Option<StructuredOutput> = None;
    for _ in 0..runs {
        let mut session = DiffSession::new();
        let output = f(&mut session);
        match &baseline {
            None => baseline = Some(output),
            Some(expected) => assert_eq!(
                expected, &output,
                "structured streaming output should be deterministic"
            ),
        }
    }
}

pub fn assert_jsonl_determinism_with_fresh_sessions<F>(runs: usize, mut f: F)
where
    F: FnMut(&mut DiffSession) -> Vec<u8>,
{
    let mut baseline: Option<JsonlOutput> = None;
    for _ in 0..runs {
        let mut session = DiffSession::new();
        let output = parse_jsonl_output(&f(&mut session));
        match &baseline {
            None => baseline = Some(output),
            Some(expected) => assert_eq!(
                expected, &output,
                "JSONL streaming output should be deterministic"
            ),
        }
    }
}

pub fn parse_jsonl_output(bytes: &[u8]) -> JsonlOutput {
    #[derive(Deserialize)]
    struct Header {
        kind: String,
        strings: Vec<String>,
    }

    let text = std::str::from_utf8(bytes).expect("output should be UTF-8");
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().expect("expected a JSON Lines header line");
    let header: Header = serde_json::from_str(header_line).expect("header should parse");
    assert_eq!(header.kind, "Header");

    let mut ops = Vec::new();
    for line in lines {
        let op: DiffOp = serde_json::from_str(line).expect("op line should parse as DiffOp");
        for id in collect_string_ids(&op) {
            assert!(
                (id.0 as usize) < header.strings.len(),
                "StringId {} out of range for header string table (len={})",
                id.0,
                header.strings.len()
            );
        }
        ops.push(op);
    }

    JsonlOutput {
        strings: header.strings,
        ops,
    }
}

pub fn collect_string_ids(op: &DiffOp) -> Vec<StringId> {
    fn collect_cell_value(ids: &mut Vec<StringId>, value: &CellValue) {
        match value {
            CellValue::Text(id) | CellValue::Error(id) => ids.push(*id),
            CellValue::Number(_) | CellValue::Bool(_) | CellValue::Blank => {}
        }
    }

    fn collect_snapshot(ids: &mut Vec<StringId>, snap: &CellSnapshot) {
        if let Some(value) = &snap.value {
            collect_cell_value(ids, value);
        }
        if let Some(formula) = snap.formula {
            ids.push(formula);
        }
    }

    fn collect_extracted_string(ids: &mut Vec<StringId>, value: &ExtractedString) {
        if let ExtractedString::Known { value } = value {
            ids.push(*value);
        }
    }

    fn collect_extracted_string_list(ids: &mut Vec<StringId>, value: &ExtractedStringList) {
        if let ExtractedStringList::Known { values } = value {
            ids.extend(values.iter().copied());
        }
    }

    fn collect_rename_pairs(ids: &mut Vec<StringId>, value: &ExtractedRenamePairs) {
        if let ExtractedRenamePairs::Known { pairs } = value {
            for RenamePair { from, to } in pairs {
                ids.push(*from);
                ids.push(*to);
            }
        }
    }

    fn collect_column_type_changes(ids: &mut Vec<StringId>, value: &ExtractedColumnTypeChanges) {
        if let ExtractedColumnTypeChanges::Known { changes } = value {
            for change in changes {
                ids.push(change.column);
            }
        }
    }

    fn collect_step_params(ids: &mut Vec<StringId>, params: &StepParams) {
        match params {
            StepParams::TableSelectRows { .. } => {}
            StepParams::TableRemoveColumns { columns } => collect_extracted_string_list(ids, columns),
            StepParams::TableRenameColumns { renames } => collect_rename_pairs(ids, renames),
            StepParams::TableTransformColumnTypes { transforms } => {
                collect_column_type_changes(ids, transforms);
            }
            StepParams::TableNestedJoin {
                left_keys,
                right_keys,
                new_column,
                ..
            } => {
                collect_extracted_string_list(ids, left_keys);
                collect_extracted_string_list(ids, right_keys);
                collect_extracted_string(ids, new_column);
            }
            StepParams::TableJoin {
                left_keys,
                right_keys,
                ..
            } => {
                collect_extracted_string_list(ids, left_keys);
                collect_extracted_string_list(ids, right_keys);
            }
            StepParams::Other { .. } => {}
        }
    }

    fn collect_step_snapshot(ids: &mut Vec<StringId>, snapshot: &StepSnapshot) {
        ids.push(snapshot.name);
        ids.extend(snapshot.source_refs.iter().copied());
        if let Some(params) = &snapshot.params {
            collect_step_params(ids, params);
        }
    }

    fn collect_step_diff(ids: &mut Vec<StringId>, diff: &StepDiff) {
        match diff {
            StepDiff::StepAdded { step } | StepDiff::StepRemoved { step } => {
                collect_step_snapshot(ids, step);
            }
            StepDiff::StepReordered { name, .. } => ids.push(*name),
            StepDiff::StepModified { before, after, changes } => {
                collect_step_snapshot(ids, before);
                collect_step_snapshot(ids, after);
                for change in changes {
                    if let StepChange::Renamed { from, to } = change {
                        ids.push(*from);
                        ids.push(*to);
                    }
                    if let StepChange::SourceRefsChanged { removed, added } = change {
                        ids.extend(removed.iter().copied());
                        ids.extend(added.iter().copied());
                    }
                }
            }
        }
    }

    fn collect_semantic_detail(ids: &mut Vec<StringId>, detail: &QuerySemanticDetail) {
        for diff in &detail.step_diffs {
            collect_step_diff(ids, diff);
        }
    }

    let mut ids = Vec::new();
    match op {
        DiffOp::SheetAdded { sheet } | DiffOp::SheetRemoved { sheet } => ids.push(*sheet),
        DiffOp::RowAdded { sheet, .. }
        | DiffOp::RowRemoved { sheet, .. }
        | DiffOp::RowReplaced { sheet, .. } => ids.push(*sheet),
        DiffOp::ColumnAdded { sheet, .. } | DiffOp::ColumnRemoved { sheet, .. } => ids.push(*sheet),
        DiffOp::BlockMovedRows { sheet, .. }
        | DiffOp::BlockMovedColumns { sheet, .. }
        | DiffOp::BlockMovedRect { sheet, .. }
        | DiffOp::RectReplaced { sheet, .. } => ids.push(*sheet),
        DiffOp::CellEdited {
            sheet, from, to, ..
        } => {
            ids.push(*sheet);
            collect_snapshot(&mut ids, from);
            collect_snapshot(&mut ids, to);
        }
        DiffOp::VbaModuleAdded { name }
        | DiffOp::VbaModuleRemoved { name }
        | DiffOp::VbaModuleChanged { name } => ids.push(*name),
        DiffOp::NamedRangeAdded { name } | DiffOp::NamedRangeRemoved { name } => ids.push(*name),
        DiffOp::NamedRangeChanged { name, old_ref, new_ref } => {
            ids.push(*name);
            ids.push(*old_ref);
            ids.push(*new_ref);
        }
        DiffOp::ChartAdded { sheet, name }
        | DiffOp::ChartRemoved { sheet, name }
        | DiffOp::ChartChanged { sheet, name } => {
            ids.push(*sheet);
            ids.push(*name);
        }
        DiffOp::QueryAdded { name }
        | DiffOp::QueryRemoved { name }
        | DiffOp::QueryDefinitionChanged { name, .. } => ids.push(*name),
        DiffOp::QueryRenamed { from, to } => {
            ids.push(*from);
            ids.push(*to);
        }
        DiffOp::QueryMetadataChanged { name, old, new, .. } => {
            ids.push(*name);
            if let Some(old) = old {
                ids.push(*old);
            }
            if let Some(new) = new {
                ids.push(*new);
            }
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { name }
        | DiffOp::MeasureRemoved { name }
        | DiffOp::MeasureDefinitionChanged { name, .. } => ids.push(*name),
        _ => {}
    }

    if let DiffOp::QueryDefinitionChanged { semantic_detail, .. } = op {
        if let Some(detail) = semantic_detail {
            collect_semantic_detail(&mut ids, detail);
        }
    }

    ids
}
