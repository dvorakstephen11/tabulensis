use std::path::Path;

use excel_diff::{CellAddress, CellValue, DiffOp, ExpressionChangeKind, QueryChangeKind};
#[cfg(feature = "model-diff")]
use excel_diff::{ModelColumnProperty, RelationshipProperty};
use rust_xlsxwriter::{Format, Workbook, XlsxError};
use thiserror::Error;

use crate::store::{DiffRunSummary, OpStore, StoreError};

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Store error: {0}")]
    Store(#[from] StoreError),
    #[error("XLSX error: {0}")]
    Xlsx(#[from] XlsxError),
}

pub fn export_audit_xlsx_from_store(
    store: &OpStore,
    diff_id: &str,
    path: &Path,
) -> Result<(), ExportError> {
    let summary = store.load_summary(diff_id)?;
    let strings = store.load_strings(diff_id)?;

    let mut workbook = Workbook::new();
    let header_format = Format::new().set_bold();

    {
        let summary_sheet = workbook.add_worksheet();
        summary_sheet.set_name("Summary")?;
        write_summary_sheet(summary_sheet, &summary, &header_format);
    }

    {
        let warnings_sheet = workbook.add_worksheet();
        warnings_sheet.set_name("Warnings")?;
        write_warnings_sheet(warnings_sheet, &summary, &header_format);
    }

    {
        let cells_sheet = workbook.add_worksheet();
        cells_sheet.set_name("Cells")?;
        write_cells_header(cells_sheet, &header_format);
    }

    {
        let structure_sheet = workbook.add_worksheet();
        structure_sheet.set_name("Structure")?;
        write_structure_header(structure_sheet, &header_format);
    }

    {
        let query_sheet = workbook.add_worksheet();
        query_sheet.set_name("PowerQuery")?;
        write_query_header(query_sheet, &header_format);
    }

    {
        let model_sheet = workbook.add_worksheet();
        model_sheet.set_name("Model")?;
        write_model_header(model_sheet, &header_format);
    }

    {
        let other_sheet = workbook.add_worksheet();
        other_sheet.set_name("OtherOps")?;
        write_other_header(other_sheet, &header_format);
    }

    let mut rows = ExportRows::default();

    store.stream_ops(diff_id, |op| {
        write_op(&op, &strings, &mut workbook, &mut rows)?;
        Ok(())
    })?;

    workbook.save(path)?;
    Ok(())
}

#[derive(Default)]
struct ExportRows {
    cells: u32,
    structure: u32,
    query: u32,
    model: u32,
    other: u32,
}

fn sheet_mut<'a>(
    workbook: &'a mut Workbook,
    name: &str,
) -> Result<&'a mut rust_xlsxwriter::Worksheet, StoreError> {
    workbook
        .worksheet_from_name(name)
        .map_err(|e| StoreError::InvalidData(e.to_string()))
}

fn write_summary_sheet(
    sheet: &mut rust_xlsxwriter::Worksheet,
    summary: &DiffRunSummary,
    header: &Format,
) {
    let mut row = 0;
    sheet
        .write_string_with_format(row, 0, "Old path", header)
        .ok();
    sheet.write_string(row, 1, &summary.old_path).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "New path", header)
        .ok();
    sheet.write_string(row, 1, &summary.new_path).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Started", header)
        .ok();
    sheet.write_string(row, 1, &summary.started_at).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Finished", header)
        .ok();
    if let Some(finished) = &summary.finished_at {
        sheet.write_string(row, 1, finished).ok();
    }
    row += 1;
    sheet.write_string_with_format(row, 0, "Mode", header).ok();
    sheet.write_string(row, 1, summary.mode.as_str()).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Status", header)
        .ok();
    sheet.write_string(row, 1, summary.status.as_str()).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Complete", header)
        .ok();
    sheet
        .write_string(row, 1, if summary.complete { "true" } else { "false" })
        .ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Op count", header)
        .ok();
    sheet.write_number(row, 1, summary.op_count as f64).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Warnings", header)
        .ok();
    sheet
        .write_number(row, 1, summary.warnings.len() as f64)
        .ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "Engine version", header)
        .ok();
    sheet.write_string(row, 1, &summary.engine_version).ok();
    row += 1;
    sheet
        .write_string_with_format(row, 0, "App version", header)
        .ok();
    sheet.write_string(row, 1, &summary.app_version).ok();
    row += 2;

    sheet.write_string_with_format(row, 0, "Sheet", header).ok();
    sheet.write_string_with_format(row, 1, "Ops", header).ok();
    sheet.write_string_with_format(row, 2, "Added", header).ok();
    sheet
        .write_string_with_format(row, 3, "Removed", header)
        .ok();
    sheet
        .write_string_with_format(row, 4, "Modified", header)
        .ok();
    sheet.write_string_with_format(row, 5, "Moved", header).ok();
    row += 1;

    for sheet_summary in &summary.sheets {
        sheet.write_string(row, 0, &sheet_summary.sheet_name).ok();
        sheet
            .write_number(row, 1, sheet_summary.op_count as f64)
            .ok();
        sheet
            .write_number(row, 2, sheet_summary.counts.added as f64)
            .ok();
        sheet
            .write_number(row, 3, sheet_summary.counts.removed as f64)
            .ok();
        sheet
            .write_number(row, 4, sheet_summary.counts.modified as f64)
            .ok();
        sheet
            .write_number(row, 5, sheet_summary.counts.moved as f64)
            .ok();
        row += 1;
    }
}

fn write_warnings_sheet(
    sheet: &mut rust_xlsxwriter::Worksheet,
    summary: &DiffRunSummary,
    header: &Format,
) {
    sheet.write_string_with_format(0, 0, "Warning", header).ok();
    let mut row = 1;
    for warning in &summary.warnings {
        sheet.write_string(row, 0, warning).ok();
        row += 1;
    }
}

fn write_cells_header(sheet: &mut rust_xlsxwriter::Worksheet, header: &Format) {
    let headers = [
        "Sheet",
        "Address",
        "Old value",
        "New value",
        "Old formula",
        "New formula",
        "Classification",
    ];
    for (idx, title) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *title, header)
            .ok();
    }
}

fn write_structure_header(sheet: &mut rust_xlsxwriter::Worksheet, header: &Format) {
    let headers = ["Kind", "Sheet", "Detail"];
    for (idx, title) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *title, header)
            .ok();
    }
}

fn write_query_header(sheet: &mut rust_xlsxwriter::Worksheet, header: &Format) {
    let headers = ["Kind", "Name", "Detail"];
    for (idx, title) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *title, header)
            .ok();
    }
}

fn write_model_header(sheet: &mut rust_xlsxwriter::Worksheet, header: &Format) {
    let headers = ["Kind", "Name", "Detail"];
    for (idx, title) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *title, header)
            .ok();
    }
}

fn write_other_header(sheet: &mut rust_xlsxwriter::Worksheet, header: &Format) {
    let headers = ["Kind", "Payload"];
    for (idx, title) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, idx as u16, *title, header)
            .ok();
    }
}

fn write_op(
    op: &DiffOp,
    strings: &[String],
    workbook: &mut Workbook,
    rows: &mut ExportRows,
) -> Result<(), StoreError> {
    match op {
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            ..
        } => {
            let row = rows.cells + 1;
            rows.cells += 1;
            let sheet_name = resolve_string(strings, *sheet);
            let addr_text = addr.to_a1();
            let old_value = render_cell_value(strings, &from.value);
            let new_value = render_cell_value(strings, &to.value);
            let old_formula = render_formula(strings, from.formula);
            let new_formula = render_formula(strings, to.formula);
            let classification =
                classify_cell_change(&old_value, &new_value, &old_formula, &new_formula);

            let cells_sheet = sheet_mut(workbook, "Cells")?;
            cells_sheet.write_string(row, 0, sheet_name).ok();
            cells_sheet.write_string(row, 1, &addr_text).ok();
            cells_sheet.write_string(row, 2, &old_value).ok();
            cells_sheet.write_string(row, 3, &new_value).ok();
            cells_sheet.write_string(row, 4, &old_formula).ok();
            cells_sheet.write_string(row, 5, &new_formula).ok();
            cells_sheet.write_string(row, 6, classification).ok();
        }
        DiffOp::RowAdded { sheet, row_idx, .. } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "RowAdded",
                strings,
                *sheet,
                format!("Row {} added", row_idx + 1),
            );
        }
        DiffOp::RowRemoved { sheet, row_idx, .. } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "RowRemoved",
                strings,
                *sheet,
                format!("Row {} removed", row_idx + 1),
            );
        }
        DiffOp::RowReplaced { sheet, row_idx } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "RowReplaced",
                strings,
                *sheet,
                format!("Row {} replaced", row_idx + 1),
            );
        }
        DiffOp::DuplicateKeyCluster {
            sheet,
            key,
            left_rows,
            right_rows,
        } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            let detail = format!(
                "Duplicate key [{}]: left rows [{}], right rows [{}]",
                format_key_values(strings, key),
                format_row_list(left_rows),
                format_row_list(right_rows)
            );
            write_structure(
                structure_sheet,
                rows,
                "DuplicateKeyCluster",
                strings,
                *sheet,
                detail,
            );
        }
        DiffOp::ColumnAdded { sheet, col_idx, .. } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "ColumnAdded",
                strings,
                *sheet,
                format!("Column {} added", col_idx + 1),
            );
        }
        DiffOp::ColumnRemoved { sheet, col_idx, .. } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "ColumnRemoved",
                strings,
                *sheet,
                format!("Column {} removed", col_idx + 1),
            );
        }
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            ..
        } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "BlockMovedRows",
                strings,
                *sheet,
                format!(
                    "Rows {}-{} moved to {}",
                    src_start_row + 1,
                    src_start_row + row_count,
                    dst_start_row + 1
                ),
            );
        }
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            ..
        } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "BlockMovedColumns",
                strings,
                *sheet,
                format!(
                    "Columns {}-{} moved to {}",
                    src_start_col + 1,
                    src_start_col + col_count,
                    dst_start_col + 1
                ),
            );
        }
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            ..
        } => {
            let src_end = CellAddress::from_coords(
                src_start_row + src_row_count.saturating_sub(1),
                src_start_col + src_col_count.saturating_sub(1),
            );
            let dst_start = CellAddress::from_coords(*dst_start_row, *dst_start_col);
            let src_start = CellAddress::from_coords(*src_start_row, *src_start_col);
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "BlockMovedRect",
                strings,
                *sheet,
                format!(
                    "{}:{} moved to {}",
                    src_start.to_a1(),
                    src_end.to_a1(),
                    dst_start.to_a1()
                ),
            );
        }
        DiffOp::RectReplaced {
            sheet,
            start_row,
            row_count,
            start_col,
            col_count,
        } => {
            let start = CellAddress::from_coords(*start_row, *start_col);
            let end = CellAddress::from_coords(
                start_row + row_count.saturating_sub(1),
                start_col + col_count.saturating_sub(1),
            );
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "RectReplaced",
                strings,
                *sheet,
                format!("{}:{} replaced", start.to_a1(), end.to_a1()),
            );
        }
        DiffOp::SheetAdded { sheet } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "SheetAdded",
                strings,
                *sheet,
                "Sheet added".to_string(),
            );
        }
        DiffOp::SheetRemoved { sheet } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            write_structure(
                structure_sheet,
                rows,
                "SheetRemoved",
                strings,
                *sheet,
                "Sheet removed".to_string(),
            );
        }
        DiffOp::SheetRenamed { sheet, from, to } => {
            let structure_sheet = sheet_mut(workbook, "Structure")?;
            let detail = format!(
                "Sheet renamed: {} -> {}",
                resolve_string(strings, *from),
                resolve_string(strings, *to)
            );
            write_structure(
                structure_sheet,
                rows,
                "SheetRenamed",
                strings,
                *sheet,
                detail,
            );
        }
        DiffOp::QueryAdded { name } => {
            let query_sheet = sheet_mut(workbook, "PowerQuery")?;
            write_query(
                query_sheet,
                rows,
                "QueryAdded",
                resolve_string(strings, *name),
                "",
            );
        }
        DiffOp::QueryRemoved { name } => {
            let query_sheet = sheet_mut(workbook, "PowerQuery")?;
            write_query(
                query_sheet,
                rows,
                "QueryRemoved",
                resolve_string(strings, *name),
                "",
            );
        }
        DiffOp::QueryRenamed { from, to } => {
            let detail = format!("Renamed to {}", resolve_string(strings, *to));
            let query_sheet = sheet_mut(workbook, "PowerQuery")?;
            write_query(
                query_sheet,
                rows,
                "QueryRenamed",
                resolve_string(strings, *from),
                &detail,
            );
        }
        DiffOp::QueryDefinitionChanged {
            name, change_kind, ..
        } => {
            let detail = match change_kind {
                QueryChangeKind::Semantic => "Semantic change",
                QueryChangeKind::FormattingOnly => "Formatting only",
                QueryChangeKind::Renamed => "Renamed",
            };
            let query_sheet = sheet_mut(workbook, "PowerQuery")?;
            write_query(
                query_sheet,
                rows,
                "QueryDefinitionChanged",
                resolve_string(strings, *name),
                detail,
            );
        }
        DiffOp::QueryMetadataChanged { name, field, .. } => {
            let query_sheet = sheet_mut(workbook, "PowerQuery")?;
            write_query(
                query_sheet,
                rows,
                "QueryMetadataChanged",
                resolve_string(strings, *name),
                &format!("{field:?}"),
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::TableAdded { name } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "TableAdded",
                resolve_string(strings, *name),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::TableRemoved { name } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "TableRemoved",
                resolve_string(strings, *name),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnAdded {
            table,
            name,
            data_type,
        } => {
            let detail = data_type
                .map(|id| format!("type={}", resolve_string(strings, id)))
                .unwrap_or_default();
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "ModelColumnAdded",
                &format_column_ref(strings, *table, *name),
                &detail,
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnRemoved { table, name } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "ModelColumnRemoved",
                &format_column_ref(strings, *table, *name),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnTypeChanged {
            table,
            name,
            old_type,
            new_type,
        } => {
            let old_str = old_type
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let new_str = new_type
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let detail = format!("type: {} -> {}", old_str, new_str);
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "ModelColumnTypeChanged",
                &format_column_ref(strings, *table, *name),
                &detail,
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnPropertyChanged {
            table,
            name,
            field,
            old,
            new,
        } => {
            let old_str = old
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let new_str = new
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let detail = format!("{}: {} -> {}", column_field_name(*field), old_str, new_str);
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "ModelColumnPropertyChanged",
                &format_column_ref(strings, *table, *name),
                &detail,
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::CalculatedColumnDefinitionChanged {
            table,
            name,
            change_kind,
            ..
        } => {
            let detail = format!(
                "definition changed ({})",
                expression_change_label(*change_kind)
            );
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "CalculatedColumnDefinitionChanged",
                &format_column_ref(strings, *table, *name),
                &detail,
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::RelationshipAdded {
            from_table,
            from_column,
            to_table,
            to_column,
        } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "RelationshipAdded",
                &format_relationship_ref(strings, *from_table, *from_column, *to_table, *to_column),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::RelationshipRemoved {
            from_table,
            from_column,
            to_table,
            to_column,
        } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "RelationshipRemoved",
                &format_relationship_ref(strings, *from_table, *from_column, *to_table, *to_column),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::RelationshipPropertyChanged {
            from_table,
            from_column,
            to_table,
            to_column,
            field,
            old,
            new,
        } => {
            let old_str = old
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let new_str = new
                .map(|id| resolve_string(strings, id))
                .unwrap_or("<none>");
            let detail = format!(
                "{}: {} -> {}",
                relationship_field_name(*field),
                old_str,
                new_str
            );
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "RelationshipPropertyChanged",
                &format_relationship_ref(strings, *from_table, *from_column, *to_table, *to_column),
                &detail,
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { name } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "MeasureAdded",
                resolve_string(strings, *name),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureRemoved { name } => {
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "MeasureRemoved",
                resolve_string(strings, *name),
                "",
            );
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged {
            name, change_kind, ..
        } => {
            let detail = format!(
                "definition changed ({})",
                expression_change_label(*change_kind)
            );
            let model_sheet = sheet_mut(workbook, "Model")?;
            write_model(
                model_sheet,
                rows,
                "MeasureDefinitionChanged",
                resolve_string(strings, *name),
                &detail,
            );
        }
        _ => {
            let row = rows.other + 1;
            rows.other += 1;
            let payload =
                serde_json::to_string(op).unwrap_or_else(|_| "<unserializable>".to_string());
            let other_sheet = sheet_mut(workbook, "OtherOps")?;
            other_sheet.write_string(row, 0, op_kind_label(op)).ok();
            other_sheet.write_string(row, 1, &payload).ok();
        }
    }

    Ok(())
}

fn write_structure(
    sheet: &mut rust_xlsxwriter::Worksheet,
    rows: &mut ExportRows,
    kind: &str,
    strings: &[String],
    sheet_id: excel_diff::StringId,
    detail: String,
) {
    let row = rows.structure + 1;
    rows.structure += 1;
    sheet.write_string(row, 0, kind).ok();
    sheet
        .write_string(row, 1, resolve_string(strings, sheet_id))
        .ok();
    sheet.write_string(row, 2, &detail).ok();
}

fn write_query(
    sheet: &mut rust_xlsxwriter::Worksheet,
    rows: &mut ExportRows,
    kind: &str,
    name: &str,
    detail: &str,
) {
    let row = rows.query + 1;
    rows.query += 1;
    sheet.write_string(row, 0, kind).ok();
    sheet.write_string(row, 1, name).ok();
    if !detail.is_empty() {
        sheet.write_string(row, 2, detail).ok();
    }
}

fn write_model(
    sheet: &mut rust_xlsxwriter::Worksheet,
    rows: &mut ExportRows,
    kind: &str,
    name: &str,
    detail: &str,
) {
    let row = rows.model + 1;
    rows.model += 1;
    sheet.write_string(row, 0, kind).ok();
    sheet.write_string(row, 1, name).ok();
    if !detail.is_empty() {
        sheet.write_string(row, 2, detail).ok();
    }
}

fn resolve_string(strings: &[String], id: excel_diff::StringId) -> &str {
    strings
        .get(id.0 as usize)
        .map(String::as_str)
        .unwrap_or("<unknown>")
}

#[cfg(feature = "model-diff")]
fn format_column_ref(
    strings: &[String],
    table: excel_diff::StringId,
    column: excel_diff::StringId,
) -> String {
    format!(
        "{}.{}",
        resolve_string(strings, table),
        resolve_string(strings, column)
    )
}

#[cfg(feature = "model-diff")]
fn format_relationship_ref(
    strings: &[String],
    from_table: excel_diff::StringId,
    from_column: excel_diff::StringId,
    to_table: excel_diff::StringId,
    to_column: excel_diff::StringId,
) -> String {
    format!(
        "{}[{}] -> {}[{}]",
        resolve_string(strings, from_table),
        resolve_string(strings, from_column),
        resolve_string(strings, to_table),
        resolve_string(strings, to_column)
    )
}

#[cfg(feature = "model-diff")]
fn column_field_name(field: ModelColumnProperty) -> &'static str {
    match field {
        ModelColumnProperty::Hidden => "hidden",
        ModelColumnProperty::FormatString => "format_string",
        ModelColumnProperty::SortBy => "sort_by",
        ModelColumnProperty::SummarizeBy => "summarize_by",
    }
}

#[cfg(feature = "model-diff")]
fn relationship_field_name(field: RelationshipProperty) -> &'static str {
    match field {
        RelationshipProperty::CrossFilteringBehavior => "cross_filtering_behavior",
        RelationshipProperty::Cardinality => "cardinality",
        RelationshipProperty::IsActive => "is_active",
    }
}

fn expression_change_label(kind: ExpressionChangeKind) -> &'static str {
    match kind {
        ExpressionChangeKind::Semantic => "semantic change",
        ExpressionChangeKind::FormattingOnly => "formatting only",
        ExpressionChangeKind::Unknown => "unknown",
    }
}

fn render_cell_value(strings: &[String], value: &Option<CellValue>) -> String {
    match value {
        None => String::new(),
        Some(CellValue::Blank) => String::new(),
        Some(CellValue::Number(n)) => n.to_string(),
        Some(CellValue::Text(id)) => resolve_string(strings, *id).to_string(),
        Some(CellValue::Bool(b)) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Some(CellValue::Error(id)) => resolve_string(strings, *id).to_string(),
    }
}

fn render_formula(strings: &[String], formula: Option<excel_diff::StringId>) -> String {
    match formula {
        Some(id) => {
            let raw = resolve_string(strings, id);
            if raw.is_empty() {
                String::new()
            } else if raw.starts_with('=') {
                raw.to_string()
            } else {
                format!("={}", raw)
            }
        }
        None => String::new(),
    }
}

fn classify_cell_change(
    old_value: &str,
    new_value: &str,
    old_formula: &str,
    new_formula: &str,
) -> &'static str {
    let value_changed = old_value != new_value;
    let formula_changed = old_formula != new_formula;

    if value_changed && formula_changed {
        "Value + Formula"
    } else if value_changed {
        if old_value.is_empty() && !new_value.is_empty() {
            "Added value"
        } else if !old_value.is_empty() && new_value.is_empty() {
            "Removed value"
        } else {
            "Value change"
        }
    } else if formula_changed {
        if old_formula.is_empty() && !new_formula.is_empty() {
            "Added formula"
        } else if !old_formula.is_empty() && new_formula.is_empty() {
            "Removed formula"
        } else {
            "Formula change"
        }
    } else {
        "Unchanged"
    }
}

fn op_kind_label(op: &DiffOp) -> &str {
    match op {
        DiffOp::VbaModuleAdded { .. } => "VbaModuleAdded",
        DiffOp::VbaModuleRemoved { .. } => "VbaModuleRemoved",
        DiffOp::VbaModuleChanged { .. } => "VbaModuleChanged",
        DiffOp::NamedRangeAdded { .. } => "NamedRangeAdded",
        DiffOp::NamedRangeRemoved { .. } => "NamedRangeRemoved",
        DiffOp::NamedRangeChanged { .. } => "NamedRangeChanged",
        DiffOp::ChartAdded { .. } => "ChartAdded",
        DiffOp::ChartRemoved { .. } => "ChartRemoved",
        DiffOp::ChartChanged { .. } => "ChartChanged",
        DiffOp::DuplicateKeyCluster { .. } => "DuplicateKeyCluster",
        _ => "Other",
    }
}

fn format_key_values(strings: &[String], key: &[Option<CellValue>]) -> String {
    let parts: Vec<String> = key
        .iter()
        .map(|value| render_cell_value(strings, value))
        .collect();
    parts.join(", ")
}

fn format_row_list(rows: &[u32]) -> String {
    let parts: Vec<String> = rows.iter().map(|row| (row + 1).to_string()).collect();
    parts.join(", ")
}
