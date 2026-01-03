## Goal of branch 4

Branch 4 is about turning a Power Query `let ... in ...` expression into an explicit “pipeline of steps” so later diffs can say “Filtered Rows step changed” rather than “hash changed.” 

Your current codebase already has the crucial AST building blocks needed for this (structured `Let`, `Ident`, `Each`, `Access`, `FunctionCall`, operators, etc.).  That means branch 4 can be implemented as a pure “AST → step pipeline model” layer, without changing how queries are extracted from `Section1.m`. Query extraction today is `Section1.m` → `SectionMember.expression_m` → `Query.expression_m`, and we should keep that unchanged. 

---

## Implementation approach

### Design principles for the step model

1. **Best-effort, non-fatal extraction**

   * If `parse_m_expression` fails or the root expression isn’t a `let`, return `None`.
   * This keeps extraction safe to call in diff paths later without risking panics or hard failures.

2. **Deterministic output**

   * Preserve **binding order** exactly as written in the `let` bindings.
   * Dependencies should be deterministic (sorted by step order).

3. **Stable step signatures**

   * Must not depend on:

     * whitespace / comments (already handled by tokenization + AST) 
     * step names (renames)
     * renames of referenced steps (when possible), by normalizing “step references” in signature hashing 

4. **Start with high-value classifications**

   * Exactly as the plan calls out: `Table.SelectRows`, `Table.RemoveColumns`, `Table.RenameColumns`, `Table.TransformColumnTypes`, `Table.NestedJoin`/`Table.Join`, else `Other`. 

---

## Step model shape

A practical internal model that matches the sprint plan intent  and is friendly for branch 5:

* `StepPipeline`

  * `steps: Vec<MStep>` (ordered)
  * `output_ref: Option<String>` (if `in` returns a step name)
  * `output_signature: u64` (always computed; useful when output isn’t a simple step ref)

* `MStep`

  * `name: String` (binding name from the let binding)
  * `kind: StepKind` (classified)
  * `source_refs: Vec<String>` (dependencies on prior steps)
  * `signature: u64` (stable signature for alignment)

* `StepKind`

  * known variants with extracted parameters
  * `Other { function_name_hash, arity, expr_hash }`

For “key semantic bits”, use typed extracted params instead of a loose string map. It will make branch 5 diffs far easier.

---

## Where to put this in the codebase

### Constraint: AST privacy

`MExpr` and `LetBinding` are private to `m_ast.rs`. 
So step extraction must live **inside the `m_ast` module**, either:

* as a submodule (`core/src/m_ast/step_model.rs`), imported by `m_ast.rs`, or
* inline inside `m_ast.rs` (less clean)

The submodule route keeps code tidy while preserving privacy.

---

## Concrete code changes

### 1) Wire a submodule into `core/src/m_ast.rs`

Replace this top-of-file import block in `core/src/m_ast.rs`:

```rust
use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;
```

With this:

```rust
use std::iter::Peekable;
use std::str::Chars;

use thiserror::Error;

mod step_model;
pub(crate) use step_model::{extract_steps, MStep, StepKind, StepPipeline};
```

This keeps the step model internal (`pub(crate)`), but makes it available to the rest of the crate (especially branch 5 later).

---

### 2) Add new file `core/src/m_ast/step_model.rs`

Create the file with the following contents:

```rust
use std::collections::{BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use crate::hashing::XXH64_SEED;

use super::{MExpr, MParam, MPrimitive, MToken};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StepPipeline {
    pub(crate) steps: Vec<MStep>,
    pub(crate) output_ref: Option<String>,
    pub(crate) output_signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MStep {
    pub(crate) name: String,
    pub(crate) kind: StepKind,
    pub(crate) source_refs: Vec<String>,
    pub(crate) signature: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum StepKind {
    TableSelectRows {
        predicate_hash: u64,
        extras: Vec<u64>,
    },
    TableRemoveColumns {
        columns: Extracted<Vec<String>>,
        extras: Vec<u64>,
    },
    TableRenameColumns {
        renames: Extracted<Vec<RenamePair>>,
        extras: Vec<u64>,
    },
    TableTransformColumnTypes {
        transforms: Extracted<Vec<ColumnTypeChange>>,
        extras: Vec<u64>,
    },
    TableNestedJoin {
        left_keys: Extracted<Vec<String>>,
        right_keys: Extracted<Vec<String>>,
        new_column: Extracted<String>,
        join_kind_hash: Option<u64>,
        extras: Vec<u64>,
    },
    TableJoin {
        left_keys: Extracted<Vec<String>>,
        right_keys: Extracted<Vec<String>>,
        join_kind_hash: Option<u64>,
        extras: Vec<u64>,
    },
    Other {
        function_name_hash: Option<u64>,
        arity: Option<usize>,
        expr_hash: u64,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Extracted<T> {
    Known(T),
    Unknown { hash: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RenamePair {
    pub(crate) from: String,
    pub(crate) to: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ColumnTypeChange {
    pub(crate) column: String,
    pub(crate) ty_hash: u64,
}

pub(crate) fn extract_steps(expr_m: &str) -> Option<StepPipeline> {
    let mut ast = super::parse_m_expression(expr_m).ok()?;
    super::canonicalize_m_ast(&mut ast);
    extract_steps_from_ast(&ast)
}

fn extract_steps_from_ast(ast: &super::MModuleAst) -> Option<StepPipeline> {
    let MExpr::Let { bindings, body } = &ast.root else {
        return None;
    };

    let step_names: BTreeSet<String> = bindings.iter().map(|b| b.name.clone()).collect();
    let mut name_to_idx: HashMap<String, usize> = HashMap::new();
    for (idx, b) in bindings.iter().enumerate() {
        name_to_idx.insert(b.name.clone(), idx);
    }

    let mut steps = Vec::with_capacity(bindings.len());
    for (idx, b) in bindings.iter().enumerate() {
        let source_refs = collect_step_refs(&b.value, &step_names, &name_to_idx, idx);
        let kind = classify_step(&b.value, &step_names);
        let signature = hash64(&kind);
        steps.push(MStep {
            name: b.name.clone(),
            kind,
            source_refs,
            signature,
        });
    }

    let output_signature = hash_expr_signature(body, &step_names);
    let output_ref = match body.as_ref() {
        MExpr::Ident { name } if step_names.contains(name) => Some(name.clone()),
        _ => None,
    };

    Some(StepPipeline {
        steps,
        output_ref,
        output_signature,
    })
}

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

#[derive(Default)]
struct BoundStack<'a> {
    names: Vec<&'a str>,
}

impl<'a> BoundStack<'a> {
    fn contains(&self, name: &str) -> bool {
        self.names.iter().any(|&n| n == name)
    }

    fn push(&mut self, name: &'a str) {
        self.names.push(name);
    }

    fn pop_n(&mut self, n: usize) {
        for _ in 0..n {
            self.names.pop();
        }
    }
}

fn collect_step_refs(
    expr: &MExpr,
    step_names: &BTreeSet<String>,
    name_to_idx: &HashMap<String, usize>,
    current_step_idx: usize,
) -> Vec<String> {
    let mut out: BTreeSet<String> = BTreeSet::new();
    let mut bound = BoundStack::default();
    collect_step_refs_inner(
        expr,
        step_names,
        name_to_idx,
        current_step_idx,
        &mut bound,
        &mut out,
    );

    let mut v: Vec<String> = out.into_iter().collect();
    v.sort_by_key(|name| name_to_idx.get(name.as_str()).copied().unwrap_or(usize::MAX));
    v
}

fn collect_step_refs_inner<'a>(
    expr: &'a MExpr,
    step_names: &BTreeSet<String>,
    name_to_idx: &HashMap<String, usize>,
    current_step_idx: usize,
    bound: &mut BoundStack<'a>,
    out: &mut BTreeSet<String>,
) {
    match expr {
        MExpr::Ident { name } => {
            if bound.contains(name) {
                return;
            }
            if !step_names.contains(name) {
                return;
            }
            let Some(&idx) = name_to_idx.get(name.as_str()) else {
                return;
            };
            if idx < current_step_idx {
                out.insert(name.clone());
            }
        }
        MExpr::Opaque(tokens) => {
            for t in tokens {
                if let MToken::Identifier(id) = t {
                    if bound.contains(id) {
                        continue;
                    }
                    if !step_names.contains(id) {
                        continue;
                    }
                    if let Some(&idx) = name_to_idx.get(id.as_str()) {
                        if idx < current_step_idx {
                            out.insert(id.clone());
                        }
                    }
                }
            }
        }
        MExpr::Let { bindings, body } => {
            for b in bindings {
                collect_step_refs_inner(
                    &b.value,
                    step_names,
                    name_to_idx,
                    current_step_idx,
                    bound,
                    out,
                );
                bound.push(b.name.as_str());
            }
            collect_step_refs_inner(
                body,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            bound.pop_n(bindings.len());
        }
        MExpr::FunctionLiteral { params, body, .. } => {
            let n = params.len();
            for p in params {
                bound.push(p.name.as_str());
            }
            collect_step_refs_inner(
                body,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            bound.pop_n(n);
        }
        MExpr::Each { body } => {
            bound.push("_");
            collect_step_refs_inner(
                body,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            bound.pop_n(1);
        }
        MExpr::Record { fields } => {
            for f in fields {
                collect_step_refs_inner(
                    &f.value,
                    step_names,
                    name_to_idx,
                    current_step_idx,
                    bound,
                    out,
                );
            }
        }
        MExpr::List { items } => {
            for i in items {
                collect_step_refs_inner(
                    i,
                    step_names,
                    name_to_idx,
                    current_step_idx,
                    bound,
                    out,
                );
            }
        }
        MExpr::FunctionCall { args, .. } => {
            for a in args {
                collect_step_refs_inner(
                    a,
                    step_names,
                    name_to_idx,
                    current_step_idx,
                    bound,
                    out,
                );
            }
        }
        MExpr::Access { base, key, .. } => {
            collect_step_refs_inner(
                base,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            collect_step_refs_inner(
                key,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_step_refs_inner(
                cond,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            collect_step_refs_inner(
                then_branch,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            collect_step_refs_inner(
                else_branch,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::UnaryOp { expr, .. } => {
            collect_step_refs_inner(
                expr,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::BinaryOp { left, right, .. } => {
            collect_step_refs_inner(
                left,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            collect_step_refs_inner(
                right,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::TypeAscription { expr, .. } => {
            collect_step_refs_inner(
                expr,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::TryOtherwise { expr, otherwise } => {
            collect_step_refs_inner(
                expr,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
            collect_step_refs_inner(
                otherwise,
                step_names,
                name_to_idx,
                current_step_idx,
                bound,
                out,
            );
        }
        MExpr::Primitive(_) => {}
    }
}

fn classify_step(expr: &MExpr, step_names: &BTreeSet<String>) -> StepKind {
    let MExpr::FunctionCall { name, args } = expr else {
        return StepKind::Other {
            function_name_hash: None,
            arity: None,
            expr_hash: hash_expr_signature(expr, step_names),
        };
    };

    let name_lc = name.to_ascii_lowercase();
    match name_lc.as_str() {
        "table.selectrows" => classify_select_rows(args, step_names),
        "table.removecolumns" => classify_remove_columns(args, step_names),
        "table.renamecolumns" => classify_rename_columns(args, step_names),
        "table.transformcolumntypes" => classify_transform_column_types(args, step_names),
        "table.nestedjoin" => classify_nested_join(args, step_names),
        "table.join" => classify_join(args, step_names),
        _ => StepKind::Other {
            function_name_hash: Some(hash64(&name_lc)),
            arity: Some(args.len()),
            expr_hash: hash_expr_signature(expr, step_names),
        },
    }
}

fn extras_hashes(args: &[MExpr], start: usize, step_names: &BTreeSet<String>) -> Vec<u64> {
    if args.len() <= start {
        return Vec::new();
    }
    args[start..]
        .iter()
        .map(|e| hash_expr_signature(e, step_names))
        .collect()
}

fn classify_select_rows(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 2 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.selectrows")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    StepKind::TableSelectRows {
        predicate_hash: hash_expr_signature(&args[1], step_names),
        extras: extras_hashes(args, 2, step_names),
    }
}

fn classify_remove_columns(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 2 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.removecolumns")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    let columns = extract_string_list(&args[1]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[1], step_names),
        }
    });

    StepKind::TableRemoveColumns {
        columns,
        extras: extras_hashes(args, 2, step_names),
    }
}

fn classify_rename_columns(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 2 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.renamecolumns")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    let renames = extract_rename_pairs(&args[1]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[1], step_names),
        }
    });

    StepKind::TableRenameColumns {
        renames,
        extras: extras_hashes(args, 2, step_names),
    }
}

fn classify_transform_column_types(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 2 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.transformcolumntypes")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    let transforms = extract_column_type_changes(&args[1], step_names)
        .map(Extracted::Known)
        .unwrap_or_else(|| Extracted::Unknown {
            hash: hash_expr_signature(&args[1], step_names),
        });

    StepKind::TableTransformColumnTypes {
        transforms,
        extras: extras_hashes(args, 2, step_names),
    }
}

fn classify_nested_join(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 5 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.nestedjoin")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    let left_keys = extract_string_list(&args[1]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[1], step_names),
        }
    });

    let right_keys = if args.len() >= 4 {
        extract_string_list(&args[3]).map(Extracted::Known).unwrap_or_else(|| {
            Extracted::Unknown {
                hash: hash_expr_signature(&args[3], step_names),
            }
        })
    } else {
        Extracted::Unknown { hash: 0 }
    };

    let new_column = extract_string(&args[4]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[4], step_names),
        }
    });

    let join_kind_hash = args.get(5).map(|e| hash_expr_signature(e, step_names));
    let extras = if args.len() > 6 {
        extras_hashes(args, 6, step_names)
    } else {
        Vec::new()
    };

    StepKind::TableNestedJoin {
        left_keys,
        right_keys,
        new_column,
        join_kind_hash,
        extras,
    }
}

fn classify_join(args: &[MExpr], step_names: &BTreeSet<String>) -> StepKind {
    if args.len() < 4 {
        return StepKind::Other {
            function_name_hash: Some(hash64(&"table.join")),
            arity: Some(args.len()),
            expr_hash: hash64(&args.len()),
        };
    }

    let left_keys = extract_string_list(&args[1]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[1], step_names),
        }
    });

    let right_keys = extract_string_list(&args[3]).map(Extracted::Known).unwrap_or_else(|| {
        Extracted::Unknown {
            hash: hash_expr_signature(&args[3], step_names),
        }
    });

    let join_kind_hash = args.get(4).map(|e| hash_expr_signature(e, step_names));
    let extras = if args.len() > 5 {
        extras_hashes(args, 5, step_names)
    } else {
        Vec::new()
    };

    StepKind::TableJoin {
        left_keys,
        right_keys,
        join_kind_hash,
        extras,
    }
}

fn extract_string(expr: &MExpr) -> Option<String> {
    match expr {
        MExpr::Primitive(MPrimitive::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn extract_string_list(expr: &MExpr) -> Option<Vec<String>> {
    match expr {
        MExpr::Primitive(MPrimitive::String(s)) => Some(vec![s.clone()]),
        MExpr::List { items } => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                let MExpr::Primitive(MPrimitive::String(s)) = it else {
                    return None;
                };
                out.push(s.clone());
            }
            Some(out)
        }
        _ => None,
    }
}

fn extract_rename_pairs(expr: &MExpr) -> Option<Vec<RenamePair>> {
    let MExpr::List { items } = expr else {
        return None;
    };

    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let MExpr::List { items: pair } = it else {
            return None;
        };
        if pair.len() < 2 {
            return None;
        }
        let MExpr::Primitive(MPrimitive::String(from)) = &pair[0] else {
            return None;
        };
        let MExpr::Primitive(MPrimitive::String(to)) = &pair[1] else {
            return None;
        };
        out.push(RenamePair {
            from: from.clone(),
            to: to.clone(),
        });
    }

    Some(out)
}

fn extract_column_type_changes(expr: &MExpr, step_names: &BTreeSet<String>) -> Option<Vec<ColumnTypeChange>> {
    let MExpr::List { items } = expr else {
        return None;
    };

    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let MExpr::List { items: pair } = it else {
            return None;
        };
        if pair.len() < 2 {
            return None;
        }
        let MExpr::Primitive(MPrimitive::String(col)) = &pair[0] else {
            return None;
        };
        let ty_hash = hash_expr_signature(&pair[1], step_names);
        out.push(ColumnTypeChange {
            column: col.clone(),
            ty_hash,
        });
    }

    Some(out)
}

fn hash_expr_signature(expr: &MExpr, step_names: &BTreeSet<String>) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    let mut bound = BoundStack::default();
    hash_expr_signature_inner(expr, step_names, &mut bound, &mut h);
    h.finish()
}

fn hash_expr_signature_inner<'a>(
    expr: &'a MExpr,
    step_names: &BTreeSet<String>,
    bound: &mut BoundStack<'a>,
    h: &mut xxhash_rust::xxh64::Xxh64,
) {
    match expr {
        MExpr::Let { bindings, body } => {
            0u8.hash(h);
            bindings.len().hash(h);
            for b in bindings {
                0u8.hash(h);
                b.name.hash(h);
                hash_expr_signature_inner(&b.value, step_names, bound, h);
                bound.push(b.name.as_str());
            }
            hash_expr_signature_inner(body, step_names, bound, h);
            bound.pop_n(bindings.len());
        }
        MExpr::Record { fields } => {
            1u8.hash(h);
            fields.len().hash(h);
            for f in fields {
                f.name.hash(h);
                hash_expr_signature_inner(&f.value, step_names, bound, h);
            }
        }
        MExpr::List { items } => {
            2u8.hash(h);
            items.len().hash(h);
            for it in items {
                hash_expr_signature_inner(it, step_names, bound, h);
            }
        }
        MExpr::FunctionCall { name, args } => {
            3u8.hash(h);
            name.to_ascii_lowercase().hash(h);
            args.len().hash(h);
            for a in args {
                hash_expr_signature_inner(a, step_names, bound, h);
            }
        }
        MExpr::FunctionLiteral { params, body, return_type } => {
            4u8.hash(h);
            params.len().hash(h);
            for p in params {
                p.name.hash(h);
                if let Some(ty) = &p.ty {
                    ty.name.to_ascii_lowercase().hash(h);
                } else {
                    0u8.hash(h);
                }
                bound.push(p.name.as_str());
            }
            if let Some(rt) = return_type {
                1u8.hash(h);
                rt.name.to_ascii_lowercase().hash(h);
            } else {
                0u8.hash(h);
            }
            hash_expr_signature_inner(body, step_names, bound, h);
            bound.pop_n(params.len());
        }
        MExpr::UnaryOp { op, expr } => {
            5u8.hash(h);
            op.hash(h);
            hash_expr_signature_inner(expr, step_names, bound, h);
        }
        MExpr::BinaryOp { op, left, right } => {
            6u8.hash(h);
            op.hash(h);
            hash_expr_signature_inner(left, step_names, bound, h);
            hash_expr_signature_inner(right, step_names, bound, h);
        }
        MExpr::TypeAscription { expr, ty } => {
            7u8.hash(h);
            hash_expr_signature_inner(expr, step_names, bound, h);
            ty.name.to_ascii_lowercase().hash(h);
        }
        MExpr::TryOtherwise { expr, otherwise } => {
            8u8.hash(h);
            hash_expr_signature_inner(expr, step_names, bound, h);
            hash_expr_signature_inner(otherwise, step_names, bound, h);
        }
        MExpr::Ident { name } => {
            9u8.hash(h);
            if bound.contains(name) {
                0u8.hash(h);
                name.hash(h);
            } else if step_names.contains(name) {
                1u8.hash(h);
            } else {
                2u8.hash(h);
                name.hash(h);
            }
        }
        MExpr::If { cond, then_branch, else_branch } => {
            10u8.hash(h);
            hash_expr_signature_inner(cond, step_names, bound, h);
            hash_expr_signature_inner(then_branch, step_names, bound, h);
            hash_expr_signature_inner(else_branch, step_names, bound, h);
        }
        MExpr::Each { body } => {
            11u8.hash(h);
            bound.push("_");
            hash_expr_signature_inner(body, step_names, bound, h);
            bound.pop_n(1);
        }
        MExpr::Access { base, kind, key } => {
            12u8.hash(h);
            kind.hash(h);
            hash_expr_signature_inner(base, step_names, bound, h);
            hash_expr_signature_inner(key, step_names, bound, h);
        }
        MExpr::Primitive(p) => {
            13u8.hash(h);
            p.hash(h);
        }
        MExpr::Opaque(tokens) => {
            14u8.hash(h);
            tokens.len().hash(h);
            for t in tokens {
                match t {
                    MToken::Identifier(id) => {
                        0u8.hash(h);
                        if bound.contains(id) {
                            0u8.hash(h);
                            id.hash(h);
                        } else if step_names.contains(id) {
                            1u8.hash(h);
                        } else {
                            2u8.hash(h);
                            id.hash(h);
                        }
                    }
                    MToken::StringLiteral(s) => {
                        1u8.hash(h);
                        s.hash(h);
                    }
                    MToken::Number(n) => {
                        2u8.hash(h);
                        n.hash(h);
                    }
                    MToken::Symbol(c) => {
                        3u8.hash(h);
                        c.hash(h);
                    }
                    _ => {
                        4u8.hash(h);
                        t.hash(h);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_filter_step_and_dependency() {
        let expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Filtered Rows" = Table.SelectRows(Source, each [Amount] > 0)
            in
                #"Filtered Rows"
        "#;

        let pipeline = extract_steps(expr).expect("pipeline should extract");
        assert_eq!(pipeline.steps.len(), 2);
        assert_eq!(pipeline.output_ref.as_deref(), Some("Filtered Rows"));

        assert_eq!(pipeline.steps[0].name, "Source");
        assert_eq!(pipeline.steps[1].name, "Filtered Rows");

        match &pipeline.steps[1].kind {
            StepKind::TableSelectRows { .. } => {}
            other => panic!("expected TableSelectRows, got {:?}", other),
        }

        assert_eq!(pipeline.steps[1].source_refs, vec!["Source".to_string()]);
    }

    #[test]
    fn extracts_remove_columns_params() {
        let expr = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"A", "B"})
            in
                #"Removed Columns"
        "#;

        let pipeline = extract_steps(expr).expect("pipeline should extract");
        assert_eq!(pipeline.steps.len(), 2);

        match &pipeline.steps[1].kind {
            StepKind::TableRemoveColumns { columns, .. } => match columns {
                Extracted::Known(cols) => assert_eq!(cols, &vec!["A".to_string(), "B".to_string()]),
                other => panic!("expected Known columns, got {:?}", other),
            },
            other => panic!("expected TableRemoveColumns, got {:?}", other),
        }
    }

    #[test]
    fn signatures_survive_step_rename_and_reference_update() {
        let a = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Removed Columns" = Table.RemoveColumns(Source, {"A", "B"}),
                #"Changed Type" = Table.TransformColumnTypes(#"Removed Columns", {{"C", type text}})
            in
                #"Changed Type"
        "#;

        let b = r#"
            let
                Source = Excel.CurrentWorkbook(){[Name="Table1"]}[Content],
                #"Dropped Columns" = Table.RemoveColumns(Source, {"A", "B"}),
                #"Changed Type" = Table.TransformColumnTypes(#"Dropped Columns", {{"C", type text}})
            in
                #"Changed Type"
        "#;

        let pa = extract_steps(a).expect("pipeline should extract");
        let pb = extract_steps(b).expect("pipeline should extract");

        assert_eq!(pa.steps.len(), 3);
        assert_eq!(pb.steps.len(), 3);

        let removed_a = &pa.steps[1];
        let removed_b = &pb.steps[1];
        assert_eq!(removed_a.signature, removed_b.signature);

        let changed_a = &pa.steps[2];
        let changed_b = &pb.steps[2];
        assert_eq!(changed_a.signature, changed_b.signature);
    }
}
```

This single module does the branch-4 essentials:

* Parse → canonicalize → walk AST 
* Identify binding order (steps are in binding order) 
* Extract dependencies via identifier references (including inside `Opaque` tokens as a fallback) 
* Classify the five high-value transforms 
* Produce rename-resilient signatures by normalizing step references during hashing 

---

## How this satisfies “no breakage” for query extraction

Nothing above touches `parse_section_members` or `build_queries`. Those functions remain the only mechanism for turning `Section1.m` into `(query name, expression_m)` today. 

As a sanity check for the branch, run the existing query-domain tests (they already cover quoted identifiers and metadata join behavior). 

---

## Edge cases and decisions (explicit)

### Supported well

* Step names with spaces / `#"<name>"` patterns:

  * tokenization and ident parsing already treat these as identifiers with the inner string, so dependencies work. 
* Access-chain sources like `Excel.CurrentWorkbook(){...}[Content]`:

  * step kind will be `Other` (fine), dependencies are still extracted (likely none).
* Nested `let` inside step values:

  * dependency walker treats nested let binding names as locally bound to avoid false step dependencies.

### Intentionally “best effort” for now

* Type expressions like `type text`:

  * your parser often represents these as `Opaque` today, so we hash them via normalized hashing rather than trying to interpret them structurally. 
* True alpha-renaming invariance for nested local lets:

  * not required by the plan; step rename invariance is the main target. 

---

## Branch 4 definition-of-done checklist

To match the plan’s definition of done , you should be able to do this in a test (or a debug hook later):

* Input: a typical Power Query `let` expression (from `Query.expression_m`) 
* Output: ordered steps with:

  * names
  * kind (one of the recognized `Table.*` kinds or `Other`)
  * extracted key params when applicable (columns, rename pairs, join keys, etc.)
  * dependencies (`source_refs`)
  * stable signatures that survive renames

That’s exactly what the code and tests above enforce.

---

## Ready integration points for branch 5

Even though branch 4 doesn’t change the diff schema, it should be implemented so branch 5 can plug it in cleanly:

* `m_diff` already computes canonical AST hashes and classifies formatting-only vs semantic changes. 
* Branch 5 will:

  * call `extract_steps(old_expr)` / `extract_steps(new_expr)`
  * align steps using `signature` + order
  * emit structured “step added/removed/modified” semantic detail 

The `StepPipeline` surface above is built specifically to make that next step straightforward.
