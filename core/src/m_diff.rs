use std::collections::HashMap;

use crate::datamashup::{DataMashup, Query, build_queries};
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
                } else {
                    diffs.push(MQueryDiff {
                        name,
                        kind: QueryChangeKind::DefinitionChanged,
                    });
                }
            }
            (None, None) => {
                debug_assert!(false, "query name missing from both maps");
            }
        }
    }

    diffs
}
