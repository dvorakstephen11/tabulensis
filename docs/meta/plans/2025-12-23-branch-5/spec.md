## Branch 5 plan: `2025-12-28-m-semantic-diff-details`

This branch is about turning **“query hash changed”** into **“here is what changed (steps, parameters, moves)”**, while still scaling to gnarly refactors. The sprint plan explicitly calls for: extending the diff report schema with an optional semantic detail payload, implementing **step-aware diffs** using the existing `StepPipeline`, and adding a **hybrid AST diff fallback** for cases where step modeling can’t explain the change (or doesn’t apply). 

---

## 0. Ground truth in the current codebase (what we are building on)

### Current M-query diff behavior

Today the M diff path:

* Computes canonical AST hashes (when semantic diff is enabled) and emits `DiffOp::QueryDefinitionChanged { name, change_kind, old_hash, new_hash }` with **no explanation** beyond kind + hashes.
* Uses `definition_change(...)` to classify as `FormattingOnly` vs `Semantic` by comparing canonical AST hashes (and falls back to raw hashing if parsing/canonicalization fails).
* `enable_m_semantic_diff` is **on by default** in `DiffConfig`, so semantic classification is already “always on” unless someone explicitly disables it. 

### Step extraction already exists (Branch 4 groundwork is present)

You already have a step model:

* `extract_steps(expr_m) -> Option<StepPipeline>` parses, canonicalizes, and extracts `StepPipeline { steps, output_ref, output_signature }`. 
* Each step is an `MStep { name, kind, source_refs, signature }`. 
* `StepKind` recognizes high-value transforms: `Table.SelectRows`, `RemoveColumns`, `RenameColumns`, `TransformColumnTypes`, `NestedJoin`, `Join`, else `Other`.
* Step signatures are designed to survive renames (and reference updates), which is perfect for alignment. 

That’s the foundation for branch 5: **use this step model to narrate semantic diffs**.

---

## 1. Target user experience for Branch 5

### What “semantic detail payload” should accomplish

For a `QueryDefinitionChanged` that is `Semantic`, the report should carry a structured payload that makes the change “obvious”:

* “Step added: Filtered Rows”
* “Step modified: Removed Columns (removed B, added C)”
* “Step reordered: Changed Type moved earlier”
* If it’s not a step pipeline (or it’s too complex): “AST summary: moved subtree X, inserted Y nodes, updated Z nodes”

This is exactly what the branch scope calls for: step diffs plus an AST fallback summary. 

---

## 2. Implementation architecture (how to wire it cleanly)

### High-level flow

When we detect a `QueryDefinitionChanged`:

1. Keep existing `QueryChangeKind` classification (do not touch the logic).
2. If semantic diff is enabled **and** kind is `Semantic`, attempt to produce semantic details:

   * First: **step-aware diff** via `extract_steps(old)` and `extract_steps(new)`
   * If steps are unavailable or unhelpful: **hybrid AST diff summary**
3. Attach the detail payload to the diff op as an **optional field** in the schema.

Key insight: the schema extension must be **backward compatible** (old JSON should still parse, and `semantic_detail` should simply be absent/`null` for older versions or non-semantic changes). The branch plan explicitly asks for “optional payload”. 

---

## 3. Schema changes (core/src/diff.rs)

### 3.1 Add a new optional field to `QueryDefinitionChanged`

Right now `DiffOp::QueryDefinitionChanged` has `{ name, change_kind, old_hash, new_hash }`. 
We’ll add:

* `semantic_detail: Option<QuerySemanticDetail>`

This is the core schema extension required by the branch scope. 

#### Replace in `core/src/diff.rs`

**Code to replace**

```rust
QueryDefinitionChanged {
    name: StringId,
    change_kind: QueryChangeKind,
    old_hash: u64,
    new_hash: u64,
},
```

**New code**

```rust
QueryDefinitionChanged {
    name: StringId,
    change_kind: QueryChangeKind,
    old_hash: u64,
    new_hash: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_detail: Option<QuerySemanticDetail>,
},
```

### 3.2 Define the semantic detail types

You want these types to be:

* **Serializable** (serde)
* **Stable** enough to be consumed by CLI + web viewer
* **Compact** but expressive

I recommend a schema centered around:

* `QuerySemanticDetail { step_diffs: Vec<StepDiff>, ast_summary: Option<AstDiffSummary> }`
* `StepDiff` contains `Added/Removed/Modified/Reordered`
* For “Modified”, include a structured “what changed” where we can (columns list diffs, join keys diffs, etc.)
* `AstDiffSummary` is counts + moves (with mode “small_exact” vs “large_heuristic”)

#### Insert near `QueryChangeKind` in `core/src/diff.rs`

**Code to replace**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryChangeKind {
    Semantic,
    FormattingOnly,
    Renamed,
}
```

**New code**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryChangeKind {
    Semantic,
    FormattingOnly,
    Renamed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuerySemanticDetail {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub step_diffs: Vec<StepDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast_summary: Option<AstDiffSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepDiff {
    StepAdded { step: StepSnapshot },
    StepRemoved { step: StepSnapshot },
    StepReordered {
        name: StringId,
        from_index: u32,
        to_index: u32,
    },
    StepModified {
        before: StepSnapshot,
        after: StepSnapshot,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        changes: Vec<StepChange>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepSnapshot {
    pub name: StringId,
    pub index: u32,
    pub step_type: StepType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub source_refs: Vec<StringId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<StepParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepType {
    TableSelectRows,
    TableRemoveColumns,
    TableRenameColumns,
    TableTransformColumnTypes,
    TableNestedJoin,
    TableJoin,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepParams {
    TableSelectRows { predicate_hash: u64 },
    TableRemoveColumns { columns: ExtractedStringList },
    TableRenameColumns { renames: ExtractedRenamePairs },
    TableTransformColumnTypes { transforms: ExtractedColumnTypeChanges },
    TableNestedJoin {
        left_keys: ExtractedStringList,
        right_keys: ExtractedStringList,
        new_column: ExtractedString,
        join_kind_hash: Option<u64>,
    },
    TableJoin {
        left_keys: ExtractedStringList,
        right_keys: ExtractedStringList,
        join_kind_hash: Option<u64>,
    },
    Other { function_name_hash: Option<u64>, arity: Option<u32>, expr_hash: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtractedString {
    Known { value: StringId },
    Unknown { hash: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtractedStringList {
    Known { values: Vec<StringId> },
    Unknown { hash: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtractedRenamePairs {
    Known { pairs: Vec<RenamePair> },
    Unknown { hash: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenamePair {
    pub from: StringId,
    pub to: StringId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtractedColumnTypeChanges {
    Known { changes: Vec<ColumnTypeChange> },
    Unknown { hash: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnTypeChange {
    pub column: StringId,
    pub ty_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StepChange {
    Renamed { from: StringId, to: StringId },
    SourceRefsChanged {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        removed: Vec<StringId>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        added: Vec<StringId>,
    },
    ParamsChanged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AstDiffMode {
    SmallExact,
    LargeHeuristic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstDiffSummary {
    pub mode: AstDiffMode,
    pub node_count_old: u32,
    pub node_count_new: u32,
    pub inserted: u32,
    pub deleted: u32,
    pub updated: u32,
    pub moved: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub move_hints: Vec<AstMoveHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstMoveHint {
    pub subtree_hash: u64,
    pub from_preorder: u32,
    pub to_preorder: u32,
    pub subtree_size: u32,
}
```

Why this shape?

* It matches the sprint plan’s required categories (`StepAdded/Removed/Modified/Reordered` + AST summary). 
* It is UI-friendly: the CLI can render it line-by-line; the web viewer can expand it structurally.
* It is stable: you can add more `StepChange` variants later without breaking.

### 3.3 Re-export new types from `core/src/lib.rs`

Right now `lib.rs` exports `DiffOp`, `DiffReport`, `QueryChangeKind`, etc. 
The CLI and wasm consumers will likely want to decode `semantic_detail`, so these types should be accessible.

#### Replace in `core/src/lib.rs`

**Code to replace**

```rust
pub use diff::{
    DiffError, DiffOp, DiffReport, DiffSummary, FormulaDiffResult, QueryChangeKind,
    QueryMetadataField, SheetId,
};
```

**New code**

```rust
pub use diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, DiffError, DiffOp, DiffReport,
    DiffSummary, ExtractedColumnTypeChanges, ExtractedRenamePairs, ExtractedString,
    ExtractedStringList, FormulaDiffResult, QueryChangeKind, QueryMetadataField,
    QuerySemanticDetail, RenamePair, SheetId, StepChange, StepDiff, StepParams, StepSnapshot,
    StepType,
};
```

---

## 4. Computing semantic detail (new core module)

### 4.1 Where this logic should live

Do not bloat `m_diff.rs` with all the alignment + AST diff logic. Add a new internal module:

* `core/src/m_semantic_detail.rs`
* It will be crate-private (`pub(crate)` functions) and called from `m_diff.rs`

You already have `m_ast` exposing `extract_steps` crate-wide: `pub(crate) use step_model::{extract_steps, ...}`. 
So `m_semantic_detail.rs` can depend on that.

### 4.2 Add module declaration in `core/src/lib.rs`

You currently declare `mod m_diff;` etc. 
You’ll add `mod m_semantic_detail;` alongside it.

#### Replace (in the module list area of `core/src/lib.rs`)

**Code to replace**

```rust
mod m_ast;
mod m_diff;
mod m_section;
```

**New code**

```rust
mod m_ast;
mod m_diff;
mod m_section;
mod m_semantic_detail;
```

### 4.3 New file: `core/src/m_semantic_detail.rs`

This file will expose one entrypoint:

* `pub(crate) fn build_query_semantic_detail(old_expr: &str, new_expr: &str, pool: &mut StringPool) -> Option<QuerySemanticDetail>`

And internally:

* Step diff computation
* AST diff summary fallback

#### New file content (treat as replacing empty file)

**Code to replace**

```rust
```

**New code**

```rust
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use crate::diff::{
    AstDiffMode, AstDiffSummary, AstMoveHint, ColumnTypeChange, ExtractedColumnTypeChanges,
    ExtractedRenamePairs, ExtractedString, ExtractedStringList, QuerySemanticDetail, RenamePair,
    StepChange, StepDiff, StepParams, StepSnapshot, StepType,
};
use crate::m_ast::{canonicalize_m_ast, extract_steps, parse_m_expression, MModuleAst, StepKind};
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

    detail.ast_summary = Some(ast_diff_summary(&old_ast, &new_ast));
    Some(detail)
}

fn diff_step_pipelines(old_steps: &[crate::m_ast::MStep], new_steps: &[crate::m_ast::MStep], pool: &mut StringPool) -> Vec<StepDiff> {
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
        if !src_removed.is_empty() || !src_added.is_empty() {
            changes.push(StepChange::SourceRefsChanged {
                removed: src_removed,
                added: src_added,
            });
        }

        let same_sig = a.signature == b.signature;
        if !same_sig || params_a != params_b {
            changes.push(StepChange::ParamsChanged);
        }

        if renamed || (!same_sig) || (!src_removed.is_empty() || !src_added.is_empty()) || params_a != params_b {
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

    out.sort_by_key(|d| step_diff_sort_key(d));
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

fn align_steps(old_steps: &[crate::m_ast::MStep], new_steps: &[crate::m_ast::MStep]) -> Vec<(usize, usize)> {
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

fn snapshot_step(s: &crate::m_ast::MStep, index: u32, pool: &mut StringPool) -> StepSnapshot {
    snapshot_step_with_params(s, index, step_params(&s.kind, pool), pool)
}

fn snapshot_step_with_params(
    s: &crate::m_ast::MStep,
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
            columns: extracted_string_list(columns, pool),
        }),
        StepKind::TableRenameColumns { renames, .. } => Some(StepParams::TableRenameColumns {
            renames: extracted_rename_pairs(renames, pool),
        }),
        StepKind::TableTransformColumnTypes { transforms, .. } => Some(StepParams::TableTransformColumnTypes {
            transforms: extracted_column_type_changes(transforms, pool),
        }),
        StepKind::TableNestedJoin { left_keys, right_keys, new_column, join_kind_hash, .. } => {
            Some(StepParams::TableNestedJoin {
                left_keys: extracted_string_list(left_keys, pool),
                right_keys: extracted_string_list(right_keys, pool),
                new_column: extracted_string(new_column, pool),
                join_kind_hash: *join_kind_hash,
            })
        }
        StepKind::TableJoin { left_keys, right_keys, join_kind_hash, .. } => Some(StepParams::TableJoin {
            left_keys: extracted_string_list(left_keys, pool),
            right_keys: extracted_string_list(right_keys, pool),
            join_kind_hash: *join_kind_hash,
        }),
        StepKind::Other { function_name_hash, arity, expr_hash } => Some(StepParams::Other {
            function_name_hash: *function_name_hash,
            arity: arity.map(|n| n as u32),
            expr_hash: *expr_hash,
        }),
    }
}

fn extracted_string(v: &crate::m_ast::Extracted<String>, pool: &mut StringPool) -> ExtractedString {
    match v {
        crate::m_ast::Extracted::Known(s) => ExtractedString::Known { value: pool.intern(s) },
        crate::m_ast::Extracted::Unknown { hash } => ExtractedString::Unknown { hash: *hash },
    }
}

fn extracted_string_list(v: &crate::m_ast::Extracted<Vec<String>>, pool: &mut StringPool) -> ExtractedStringList {
    match v {
        crate::m_ast::Extracted::Known(xs) => {
            let mut values = Vec::with_capacity(xs.len());
            for s in xs {
                values.push(pool.intern(s));
            }
            ExtractedStringList::Known { values }
        }
        crate::m_ast::Extracted::Unknown { hash } => ExtractedStringList::Unknown { hash: *hash },
    }
}

fn extracted_rename_pairs(v: &crate::m_ast::Extracted<Vec<crate::m_ast::RenamePair>>, pool: &mut StringPool) -> ExtractedRenamePairs {
    match v {
        crate::m_ast::Extracted::Known(pairs) => {
            let mut out = Vec::with_capacity(pairs.len());
            for p in pairs {
                out.push(RenamePair {
                    from: pool.intern(&p.from),
                    to: pool.intern(&p.to),
                });
            }
            ExtractedRenamePairs::Known { pairs: out }
        }
        crate::m_ast::Extracted::Unknown { hash } => ExtractedRenamePairs::Unknown { hash: *hash },
    }
}

fn extracted_column_type_changes(
    v: &crate::m_ast::Extracted<Vec<crate::m_ast::ColumnTypeChange>>,
    pool: &mut StringPool,
) -> ExtractedColumnTypeChanges {
    match v {
        crate::m_ast::Extracted::Known(changes) => {
            let mut out = Vec::with_capacity(changes.len());
            for c in changes {
                out.push(ColumnTypeChange {
                    column: pool.intern(&c.column),
                    ty_hash: c.ty_hash,
                });
            }
            ExtractedColumnTypeChanges::Known { changes: out }
        }
        crate::m_ast::Extracted::Unknown { hash } => ExtractedColumnTypeChanges::Unknown { hash: *hash },
    }
}

fn diff_string_sets(a: &[String], b: &[String], pool: &mut StringPool) -> (Vec<StringId>, Vec<StringId>) {
    let sa: BTreeSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let sb: BTreeSet<&str> = b.iter().map(|s| s.as_str()).collect();

    let mut removed = Vec::new();
    let mut added = Vec::new();

    for x in &sa {
        if !sb.contains(x) {
            removed.push(pool.intern(*x));
        }
    }
    for x in &sb {
        if !sa.contains(x) {
            added.push(pool.intern(*x));
        }
    }

    (removed, added)
}

fn ast_diff_summary(old_ast: &MModuleAst, new_ast: &MModuleAst) -> AstDiffSummary {
    let old_tree = FlatTree::from_ast(old_ast);
    let new_tree = FlatTree::from_ast(new_ast);

    let mode = if old_tree.nodes.len().max(new_tree.nodes.len()) <= 250 {
        AstDiffMode::SmallExact
    } else {
        AstDiffMode::LargeHeuristic
    };

    match mode {
        AstDiffMode::SmallExact => small_exact_ast_summary(&old_tree, &new_tree),
        AstDiffMode::LargeHeuristic => large_heuristic_ast_summary(&old_tree, &new_tree),
    }
}

#[derive(Clone)]
struct FlatNode {
    label: u64,
    parent: Option<usize>,
    children: Vec<usize>,
    subtree_hash: u64,
    subtree_size: u32,
}

struct FlatTree {
    nodes: Vec<FlatNode>,
    root: usize,
}

impl FlatTree {
    fn from_ast(ast: &MModuleAst) -> FlatTree {
        let mut nodes = Vec::new();
        let root = flatten_expr(ast, &mut nodes, None);
        compute_subtree_hashes(root, &mut nodes);
        FlatTree { nodes, root }
    }
}

fn flatten_expr(ast: &MModuleAst, nodes: &mut Vec<FlatNode>, parent: Option<usize>) -> usize {
    let root_expr = unsafe { &*(&ast as *const MModuleAst) };
    flatten_expr_inner(&root_expr.root, nodes, parent)
}

fn flatten_expr_inner(expr: &crate::m_ast::MExpr, nodes: &mut Vec<FlatNode>, parent: Option<usize>) -> usize {
    let idx = nodes.len();
    nodes.push(FlatNode {
        label: label_hash(expr),
        parent,
        children: Vec::new(),
        subtree_hash: 0,
        subtree_size: 0,
    });

    let children_exprs = expr_children(expr);
    for ch in children_exprs {
        let cidx = flatten_expr_inner(ch, nodes, Some(idx));
        nodes[idx].children.push(cidx);
    }

    idx
}

fn label_hash(expr: &crate::m_ast::MExpr) -> u64 {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    expr.kind().hash(&mut h);
    h.finish()
}

fn expr_children<'a>(expr: &'a crate::m_ast::MExpr) -> Vec<&'a crate::m_ast::MExpr> {
    use crate::m_ast::MExpr;
    let mut out = Vec::new();
    match expr {
        MExpr::Let { bindings, body } => {
            for b in bindings {
                out.push(&b.value);
            }
            out.push(body);
        }
        MExpr::Record { fields } => {
            for f in fields {
                out.push(&f.value);
            }
        }
        MExpr::List { items } => {
            for it in items {
                out.push(it);
            }
        }
        MExpr::FunctionCall { args, .. } => {
            for a in args {
                out.push(a);
            }
        }
        MExpr::FunctionLiteral { params: _, return_type: _, body } => {
            out.push(body);
        }
        MExpr::UnaryOp { expr, .. } => out.push(expr),
        MExpr::BinaryOp { left, right, .. } => {
            out.push(left);
            out.push(right);
        }
        MExpr::TypeAscription { expr, .. } => out.push(expr),
        MExpr::TryOtherwise { expr, otherwise } => {
            out.push(expr);
            out.push(otherwise);
        }
        MExpr::Ident { .. } => {}
        MExpr::If { cond, then_branch, else_branch } => {
            out.push(cond);
            out.push(then_branch);
            out.push(else_branch);
        }
        MExpr::Each { body } => out.push(body),
        MExpr::Access { base, key, .. } => {
            out.push(base);
            out.push(key);
        }
        MExpr::Primitive(_) => {}
        MExpr::Opaque(_) => {}
    }
    out
}

fn compute_subtree_hashes(root: usize, nodes: &mut [FlatNode]) -> (u64, u32) {
    use crate::hashing::XXH64_SEED;
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    nodes[root].label.hash(&mut h);

    let mut size: u32 = 1;
    for &c in &nodes[root].children {
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

    AstDiffSummary {
        mode: AstDiffMode::SmallExact,
        node_count_old,
        node_count_new,
        inserted: 0,
        deleted: 0,
        updated: 0,
        moved: 0,
        move_hints: Vec::new(),
    }
}

fn large_heuristic_ast_summary(old_tree: &FlatTree, new_tree: &FlatTree) -> AstDiffSummary {
    let node_count_old = old_tree.nodes.len() as u32;
    let node_count_new = new_tree.nodes.len() as u32;

    let mut old_map: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut new_map: HashMap<u64, Vec<usize>> = HashMap::new();

    for (i, n) in old_tree.nodes.iter().enumerate() {
        old_map.entry(n.subtree_hash).or_default().push(i);
    }
    for (i, n) in new_tree.nodes.iter().enumerate() {
        new_map.entry(n.subtree_hash).or_default().push(i);
    }

    let mut matched_old: HashSet<usize> = HashSet::new();
    let mut matched_new: HashSet<usize> = HashSet::new();

    let mut move_hints = Vec::new();
    let mut moved: u32 = 0;

    for (h, ois) in &old_map {
        let nis = match new_map.get(h) {
            Some(v) => v,
            None => continue,
        };
        if ois.len() == 1 && nis.len() == 1 {
            let oi = ois[0];
            let ni = nis[0];

            let sz = old_tree.nodes[oi].subtree_size;
            if sz >= 6 {
                let op = old_tree.nodes[oi].parent.map(|p| old_tree.nodes[p].subtree_hash);
                let np = new_tree.nodes[ni].parent.map(|p| new_tree.nodes[p].subtree_hash);
                if op != np {
                    moved += 1;
                    move_hints.push(AstMoveHint {
                        subtree_hash: *h,
                        from_preorder: oi as u32,
                        to_preorder: ni as u32,
                        subtree_size: sz,
                    });
                }
            }

            matched_old.insert(oi);
            matched_new.insert(ni);
        }
    }

    let deleted = (node_count_old as usize).saturating_sub(matched_old.len()) as u32;
    let inserted = (node_count_new as usize).saturating_sub(matched_new.len()) as u32;

    AstDiffSummary {
        mode: AstDiffMode::LargeHeuristic,
        node_count_old,
        node_count_new,
        inserted,
        deleted,
        updated: 0,
        moved,
        move_hints,
    }
}
```

Notes on this file:

* The step diff alignment is intentionally “phase 1”: **name-based alignment + unique signature alignment**, matching the sprint plan guidance “order-based + signature similarity”. 
* You can later add a third alignment pass (“soft matching”) without changing the schema.

Also: the `small_exact_ast_summary` placeholder is where you’ll implement the Zhang–Shasha-style exact TED. The plan below explains how to do it cleanly.

---

## 5. Wire semantic detail into the existing diff emission (core/src/m_diff.rs)

### Where to attach semantic detail

`diff_queries_to_ops` is where `QueryDefinitionChanged` ops are emitted. 
Currently it calls `definition_change(...)` and then pushes the op with hashes only.

We will:

* Compute semantic detail only when:

  * `config.enable_m_semantic_diff` is true (it usually is) 
  * The computed `change_kind` is `Semantic`

This keeps output noise down and avoids doing extra work for `FormattingOnly`.

### Replace the op emission block

**Code to replace**

```rust
let Some((kind, old_h, new_h)) =
    definition_change(&old_q.expression_m, &new_q.expression_m, semantic)
else {
    continue;
};

ops.push(DiffOp::QueryDefinitionChanged {
    name: pool.intern(&old_q.name),
    change_kind: kind,
    old_hash: old_h,
    new_hash: new_h,
});
```



**New code**

```rust
let Some((kind, old_h, new_h)) =
    definition_change(&old_q.expression_m, &new_q.expression_m, semantic)
else {
    continue;
};

let semantic_detail = if semantic && kind == DiffQueryChangeKind::Semantic {
    crate::m_semantic_detail::build_query_semantic_detail(
        &old_q.expression_m,
        &new_q.expression_m,
        pool,
    )
} else {
    None
};

ops.push(DiffOp::QueryDefinitionChanged {
    name: pool.intern(&old_q.name),
    change_kind: kind,
    old_hash: old_h,
    new_hash: new_h,
    semantic_detail,
});
```

This satisfies the branch scope requirement: optional payload when semantic diff is enabled and the change is semantic. 

---

## 6. CLI output changes (compact summary + verbose expansion)

### Current behavior

In text output, `QueryDefinitionChanged` renders a single line like:

* “Query definition changed (semantic change)” or “(formatting-only change)”. 

In git-diff output, it similarly prints a one-liner header for query changes. 

Branch 5 requires: “compact semantic summary under each changed query”. 

### 6.1 `cli/src/output/text.rs`: render semantic details

You already return a `Vec<String>` from `render_op`, so adding multi-line output is straightforward. 

#### Replace the `QueryDefinitionChanged` match arm

**Code to replace**

```rust
DiffOp::QueryDefinitionChanged {
    name, change_kind, ..
} => {
    let k = match change_kind {
        QueryChangeKind::Semantic => "semantic change",
        QueryChangeKind::FormattingOnly => "formatting-only change",
        QueryChangeKind::Renamed => "rename",
    };
    vec![format!(
        "[M] Query definition changed: {} ({})",
        report.resolve(*name),
        k
    )]
}
```



**New code**

```rust
DiffOp::QueryDefinitionChanged {
    name,
    change_kind,
    semantic_detail,
    ..
} => {
    let k = match change_kind {
        QueryChangeKind::Semantic => "semantic change",
        QueryChangeKind::FormattingOnly => "formatting-only change",
        QueryChangeKind::Renamed => "rename",
    };

    let mut lines = vec![format!(
        "[M] Query definition changed: {} ({})",
        report.resolve(*name),
        k
    )];

    let Some(detail) = semantic_detail else {
        return lines;
    };

    if !detail.step_diffs.is_empty() {
        let mut added = 0usize;
        let mut removed = 0usize;
        let mut modified = 0usize;
        let mut reordered = 0usize;

        for d in &detail.step_diffs {
            match d {
                excel_diff::StepDiff::StepAdded { .. } => added += 1,
                excel_diff::StepDiff::StepRemoved { .. } => removed += 1,
                excel_diff::StepDiff::StepModified { .. } => modified += 1,
                excel_diff::StepDiff::StepReordered { .. } => reordered += 1,
            }
        }

        lines.push(format!(
            "    steps: +{} -{} ~{} r{}",
            added, removed, modified, reordered
        ));

        let max_lines = if verbosity == Verbosity::Verbose { 50 } else { 5 };
        for d in detail.step_diffs.iter().take(max_lines) {
            lines.push(format!("    {}", format_step_diff(report, d)));
        }
        if detail.step_diffs.len() > max_lines {
            lines.push(format!("    ... ({} more)", detail.step_diffs.len() - max_lines));
        }

        return lines;
    }

    if let Some(ast) = &detail.ast_summary {
        lines.push(format!(
            "    ast: mode={:?} moved={} inserted={} deleted={} updated={}",
            ast.mode, ast.moved, ast.inserted, ast.deleted, ast.updated
        ));
        if verbosity == Verbosity::Verbose && !ast.move_hints.is_empty() {
            for mh in ast.move_hints.iter().take(8) {
                lines.push(format!(
                    "    ast_move: hash={} size={} from={} to={}",
                    mh.subtree_hash, mh.subtree_size, mh.from_preorder, mh.to_preorder
                ));
            }
        }
    }

    lines
}
```

You’ll also add a helper:

* `format_step_diff(report: &DiffReport, d: &StepDiff) -> String`

That helper will ensure the output “mentions the step name or type”, satisfying the testability requirement in the sprint plan. 

### 6.2 `cli/src/output/git_diff.rs`: include the same compact summary

The git-diff renderer currently prints a one-line header. 
Add an indented summary similar to text output (but keep it short so it doesn’t overwhelm diffs).

---

## 7. Web viewer updates (expand/collapse by query)

Even though the `web/` source content isn’t present in the provided context, the sprint plan calls for:

* “Web: expand/collapse per query with step details.” 

Implementation approach (UI-agnostic and minimal):

* Group ops by query name in the view layer.
* For `QueryDefinitionChanged` ops, show:

  * summary line (change kind)
  * if `semantic_detail.step_diffs` exists: show “steps: + - ~ r”
  * a toggle “details”
  * in details: render each `StepDiff`

Pseudo-JS rendering snippet (adapt to your existing `main.js`):

**Code to replace**

```js
// existing render for QueryDefinitionChanged
```

**New code**

```js
function renderQueryDefinitionChanged(op, strings) {
  const name = strings[op.name] || "(unknown)";
  const header = document.createElement("div");
  header.className = "op query-def-changed";
  header.textContent = `Query definition changed: ${name} (${op.change_kind})`;

  const detail = op.semantic_detail;
  if (!detail) return header;

  const summary = document.createElement("div");
  summary.className = "op-detail-summary";

  if (detail.step_diffs && detail.step_diffs.length) {
    let added = 0, removed = 0, modified = 0, reordered = 0;
    for (const d of detail.step_diffs) {
      if (d.kind === "step_added") added++;
      else if (d.kind === "step_removed") removed++;
      else if (d.kind === "step_modified") modified++;
      else if (d.kind === "step_reordered") reordered++;
    }
    summary.textContent = `steps: +${added} -${removed} ~${modified} r${reordered}`;
  } else if (detail.ast_summary) {
    const a = detail.ast_summary;
    summary.textContent = `ast: moved=${a.moved} inserted=${a.inserted} deleted=${a.deleted} updated=${a.updated}`;
  }

  const btn = document.createElement("button");
  btn.textContent = "details";
  btn.className = "toggle";

  const body = document.createElement("div");
  body.className = "op-detail-body";
  body.style.display = "none";

  btn.onclick = () => {
    body.style.display = body.style.display === "none" ? "block" : "none";
  };

  if (detail.step_diffs && detail.step_diffs.length) {
    const ul = document.createElement("ul");
    for (const d of detail.step_diffs) {
      ul.appendChild(renderStepDiff(d, strings));
    }
    body.appendChild(ul);
  } else if (detail.ast_summary && detail.ast_summary.move_hints) {
    const ul = document.createElement("ul");
    for (const mh of detail.ast_summary.move_hints.slice(0, 20)) {
      const li = document.createElement("li");
      li.textContent = `move hash=${mh.subtree_hash} size=${mh.subtree_size} from=${mh.from_preorder} to=${mh.to_preorder}`;
      ul.appendChild(li);
    }
    body.appendChild(ul);
  }

  header.appendChild(summary);
  header.appendChild(btn);
  header.appendChild(body);
  return header;
}

function renderStepDiff(d, strings) {
  const li = document.createElement("li");

  if (d.kind === "step_added") {
    const s = d.step;
    li.textContent = `+ ${strings[s.name]} (${s.step_type})`;
  } else if (d.kind === "step_removed") {
    const s = d.step;
    li.textContent = `- ${strings[s.name]} (${s.step_type})`;
  } else if (d.kind === "step_reordered") {
    li.textContent = `r ${strings[d.name]} ${d.from_index} -> ${d.to_index}`;
  } else if (d.kind === "step_modified") {
    const a = d.after;
    li.textContent = `~ ${strings[a.name]} (${a.step_type})`;
  } else {
    li.textContent = JSON.stringify(d);
  }

  return li;
}
```

---

## 8. Hybrid AST diff strategy (finish the hard part properly)

The sprint plan explicitly wants:

* small AST: **exact tree edit distance**
* large AST: **move-aware mapping** + reduced diff on unmatched regions 

### 8.1 Definitions (so the design is crisp)

* **AST (Abstract Syntax Tree):** a tree representation of structured code. In your repo, that’s `MModuleAst { root: MExpr }` with variants like `Let`, `If`, `FunctionCall`, etc. 
* **Tree Edit Distance (TED):** minimum-cost sequence of node edits (insert/delete/update) to transform one tree into another.
* **Move-aware diff (GumTree-like):** heuristics to detect when an identical (or near-identical) subtree has moved, instead of reporting delete+insert everywhere.

### 8.2 Large heuristic mode (implement first; it gives immediate value)

The code stub above already matches unique subtree hashes and marks a move if the parent subtree differs, with a size threshold to avoid noise.

To make it meet the sprint plan’s “usable structural diff” requirement, you should:

1. Prefer matching **larger** subtrees first:

   * Sort candidate subtree hashes by subtree_size descending
   * Match unique ones first
2. Avoid overlapping matches:

   * When you match subtree root `oi`, mark all descendants as “covered” so they don’t create redundant move hints
3. Compute a reduced skeleton diff:

   * Collapse matched subtrees into “atoms”
   * Count insert/delete/update on unmatched nodes in that reduced tree

This yields a summary that accurately says:

* “moved 3 blocks”
* “inserted 10 nodes”
* “updated 2 nodes”

### 8.3 Small exact mode (implement second; it closes correctness)

For trees up to ~200–300 nodes:

* Implement Zhang–Shasha (classic ordered tree edit distance)
* Output:

  * inserted/deleted/updated counts
  * (moves are 0 in exact mode; TED doesn’t do moves)

Practical plan:

* Flatten into postorder
* Precompute:

  * `lmd[i]` = leftmost descendant of node i in postorder
  * `keyroots[]`
* Use the standard dynamic program to compute distances for each keyroot pair

Once you have counts, plug them into `AstDiffSummary { mode: SmallExact, inserted, deleted, updated, moved: 0 }`.

---

## 9. Tests and fixtures (must satisfy branch DoD)

The sprint plan requires that output supports assertions like “exactly one semantically significant change” and “mentions the step name or type”, plus fixtures for hybrid AST behavior.

### 9.1 Step-aware diff unit tests (fast, deterministic)

Put these in `core/src/m_semantic_detail.rs` under `#[cfg(test)]` so you can directly call private helpers.

Test cases:

1. **Single parameter change**

   * Old: `Table.RemoveColumns(Source, {"A","B"})`
   * New: `Table.RemoveColumns(Source, {"A","C"})`
   * Expect: exactly one `StepModified`, and the modified step has type `TableRemoveColumns`.

2. **Step rename only**

   * Old step `"Removed Columns"`
   * New step `"Dropped Columns"` with references updated
   * Expect: `StepModified` includes `StepChange::Renamed`, and no removed+added.

This aligns with the existing guarantee that signatures survive rename + reference update. 

3. **Dependency change**

   * Change the input of a step (source ref changes)
   * Expect: `StepChange::SourceRefsChanged` is present (because `source_refs` are tracked independently of `StepKind`). 

4. **Reorder**

   * Two independent steps swapped
   * Expect: `StepReordered`

### 9.2 AST hybrid fixtures (no need for XLSX; raw M strings are enough)

The plan mentions deep skewed IF tree and moved-block refactors. 

Create a helper in tests to generate:

* **Deep skewed IF**:

  * `if c1 then (if c2 then (if c3 ...)) else ...`
  * Make two versions that differ only in one leaf; ensure exact mode handles it and doesn’t blow up.
* **Moved subtree** (non-let query):

  * Old: `if x then BIG_RECORD else SMALL_RECORD`
  * New: `if x then SMALL_RECORD else BIG_RECORD`
  * Expect: `AstDiffSummary.moved >= 1` and `move_hints` contains the big subtree hash.

### 9.3 CLI rendering smoke tests (optional but nice)

If you have CLI test harnesses, assert that the text output line includes the step name/type. If not, treat it as “manual verify” for this sprint.

---

## 10. “Definition of done” mapping (directly to Branch 5 scope)

This plan satisfies Branch 5 scope items:

1. **Schema extension**: add optional semantic detail payload on `QueryDefinitionChanged`.
2. **Step-aware diff**: align by name + signature, produce `StepAdded/Removed/Modified/Reordered`, and make it testable.
3. **Hybrid AST fallback**: implement move-aware mapping for large trees and exact TED for small trees (small exact can land as phase 2 if needed, but the plan includes it).
4. **Fixtures/tests**: raw M fixtures for step diffs + AST hybrid scenarios, plus bounded-time tests. 
5. **CLI/web presentation**: compact summary + expandability.

---

## 11. Key risks and mitigation (so implementation stays “exquisite”)

1. **Step alignment ambiguity**

   * Duplicate signatures happen (same transform repeated).
   * Mitigation: name-first alignment (already in code above), then unique signature alignment, later add dependency+position scoring as a third pass.

2. **Signature misses semantic dependency changes**

   * `StepKind` intentionally ignores the first “input table” arg; dependency is stored in `source_refs`.
   * Mitigation: treat `source_refs` delta as a modification even if signature matches.

3. **AST diff performance**

   * Mitigation: strict node-count threshold for exact mode; heuristic mode is O(n)ish and should be bounded. 

4. **Schema bloat**

   * Mitigation: keep payload optional and don’t attach it for `FormattingOnly`.

