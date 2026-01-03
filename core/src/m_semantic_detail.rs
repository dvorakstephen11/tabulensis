use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::diff::{
    ColumnTypeChange, ExtractedColumnTypeChanges, ExtractedRenamePairs, ExtractedString,
    ExtractedStringList, QuerySemanticDetail, RenamePair, StepChange, StepDiff, StepParams,
    StepSnapshot, StepType,
};
#[cfg(test)]
use crate::diff::AstDiffMode;
use crate::m_ast::{
    canonicalize_m_ast, extract_steps, parse_m_expression, MStep, StepColumnTypeChange,
    StepExtracted, StepKind, StepRenamePair,
};
use crate::m_ast_diff;
use crate::string_pool::{StringId, StringPool};

pub(crate) fn build_query_semantic_detail(
    old_expr: &str,
    new_expr: &str,
    pool: &mut StringPool,
) -> Option<QuerySemanticDetail> {
    let mut detail = QuerySemanticDetail {
        step_diffs: Vec::new(),
        ast_summary: None,
    };

    let old_steps = extract_steps(old_expr);
    let new_steps = extract_steps(new_expr);

    if let (Some(oldp), Some(newp)) = (old_steps, new_steps) {
        detail.step_diffs = diff_step_pipelines(&oldp.steps, &newp.steps, pool);
        if !detail.step_diffs.is_empty() {
            return Some(detail);
        }
    }

    let mut old_ast = parse_m_expression(old_expr).ok()?;
    let mut new_ast = parse_m_expression(new_expr).ok()?;
    canonicalize_m_ast(&mut old_ast);
    canonicalize_m_ast(&mut new_ast);

    detail.ast_summary = Some(m_ast_diff::diff_summary(&old_ast, &new_ast));
    Some(detail)
}

fn diff_step_pipelines(
    old_steps: &[MStep],
    new_steps: &[MStep],
    pool: &mut StringPool,
) -> Vec<StepDiff> {
    let matches = align_steps(old_steps, new_steps);

    let mut out = Vec::new();

    let mut matched_old: HashSet<usize> = HashSet::new();
    let mut matched_new: HashSet<usize> = HashSet::new();
    for (oi, ni) in &matches {
        matched_old.insert(*oi);
        matched_new.insert(*ni);
    }

    for (oi, s) in old_steps.iter().enumerate() {
        if matched_old.contains(&oi) {
            continue;
        }
        out.push(StepDiff::StepRemoved {
            step: snapshot_step(s, oi as u32, pool),
        });
    }

    for (ni, s) in new_steps.iter().enumerate() {
        if matched_new.contains(&ni) {
            continue;
        }
        out.push(StepDiff::StepAdded {
            step: snapshot_step(s, ni as u32, pool),
        });
    }

    for (oi, ni) in matches {
        let a = &old_steps[oi];
        let b = &new_steps[ni];

        let renamed = a.name != b.name;
        let reordered = oi != ni;

        let params_a = step_params(&a.kind, pool);
        let params_b = step_params(&b.kind, pool);

        let mut changes = Vec::new();
        if renamed {
            changes.push(StepChange::Renamed {
                from: pool.intern(&a.name),
                to: pool.intern(&b.name),
            });
        }

        let (src_removed, src_added) = diff_string_sets(&a.source_refs, &b.source_refs, pool);
        let has_source_change = !src_removed.is_empty() || !src_added.is_empty();
        if has_source_change {
            changes.push(StepChange::SourceRefsChanged {
                removed: src_removed,
                added: src_added,
            });
        }

        let same_sig = a.signature == b.signature;
        if !same_sig || params_a != params_b {
            changes.push(StepChange::ParamsChanged);
        }

        if renamed || !same_sig || has_source_change || params_a != params_b {
            out.push(StepDiff::StepModified {
                before: snapshot_step_with_params(a, oi as u32, params_a, pool),
                after: snapshot_step_with_params(b, ni as u32, params_b, pool),
                changes,
            });
        } else if reordered {
            out.push(StepDiff::StepReordered {
                name: pool.intern(&a.name),
                from_index: oi as u32,
                to_index: ni as u32,
            });
        }
    }

    out.sort_by_key(step_diff_sort_key);
    out
}

fn step_diff_sort_key(d: &StepDiff) -> u32 {
    match d {
        StepDiff::StepAdded { step } => step.index,
        StepDiff::StepRemoved { step } => step.index,
        StepDiff::StepReordered { to_index, .. } => *to_index,
        StepDiff::StepModified { after, .. } => after.index,
    }
}

fn align_steps(old_steps: &[MStep], new_steps: &[MStep]) -> Vec<(usize, usize)> {
    let mut matches = align_steps_dp(old_steps, new_steps);

    let mut matched_old = vec![false; old_steps.len()];
    let mut matched_new = vec![false; new_steps.len()];
    for (oi, ni) in &matches {
        matched_old[*oi] = true;
        matched_new[*ni] = true;
    }

    let mut extra = match_unordered_steps(old_steps, new_steps, &matched_old, &matched_new);
    for (oi, ni) in &extra {
        matched_old[*oi] = true;
        matched_new[*ni] = true;
    }

    matches.append(&mut extra);
    matches.sort_by_key(|(oi, _)| *oi);
    matches
}

#[derive(Clone, Copy)]
enum AlignOp {
    Match,
    Insert,
    Delete,
    None,
}

#[derive(Clone, Copy)]
struct AlignCell {
    cost: u32,
    matches: u32,
    op: AlignOp,
}

fn align_steps_dp(old_steps: &[MStep], new_steps: &[MStep]) -> Vec<(usize, usize)> {
    let m = old_steps.len();
    let n = new_steps.len();
    if m == 0 || n == 0 {
        return Vec::new();
    }

    const COST_INSERT: u32 = 1;
    const COST_DELETE: u32 = 1;

    let idx = |i: usize, j: usize, n: usize| -> usize { i * (n + 1) + j };
    let mut dp = vec![
        AlignCell {
            cost: 0,
            matches: 0,
            op: AlignOp::None,
        };
        (m + 1) * (n + 1)
    ];

    for i in 1..=m {
        let prev = dp[idx(i - 1, 0, n)];
        dp[idx(i, 0, n)] = AlignCell {
            cost: prev.cost + COST_DELETE,
            matches: prev.matches,
            op: AlignOp::Delete,
        };
    }
    for j in 1..=n {
        let prev = dp[idx(0, j - 1, n)];
        dp[idx(0, j, n)] = AlignCell {
            cost: prev.cost + COST_INSERT,
            matches: prev.matches,
            op: AlignOp::Insert,
        };
    }

    for i in 1..=m {
        for j in 1..=n {
            let mut best = dp[idx(i - 1, j, n)];
            best.cost = best.cost.saturating_add(COST_DELETE);
            best.op = AlignOp::Delete;

            let ins = {
                let prev = dp[idx(i, j - 1, n)];
                AlignCell {
                    cost: prev.cost.saturating_add(COST_INSERT),
                    matches: prev.matches,
                    op: AlignOp::Insert,
                }
            };
            if is_better(ins, best) {
                best = ins;
            }

            if let Some(match_cost) = step_match_cost(&old_steps[i - 1], &new_steps[j - 1]) {
                let prev = dp[idx(i - 1, j - 1, n)];
                let mat = AlignCell {
                    cost: prev.cost.saturating_add(match_cost),
                    matches: prev.matches.saturating_add(1),
                    op: AlignOp::Match,
                };
                if is_better(mat, best) {
                    best = mat;
                }
            }

            dp[idx(i, j, n)] = best;
        }
    }

    let mut out = Vec::new();
    let mut i = m;
    let mut j = n;
    while i > 0 || j > 0 {
        match dp[idx(i, j, n)].op {
            AlignOp::Match => {
                out.push((i - 1, j - 1));
                i -= 1;
                j -= 1;
            }
            AlignOp::Delete => {
                i = i.saturating_sub(1);
            }
            AlignOp::Insert => {
                j = j.saturating_sub(1);
            }
            AlignOp::None => break,
        }
    }

    out.reverse();
    out
}

fn is_better(a: AlignCell, b: AlignCell) -> bool {
    if a.cost != b.cost {
        return a.cost < b.cost;
    }
    if a.matches != b.matches {
        return a.matches > b.matches;
    }
    align_op_priority(a.op) > align_op_priority(b.op)
}

fn align_op_priority(op: AlignOp) -> u8 {
    match op {
        AlignOp::Match => 2,
        AlignOp::Delete => 1,
        AlignOp::Insert => 0,
        AlignOp::None => 0,
    }
}

fn match_unordered_steps(
    old_steps: &[MStep],
    new_steps: &[MStep],
    matched_old: &[bool],
    matched_new: &[bool],
) -> Vec<(usize, usize)> {
    let mut extra = Vec::new();

    let mut sig_old: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut sig_new: HashMap<u64, Vec<usize>> = HashMap::new();

    for (idx, step) in old_steps.iter().enumerate() {
        if matched_old.get(idx).copied().unwrap_or(false) {
            continue;
        }
        sig_old.entry(step.signature).or_default().push(idx);
    }
    for (idx, step) in new_steps.iter().enumerate() {
        if matched_new.get(idx).copied().unwrap_or(false) {
            continue;
        }
        sig_new.entry(step.signature).or_default().push(idx);
    }

    let mut used_old = HashSet::new();
    let mut used_new = HashSet::new();

    for (sig, old_idxs) in &sig_old {
        let Some(new_idxs) = sig_new.get(sig) else {
            continue;
        };
        if old_idxs.len() == 1 && new_idxs.len() == 1 {
            let oi = old_idxs[0];
            let ni = new_idxs[0];
            extra.push((oi, ni));
            used_old.insert(oi);
            used_new.insert(ni);
        }
    }

    let mut name_old: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut name_new: HashMap<&str, Vec<usize>> = HashMap::new();

    for (idx, step) in old_steps.iter().enumerate() {
        if matched_old.get(idx).copied().unwrap_or(false) || used_old.contains(&idx) {
            continue;
        }
        name_old.entry(step.name.as_str()).or_default().push(idx);
    }
    for (idx, step) in new_steps.iter().enumerate() {
        if matched_new.get(idx).copied().unwrap_or(false) || used_new.contains(&idx) {
            continue;
        }
        name_new.entry(step.name.as_str()).or_default().push(idx);
    }

    for (name, old_idxs) in name_old {
        let Some(new_idxs) = name_new.get(name) else {
            continue;
        };
        if old_idxs.len() == 1 && new_idxs.len() == 1 {
            extra.push((old_idxs[0], new_idxs[0]));
        }
    }

    extra
}

fn step_match_cost(a: &MStep, b: &MStep) -> Option<u32> {
    let kind_a = step_kind_id(&a.kind);
    let kind_b = step_kind_id(&b.kind);

    if kind_a == kind_b {
        let key_a = step_key_hash(&a.kind);
        let key_b = step_key_hash(&b.kind);
        if key_a == key_b {
            return Some(0);
        }
        return Some(1);
    }

    if a.name == b.name {
        return Some(2);
    }

    None
}

fn step_kind_id(kind: &StepKind) -> u8 {
    match kind {
        StepKind::TableSelectRows { .. } => 1,
        StepKind::TableRemoveColumns { .. } => 2,
        StepKind::TableRenameColumns { .. } => 3,
        StepKind::TableTransformColumnTypes { .. } => 4,
        StepKind::TableNestedJoin { .. } => 5,
        StepKind::TableJoin { .. } => 6,
        StepKind::Other { .. } => 7,
    }
}

fn step_key_hash(kind: &StepKind) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(crate::hashing::XXH64_SEED);
    step_kind_id(kind).hash(&mut h);
    match kind {
        StepKind::TableSelectRows { predicate_hash, .. } => {
            predicate_hash.hash(&mut h);
        }
        StepKind::TableRemoveColumns { columns, .. } => {
            columns.hash(&mut h);
        }
        StepKind::TableRenameColumns { renames, .. } => {
            renames.hash(&mut h);
        }
        StepKind::TableTransformColumnTypes { transforms, .. } => {
            transforms.hash(&mut h);
        }
        StepKind::TableNestedJoin {
            left_keys,
            right_keys,
            new_column,
            join_kind_hash,
            ..
        } => {
            left_keys.hash(&mut h);
            right_keys.hash(&mut h);
            new_column.hash(&mut h);
            join_kind_hash.hash(&mut h);
        }
        StepKind::TableJoin {
            left_keys,
            right_keys,
            join_kind_hash,
            ..
        } => {
            left_keys.hash(&mut h);
            right_keys.hash(&mut h);
            join_kind_hash.hash(&mut h);
        }
        StepKind::Other {
            function_name_hash,
            arity,
            ..
        } => {
            function_name_hash.hash(&mut h);
            arity.hash(&mut h);
        }
    }
    h.finish()
}

fn snapshot_step(s: &MStep, index: u32, pool: &mut StringPool) -> StepSnapshot {
    snapshot_step_with_params(s, index, step_params(&s.kind, pool), pool)
}

fn snapshot_step_with_params(
    s: &MStep,
    index: u32,
    params: Option<StepParams>,
    pool: &mut StringPool,
) -> StepSnapshot {
    let name = pool.intern(&s.name);
    let step_type = step_type(&s.kind);
    let mut source_refs = Vec::with_capacity(s.source_refs.len());
    for r in &s.source_refs {
        source_refs.push(pool.intern(r));
    }

    StepSnapshot {
        name,
        index,
        step_type,
        source_refs,
        params,
        signature: Some(s.signature),
    }
}

fn step_type(k: &StepKind) -> StepType {
    match k {
        StepKind::TableSelectRows { .. } => StepType::TableSelectRows,
        StepKind::TableRemoveColumns { .. } => StepType::TableRemoveColumns,
        StepKind::TableRenameColumns { .. } => StepType::TableRenameColumns,
        StepKind::TableTransformColumnTypes { .. } => StepType::TableTransformColumnTypes,
        StepKind::TableNestedJoin { .. } => StepType::TableNestedJoin,
        StepKind::TableJoin { .. } => StepType::TableJoin,
        StepKind::Other { .. } => StepType::Other,
    }
}

fn step_params(k: &StepKind, pool: &mut StringPool) -> Option<StepParams> {
    match k {
        StepKind::TableSelectRows { predicate_hash, .. } => Some(StepParams::TableSelectRows {
            predicate_hash: *predicate_hash,
        }),
        StepKind::TableRemoveColumns { columns, .. } => Some(StepParams::TableRemoveColumns {
            columns: map_string_list(columns, pool),
        }),
        StepKind::TableRenameColumns { renames, .. } => Some(StepParams::TableRenameColumns {
            renames: map_rename_pairs(renames, pool),
        }),
        StepKind::TableTransformColumnTypes { transforms, .. } => {
            Some(StepParams::TableTransformColumnTypes {
                transforms: map_column_type_changes(transforms, pool),
            })
        }
        StepKind::TableNestedJoin {
            left_keys,
            right_keys,
            new_column,
            join_kind_hash,
            ..
        } => Some(StepParams::TableNestedJoin {
            left_keys: map_string_list(left_keys, pool),
            right_keys: map_string_list(right_keys, pool),
            new_column: map_string(new_column, pool),
            join_kind_hash: *join_kind_hash,
        }),
        StepKind::TableJoin {
            left_keys,
            right_keys,
            join_kind_hash,
            ..
        } => Some(StepParams::TableJoin {
            left_keys: map_string_list(left_keys, pool),
            right_keys: map_string_list(right_keys, pool),
            join_kind_hash: *join_kind_hash,
        }),
        StepKind::Other {
            function_name_hash,
            arity,
            expr_hash,
        } => Some(StepParams::Other {
            function_name_hash: *function_name_hash,
            arity: arity.map(|v| v as u32),
            expr_hash: *expr_hash,
        }),
    }
}

fn map_string(v: &StepExtracted<String>, pool: &mut StringPool) -> ExtractedString {
    match v {
        StepExtracted::Known(value) => ExtractedString::Known {
            value: pool.intern(value),
        },
        StepExtracted::Unknown { hash } => ExtractedString::Unknown { hash: *hash },
    }
}

fn map_string_list(v: &StepExtracted<Vec<String>>, pool: &mut StringPool) -> ExtractedStringList {
    match v {
        StepExtracted::Known(values) => ExtractedStringList::Known {
            values: values.iter().map(|value| pool.intern(value)).collect(),
        },
        StepExtracted::Unknown { hash } => ExtractedStringList::Unknown { hash: *hash },
    }
}

fn map_rename_pairs(
    v: &StepExtracted<Vec<StepRenamePair>>,
    pool: &mut StringPool,
) -> ExtractedRenamePairs {
    match v {
        StepExtracted::Known(pairs) => ExtractedRenamePairs::Known {
            pairs: pairs
                .iter()
                .map(|p| RenamePair {
                    from: pool.intern(&p.from),
                    to: pool.intern(&p.to),
                })
                .collect(),
        },
        StepExtracted::Unknown { hash } => ExtractedRenamePairs::Unknown { hash: *hash },
    }
}

fn map_column_type_changes(
    v: &StepExtracted<Vec<StepColumnTypeChange>>,
    pool: &mut StringPool,
) -> ExtractedColumnTypeChanges {
    match v {
        StepExtracted::Known(changes) => ExtractedColumnTypeChanges::Known {
            changes: changes
                .iter()
                .map(|c| ColumnTypeChange {
                    column: pool.intern(&c.column),
                    ty_hash: c.ty_hash,
                })
                .collect(),
        },
        StepExtracted::Unknown { hash } => ExtractedColumnTypeChanges::Unknown { hash: *hash },
    }
}

fn diff_string_sets(
    old: &[String],
    new: &[String],
    pool: &mut StringPool,
) -> (Vec<StringId>, Vec<StringId>) {
    let old_set: HashSet<&str> = old.iter().map(|v| v.as_str()).collect();
    let new_set: HashSet<&str> = new.iter().map(|v| v.as_str()).collect();

    let removed = old
        .iter()
        .filter(|v| !new_set.contains(v.as_str()))
        .map(|v| pool.intern(v))
        .collect();
    let added = new
        .iter()
        .filter(|v| !old_set.contains(v.as_str()))
        .map(|v| pool.intern(v))
        .collect();

    (removed, added)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::string_pool::StringPool;

    fn detail_for(old_expr: &str, new_expr: &str) -> QuerySemanticDetail {
        let mut pool = StringPool::default();
        build_query_semantic_detail(old_expr, new_expr, &mut pool).expect("detail")
    }

    #[test]
    fn step_param_change_produces_modified_step() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"A", "B"})
            in
                #"Removed Columns"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"A", "C"})
            in
                #"Removed Columns"
        "#;

        let detail = detail_for(old_expr, new_expr);
        assert_eq!(detail.step_diffs.len(), 1);
        match &detail.step_diffs[0] {
            StepDiff::StepModified { after, .. } => {
                assert_eq!(after.step_type, StepType::TableRemoveColumns);
            }
            other => panic!("expected StepModified, got {:?}", other),
        }
    }

    #[test]
    fn step_rename_produces_modified_with_rename_change() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"A", "B"}),
                #"Changed Type" = Table.TransformColumnTypes(#"Removed Columns", {{"C", type text}})
            in
                #"Changed Type"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Dropped Columns" = Table.RemoveColumns(Source, {"A", "B"}),
                #"Changed Type" = Table.TransformColumnTypes(#"Dropped Columns", {{"C", type text}})
            in
                #"Changed Type"
        "#;

        let detail = detail_for(old_expr, new_expr);
        let mut has_rename = false;
        for diff in &detail.step_diffs {
            if let StepDiff::StepModified { changes, .. } = diff {
                if changes.iter().any(|c| matches!(c, StepChange::Renamed { .. })) {
                    has_rename = true;
                }
            }
        }
        assert!(has_rename, "expected StepChange::Renamed");
    }

    #[test]
    fn dependency_change_sets_source_refs_changed() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [A] > 0),
                #"Removed Columns" = Table.RemoveColumns(#"Filtered Rows", {"A"})
            in
                #"Removed Columns"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [A] > 0),
                #"Removed Columns" = Table.RemoveColumns(Source, {"A"})
            in
                #"Removed Columns"
        "#;

        let detail = detail_for(old_expr, new_expr);
        let mut has_source_change = false;
        for diff in &detail.step_diffs {
            if let StepDiff::StepModified { changes, .. } = diff {
                if changes
                    .iter()
                    .any(|c| matches!(c, StepChange::SourceRefsChanged { .. }))
                {
                    has_source_change = true;
                }
            }
        }
        assert!(has_source_change, "expected SourceRefsChanged");
    }

    #[test]
    fn reorder_produces_step_reordered() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [A] > 0),
                #"Removed Columns" = Table.RemoveColumns(Source, {"B"})
            in
                #"Removed Columns"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"B"}),
                #"Filtered Rows" = Table.SelectRows(Source, each [A] > 0)
            in
                #"Removed Columns"
        "#;

        let detail = detail_for(old_expr, new_expr);
        let reordered: Vec<_> = detail
            .step_diffs
            .iter()
            .filter(|d| matches!(d, StepDiff::StepReordered { .. }))
            .collect();
        assert_eq!(reordered.len(), 2);
    }

    #[test]
    fn filter_change_reports_params_changed() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [Amount] > 0)
            in
                #"Filtered Rows"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [Amount] > 10)
            in
                #"Filtered Rows"
        "#;

        let detail = detail_for(old_expr, new_expr);
        assert_eq!(detail.step_diffs.len(), 1);
        match &detail.step_diffs[0] {
            StepDiff::StepModified { after, changes, .. } => {
                assert_eq!(after.step_type, StepType::TableSelectRows);
                assert!(changes.iter().any(|c| matches!(c, StepChange::ParamsChanged)));
            }
            other => panic!("expected StepModified, got {:?}", other),
        }
    }

    #[test]
    fn join_change_reports_params_changed() {
        let old_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Joined" = Table.Join(Source, {"A"}, Source, {"A"}, JoinKind.Inner)
            in
                #"Joined"
        "#;
        let new_expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Joined" = Table.Join(Source, {"B"}, Source, {"B"}, JoinKind.Inner)
            in
                #"Joined"
        "#;

        let detail = detail_for(old_expr, new_expr);
        assert_eq!(detail.step_diffs.len(), 1);
        match &detail.step_diffs[0] {
            StepDiff::StepModified { after, changes, .. } => {
                assert_eq!(after.step_type, StepType::TableJoin);
                assert!(changes.iter().any(|c| matches!(c, StepChange::ParamsChanged)));
            }
            other => panic!("expected StepModified, got {:?}", other),
        }
    }

    #[test]
    fn small_exact_ast_summary_handles_deep_if_change() {
        fn nested_if(depth: usize, leaf: &str) -> String {
            let mut expr = leaf.to_string();
            for i in (1..=depth).rev() {
                expr = format!("if c{} then ({}) else 0", i, expr);
            }
            expr
        }

        let old_expr = nested_if(8, "1");
        let new_expr = nested_if(8, "2");

        let detail = detail_for(&old_expr, &new_expr);
        let summary = detail.ast_summary.expect("ast summary");
        assert_eq!(summary.mode, AstDiffMode::SmallExact);
        assert!(summary.updated > 0);
        assert_eq!(summary.moved, 0);
    }

    #[test]
    fn large_ast_summary_reports_moved_subtree() {
        fn record(prefix: &str, count: usize) -> String {
            let mut fields = Vec::with_capacity(count);
            for i in 0..count {
                fields.push(format!(r#"{prefix}{i} = "{prefix}{i}""#));
            }
            format!("[{}]", fields.join(", "))
        }

        let big = record("Big", 260);
        let small = record("Small", 2);

        let old_expr = format!("if x then {} else {}", big, small);
        let new_expr = format!("if x then {} else {}", small, big);

        let detail = detail_for(&old_expr, &new_expr);
        let summary = detail.ast_summary.expect("ast summary");
        assert_eq!(summary.mode, AstDiffMode::LargeHeuristic);
        assert!(summary.moved >= 1);
        assert!(
            summary.move_hints.iter().any(|mh| mh.subtree_size > 50),
            "expected a large moved subtree hint"
        );
    }

    #[test]
    fn wrap_unwrap_reports_structural_insert() {
        let old_expr = "1";
        let new_expr = "if cond then 1 else 0";

        let detail = detail_for(old_expr, new_expr);
        let summary = detail.ast_summary.expect("ast summary");
        assert!(summary.inserted > 0, "expected inserted nodes for wrap");
    }
}
