use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

use crate::config::DiffConfig;
use crate::datamashup::{DataMashup, Query, build_queries};
use crate::diff::{DiffOp, QueryChangeKind as DiffQueryChangeKind, QueryMetadataField};
use crate::hashing::XXH64_SEED;
use crate::m_ast::{MModuleAst, canonicalize_m_ast, parse_m_expression};
use crate::string_pool::{StringId, StringPool};

#[deprecated(note = "use WorkbookPackage::diff instead")]
pub fn diff_m_queries(old_queries: &[Query], new_queries: &[Query], config: &DiffConfig) -> Vec<DiffOp> {
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

fn canonical_ast_and_hash(expr: &str) -> Option<(MModuleAst, u64)> {
    let mut ast = parse_m_expression(expr).ok()?;
    canonicalize_m_ast(&mut ast);
    let h = hash64(&ast);
    Some((ast, h))
}

fn definition_change(
    old_expr: &str,
    new_expr: &str,
    enable_semantic: bool,
) -> Option<(DiffQueryChangeKind, u64, u64)> {
    if old_expr == new_expr {
        return None;
    }

    if enable_semantic {
        if let (Some((_, old_h)), Some((_, new_h))) =
            (canonical_ast_and_hash(old_expr), canonical_ast_and_hash(new_expr))
        {
            let kind = if old_h == new_h {
                DiffQueryChangeKind::FormattingOnly
            } else {
                DiffQueryChangeKind::Semantic
            };
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

    let mut old_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();
    let mut new_hash_map: BTreeMap<u64, Vec<&Query>> = BTreeMap::new();

    for q in &old_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m)
                .map(|(_, h)| h)
                .unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        old_hash_map.entry(h).or_default().push(*q);
    }

    for q in &new_only {
        let h = if config.enable_m_semantic_diff {
            canonical_ast_and_hash(&q.expression_m)
                .map(|(_, h)| h)
                .unwrap_or_else(|| hash64(&q.expression_m))
        } else {
            hash64(&q.expression_m)
        };
        new_hash_map.entry(h).or_default().push(*q);
    }

    for (h, olds) in &old_hash_map {
        if let Some(news) = new_hash_map.get(h) {
            if olds.len() == 1 && news.len() == 1 {
                let old_q = olds[0];
                let new_q = news[0];
                let from = pool.intern(old_q.name.as_str());
                let to = pool.intern(new_q.name.as_str());
                renamed_old.insert(old_q.name.as_str());
                renamed_new.insert(new_q.name.as_str());
                rename_ops.push((from, to, old_q, new_q));
            }
        }
    }

    rename_ops.sort_by(|a, b| {
        let from_a = pool.resolve(a.0);
        let from_b = pool.resolve(b.0);
        from_a.cmp(from_b)
    });

    let mut ops: Vec<DiffOp> = Vec::new();

    for (from, to, old_q, new_q) in rename_ops {
        ops.push(DiffOp::QueryRenamed { from, to });
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
                ) {
                    ops.push(DiffOp::QueryDefinitionChanged {
                        name: name_id,
                        change_kind: kind,
                        old_hash: old_h,
                        new_hash: new_h,
                    });
                }

                emit_metadata_diffs(pool, &mut ops, name_id, old_q, new_q);
            }
            (None, None) => {}
        }
    }

    ops
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
            let old_q = match build_queries(old_dm) {
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
            let new_q = match build_queries(new_dm) {
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
            let old_q = match build_queries(old_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            let new_q = match build_queries(new_dm) {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            };
            diff_queries_to_ops(&old_q, &new_q, pool, config)
        }
    }
}
