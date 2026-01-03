use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::config::{DiffConfig, SemanticNoisePolicy};
use crate::dax;
use crate::diff::{DiffOp, ExpressionChangeKind, ModelColumnProperty, RelationshipProperty};
use crate::hashing::XXH64_SEED;
use crate::model::{Measure, Model, ModelColumn, ModelRelationship, ModelTable};
use crate::string_pool::{StringId, StringPool};

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDiffResult {
    pub ops: Vec<DiffOp>,
    pub complete: bool,
    pub warnings: Vec<String>,
}

impl ModelDiffResult {
    fn new(ops: Vec<DiffOp>) -> Self {
        Self {
            ops,
            complete: true,
            warnings: Vec::new(),
        }
    }
}

struct OpEmitter {
    ops: Vec<DiffOp>,
    max_ops: Option<usize>,
    truncated: bool,
}

impl OpEmitter {
    fn new(max_ops: Option<usize>) -> Self {
        Self {
            ops: Vec::new(),
            max_ops,
            truncated: false,
        }
    }

    fn push(&mut self, op: DiffOp) {
        if self.truncated {
            return;
        }
        if let Some(max) = self.max_ops {
            if self.ops.len() >= max {
                self.truncated = true;
                return;
            }
        }
        self.ops.push(op);
    }
}

/// Diff two tabular models (tables/columns/relationships/measures).
pub fn diff_models(
    old: &Model,
    new: &Model,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> ModelDiffResult {
    let mut emitter = OpEmitter::new(config.hardening.max_ops);

    diff_tables(old, new, pool, config, &mut emitter);
    if !emitter.truncated {
        diff_relationships(old, new, pool, &mut emitter);
    }
    if !emitter.truncated {
        diff_measures(old, new, pool, config, &mut emitter);
    }

    let mut result = ModelDiffResult::new(emitter.ops);
    if emitter.truncated {
        result.complete = false;
        let limit = config.hardening.max_ops.unwrap_or(0);
        result.warnings.push(format!(
            "model-diff: max ops limit ({}) reached; model ops truncated",
            limit
        ));
    }
    result
}

fn diff_tables(
    old: &Model,
    new: &Model,
    pool: &mut StringPool,
    config: &DiffConfig,
    emitter: &mut OpEmitter,
) {
    let old_tables = map_tables(&old.tables, pool);
    let new_tables = map_tables(&new.tables, pool);

    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.extend(old_tables.keys().cloned());
    keys.extend(new_tables.keys().cloned());

    for key in keys {
        if emitter.truncated {
            return;
        }
        match (old_tables.get(&key), new_tables.get(&key)) {
            (None, Some(new_table)) => {
                emitter.push(DiffOp::TableAdded {
                    name: new_table.name,
                });
            }
            (Some(old_table), None) => {
                emitter.push(DiffOp::TableRemoved {
                    name: old_table.name,
                });
            }
            (Some(old_table), Some(new_table)) => {
                diff_columns(old_table, new_table, pool, config, emitter);
            }
            (None, None) => {}
        }
    }
}

fn diff_columns(
    old_table: &ModelTable,
    new_table: &ModelTable,
    pool: &mut StringPool,
    config: &DiffConfig,
    emitter: &mut OpEmitter,
) {
    let old_cols = map_columns(&old_table.columns, pool);
    let new_cols = map_columns(&new_table.columns, pool);
    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.extend(old_cols.keys().cloned());
    keys.extend(new_cols.keys().cloned());

    let table_id = new_table.name;

    for key in keys {
        if emitter.truncated {
            return;
        }
        match (old_cols.get(&key), new_cols.get(&key)) {
            (None, Some(new_col)) => {
                emitter.push(DiffOp::ModelColumnAdded {
                    table: table_id,
                    name: new_col.name,
                    data_type: new_col.data_type,
                });
            }
            (Some(old_col), None) => {
                emitter.push(DiffOp::ModelColumnRemoved {
                    table: table_id,
                    name: old_col.name,
                });
            }
            (Some(old_col), Some(new_col)) => {
                diff_column_properties(table_id, old_col, new_col, pool, config, emitter);
            }
            (None, None) => {}
        }
    }
}

fn diff_column_properties(
    table_id: StringId,
    old_col: &ModelColumn,
    new_col: &ModelColumn,
    pool: &mut StringPool,
    config: &DiffConfig,
    emitter: &mut OpEmitter,
) {
    if old_col.data_type != new_col.data_type {
        emitter.push(DiffOp::ModelColumnTypeChanged {
            table: table_id,
            name: new_col.name,
            old_type: old_col.data_type,
            new_type: new_col.data_type,
        });
    }

    if old_col.is_hidden != new_col.is_hidden {
        emitter.push(DiffOp::ModelColumnPropertyChanged {
            table: table_id,
            name: new_col.name,
            field: ModelColumnProperty::Hidden,
            old: old_col.is_hidden.map(|v| intern_bool(pool, v)),
            new: new_col.is_hidden.map(|v| intern_bool(pool, v)),
        });
    }

    if old_col.format_string != new_col.format_string {
        emitter.push(DiffOp::ModelColumnPropertyChanged {
            table: table_id,
            name: new_col.name,
            field: ModelColumnProperty::FormatString,
            old: old_col.format_string,
            new: new_col.format_string,
        });
    }

    if old_col.sort_by != new_col.sort_by {
        emitter.push(DiffOp::ModelColumnPropertyChanged {
            table: table_id,
            name: new_col.name,
            field: ModelColumnProperty::SortBy,
            old: old_col.sort_by,
            new: new_col.sort_by,
        });
    }

    if old_col.summarize_by != new_col.summarize_by {
        emitter.push(DiffOp::ModelColumnPropertyChanged {
            table: table_id,
            name: new_col.name,
            field: ModelColumnProperty::SummarizeBy,
            old: old_col.summarize_by,
            new: new_col.summarize_by,
        });
    }

    if emitter.truncated {
        return;
    }

    if let Some((kind, old_hash, new_hash)) =
        column_expression_change(old_col.expression, new_col.expression, pool, config)
    {
        emitter.push(DiffOp::CalculatedColumnDefinitionChanged {
            table: table_id,
            name: new_col.name,
            change_kind: kind,
            old_hash,
            new_hash,
        });
    }
}

fn diff_relationships(
    old: &Model,
    new: &Model,
    pool: &mut StringPool,
    emitter: &mut OpEmitter,
) {
    let old_rels = map_relationships(&old.relationships, pool);
    let new_rels = map_relationships(&new.relationships, pool);
    let mut keys: BTreeSet<RelationshipKey> = BTreeSet::new();
    keys.extend(old_rels.keys().cloned());
    keys.extend(new_rels.keys().cloned());

    for key in keys {
        if emitter.truncated {
            return;
        }
        match (old_rels.get(&key), new_rels.get(&key)) {
            (None, Some(new_rel)) => {
                emitter.push(DiffOp::RelationshipAdded {
                    from_table: new_rel.from_table,
                    from_column: new_rel.from_column,
                    to_table: new_rel.to_table,
                    to_column: new_rel.to_column,
                });
            }
            (Some(old_rel), None) => {
                emitter.push(DiffOp::RelationshipRemoved {
                    from_table: old_rel.from_table,
                    from_column: old_rel.from_column,
                    to_table: old_rel.to_table,
                    to_column: old_rel.to_column,
                });
            }
            (Some(old_rel), Some(new_rel)) => {
                if old_rel.cross_filtering_behavior != new_rel.cross_filtering_behavior {
                    emitter.push(DiffOp::RelationshipPropertyChanged {
                        from_table: new_rel.from_table,
                        from_column: new_rel.from_column,
                        to_table: new_rel.to_table,
                        to_column: new_rel.to_column,
                        field: RelationshipProperty::CrossFilteringBehavior,
                        old: old_rel.cross_filtering_behavior,
                        new: new_rel.cross_filtering_behavior,
                    });
                }

                if old_rel.cardinality != new_rel.cardinality {
                    emitter.push(DiffOp::RelationshipPropertyChanged {
                        from_table: new_rel.from_table,
                        from_column: new_rel.from_column,
                        to_table: new_rel.to_table,
                        to_column: new_rel.to_column,
                        field: RelationshipProperty::Cardinality,
                        old: old_rel.cardinality,
                        new: new_rel.cardinality,
                    });
                }

                if old_rel.is_active != new_rel.is_active {
                    emitter.push(DiffOp::RelationshipPropertyChanged {
                        from_table: new_rel.from_table,
                        from_column: new_rel.from_column,
                        to_table: new_rel.to_table,
                        to_column: new_rel.to_column,
                        field: RelationshipProperty::IsActive,
                        old: old_rel.is_active.map(|v| intern_bool(pool, v)),
                        new: new_rel.is_active.map(|v| intern_bool(pool, v)),
                    });
                }
            }
            (None, None) => {}
        }
    }
}

fn diff_measures(
    old: &Model,
    new: &Model,
    pool: &mut StringPool,
    config: &DiffConfig,
    emitter: &mut OpEmitter,
) {
    let old_measures = map_measures(&old.measures, pool);
    let new_measures = map_measures(&new.measures, pool);
    let mut keys: BTreeSet<String> = BTreeSet::new();
    keys.extend(old_measures.keys().cloned());
    keys.extend(new_measures.keys().cloned());

    for key in keys {
        if emitter.truncated {
            return;
        }
        match (old_measures.get(&key), new_measures.get(&key)) {
            (Some(old_measure), None) => {
                emitter.push(DiffOp::MeasureRemoved {
                    name: old_measure.name,
                });
            }
            (None, Some(new_measure)) => {
                emitter.push(DiffOp::MeasureAdded {
                    name: new_measure.name,
                });
            }
            (Some(old_measure), Some(new_measure)) => {
                let old_expr = pool.resolve(old_measure.expression);
                let new_expr = pool.resolve(new_measure.expression);
                if let Some((kind, old_hash, new_hash)) =
                    expression_change(old_expr, new_expr, config)
                {
                    emitter.push(DiffOp::MeasureDefinitionChanged {
                        name: new_measure.name,
                        change_kind: kind,
                        old_hash,
                        new_hash,
                    });
                }
            }
            (None, None) => {}
        }
    }
}

fn column_expression_change(
    old_expr: Option<StringId>,
    new_expr: Option<StringId>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Option<(ExpressionChangeKind, u64, u64)> {
    match (old_expr, new_expr) {
        (None, None) => None,
        (Some(old_id), Some(new_id)) => {
            let old_expr = pool.resolve(old_id);
            let new_expr = pool.resolve(new_id);
            expression_change(old_expr, new_expr, config)
        }
        (Some(old_id), None) => {
            let old_expr = pool.resolve(old_id);
            let old_hash = hash64(&old_expr);
            let new_hash = hash64(&"");
            Some((ExpressionChangeKind::Semantic, old_hash, new_hash))
        }
        (None, Some(new_id)) => {
            let new_expr = pool.resolve(new_id);
            let old_hash = hash64(&"");
            let new_hash = hash64(&new_expr);
            Some((ExpressionChangeKind::Semantic, old_hash, new_hash))
        }
    }
}

fn expression_change(
    old_expr: &str,
    new_expr: &str,
    config: &DiffConfig,
) -> Option<(ExpressionChangeKind, u64, u64)> {
    if old_expr == new_expr {
        return None;
    }

    if config.semantic.enable_dax_semantic_diff {
        if let (Ok(old_h), Ok(new_h)) = (dax::semantic_hash(old_expr), dax::semantic_hash(new_expr)) {
            let kind = if old_h == new_h {
                ExpressionChangeKind::FormattingOnly
            } else {
                ExpressionChangeKind::Semantic
            };
            if kind == ExpressionChangeKind::FormattingOnly
                && matches!(config.semantic.semantic_noise_policy, SemanticNoisePolicy::SuppressFormattingOnly)
            {
                return None;
            }
            return Some((kind, old_h, new_h));
        }
    }

    let old_h = hash64(&old_expr);
    let new_h = hash64(&new_expr);
    Some((ExpressionChangeKind::Unknown, old_h, new_h))
}

fn map_tables<'a>(tables: &'a [ModelTable], pool: &StringPool) -> BTreeMap<String, &'a ModelTable> {
    let mut out = BTreeMap::new();
    for table in tables {
        let key = pool.resolve(table.name).to_lowercase();
        out.entry(key).or_insert(table);
    }
    out
}

fn map_columns<'a>(
    cols: &'a [ModelColumn],
    pool: &StringPool,
) -> BTreeMap<String, &'a ModelColumn> {
    let mut out = BTreeMap::new();
    for col in cols {
        let key = pool.resolve(col.name).to_lowercase();
        out.entry(key).or_insert(col);
    }
    out
}

fn map_measures<'a>(
    measures: &'a [Measure],
    pool: &StringPool,
) -> BTreeMap<String, &'a Measure> {
    let mut out = BTreeMap::new();
    for measure in measures {
        let key = pool.resolve(measure.name).to_lowercase();
        out.entry(key).or_insert(measure);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RelationshipKey {
    from_table: String,
    from_column: String,
    to_table: String,
    to_column: String,
}

fn map_relationships<'a>(
    relationships: &'a [ModelRelationship],
    pool: &StringPool,
) -> BTreeMap<RelationshipKey, &'a ModelRelationship> {
    let mut out = BTreeMap::new();
    for rel in relationships {
        let key = RelationshipKey {
            from_table: pool.resolve(rel.from_table).to_lowercase(),
            from_column: pool.resolve(rel.from_column).to_lowercase(),
            to_table: pool.resolve(rel.to_table).to_lowercase(),
            to_column: pool.resolve(rel.to_column).to_lowercase(),
        };
        out.entry(key).or_insert(rel);
    }
    out
}

fn intern_bool(pool: &mut StringPool, v: bool) -> StringId {
    if v {
        pool.intern("true")
    } else {
        pool.intern("false")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_measure_model(pool: &mut StringPool, expr: &str) -> Model {
        let name = pool.intern("Sales/Total");
        let expr_id = pool.intern(expr);
        Model {
            measures: vec![Measure {
                name,
                expression: expr_id,
            }],
            ..Default::default()
        }
    }

    fn make_calc_column_model(pool: &mut StringPool, expr: &str) -> Model {
        let table_name = pool.intern("Sales");
        let column_name = pool.intern("Calc");
        let expr_id = pool.intern(expr);
        let column = ModelColumn {
            name: column_name,
            data_type: None,
            is_hidden: None,
            format_string: None,
            sort_by: None,
            summarize_by: None,
            expression: Some(expr_id),
        };
        let table = ModelTable {
            name: table_name,
            columns: vec![column],
        };
        Model {
            tables: vec![table],
            ..Default::default()
        }
    }

    #[test]
    fn dax_formatting_only_suppressed_when_configured() {
        let mut pool = StringPool::new();
        let old_model = make_measure_model(&mut pool, "SUM( Sales[Amount] )");
        let new_model = make_measure_model(&mut pool, "sum(sales[amount]) // comment");

        let mut config = DiffConfig::default();
        config.semantic.enable_dax_semantic_diff = true;
        config.semantic.semantic_noise_policy = SemanticNoisePolicy::SuppressFormattingOnly;

        let result = diff_models(&old_model, &new_model, &mut pool, &config);
        assert!(result.ops.is_empty(), "formatting-only change should be suppressed");
        assert!(result.complete);
    }

    #[test]
    fn dax_semantic_change_reports_change_kind() {
        let mut pool = StringPool::new();
        let old_model = make_measure_model(&mut pool, "SUM(Sales[Amount])");
        let new_model = make_measure_model(&mut pool, "SUM(Sales[Net])");

        let mut config = DiffConfig::default();
        config.semantic.enable_dax_semantic_diff = true;

        let result = diff_models(&old_model, &new_model, &mut pool, &config);
        assert_eq!(result.ops.len(), 1);
        match &result.ops[0] {
            DiffOp::MeasureDefinitionChanged { change_kind, .. } => {
                assert_eq!(*change_kind, ExpressionChangeKind::Semantic);
            }
            other => panic!("unexpected op: {:?}", other),
        }
    }

    #[test]
    fn dax_formatting_only_column_change_is_classified() {
        let mut pool = StringPool::new();
        let old_model = make_calc_column_model(&mut pool, "1+2");
        let new_model = make_calc_column_model(&mut pool, "1 + 2");

        let mut config = DiffConfig::default();
        config.semantic.enable_dax_semantic_diff = true;

        let result = diff_models(&old_model, &new_model, &mut pool, &config);
        assert_eq!(result.ops.len(), 1);
        match &result.ops[0] {
            DiffOp::CalculatedColumnDefinitionChanged { change_kind, .. } => {
                assert_eq!(*change_kind, ExpressionChangeKind::FormattingOnly);
            }
            other => panic!("unexpected op: {:?}", other),
        }
    }

    #[test]
    fn dax_parse_failure_sets_unknown_change_kind() {
        let mut pool = StringPool::new();
        let old_model = make_measure_model(&mut pool, "SUM(");
        let new_model = make_measure_model(&mut pool, "SUMX(");

        let mut config = DiffConfig::default();
        config.semantic.enable_dax_semantic_diff = true;

        let result = diff_models(&old_model, &new_model, &mut pool, &config);
        assert_eq!(result.ops.len(), 1);
        match &result.ops[0] {
            DiffOp::MeasureDefinitionChanged { change_kind, .. } => {
                assert_eq!(*change_kind, ExpressionChangeKind::Unknown);
            }
            other => panic!("unexpected op: {:?}", other),
        }
    }
}
