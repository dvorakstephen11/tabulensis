use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, ExtractedColumnTypeChanges,
    ExtractedRenamePairs, ExtractedString, ExtractedStringList, QuerySemanticDetail, RenamePair,
    StepChange, StepDiff, StepParams, StepSnapshot, StepType,
};
use crate::m_ast::{
    canonicalize_m_ast, extract_steps, parse_m_expression, MExpr, MModuleAst, MStep,
    StepColumnTypeChange, StepExtracted, StepKind, StepRenamePair,
};
use crate::string_pool::{StringId, StringPool};

const SMALL_AST_NODE_LIMIT: usize = 240;
const REDUCED_TED_MAX_NODES: usize = 320;
const MOVE_SUBTREE_MIN_SIZE: u32 = 6;

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

    detail.ast_summary = Some(ast_diff_summary(&old_ast, &new_ast));
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
    let mut out = Vec::new();

    let mut new_by_name: HashMap<&str, usize> = HashMap::new();
    for (i, s) in new_steps.iter().enumerate() {
        new_by_name.insert(s.name.as_str(), i);
    }

    let mut used_old: HashSet<usize> = HashSet::new();
    let mut used_new: HashSet<usize> = HashSet::new();

    for (oi, s) in old_steps.iter().enumerate() {
        if let Some(&ni) = new_by_name.get(s.name.as_str()) {
            out.push((oi, ni));
            used_old.insert(oi);
            used_new.insert(ni);
        }
    }

    let mut old_by_sig: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut new_by_sig: HashMap<u64, Vec<usize>> = HashMap::new();

    for (oi, s) in old_steps.iter().enumerate() {
        if used_old.contains(&oi) {
            continue;
        }
        old_by_sig.entry(s.signature).or_default().push(oi);
    }
    for (ni, s) in new_steps.iter().enumerate() {
        if used_new.contains(&ni) {
            continue;
        }
        new_by_sig.entry(s.signature).or_default().push(ni);
    }

    for (sig, ois) in &old_by_sig {
        let nis = match new_by_sig.get(sig) {
            Some(v) => v,
            None => continue,
        };
        if ois.len() == 1 && nis.len() == 1 {
            let oi = ois[0];
            let ni = nis[0];
            out.push((oi, ni));
            used_old.insert(oi);
            used_new.insert(ni);
        }
    }

    out.sort_by_key(|(oi, _)| *oi);
    out
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

fn ast_diff_summary(old_ast: &MModuleAst, new_ast: &MModuleAst) -> AstDiffSummary {
    let mut old_tree = build_flat_tree(old_ast);
    let mut new_tree = build_flat_tree(new_ast);

    if !old_tree.nodes.is_empty() {
        compute_subtree_hashes(0, &mut old_tree.nodes);
    }
    if !new_tree.nodes.is_empty() {
        compute_subtree_hashes(0, &mut new_tree.nodes);
    }

    let old_count = old_tree.nodes.len();
    let new_count = new_tree.nodes.len();
    if old_count.max(new_count) <= SMALL_AST_NODE_LIMIT {
        return small_exact_ast_summary(&old_tree, &new_tree);
    }

    large_heuristic_ast_summary(&old_tree, &new_tree)
}

#[derive(Clone)]
struct FlatNode {
    label: u64,
    parent: Option<usize>,
    child_index: u32,
    children: Vec<usize>,
    subtree_hash: u64,
    subtree_size: u32,
}

#[derive(Clone)]
struct FlatTree {
    nodes: Vec<FlatNode>,
}

fn build_flat_tree(ast: &MModuleAst) -> FlatTree {
    let mut nodes = Vec::new();
    let root = ast.root_expr();
    build_flat_tree_inner(root, None, 0, &mut nodes);
    FlatTree { nodes }
}

fn build_flat_tree_inner(
    expr: &MExpr,
    parent: Option<usize>,
    child_index: u32,
    nodes: &mut Vec<FlatNode>,
) -> usize {
    let idx = nodes.len();
    nodes.push(FlatNode {
        label: expr.diff_label_hash(),
        parent,
        child_index,
        children: Vec::new(),
        subtree_hash: 0,
        subtree_size: 0,
    });

    let children = expr.diff_children();
    for (i, child) in children.iter().enumerate() {
        let child_idx = build_flat_tree_inner(child, Some(idx), i as u32, nodes);
        nodes[idx].children.push(child_idx);
    }

    idx
}

fn compute_subtree_hashes(root: usize, nodes: &mut [FlatNode]) -> (u64, u32) {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    nodes[root].label.hash(&mut h);

    let mut size: u32 = 1;
    let children = nodes[root].children.clone();
    for c in children {
        let (ch, cs) = compute_subtree_hashes(c, nodes);
        ch.hash(&mut h);
        size += cs;
    }
    let hash = h.finish();
    nodes[root].subtree_hash = hash;
    nodes[root].subtree_size = size;
    (hash, size)
}

fn small_exact_ast_summary(old_tree: &FlatTree, new_tree: &FlatTree) -> AstDiffSummary {
    let node_count_old = old_tree.nodes.len() as u32;
    let node_count_new = new_tree.nodes.len() as u32;

    let old_simple = simple_tree_from_flat(old_tree);
    let new_simple = simple_tree_from_flat(new_tree);
    let counts = tree_edit_counts(&old_simple, &new_simple);

    AstDiffSummary {
        mode: AstDiffMode::SmallExact,
        node_count_old,
        node_count_new,
        inserted: counts.inserted,
        deleted: counts.deleted,
        updated: counts.updated,
        moved: 0,
        move_hints: Vec::new(),
    }
}

fn large_heuristic_ast_summary(old_tree: &FlatTree, new_tree: &FlatTree) -> AstDiffSummary {
    let node_count_old = old_tree.nodes.len() as u32;
    let node_count_new = new_tree.nodes.len() as u32;

    let candidates = unique_subtree_candidates(old_tree, new_tree);
    let mut matches: Vec<SubtreeMatch> = Vec::new();

    let mut covered_old = vec![false; old_tree.nodes.len()];
    let mut covered_new = vec![false; new_tree.nodes.len()];

    for cand in candidates {
        if range_overlaps(&covered_old, cand.old_idx, cand.size)
            || range_overlaps(&covered_new, cand.new_idx, cand.size)
        {
            continue;
        }
        mark_range(&mut covered_old, cand.old_idx, cand.size);
        mark_range(&mut covered_new, cand.new_idx, cand.size);
        matches.push(cand);
    }

    let mut move_hints = Vec::new();
    for m in &matches {
        let old_node = &old_tree.nodes[m.old_idx];
        let new_node = &new_tree.nodes[m.new_idx];
        if m.size < MOVE_SUBTREE_MIN_SIZE {
            continue;
        }
        let (Some(op), Some(np)) = (old_node.parent, new_node.parent) else {
            continue;
        };
        let old_parent_hash = old_tree.nodes[op].subtree_hash;
        let new_parent_hash = new_tree.nodes[np].subtree_hash;
        if old_parent_hash != new_parent_hash || old_node.child_index != new_node.child_index {
            move_hints.push(AstMoveHint {
                subtree_hash: old_node.subtree_hash,
                from_preorder: m.old_idx as u32,
                to_preorder: m.new_idx as u32,
                subtree_size: m.size,
            });
        }
    }

    let reduced_old = build_reduced_tree(old_tree, &covered_old);
    let reduced_new = build_reduced_tree(new_tree, &covered_new);

    let counts = if reduced_old.labels.len().max(reduced_new.labels.len()) <= REDUCED_TED_MAX_NODES
    {
        tree_edit_counts(&reduced_old, &reduced_new)
    } else {
        approximate_counts(&reduced_old, &reduced_new)
    };

    AstDiffSummary {
        mode: AstDiffMode::LargeHeuristic,
        node_count_old,
        node_count_new,
        inserted: counts.inserted,
        deleted: counts.deleted,
        updated: counts.updated,
        moved: move_hints.len() as u32,
        move_hints,
    }
}

#[derive(Clone)]
struct SimpleTree {
    labels: Vec<u64>,
    children: Vec<Vec<usize>>,
    subtree_size: Vec<u32>,
}

fn simple_tree_from_flat(tree: &FlatTree) -> SimpleTree {
    let mut labels = Vec::with_capacity(tree.nodes.len());
    let mut children = Vec::with_capacity(tree.nodes.len());
    for node in &tree.nodes {
        labels.push(node.label);
        children.push(node.children.clone());
    }
    let mut out = SimpleTree {
        labels,
        children,
        subtree_size: vec![0; tree.nodes.len()],
    };
    if !out.labels.is_empty() {
        compute_simple_subtree_sizes(&mut out, 0);
    }
    out
}

fn build_reduced_tree(tree: &FlatTree, covered: &[bool]) -> SimpleTree {
    let mut labels = Vec::new();
    let mut children: Vec<Vec<usize>> = Vec::new();

    fn visit(
        idx: usize,
        tree: &FlatTree,
        covered: &[bool],
        parent_covered: bool,
        labels: &mut Vec<u64>,
        children: &mut Vec<Vec<usize>>,
    ) -> Option<usize> {
        let is_covered = covered.get(idx).copied().unwrap_or(false);
        if is_covered && !parent_covered {
            let new_idx = labels.len();
            labels.push(atom_label(tree.nodes[idx].subtree_hash));
            children.push(Vec::new());
            return Some(new_idx);
        }
        if is_covered && parent_covered {
            return None;
        }

        let new_idx = labels.len();
        labels.push(tree.nodes[idx].label);
        children.push(Vec::new());
        for &child in &tree.nodes[idx].children {
            if let Some(child_idx) = visit(child, tree, covered, is_covered, labels, children) {
                children[new_idx].push(child_idx);
            }
        }
        Some(new_idx)
    }

    let _root = visit(0, tree, covered, false, &mut labels, &mut children);

    let labels_len = labels.len();
    let mut out = SimpleTree {
        labels,
        children,
        subtree_size: vec![0; labels_len],
    };
    if !out.labels.is_empty() {
        compute_simple_subtree_sizes(&mut out, 0);
    }
    out
}

fn atom_label(subtree_hash: u64) -> u64 {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    0xA7u8.hash(&mut h);
    subtree_hash.hash(&mut h);
    h.finish()
}

fn compute_simple_subtree_sizes(tree: &mut SimpleTree, idx: usize) -> u32 {
    let mut size: u32 = 1;
    let children = tree.children[idx].clone();
    for child in children {
        size += compute_simple_subtree_sizes(tree, child);
    }
    tree.subtree_size[idx] = size;
    size
}

#[derive(Clone, Copy)]
struct EditCounts {
    cost: u32,
    inserted: u32,
    deleted: u32,
    updated: u32,
}

impl EditCounts {
    fn zero() -> Self {
        EditCounts {
            cost: 0,
            inserted: 0,
            deleted: 0,
            updated: 0,
        }
    }

    fn add(self, other: EditCounts) -> EditCounts {
        EditCounts {
            cost: self.cost + other.cost,
            inserted: self.inserted + other.inserted,
            deleted: self.deleted + other.deleted,
            updated: self.updated + other.updated,
        }
    }
}

fn better(a: EditCounts, b: EditCounts) -> EditCounts {
    if a.cost != b.cost {
        return if a.cost < b.cost { a } else { b };
    }
    if a.updated != b.updated {
        return if a.updated < b.updated { a } else { b };
    }
    let a_id = a.inserted + a.deleted;
    let b_id = b.inserted + b.deleted;
    if a_id != b_id {
        return if a_id < b_id { a } else { b };
    }
    a
}

fn tree_edit_counts(old: &SimpleTree, new: &SimpleTree) -> EditCounts {
    if old.labels.is_empty() {
        return EditCounts {
            cost: new.labels.len() as u32,
            inserted: new.labels.len() as u32,
            deleted: 0,
            updated: 0,
        };
    }
    if new.labels.is_empty() {
        return EditCounts {
            cost: old.labels.len() as u32,
            inserted: 0,
            deleted: old.labels.len() as u32,
            updated: 0,
        };
    }

    let mut memo: HashMap<(usize, usize), EditCounts> = HashMap::new();
    tree_edit_counts_at(old, new, 0, 0, &mut memo)
}

fn tree_edit_counts_at(
    old: &SimpleTree,
    new: &SimpleTree,
    oi: usize,
    ni: usize,
    memo: &mut HashMap<(usize, usize), EditCounts>,
) -> EditCounts {
    if let Some(v) = memo.get(&(oi, ni)) {
        return *v;
    }

    let mut base = if old.labels[oi] == new.labels[ni] {
        EditCounts::zero()
    } else {
        EditCounts {
            cost: 1,
            inserted: 0,
            deleted: 0,
            updated: 1,
        }
    };

    let children_cost = align_children(old, new, &old.children[oi], &new.children[ni], memo);
    base = base.add(children_cost);
    memo.insert((oi, ni), base);
    base
}

fn align_children(
    old: &SimpleTree,
    new: &SimpleTree,
    old_children: &[usize],
    new_children: &[usize],
    memo: &mut HashMap<(usize, usize), EditCounts>,
) -> EditCounts {
    let m = old_children.len();
    let n = new_children.len();
    let mut dp = vec![EditCounts::zero(); (m + 1) * (n + 1)];

    let idx = |i: usize, j: usize, n: usize| -> usize { i * (n + 1) + j };

    for i in 1..=m {
        let del = delete_cost(old, old_children[i - 1]);
        let prev = dp[idx(i - 1, 0, n)];
        dp[idx(i, 0, n)] = prev.add(del);
    }
    for j in 1..=n {
        let ins = insert_cost(new, new_children[j - 1]);
        let prev = dp[idx(0, j - 1, n)];
        dp[idx(0, j, n)] = prev.add(ins);
    }

    for i in 1..=m {
        for j in 1..=n {
            let del = dp[idx(i - 1, j, n)].add(delete_cost(old, old_children[i - 1]));
            let ins = dp[idx(i, j - 1, n)].add(insert_cost(new, new_children[j - 1]));
            let sub = dp[idx(i - 1, j - 1, n)].add(tree_edit_counts_at(
                old,
                new,
                old_children[i - 1],
                new_children[j - 1],
                memo,
            ));
            dp[idx(i, j, n)] = better(better(del, ins), sub);
        }
    }

    dp[idx(m, n, n)]
}

fn delete_cost(tree: &SimpleTree, idx: usize) -> EditCounts {
    let size = tree.subtree_size[idx];
    EditCounts {
        cost: size,
        inserted: 0,
        deleted: size,
        updated: 0,
    }
}

fn insert_cost(tree: &SimpleTree, idx: usize) -> EditCounts {
    let size = tree.subtree_size[idx];
    EditCounts {
        cost: size,
        inserted: size,
        deleted: 0,
        updated: 0,
    }
}

fn approximate_counts(old: &SimpleTree, new: &SimpleTree) -> EditCounts {
    let old_count = old.labels.len() as u32;
    let new_count = new.labels.len() as u32;

    let mut old_hist: HashMap<u64, u32> = HashMap::new();
    for label in &old.labels {
        *old_hist.entry(*label).or_default() += 1;
    }

    let mut common: u32 = 0;
    for label in &new.labels {
        let Some(v) = old_hist.get_mut(label) else {
            continue;
        };
        if *v > 0 {
            *v -= 1;
            common += 1;
        }
    }

    let min_nodes = old_count.min(new_count);
    let updated = min_nodes.saturating_sub(common);
    let deleted = old_count.saturating_sub(min_nodes);
    let inserted = new_count.saturating_sub(min_nodes);

    EditCounts {
        cost: inserted + deleted + updated,
        inserted,
        deleted,
        updated,
    }
}

#[derive(Clone, Copy)]
struct SubtreeMatch {
    old_idx: usize,
    new_idx: usize,
    size: u32,
}

fn unique_subtree_candidates(old_tree: &FlatTree, new_tree: &FlatTree) -> Vec<SubtreeMatch> {
    let mut old_map: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut new_map: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, n) in old_tree.nodes.iter().enumerate() {
        old_map.entry(n.subtree_hash).or_default().push(i);
    }
    for (i, n) in new_tree.nodes.iter().enumerate() {
        new_map.entry(n.subtree_hash).or_default().push(i);
    }

    let mut out = Vec::new();
    for (h, ois) in old_map {
        let Some(nis) = new_map.get(&h) else {
            continue;
        };
        if ois.len() == 1 && nis.len() == 1 {
            let oi = ois[0];
            let ni = nis[0];
            let size = old_tree.nodes[oi].subtree_size;
            out.push(SubtreeMatch {
                old_idx: oi,
                new_idx: ni,
                size,
            });
        }
    }

    out.sort_by(|a, b| b.size.cmp(&a.size));
    out
}

fn range_overlaps(covered: &[bool], start: usize, size: u32) -> bool {
    let end = start.saturating_add(size as usize);
    covered[start..end].iter().any(|v| *v)
}

fn mark_range(covered: &mut [bool], start: usize, size: u32) {
    let end = start.saturating_add(size as usize);
    for v in covered.iter_mut().take(end).skip(start) {
        *v = true;
    }
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
}
