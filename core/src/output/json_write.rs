use crate::diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, DiffOp, ExtractedColumnTypeChanges,
    ExtractedRenamePairs, ExtractedString, ExtractedStringList, FormulaDiffResult, QueryChangeKind,
    QueryMetadataField, QuerySemanticDetail, RenamePair, StepChange, StepDiff, StepParams,
    StepSnapshot, StepType,
};
use crate::string_pool::StringId;
use crate::workbook::{CellAddress, CellSnapshot, CellValue, ColSignature, RowSignature};
use std::io::{self, Write};

#[cfg(feature = "model-diff")]
use crate::diff::{ExpressionChangeKind, ModelColumnProperty, RelationshipProperty};

const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";

pub fn write_jsonl_header(w: &mut impl Write, version: &str, strings: &[String]) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    write_json_string_lit(w, "Header")?;
    w.write_all(b",")?;
    write_json_key(w, "version")?;
    write_json_string(w, version)?;
    w.write_all(b",")?;
    write_json_key(w, "strings")?;
    write_json_string_array(w, strings)?;
    w.write_all(b"}")?;
    Ok(())
}

pub fn write_diff_op(w: &mut impl Write, op: &DiffOp) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match op {
        DiffOp::SheetAdded { sheet } => {
            write_json_string_lit(w, "SheetAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
        }
        DiffOp::SheetRemoved { sheet } => {
            write_json_string_lit(w, "SheetRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
        }
        DiffOp::SheetRenamed { sheet, from, to } => {
            write_json_string_lit(w, "SheetRenamed")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "from")?;
            write_string_id(w, *from)?;
            w.write_all(b",")?;
            write_json_key(w, "to")?;
            write_string_id(w, *to)?;
        }
        DiffOp::RowAdded {
            sheet,
            row_idx,
            row_signature,
        } => {
            write_json_string_lit(w, "RowAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "row_idx")?;
            write_u32(w, *row_idx)?;
            if let Some(sig) = row_signature {
                w.write_all(b",")?;
                write_json_key(w, "row_signature")?;
                write_row_signature(w, sig)?;
            }
        }
        DiffOp::RowRemoved {
            sheet,
            row_idx,
            row_signature,
        } => {
            write_json_string_lit(w, "RowRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "row_idx")?;
            write_u32(w, *row_idx)?;
            if let Some(sig) = row_signature {
                w.write_all(b",")?;
                write_json_key(w, "row_signature")?;
                write_row_signature(w, sig)?;
            }
        }
        DiffOp::DuplicateKeyCluster {
            sheet,
            key,
            left_rows,
            right_rows,
        } => {
            write_json_string_lit(w, "DuplicateKeyCluster")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "key")?;
            write_cell_value_opt_array(w, key)?;
            w.write_all(b",")?;
            write_json_key(w, "left_rows")?;
            write_u32_array(w, left_rows)?;
            w.write_all(b",")?;
            write_json_key(w, "right_rows")?;
            write_u32_array(w, right_rows)?;
        }
        DiffOp::RowReplaced { sheet, row_idx } => {
            write_json_string_lit(w, "RowReplaced")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "row_idx")?;
            write_u32(w, *row_idx)?;
        }
        DiffOp::ColumnAdded {
            sheet,
            col_idx,
            col_signature,
        } => {
            write_json_string_lit(w, "ColumnAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "col_idx")?;
            write_u32(w, *col_idx)?;
            if let Some(sig) = col_signature {
                w.write_all(b",")?;
                write_json_key(w, "col_signature")?;
                write_col_signature(w, sig)?;
            }
        }
        DiffOp::ColumnRemoved {
            sheet,
            col_idx,
            col_signature,
        } => {
            write_json_string_lit(w, "ColumnRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "col_idx")?;
            write_u32(w, *col_idx)?;
            if let Some(sig) = col_signature {
                w.write_all(b",")?;
                write_json_key(w, "col_signature")?;
                write_col_signature(w, sig)?;
            }
        }
        DiffOp::BlockMovedRows {
            sheet,
            src_start_row,
            row_count,
            dst_start_row,
            block_hash,
        } => {
            write_json_string_lit(w, "BlockMovedRows")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "src_start_row")?;
            write_u32(w, *src_start_row)?;
            w.write_all(b",")?;
            write_json_key(w, "row_count")?;
            write_u32(w, *row_count)?;
            w.write_all(b",")?;
            write_json_key(w, "dst_start_row")?;
            write_u32(w, *dst_start_row)?;
            if let Some(hash) = block_hash {
                w.write_all(b",")?;
                write_json_key(w, "block_hash")?;
                write_u64(w, *hash)?;
            }
        }
        DiffOp::BlockMovedColumns {
            sheet,
            src_start_col,
            col_count,
            dst_start_col,
            block_hash,
        } => {
            write_json_string_lit(w, "BlockMovedColumns")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "src_start_col")?;
            write_u32(w, *src_start_col)?;
            w.write_all(b",")?;
            write_json_key(w, "col_count")?;
            write_u32(w, *col_count)?;
            w.write_all(b",")?;
            write_json_key(w, "dst_start_col")?;
            write_u32(w, *dst_start_col)?;
            if let Some(hash) = block_hash {
                w.write_all(b",")?;
                write_json_key(w, "block_hash")?;
                write_u64(w, *hash)?;
            }
        }
        DiffOp::BlockMovedRect {
            sheet,
            src_start_row,
            src_row_count,
            src_start_col,
            src_col_count,
            dst_start_row,
            dst_start_col,
            block_hash,
        } => {
            write_json_string_lit(w, "BlockMovedRect")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "src_start_row")?;
            write_u32(w, *src_start_row)?;
            w.write_all(b",")?;
            write_json_key(w, "src_row_count")?;
            write_u32(w, *src_row_count)?;
            w.write_all(b",")?;
            write_json_key(w, "src_start_col")?;
            write_u32(w, *src_start_col)?;
            w.write_all(b",")?;
            write_json_key(w, "src_col_count")?;
            write_u32(w, *src_col_count)?;
            w.write_all(b",")?;
            write_json_key(w, "dst_start_row")?;
            write_u32(w, *dst_start_row)?;
            w.write_all(b",")?;
            write_json_key(w, "dst_start_col")?;
            write_u32(w, *dst_start_col)?;
            if let Some(hash) = block_hash {
                w.write_all(b",")?;
                write_json_key(w, "block_hash")?;
                write_u64(w, *hash)?;
            }
        }
        DiffOp::RectReplaced {
            sheet,
            start_row,
            row_count,
            start_col,
            col_count,
        } => {
            write_json_string_lit(w, "RectReplaced")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "start_row")?;
            write_u32(w, *start_row)?;
            w.write_all(b",")?;
            write_json_key(w, "row_count")?;
            write_u32(w, *row_count)?;
            w.write_all(b",")?;
            write_json_key(w, "start_col")?;
            write_u32(w, *start_col)?;
            w.write_all(b",")?;
            write_json_key(w, "col_count")?;
            write_u32(w, *col_count)?;
        }
        DiffOp::CellEdited {
            sheet,
            addr,
            from,
            to,
            formula_diff,
        } => {
            write_json_string_lit(w, "CellEdited")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "addr")?;
            write_cell_address(w, *addr)?;
            w.write_all(b",")?;
            write_json_key(w, "from")?;
            write_cell_snapshot(w, from)?;
            w.write_all(b",")?;
            write_json_key(w, "to")?;
            write_cell_snapshot(w, to)?;
            w.write_all(b",")?;
            write_json_key(w, "formula_diff")?;
            write_formula_diff_result(w, *formula_diff)?;
        }
        DiffOp::VbaModuleAdded { name } => {
            write_json_string_lit(w, "VbaModuleAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::VbaModuleRemoved { name } => {
            write_json_string_lit(w, "VbaModuleRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::VbaModuleChanged { name } => {
            write_json_string_lit(w, "VbaModuleChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::NamedRangeAdded { name } => {
            write_json_string_lit(w, "NamedRangeAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::NamedRangeRemoved { name } => {
            write_json_string_lit(w, "NamedRangeRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::NamedRangeChanged {
            name,
            old_ref,
            new_ref,
        } => {
            write_json_string_lit(w, "NamedRangeChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "old_ref")?;
            write_string_id(w, *old_ref)?;
            w.write_all(b",")?;
            write_json_key(w, "new_ref")?;
            write_string_id(w, *new_ref)?;
        }
        DiffOp::ChartAdded { sheet, name } => {
            write_json_string_lit(w, "ChartAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::ChartRemoved { sheet, name } => {
            write_json_string_lit(w, "ChartRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::ChartChanged { sheet, name } => {
            write_json_string_lit(w, "ChartChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "sheet")?;
            write_string_id(w, *sheet)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::QueryAdded { name } => {
            write_json_string_lit(w, "QueryAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::QueryRemoved { name } => {
            write_json_string_lit(w, "QueryRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        DiffOp::QueryRenamed { from, to } => {
            write_json_string_lit(w, "QueryRenamed")?;
            w.write_all(b",")?;
            write_json_key(w, "from")?;
            write_string_id(w, *from)?;
            w.write_all(b",")?;
            write_json_key(w, "to")?;
            write_string_id(w, *to)?;
        }
        DiffOp::QueryDefinitionChanged {
            name,
            change_kind,
            old_hash,
            new_hash,
            semantic_detail,
        } => {
            write_json_string_lit(w, "QueryDefinitionChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "change_kind")?;
            write_query_change_kind(w, *change_kind)?;
            w.write_all(b",")?;
            write_json_key(w, "old_hash")?;
            write_u64(w, *old_hash)?;
            w.write_all(b",")?;
            write_json_key(w, "new_hash")?;
            write_u64(w, *new_hash)?;
            if let Some(detail) = semantic_detail {
                w.write_all(b",")?;
                write_json_key(w, "semantic_detail")?;
                write_query_semantic_detail(w, detail)?;
            }
        }
        DiffOp::QueryMetadataChanged {
            name,
            field,
            old,
            new,
        } => {
            write_json_string_lit(w, "QueryMetadataChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "field")?;
            write_query_metadata_field(w, *field)?;
            w.write_all(b",")?;
            write_json_key(w, "old")?;
            write_option_string_id(w, *old)?;
            w.write_all(b",")?;
            write_json_key(w, "new")?;
            write_option_string_id(w, *new)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::TableAdded { name } => {
            write_json_string_lit(w, "TableAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::TableRemoved { name } => {
            write_json_string_lit(w, "TableRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnAdded {
            table,
            name,
            data_type,
        } => {
            write_json_string_lit(w, "ModelColumnAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "table")?;
            write_string_id(w, *table)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            if let Some(dt) = data_type {
                w.write_all(b",")?;
                write_json_key(w, "data_type")?;
                write_string_id(w, *dt)?;
            }
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnRemoved { table, name } => {
            write_json_string_lit(w, "ModelColumnRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "table")?;
            write_string_id(w, *table)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnTypeChanged {
            table,
            name,
            old_type,
            new_type,
        } => {
            write_json_string_lit(w, "ModelColumnTypeChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "table")?;
            write_string_id(w, *table)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            if let Some(ty) = old_type {
                w.write_all(b",")?;
                write_json_key(w, "old_type")?;
                write_string_id(w, *ty)?;
            }
            if let Some(ty) = new_type {
                w.write_all(b",")?;
                write_json_key(w, "new_type")?;
                write_string_id(w, *ty)?;
            }
        }
        #[cfg(feature = "model-diff")]
        DiffOp::ModelColumnPropertyChanged {
            table,
            name,
            field,
            old,
            new,
        } => {
            write_json_string_lit(w, "ModelColumnPropertyChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "table")?;
            write_string_id(w, *table)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "field")?;
            write_model_column_property(w, *field)?;
            if let Some(val) = old {
                w.write_all(b",")?;
                write_json_key(w, "old")?;
                write_string_id(w, *val)?;
            }
            if let Some(val) = new {
                w.write_all(b",")?;
                write_json_key(w, "new")?;
                write_string_id(w, *val)?;
            }
        }
        #[cfg(feature = "model-diff")]
        DiffOp::CalculatedColumnDefinitionChanged {
            table,
            name,
            change_kind,
            old_hash,
            new_hash,
        } => {
            write_json_string_lit(w, "CalculatedColumnDefinitionChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "table")?;
            write_string_id(w, *table)?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "change_kind")?;
            write_expression_change_kind(w, *change_kind)?;
            w.write_all(b",")?;
            write_json_key(w, "old_hash")?;
            write_u64(w, *old_hash)?;
            w.write_all(b",")?;
            write_json_key(w, "new_hash")?;
            write_u64(w, *new_hash)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::RelationshipAdded {
            from_table,
            from_column,
            to_table,
            to_column,
        } => {
            write_json_string_lit(w, "RelationshipAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "from_table")?;
            write_string_id(w, *from_table)?;
            w.write_all(b",")?;
            write_json_key(w, "from_column")?;
            write_string_id(w, *from_column)?;
            w.write_all(b",")?;
            write_json_key(w, "to_table")?;
            write_string_id(w, *to_table)?;
            w.write_all(b",")?;
            write_json_key(w, "to_column")?;
            write_string_id(w, *to_column)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::RelationshipRemoved {
            from_table,
            from_column,
            to_table,
            to_column,
        } => {
            write_json_string_lit(w, "RelationshipRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "from_table")?;
            write_string_id(w, *from_table)?;
            w.write_all(b",")?;
            write_json_key(w, "from_column")?;
            write_string_id(w, *from_column)?;
            w.write_all(b",")?;
            write_json_key(w, "to_table")?;
            write_string_id(w, *to_table)?;
            w.write_all(b",")?;
            write_json_key(w, "to_column")?;
            write_string_id(w, *to_column)?;
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
            write_json_string_lit(w, "RelationshipPropertyChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "from_table")?;
            write_string_id(w, *from_table)?;
            w.write_all(b",")?;
            write_json_key(w, "from_column")?;
            write_string_id(w, *from_column)?;
            w.write_all(b",")?;
            write_json_key(w, "to_table")?;
            write_string_id(w, *to_table)?;
            w.write_all(b",")?;
            write_json_key(w, "to_column")?;
            write_string_id(w, *to_column)?;
            w.write_all(b",")?;
            write_json_key(w, "field")?;
            write_relationship_property(w, *field)?;
            if let Some(val) = old {
                w.write_all(b",")?;
                write_json_key(w, "old")?;
                write_string_id(w, *val)?;
            }
            if let Some(val) = new {
                w.write_all(b",")?;
                write_json_key(w, "new")?;
                write_string_id(w, *val)?;
            }
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureAdded { name } => {
            write_json_string_lit(w, "MeasureAdded")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureRemoved { name } => {
            write_json_string_lit(w, "MeasureRemoved")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
        }
        #[cfg(feature = "model-diff")]
        DiffOp::MeasureDefinitionChanged {
            name,
            change_kind,
            old_hash,
            new_hash,
        } => {
            write_json_string_lit(w, "MeasureDefinitionChanged")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "change_kind")?;
            write_expression_change_kind(w, *change_kind)?;
            w.write_all(b",")?;
            write_json_key(w, "old_hash")?;
            write_u64(w, *old_hash)?;
            w.write_all(b",")?;
            write_json_key(w, "new_hash")?;
            write_u64(w, *new_hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

pub fn write_json_string(w: &mut impl Write, s: &str) -> io::Result<()> {
    w.write_all(b"\"")?;

    let bytes = s.as_bytes();
    let mut start = 0usize;
    for (i, &b) in bytes.iter().enumerate() {
        let esc: Option<&'static [u8]> = match b {
            b'\"' => Some(br#"\""#),
            b'\\' => Some(br#"\\"#),
            b'\n' => Some(br#"\n"#),
            b'\r' => Some(br#"\r"#),
            b'\t' => Some(br#"\t"#),
            0x08 => Some(br#"\b"#),
            0x0c => Some(br#"\f"#),
            b if b < 0x20 => None,
            _ => continue,
        };

        w.write_all(&bytes[start..i])?;
        match esc {
            Some(seq) => w.write_all(seq)?,
            None => {
                let mut buf = [0u8; 6];
                buf[0] = b'\\';
                buf[1] = b'u';
                buf[2] = b'0';
                buf[3] = b'0';
                buf[4] = HEX_LOWER[(b >> 4) as usize];
                buf[5] = HEX_LOWER[(b & 0x0f) as usize];
                w.write_all(&buf)?;
            }
        }
        start = i + 1;
    }
    w.write_all(&bytes[start..])?;
    w.write_all(b"\"")?;
    Ok(())
}

fn write_json_string_lit(w: &mut impl Write, s: &'static str) -> io::Result<()> {
    w.write_all(b"\"")?;
    w.write_all(s.as_bytes())?;
    w.write_all(b"\"")?;
    Ok(())
}

fn write_json_key(w: &mut impl Write, key: &'static str) -> io::Result<()> {
    w.write_all(b"\"")?;
    w.write_all(key.as_bytes())?;
    w.write_all(b"\":")?;
    Ok(())
}

fn write_json_string_array(w: &mut impl Write, strings: &[String]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, s) in strings.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        write_json_string(w, s)?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_u32(w: &mut impl Write, value: u32) -> io::Result<()> {
    write_u64(w, value as u64)
}

fn write_u64(w: &mut impl Write, mut value: u64) -> io::Result<()> {
    if value == 0 {
        return w.write_all(b"0");
    }

    let mut buf = [0u8; 20];
    let mut i = buf.len();
    while value != 0 {
        i -= 1;
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    w.write_all(&buf[i..])
}

fn write_option_u64(w: &mut impl Write, value: Option<u64>) -> io::Result<()> {
    match value {
        Some(v) => write_u64(w, v),
        None => w.write_all(b"null"),
    }
}

fn write_option_u32(w: &mut impl Write, value: Option<u32>) -> io::Result<()> {
    match value {
        Some(v) => write_u32(w, v),
        None => w.write_all(b"null"),
    }
}

fn write_bool(w: &mut impl Write, value: bool) -> io::Result<()> {
    if value {
        w.write_all(b"true")
    } else {
        w.write_all(b"false")
    }
}

fn write_f64(w: &mut impl Write, value: f64) -> io::Result<()> {
    if !value.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "non-finite numbers are not supported in JSON output",
        ));
    }

    let mut buf = ryu::Buffer::new();
    let s = buf.format_finite(value);
    w.write_all(s.as_bytes())
}

fn write_string_id(w: &mut impl Write, value: StringId) -> io::Result<()> {
    write_u32(w, value.0)
}

fn write_option_string_id(w: &mut impl Write, value: Option<StringId>) -> io::Result<()> {
    match value {
        Some(v) => write_string_id(w, v),
        None => w.write_all(b"null"),
    }
}

fn write_u32_array(w: &mut impl Write, values: &[u32]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, v) in values.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        write_u32(w, *v)?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_string_id_array(w: &mut impl Write, values: &[StringId]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, v) in values.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        write_string_id(w, *v)?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_cell_address(w: &mut impl Write, addr: CellAddress) -> io::Result<()> {
    w.write_all(b"\"")?;

    // Same behavior as `addressing::index_to_address`, but without allocating.
    let mut col_index = addr.col;
    let mut letters = [0u8; 16];
    let mut letters_len: usize = 0;
    loop {
        let rem = (col_index % 26) as u8;
        letters[letters_len] = b'A' + rem;
        letters_len += 1;
        if col_index < 26 {
            break;
        }
        col_index = col_index / 26 - 1;
    }

    let mut buf = [0u8; 32];
    let mut pos = 0usize;
    for i in (0..letters_len).rev() {
        buf[pos] = letters[i];
        pos += 1;
    }

    let mut row_num = addr.row.saturating_add(1);
    let mut digits = [0u8; 10];
    let mut digits_len: usize = 0;
    while row_num != 0 {
        digits[digits_len] = b'0' + (row_num % 10) as u8;
        digits_len += 1;
        row_num /= 10;
    }
    for i in (0..digits_len).rev() {
        buf[pos] = digits[i];
        pos += 1;
    }

    w.write_all(&buf[..pos])?;
    w.write_all(b"\"")?;
    Ok(())
}

fn write_cell_value(w: &mut impl Write, value: &CellValue) -> io::Result<()> {
    match value {
        CellValue::Blank => write_json_string_lit(w, "Blank"),
        CellValue::Number(n) => {
            w.write_all(b"{")?;
            write_json_key(w, "Number")?;
            write_f64(w, *n)?;
            w.write_all(b"}")?;
            Ok(())
        }
        CellValue::Text(id) => {
            w.write_all(b"{")?;
            write_json_key(w, "Text")?;
            write_string_id(w, *id)?;
            w.write_all(b"}")?;
            Ok(())
        }
        CellValue::Bool(b) => {
            w.write_all(b"{")?;
            write_json_key(w, "Bool")?;
            write_bool(w, *b)?;
            w.write_all(b"}")?;
            Ok(())
        }
        CellValue::Error(id) => {
            w.write_all(b"{")?;
            write_json_key(w, "Error")?;
            write_string_id(w, *id)?;
            w.write_all(b"}")?;
            Ok(())
        }
    }
}

fn write_option_cell_value(w: &mut impl Write, value: Option<&CellValue>) -> io::Result<()> {
    match value {
        Some(v) => write_cell_value(w, v),
        None => w.write_all(b"null"),
    }
}

fn write_cell_value_opt_array(w: &mut impl Write, values: &[Option<CellValue>]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, v) in values.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        match v {
            Some(cv) => write_cell_value(w, cv)?,
            None => w.write_all(b"null")?,
        }
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_cell_snapshot(w: &mut impl Write, snap: &CellSnapshot) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "addr")?;
    write_cell_address(w, snap.addr)?;
    w.write_all(b",")?;
    write_json_key(w, "value")?;
    write_option_cell_value(w, snap.value.as_ref())?;
    w.write_all(b",")?;
    write_json_key(w, "formula")?;
    write_option_string_id(w, snap.formula)?;
    w.write_all(b"}")?;
    Ok(())
}

fn write_u128_hex_32(w: &mut impl Write, value: u128) -> io::Result<()> {
    let mut buf = [0u8; 32];
    for i in 0..32 {
        let shift = 4 * (31 - i);
        let nibble = ((value >> shift) & 0x0f) as usize;
        buf[i] = HEX_LOWER[nibble];
    }
    w.write_all(&buf)
}

fn write_row_signature(w: &mut impl Write, sig: &RowSignature) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "hash")?;
    w.write_all(b"\"")?;
    write_u128_hex_32(w, sig.hash)?;
    w.write_all(b"\"")?;
    w.write_all(b"}")?;
    Ok(())
}

fn write_col_signature(w: &mut impl Write, sig: &ColSignature) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "hash")?;
    w.write_all(b"\"")?;
    write_u128_hex_32(w, sig.hash)?;
    w.write_all(b"\"")?;
    w.write_all(b"}")?;
    Ok(())
}

fn write_formula_diff_result(w: &mut impl Write, value: FormulaDiffResult) -> io::Result<()> {
    let s = match value {
        FormulaDiffResult::Unknown => "unknown",
        FormulaDiffResult::Unchanged => "unchanged",
        FormulaDiffResult::Added => "added",
        FormulaDiffResult::Removed => "removed",
        FormulaDiffResult::FormattingOnly => "formatting_only",
        FormulaDiffResult::Filled => "filled",
        FormulaDiffResult::SemanticChange => "semantic_change",
        FormulaDiffResult::TextChange => "text_change",
    };
    write_json_string_lit(w, s)
}

fn write_query_change_kind(w: &mut impl Write, value: QueryChangeKind) -> io::Result<()> {
    let s = match value {
        QueryChangeKind::Semantic => "semantic",
        QueryChangeKind::FormattingOnly => "formatting_only",
        QueryChangeKind::Renamed => "renamed",
    };
    write_json_string_lit(w, s)
}

#[cfg(feature = "model-diff")]
fn write_expression_change_kind(w: &mut impl Write, value: ExpressionChangeKind) -> io::Result<()> {
    let s = match value {
        ExpressionChangeKind::Semantic => "semantic",
        ExpressionChangeKind::FormattingOnly => "formatting_only",
        ExpressionChangeKind::Unknown => "unknown",
    };
    write_json_string_lit(w, s)
}

fn write_query_metadata_field(w: &mut impl Write, value: QueryMetadataField) -> io::Result<()> {
    let s = match value {
        QueryMetadataField::LoadToSheet => "LoadToSheet",
        QueryMetadataField::LoadToModel => "LoadToModel",
        QueryMetadataField::GroupPath => "GroupPath",
        QueryMetadataField::ConnectionOnly => "ConnectionOnly",
    };
    write_json_string_lit(w, s)
}

#[cfg(feature = "model-diff")]
fn write_model_column_property(w: &mut impl Write, value: ModelColumnProperty) -> io::Result<()> {
    let s = match value {
        ModelColumnProperty::Hidden => "hidden",
        ModelColumnProperty::FormatString => "format_string",
        ModelColumnProperty::SortBy => "sort_by",
        ModelColumnProperty::SummarizeBy => "summarize_by",
    };
    write_json_string_lit(w, s)
}

#[cfg(feature = "model-diff")]
fn write_relationship_property(w: &mut impl Write, value: RelationshipProperty) -> io::Result<()> {
    let s = match value {
        RelationshipProperty::CrossFilteringBehavior => "cross_filtering_behavior",
        RelationshipProperty::Cardinality => "cardinality",
        RelationshipProperty::IsActive => "is_active",
    };
    write_json_string_lit(w, s)
}

fn write_step_type(w: &mut impl Write, value: StepType) -> io::Result<()> {
    let s = match value {
        StepType::TableSelectRows => "table_select_rows",
        StepType::TableRemoveColumns => "table_remove_columns",
        StepType::TableRenameColumns => "table_rename_columns",
        StepType::TableTransformColumnTypes => "table_transform_column_types",
        StepType::TableNestedJoin => "table_nested_join",
        StepType::TableJoin => "table_join",
        StepType::Other => "other",
    };
    write_json_string_lit(w, s)
}

fn write_ast_diff_mode(w: &mut impl Write, value: AstDiffMode) -> io::Result<()> {
    let s = match value {
        AstDiffMode::SmallExact => "small_exact",
        AstDiffMode::LargeHeuristic => "large_heuristic",
    };
    write_json_string_lit(w, s)
}

fn write_query_semantic_detail(w: &mut impl Write, detail: &QuerySemanticDetail) -> io::Result<()> {
    w.write_all(b"{")?;
    let mut wrote_any = false;

    if !detail.step_diffs.is_empty() {
        wrote_any = true;
        write_json_key(w, "step_diffs")?;
        write_step_diff_array(w, &detail.step_diffs)?;
    }

    if let Some(ast) = &detail.ast_summary {
        if wrote_any {
            w.write_all(b",")?;
        }
        write_json_key(w, "ast_summary")?;
        write_ast_diff_summary(w, ast)?;
    }

    w.write_all(b"}")?;
    Ok(())
}

fn write_step_diff_array(w: &mut impl Write, diffs: &[StepDiff]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, d) in diffs.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        write_step_diff(w, d)?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_step_diff(w: &mut impl Write, diff: &StepDiff) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match diff {
        StepDiff::StepAdded { step } => {
            write_json_string_lit(w, "step_added")?;
            w.write_all(b",")?;
            write_json_key(w, "step")?;
            write_step_snapshot(w, step)?;
        }
        StepDiff::StepRemoved { step } => {
            write_json_string_lit(w, "step_removed")?;
            w.write_all(b",")?;
            write_json_key(w, "step")?;
            write_step_snapshot(w, step)?;
        }
        StepDiff::StepReordered {
            name,
            from_index,
            to_index,
        } => {
            write_json_string_lit(w, "step_reordered")?;
            w.write_all(b",")?;
            write_json_key(w, "name")?;
            write_string_id(w, *name)?;
            w.write_all(b",")?;
            write_json_key(w, "from_index")?;
            write_u32(w, *from_index)?;
            w.write_all(b",")?;
            write_json_key(w, "to_index")?;
            write_u32(w, *to_index)?;
        }
        StepDiff::StepModified {
            before,
            after,
            changes,
        } => {
            write_json_string_lit(w, "step_modified")?;
            w.write_all(b",")?;
            write_json_key(w, "before")?;
            write_step_snapshot(w, before)?;
            w.write_all(b",")?;
            write_json_key(w, "after")?;
            write_step_snapshot(w, after)?;
            if !changes.is_empty() {
                w.write_all(b",")?;
                write_json_key(w, "changes")?;
                write_step_change_array(w, changes)?;
            }
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_step_snapshot(w: &mut impl Write, snap: &StepSnapshot) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "name")?;
    write_string_id(w, snap.name)?;
    w.write_all(b",")?;
    write_json_key(w, "index")?;
    write_u32(w, snap.index)?;
    w.write_all(b",")?;
    write_json_key(w, "step_type")?;
    write_step_type(w, snap.step_type)?;

    if !snap.source_refs.is_empty() {
        w.write_all(b",")?;
        write_json_key(w, "source_refs")?;
        write_string_id_array(w, &snap.source_refs)?;
    }

    if let Some(params) = &snap.params {
        w.write_all(b",")?;
        write_json_key(w, "params")?;
        write_step_params(w, params)?;
    }

    if let Some(sig) = snap.signature {
        w.write_all(b",")?;
        write_json_key(w, "signature")?;
        write_u64(w, sig)?;
    }

    w.write_all(b"}")?;
    Ok(())
}

fn write_step_params(w: &mut impl Write, params: &StepParams) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match params {
        StepParams::TableSelectRows { predicate_hash } => {
            write_json_string_lit(w, "table_select_rows")?;
            w.write_all(b",")?;
            write_json_key(w, "predicate_hash")?;
            write_u64(w, *predicate_hash)?;
        }
        StepParams::TableRemoveColumns { columns } => {
            write_json_string_lit(w, "table_remove_columns")?;
            w.write_all(b",")?;
            write_json_key(w, "columns")?;
            write_extracted_string_list(w, columns)?;
        }
        StepParams::TableRenameColumns { renames } => {
            write_json_string_lit(w, "table_rename_columns")?;
            w.write_all(b",")?;
            write_json_key(w, "renames")?;
            write_extracted_rename_pairs(w, renames)?;
        }
        StepParams::TableTransformColumnTypes { transforms } => {
            write_json_string_lit(w, "table_transform_column_types")?;
            w.write_all(b",")?;
            write_json_key(w, "transforms")?;
            write_extracted_column_type_changes(w, transforms)?;
        }
        StepParams::TableNestedJoin {
            left_keys,
            right_keys,
            new_column,
            join_kind_hash,
        } => {
            write_json_string_lit(w, "table_nested_join")?;
            w.write_all(b",")?;
            write_json_key(w, "left_keys")?;
            write_extracted_string_list(w, left_keys)?;
            w.write_all(b",")?;
            write_json_key(w, "right_keys")?;
            write_extracted_string_list(w, right_keys)?;
            w.write_all(b",")?;
            write_json_key(w, "new_column")?;
            write_extracted_string(w, new_column)?;
            w.write_all(b",")?;
            write_json_key(w, "join_kind_hash")?;
            write_option_u64(w, *join_kind_hash)?;
        }
        StepParams::TableJoin {
            left_keys,
            right_keys,
            join_kind_hash,
        } => {
            write_json_string_lit(w, "table_join")?;
            w.write_all(b",")?;
            write_json_key(w, "left_keys")?;
            write_extracted_string_list(w, left_keys)?;
            w.write_all(b",")?;
            write_json_key(w, "right_keys")?;
            write_extracted_string_list(w, right_keys)?;
            w.write_all(b",")?;
            write_json_key(w, "join_kind_hash")?;
            write_option_u64(w, *join_kind_hash)?;
        }
        StepParams::Other {
            function_name_hash,
            arity,
            expr_hash,
        } => {
            write_json_string_lit(w, "other")?;
            w.write_all(b",")?;
            write_json_key(w, "function_name_hash")?;
            write_option_u64(w, *function_name_hash)?;
            w.write_all(b",")?;
            write_json_key(w, "arity")?;
            write_option_u32(w, *arity)?;
            w.write_all(b",")?;
            write_json_key(w, "expr_hash")?;
            write_u64(w, *expr_hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_extracted_string(w: &mut impl Write, value: &ExtractedString) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match value {
        ExtractedString::Known { value } => {
            write_json_string_lit(w, "known")?;
            w.write_all(b",")?;
            write_json_key(w, "value")?;
            write_string_id(w, *value)?;
        }
        ExtractedString::Unknown { hash } => {
            write_json_string_lit(w, "unknown")?;
            w.write_all(b",")?;
            write_json_key(w, "hash")?;
            write_u64(w, *hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_extracted_string_list(w: &mut impl Write, value: &ExtractedStringList) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match value {
        ExtractedStringList::Known { values } => {
            write_json_string_lit(w, "known")?;
            w.write_all(b",")?;
            write_json_key(w, "values")?;
            write_string_id_array(w, values)?;
        }
        ExtractedStringList::Unknown { hash } => {
            write_json_string_lit(w, "unknown")?;
            w.write_all(b",")?;
            write_json_key(w, "hash")?;
            write_u64(w, *hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_extracted_rename_pairs(
    w: &mut impl Write,
    value: &ExtractedRenamePairs,
) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match value {
        ExtractedRenamePairs::Known { pairs } => {
            write_json_string_lit(w, "known")?;
            w.write_all(b",")?;
            write_json_key(w, "pairs")?;
            write_rename_pair_array(w, pairs)?;
        }
        ExtractedRenamePairs::Unknown { hash } => {
            write_json_string_lit(w, "unknown")?;
            w.write_all(b",")?;
            write_json_key(w, "hash")?;
            write_u64(w, *hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_rename_pair_array(w: &mut impl Write, pairs: &[RenamePair]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, p) in pairs.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        w.write_all(b"{")?;
        write_json_key(w, "from")?;
        write_string_id(w, p.from)?;
        w.write_all(b",")?;
        write_json_key(w, "to")?;
        write_string_id(w, p.to)?;
        w.write_all(b"}")?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_extracted_column_type_changes(
    w: &mut impl Write,
    value: &ExtractedColumnTypeChanges,
) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match value {
        ExtractedColumnTypeChanges::Known { changes } => {
            write_json_string_lit(w, "known")?;
            w.write_all(b",")?;
            write_json_key(w, "changes")?;
            write_column_type_change_array(w, changes)?;
        }
        ExtractedColumnTypeChanges::Unknown { hash } => {
            write_json_string_lit(w, "unknown")?;
            w.write_all(b",")?;
            write_json_key(w, "hash")?;
            write_u64(w, *hash)?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_column_type_change_array(
    w: &mut impl Write,
    changes: &[ColumnTypeChange],
) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, c) in changes.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        w.write_all(b"{")?;
        write_json_key(w, "column")?;
        write_string_id(w, c.column)?;
        w.write_all(b",")?;
        write_json_key(w, "ty_hash")?;
        write_u64(w, c.ty_hash)?;
        w.write_all(b"}")?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_step_change_array(w: &mut impl Write, changes: &[StepChange]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, c) in changes.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        write_step_change(w, c)?;
    }
    w.write_all(b"]")?;
    Ok(())
}

fn write_step_change(w: &mut impl Write, change: &StepChange) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "kind")?;
    match change {
        StepChange::Renamed { from, to } => {
            write_json_string_lit(w, "renamed")?;
            w.write_all(b",")?;
            write_json_key(w, "from")?;
            write_string_id(w, *from)?;
            w.write_all(b",")?;
            write_json_key(w, "to")?;
            write_string_id(w, *to)?;
        }
        StepChange::SourceRefsChanged { removed, added } => {
            write_json_string_lit(w, "source_refs_changed")?;
            if !removed.is_empty() {
                w.write_all(b",")?;
                write_json_key(w, "removed")?;
                write_string_id_array(w, removed)?;
            }
            if !added.is_empty() {
                w.write_all(b",")?;
                write_json_key(w, "added")?;
                write_string_id_array(w, added)?;
            }
        }
        StepChange::ParamsChanged => {
            write_json_string_lit(w, "params_changed")?;
        }
    }
    w.write_all(b"}")?;
    Ok(())
}

fn write_ast_diff_summary(w: &mut impl Write, summary: &AstDiffSummary) -> io::Result<()> {
    w.write_all(b"{")?;
    write_json_key(w, "mode")?;
    write_ast_diff_mode(w, summary.mode)?;
    w.write_all(b",")?;
    write_json_key(w, "node_count_old")?;
    write_u32(w, summary.node_count_old)?;
    w.write_all(b",")?;
    write_json_key(w, "node_count_new")?;
    write_u32(w, summary.node_count_new)?;
    w.write_all(b",")?;
    write_json_key(w, "inserted")?;
    write_u32(w, summary.inserted)?;
    w.write_all(b",")?;
    write_json_key(w, "deleted")?;
    write_u32(w, summary.deleted)?;
    w.write_all(b",")?;
    write_json_key(w, "updated")?;
    write_u32(w, summary.updated)?;
    w.write_all(b",")?;
    write_json_key(w, "moved")?;
    write_u32(w, summary.moved)?;

    if !summary.move_hints.is_empty() {
        w.write_all(b",")?;
        write_json_key(w, "move_hints")?;
        write_ast_move_hint_array(w, &summary.move_hints)?;
    }

    w.write_all(b"}")?;
    Ok(())
}

fn write_ast_move_hint_array(w: &mut impl Write, hints: &[AstMoveHint]) -> io::Result<()> {
    w.write_all(b"[")?;
    for (i, h) in hints.iter().enumerate() {
        if i != 0 {
            w.write_all(b",")?;
        }
        w.write_all(b"{")?;
        write_json_key(w, "subtree_hash")?;
        write_u64(w, h.subtree_hash)?;
        w.write_all(b",")?;
        write_json_key(w, "from_preorder")?;
        write_u32(w, h.from_preorder)?;
        w.write_all(b",")?;
        write_json_key(w, "to_preorder")?;
        write_u32(w, h.to_preorder)?;
        w.write_all(b",")?;
        write_json_key(w, "subtree_size")?;
        write_u32(w, h.subtree_size)?;
        w.write_all(b"}")?;
    }
    w.write_all(b"]")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffReport, SheetId};
    use crate::string_pool::StringPool;
    use crate::workbook::CellSnapshot;

    fn sid(v: u32) -> StringId {
        StringId(v)
    }

    fn sheet(v: u32) -> SheetId {
        sid(v)
    }

    fn snapshot(
        addr: CellAddress,
        value: Option<CellValue>,
        formula: Option<StringId>,
    ) -> CellSnapshot {
        CellSnapshot {
            addr,
            value,
            formula,
        }
    }

    fn sample_ops() -> Vec<DiffOp> {
        let addr = CellAddress::from_indices(0, 0);
        vec![
            DiffOp::SheetAdded { sheet: sheet(1) },
            DiffOp::SheetRemoved { sheet: sheet(2) },
            DiffOp::SheetRenamed {
                sheet: sheet(3),
                from: sheet(4),
                to: sheet(5),
            },
            DiffOp::RowAdded {
                sheet: sheet(1),
                row_idx: 42,
                row_signature: Some(RowSignature {
                    hash: 0x0123456789abcdef0123456789abcdef,
                }),
            },
            DiffOp::RowRemoved {
                sheet: sheet(1),
                row_idx: 7,
                row_signature: None,
            },
            DiffOp::DuplicateKeyCluster {
                sheet: sheet(9),
                key: vec![
                    Some(CellValue::Number(1.0)),
                    Some(CellValue::Text(sid(10))),
                    None,
                    Some(CellValue::Blank),
                ],
                left_rows: vec![1, 2, 3],
                right_rows: vec![4, 5],
            },
            DiffOp::RowReplaced {
                sheet: sheet(1),
                row_idx: 123,
            },
            DiffOp::ColumnAdded {
                sheet: sheet(1),
                col_idx: 2,
                col_signature: Some(ColSignature {
                    hash: 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,
                }),
            },
            DiffOp::ColumnRemoved {
                sheet: sheet(1),
                col_idx: 3,
                col_signature: None,
            },
            DiffOp::BlockMovedRows {
                sheet: sheet(1),
                src_start_row: 10,
                row_count: 3,
                dst_start_row: 20,
                block_hash: Some(123456789),
            },
            DiffOp::BlockMovedColumns {
                sheet: sheet(1),
                src_start_col: 10,
                col_count: 3,
                dst_start_col: 20,
                block_hash: None,
            },
            DiffOp::BlockMovedRect {
                sheet: sheet(1),
                src_start_row: 1,
                src_row_count: 2,
                src_start_col: 3,
                src_col_count: 4,
                dst_start_row: 5,
                dst_start_col: 6,
                block_hash: Some(0xdeadbeef),
            },
            DiffOp::RectReplaced {
                sheet: sheet(1),
                start_row: 1,
                row_count: 2,
                start_col: 3,
                col_count: 4,
            },
            DiffOp::CellEdited {
                sheet: sheet(1),
                addr,
                from: snapshot(addr, Some(CellValue::Number(1.5)), Some(sid(99))),
                to: snapshot(addr, Some(CellValue::Bool(true)), None),
                formula_diff: FormulaDiffResult::SemanticChange,
            },
            DiffOp::VbaModuleAdded { name: sid(1) },
            DiffOp::VbaModuleRemoved { name: sid(2) },
            DiffOp::VbaModuleChanged { name: sid(3) },
            DiffOp::NamedRangeAdded { name: sid(11) },
            DiffOp::NamedRangeRemoved { name: sid(12) },
            DiffOp::NamedRangeChanged {
                name: sid(13),
                old_ref: sid(14),
                new_ref: sid(15),
            },
            DiffOp::ChartAdded {
                sheet: sid(16),
                name: sid(17),
            },
            DiffOp::ChartRemoved {
                sheet: sid(18),
                name: sid(19),
            },
            DiffOp::ChartChanged {
                sheet: sid(20),
                name: sid(21),
            },
            DiffOp::QueryAdded { name: sid(22) },
            DiffOp::QueryRemoved { name: sid(23) },
            DiffOp::QueryRenamed {
                from: sid(24),
                to: sid(25),
            },
            DiffOp::QueryDefinitionChanged {
                name: sid(26),
                change_kind: QueryChangeKind::FormattingOnly,
                old_hash: 123,
                new_hash: 456,
                semantic_detail: Some(QuerySemanticDetail {
                    step_diffs: vec![
                        StepDiff::StepAdded {
                            step: StepSnapshot {
                                name: sid(30),
                                index: 0,
                                step_type: StepType::Other,
                                source_refs: vec![sid(31)],
                                params: Some(StepParams::Other {
                                    function_name_hash: Some(1),
                                    arity: Some(2),
                                    expr_hash: 3,
                                }),
                                signature: Some(0xabc),
                            },
                        },
                        StepDiff::StepModified {
                            before: StepSnapshot {
                                name: sid(32),
                                index: 1,
                                step_type: StepType::TableJoin,
                                source_refs: Vec::new(),
                                params: None,
                                signature: None,
                            },
                            after: StepSnapshot {
                                name: sid(33),
                                index: 1,
                                step_type: StepType::TableJoin,
                                source_refs: Vec::new(),
                                params: Some(StepParams::TableJoin {
                                    left_keys: ExtractedStringList::Unknown { hash: 9 },
                                    right_keys: ExtractedStringList::Known {
                                        values: vec![sid(40)],
                                    },
                                    join_kind_hash: None,
                                }),
                                signature: None,
                            },
                            changes: vec![
                                StepChange::Renamed {
                                    from: sid(50),
                                    to: sid(51),
                                },
                                StepChange::SourceRefsChanged {
                                    removed: vec![sid(52)],
                                    added: Vec::new(),
                                },
                                StepChange::ParamsChanged,
                            ],
                        },
                    ],
                    ast_summary: Some(AstDiffSummary {
                        mode: AstDiffMode::SmallExact,
                        node_count_old: 1,
                        node_count_new: 2,
                        inserted: 3,
                        deleted: 4,
                        updated: 5,
                        moved: 6,
                        move_hints: vec![AstMoveHint {
                            subtree_hash: 7,
                            from_preorder: 8,
                            to_preorder: 9,
                            subtree_size: 10,
                        }],
                    }),
                }),
            },
            DiffOp::QueryMetadataChanged {
                name: sid(27),
                field: QueryMetadataField::GroupPath,
                old: Some(sid(28)),
                new: None,
            },
        ]
    }

    #[test]
    fn jsonl_header_parity_matches_serde_json_as_value() {
        let mut pool = StringPool::new();
        pool.intern("a");
        pool.intern("line\nbreak");
        pool.intern("quote: \"");

        let header = serde_json::json!({
            "kind": "Header",
            "version": DiffReport::SCHEMA_VERSION,
            "strings": pool.strings(),
        });

        let serde = serde_json::to_vec(&header).unwrap();
        let mut custom = Vec::new();
        write_jsonl_header(&mut custom, DiffReport::SCHEMA_VERSION, pool.strings()).unwrap();

        let serde_val: serde_json::Value = serde_json::from_slice(&serde).unwrap();
        let custom_val: serde_json::Value = serde_json::from_slice(&custom).unwrap();
        assert_eq!(serde_val, custom_val);
    }

    #[test]
    fn diff_op_writer_matches_serde_json_as_value_for_sample_corpus() {
        for op in sample_ops() {
            let serde = serde_json::to_vec(&op).unwrap();
            let mut custom = Vec::new();
            write_diff_op(&mut custom, &op).unwrap();

            let serde_val: serde_json::Value = serde_json::from_slice(&serde).unwrap();
            let custom_val: serde_json::Value = serde_json::from_slice(&custom).unwrap();
            assert_eq!(serde_val, custom_val);
        }
    }
}
