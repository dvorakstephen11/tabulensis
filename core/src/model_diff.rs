use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use crate::diff::DiffOp;
use crate::hashing::XXH64_SEED;
use crate::model::Model;
use crate::string_pool::{StringId, StringPool};

fn hash64<T: Hash>(value: &T) -> u64 {
    let mut h = xxhash_rust::xxh64::Xxh64::new(XXH64_SEED);
    value.hash(&mut h);
    h.finish()
}

/// Diff two tabular models at a minimal "measure" level.
pub fn diff_models(old: &Model, new: &Model, pool: &mut StringPool) -> Vec<DiffOp> {
    let mut ops = Vec::new();

    let mut old_by_name: BTreeMap<StringId, StringId> = BTreeMap::new();
    for measure in &old.measures {
        old_by_name.insert(measure.name, measure.expression);
    }

    let mut new_by_name: BTreeMap<StringId, StringId> = BTreeMap::new();
    for measure in &new.measures {
        new_by_name.insert(measure.name, measure.expression);
    }

    for name in old_by_name.keys().chain(new_by_name.keys()) {
        match (old_by_name.get(name), new_by_name.get(name)) {
            (Some(_old_expr), None) => ops.push(DiffOp::MeasureRemoved { name: *name }),
            (None, Some(_new_expr)) => ops.push(DiffOp::MeasureAdded { name: *name }),
            (Some(old_expr), Some(new_expr)) => {
                if old_expr != new_expr {
                    let old_hash = hash64(&pool.resolve(*old_expr));
                    let new_hash = hash64(&pool.resolve(*new_expr));
                    ops.push(DiffOp::MeasureDefinitionChanged {
                        name: *name,
                        old_hash,
                        new_hash,
                    });
                }
            }
            (None, None) => {}
        }
    }

    ops
}
