use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

use crate::config::{DiffConfig, SemanticNoisePolicy};
use crate::datamashup::{DataMashup, Query, build_embedded_queries, build_queries};
use crate::diff::{DiffOp, QueryChangeKind as DiffQueryChangeKind, QueryMetadataField};
use crate::diffable::{DiffContext, Diffable};
use crate::hashing::XXH64_SEED;
use crate::m_ast::{StepKind, canonicalize_m_ast, extract_steps, parse_m_expression};
use crate::matching::hungarian;
use crate::string_pool::{StringId, StringPool};
use crate::m_section::SectionParseError;

#[deprecated(note = "use WorkbookPackage::diff instead")]
#[cfg(any(test, feature = "dev-apis"))]
pub fn diff_m_queries(
    old_queries: &[Query],
    new_queries: &[Query],
    config: &DiffConfig,
) -> Vec<DiffOp> {
    crate::with_default_session(|session| {
        diff_queries_to_ops(old_queries, new_queries, &mut session.strings, config)
    })
}

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

fn intern_bool(pool: &mut StringPool, v: bool) -> StringId {
    if v {
        pool.intern("true")
    } else {
        pool.intern("false")
    }
}

fn canonical_ast_and_hash(expr: &str) -> Option<u64> {
    let mut ast = parse_m_expression(expr).ok()?;
    canonicalize_m_ast(&mut ast);
    Some(hash64(&ast))
}

#[derive(Clone)]
struct QuerySignature {
    ast_hash: Option<u64>,
    expr_hash: u64,
    step_counts: Option<HashMap<u8, u32>>,
}

fn build_query_signature(query: &Query) -> QuerySignature {
    let ast_hash = canonical_ast_and_hash(&query.expression_m);
    let expr_hash = hash64(&query.expression_m);
    let step_counts = extract_steps(&query.expression_m).map(|pipeline| {
        let mut counts = HashMap::new();
        for step in &pipeline.steps {
            let tag = step_kind_tag(&step.kind);
            *counts.entry(tag).or_insert(0) += 1;
        }
        counts
    });

    QuerySignature {
        ast_hash,
        expr_hash,
        step_counts,
    }
}

fn step_kind_tag(kind: &StepKind) -> u8 {
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

fn multiset_similarity(a: &HashMap<u8, u32>, b: &HashMap<u8, u32>) -> f64 {
    let mut union = 0u32;
    let mut intersection = 0u32;

    for (k, av) in a {
        let av = *av;
        let bv = b.get(k).copied().unwrap_or(0);
        union += av.max(bv);
        intersection += av.min(bv);
    }
    for (k, bv) in b {
        if !a.contains_key(k) {
            union += *bv;
        }
    }

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn query_similarity(a: &QuerySignature, b: &QuerySignature) -> f64 {
    let ast_sim = match (a.ast_hash, b.ast_hash) {
        (Some(ha), Some(hb)) => {
            if ha == hb {
                1.0
            } else {
                0.0
            }
        }
        _ => {
            if a.expr_hash == b.expr_hash {
                1.0
            } else {
                0.0
            }
        }
    };

    let mut parts = vec![ast_sim];
    if let (Some(sa), Some(sb)) = (&a.step_counts, &b.step_counts) {
        parts.push(multiset_similarity(sa, sb));
    }

    if parts.is_empty() {
        0.0
    } else {
        parts.iter().sum::<f64>() / parts.len() as f64
    }
}

fn definition_change(
    old_expr: &str,
    new_expr: &str,
    enable_semantic: bool,
    noise_policy: SemanticNoisePolicy,
) -> Option<(DiffQueryChangeKind, u64, u64)> {
    if old_expr == new_expr {
        return None;
    }

    if enable_semantic {
        if let (Some(old_h), Some(new_h)) =
            (canonical_ast_and_hash(old_expr), canonical_ast_and_hash(new_expr))
        {
            let kind = if old_h == new_h {
                DiffQueryChangeKind::FormattingOnly
            } else {
                DiffQueryChangeKind::Semantic
            };
            if kind == DiffQueryChangeKind::FormattingOnly
                && matches!(noise_policy, SemanticNoisePolicy::SuppressFormattingOnly)
            {
                return None;
            }
            return Some((kind, old_h, new_h));
        }
    }

    let old_h = hash64(&old_expr);
    let new_h = hash64(&new_expr);
    Some((DiffQueryChangeKind::Semantic, old_h, new_h))
}

fn emit_metadata_diffs(
    pool: &mut StringPool,
    out: &mut Vec<DiffOp>,
    name: StringId,
    old_q: &Query,
    new_q: &Query,
) {
    if old_q.metadata.load_to_sheet != new_q.metadata.load_to_sheet {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToSheet,
            old: Some(intern_bool(pool, old_q.metadata.load_to_sheet)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_sheet)),
        });
    }

    if old_q.metadata.load_to_model != new_q.metadata.load_to_model {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::LoadToModel,
            old: Some(intern_bool(pool, old_q.metadata.load_to_model)),
            new: Some(intern_bool(pool, new_q.metadata.load_to_model)),
        });
    }

    if old_q.metadata.is_connection_only != new_q.metadata.is_connection_only {
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::ConnectionOnly,
            old: Some(intern_bool(pool, old_q.metadata.is_connection_only)),
            new: Some(intern_bool(pool, new_q.metadata.is_connection_only)),
        });
    }

    if old_q.metadata.group_path != new_q.metadata.group_path {
        let old = old_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        let new = new_q.metadata.group_path.as_deref().map(|s| pool.intern(s));
        out.push(DiffOp::QueryMetadataChanged {
            name,
            field: QueryMetadataField::GroupPath,
            old,
            new,
        });
    }
}

fn match_query_renames(
    old_only: &[&Query],
    new_only: &[&Query],
    similarity_threshold: f64,
) -> Vec<(usize, usize)> {
    if old_only.is_empty() || new_only.is_empty() {
        return Vec::new();
    }

    let old_sigs: Vec<QuerySignature> = old_only.iter().map(|q| build_query_signature(q)).collect();
    let new_sigs: Vec<QuerySignature> = new_only.iter().map(|q| build_query_signature(q)).collect();

    let rows = old_only.len();
    let cols = new_only.len();
    let size = rows.max(cols);
    let bias_scale = (size * size + 1) as i64;
    let cost_unit = 1000i64;
    let pad_cost = cost_unit.saturating_mul(bias_scale).saturating_mul(10);
    let reject_cost = pad_cost.saturating_add(bias_scale);

    let mut costs = vec![vec![pad_cost; cols]; rows];
    let mut sims = vec![vec![0.0f64; cols]; rows];

    for i in 0..rows {
        for j in 0..cols {
            let sim = query_similarity(&old_sigs[i], &new_sigs[j]);
            sims[i][j] = sim;
            if sim >= similarity_threshold {
                let base_cost = ((1.0 - sim) * cost_unit as f64).round() as i64;
                let bias = (i as i64).saturating_mul(size as i64).saturating_add(j as i64);
                costs[i][j] = base_cost.saturating_mul(bias_scale).saturating_add(bias);
            } else {
                costs[i][j] = reject_cost;
            }
        }
    }

    let assignment = hungarian::solve_rect(&costs, pad_cost);
    let mut matches = Vec::new();

    for i in 0..rows {
        let j = assignment.get(i).copied().unwrap_or(cols);
        if j < cols && sims[i][j] >= similarity_threshold {
            matches.push((i, j));
        }
    }

    matches
}

fn diff_queries_to_ops(
    old_queries: &[Query],
    new_queries: &[Query],
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    let mut old_by_name: BTreeMap<&str, &Query> = BTreeMap::new();
    let mut new_by_name: BTreeMap<&str, &Query> = BTreeMap::new();

    for q in old_queries {
        old_by_name.insert(q.name.as_str(), q);
    }
    for q in new_queries {
        new_by_name.insert(q.name.as_str(), q);
    }

    let old_only: Vec<&Query> = old_by_name
        .iter()
        .filter_map(|(name, q)| {
            if new_by_name.contains_key(*name) {
                None
            } else {
                Some(*q)
            }
        })
        .collect();

    let new_only: Vec<&Query> = new_by_name
        .iter()
        .filter_map(|(name, q)| {
            if old_by_name.contains_key(*name) {
                None
            } else {
                Some(*q)
            }
        })
        .collect();

    let mut renamed_old: BTreeSet<&str> = BTreeSet::new();
    let mut renamed_new: BTreeSet<&str> = BTreeSet::new();
    let mut rename_ops: Vec<(StringId, StringId, &Query, &Query)> = Vec::new();
    let rename_pairs = match_query_renames(
        &old_only,
        &new_only,
        config.fuzzy_similarity_threshold,
    );
    for (old_idx, new_idx) in rename_pairs {
        let old_q = old_only[old_idx];
        let new_q = new_only[new_idx];
        let from = pool.intern(old_q.name.as_str());
        let to = pool.intern(new_q.name.as_str());
        renamed_old.insert(old_q.name.as_str());
        renamed_new.insert(new_q.name.as_str());
        rename_ops.push((from, to, old_q, new_q));
    }

    rename_ops.sort_by(|a, b| {
        let from_a = pool.resolve(a.0);
        let from_b = pool.resolve(b.0);
        from_a.cmp(from_b)
    });

    let mut ops: Vec<DiffOp> = Vec::new();

    for (from, to, old_q, new_q) in rename_ops {
        ops.push(DiffOp::QueryRenamed { from, to });
        if let Some((kind, old_h, new_h)) = definition_change(
            &old_q.expression_m,
            &new_q.expression_m,
            config.enable_m_semantic_diff,
            config.semantic_noise_policy,
        ) {
            let semantic_detail =
                if config.enable_m_semantic_diff && kind == DiffQueryChangeKind::Semantic {
                    crate::m_semantic_detail::build_query_semantic_detail(
                        &old_q.expression_m,
                        &new_q.expression_m,
                        pool,
                    )
                } else {
                    None
                };
            ops.push(DiffOp::QueryDefinitionChanged {
                name: to,
                change_kind: kind,
                old_hash: old_h,
                new_hash: new_h,
                semantic_detail,
            });
        }
        emit_metadata_diffs(pool, &mut ops, to, old_q, new_q);
    }

    let mut all_names: Vec<&str> = old_by_name
        .keys()
        .copied()
        .chain(new_by_name.keys().copied())
        .collect();
    all_names.sort();
    all_names.dedup();

    for name in all_names {
        if renamed_old.contains(name) || renamed_new.contains(name) {
            continue;
        }

        match (old_by_name.get(name), new_by_name.get(name)) {
            (None, Some(_new_q)) => {
                ops.push(DiffOp::QueryAdded {
                    name: pool.intern(name),
                });
            }
            (Some(_old_q), None) => {
                ops.push(DiffOp::QueryRemoved {
                    name: pool.intern(name),
                });
            }
            (Some(old_q), Some(new_q)) => {
                let name_id = pool.intern(name);

                if let Some((kind, old_h, new_h)) = definition_change(
                    &old_q.expression_m,
                    &new_q.expression_m,
                    config.enable_m_semantic_diff,
                    config.semantic_noise_policy,
                ) {
                    let semantic_detail =
                        if config.enable_m_semantic_diff && kind == DiffQueryChangeKind::Semantic
                        {
                            crate::m_semantic_detail::build_query_semantic_detail(
                                &old_q.expression_m,
                                &new_q.expression_m,
                                pool,
                            )
                        } else {
                            None
                        };

                    ops.push(DiffOp::QueryDefinitionChanged {
                        name: name_id,
                        change_kind: kind,
                        old_hash: old_h,
                        new_hash: new_h,
                        semantic_detail,
                    });
                }

                emit_metadata_diffs(pool, &mut ops, name_id, old_q, new_q);
            }
            (None, None) => {}
        }
    }

    ops
}

fn build_all_queries(dm: &DataMashup) -> Result<Vec<Query>, SectionParseError> {
    let mut q = build_queries(dm)?;
    q.extend(build_embedded_queries(dm));
    Ok(q)
}

pub(crate) fn diff_m_ops_for_packages(
    old_dm: &Option<DataMashup>,
    new_dm: &Option<DataMashup>,
    pool: &mut StringPool,
    config: &DiffConfig,
) -> Vec<DiffOp> {
    match (old_dm.as_ref(), new_dm.as_ref()) {
        (None, None) => Vec::new(),
        (Some(old_dm), None) => {
            let old_q = match build_all_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in old_q {
                ops.push(DiffOp::QueryRemoved {
                    name: pool.intern(&q.name),
                });
            }
            ops
        }
        (None, Some(new_dm)) => {
            let new_q = match build_all_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let mut ops = Vec::new();
            for q in new_q {
                ops.push(DiffOp::QueryAdded {
                    name: pool.intern(&q.name),
                });
            }
            ops
        }
        (Some(old_dm), Some(new_dm)) => {
            let old_q = match build_all_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_all_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}

impl Diffable for Vec<Query> {
    type Output = Vec<DiffOp>;

    fn diff(&self, other: &Self, ctx: &mut DiffContext<'_>) -> Vec<DiffOp> {
        diff_queries_to_ops(self, other, ctx.pool, ctx.config)
    }
}
