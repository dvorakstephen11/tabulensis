use std::collections::HashMap;

use crate::datamashup::{DataMashup, Query, build_queries};
use crate::m_ast::{ast_semantically_equal, canonicalize_m_ast, parse_m_expression};
use crate::m_section::SectionParseError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryChangeKind {
    Added,
    Removed,
    Renamed { from: String, to: String }, // present for forward compatibility; not emitted yet
    DefinitionChanged,
    MetadataChangedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MQueryDiff {
    pub name: String,
    pub kind: QueryChangeKind,
}

pub fn diff_m_queries(
    old_dm: &DataMashup,
    new_dm: &DataMashup,
) -> Result<Vec<MQueryDiff>, SectionParseError> {
    let old_queries = build_queries(old_dm)?;
    let new_queries = build_queries(new_dm)?;
    Ok(diff_queries(&old_queries, &new_queries))
}

fn diff_queries(old_queries: &[Query], new_queries: &[Query]) -> Vec<MQueryDiff> {
    let mut old_map: HashMap<String, &Query> = HashMap::new();
    for query in old_queries {
        old_map.insert(query.name.clone(), query);
    }

    let mut new_map: HashMap<String, &Query> = HashMap::new();
    for query in new_queries {
        new_map.insert(query.name.clone(), query);
    }

    let mut names: Vec<String> = old_map.keys().chain(new_map.keys()).cloned().collect();
    names.sort();
    names.dedup();

    let mut diffs = Vec::new();
    for name in names {
        match (old_map.get(&name), new_map.get(&name)) {
            (None, Some(_)) => diffs.push(MQueryDiff {
                name,
                kind: QueryChangeKind::Added,
            }),
            (Some(_), None) => diffs.push(MQueryDiff {
                name,
                kind: QueryChangeKind::Removed,
            }),
            (Some(old_q), Some(new_q)) => {
                if old_q.expression_m == new_q.expression_m {
                    if old_q.metadata != new_q.metadata {
                        diffs.push(MQueryDiff {
                            name,
                            kind: QueryChangeKind::MetadataChangedOnly,
                        });
                    }
                    continue;
                }

                if old_q.metadata == new_q.metadata
                    && expressions_semantically_equal(&old_q.expression_m, &new_q.expression_m)
                {
                    continue;
                }

                diffs.push(MQueryDiff {
                    name,
                    kind: QueryChangeKind::DefinitionChanged,
                });
            }
            (None, None) => {
                debug_assert!(false, "query name missing from both maps");
            }
        }
    }

    diffs
}

fn expressions_semantically_equal(old_expr: &str, new_expr: &str) -> bool {
    let Ok(mut old_ast) = parse_m_expression(old_expr) else {
        return false;
    };
    let Ok(mut new_ast) = parse_m_expression(new_expr) else {
        return false;
    };

    canonicalize_m_ast(&mut old_ast);
    canonicalize_m_ast(&mut new_ast);

    ast_semantically_equal(&old_ast, &new_ast)
}
