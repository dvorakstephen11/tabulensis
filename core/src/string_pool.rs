use rustc_hash::{FxHashMap, FxHasher};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
enum Bucket {
    One(StringId),
    Many(Vec<StringId>),
}

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<u64, Bucket>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.intern("");
        pool
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        let h = hash_str(s);
        let strings = &mut self.strings;
        let index = &mut self.index;

        if let Some(bucket) = index.get_mut(&h) {
            match bucket {
                Bucket::One(existing) => {
                    let id = *existing;
                    if strings[id.0 as usize] == s {
                        return id;
                    }
                    let new_id = StringId(strings.len() as u32);
                    strings.push(s.to_owned());
                    let mut ids = Vec::with_capacity(2);
                    ids.push(id);
                    ids.push(new_id);
                    *bucket = Bucket::Many(ids);
                    return new_id;
                }
                Bucket::Many(ids) => {
                    for &id in ids.iter() {
                        if strings[id.0 as usize] == s {
                            return id;
                        }
                    }
                    let new_id = StringId(strings.len() as u32);
                    strings.push(s.to_owned());
                    ids.push(new_id);
                    return new_id;
                }
            }
        }

        let id = StringId(strings.len() as u32);
        strings.push(s.to_owned());
        index.insert(h, Bucket::One(id));
        id
    }

    pub fn resolve(&self, id: StringId) -> &str {
        &self.strings[id.0 as usize]
    }

    pub fn strings(&self) -> &[String] {
        &self.strings
    }

    pub fn into_strings(self) -> Vec<String> {
        self.strings
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn estimated_bytes(&self) -> u64 {
        use std::mem::size_of;

        let strings_overhead = self.strings.capacity().saturating_mul(size_of::<String>());
        let strings_payload: usize = self.strings.iter().map(|s| s.capacity()).sum();
        let index_overhead = self
            .index
            .capacity()
            .saturating_mul(size_of::<(u64, Bucket)>());

        let mut collision_payload = 0usize;
        for bucket in self.index.values() {
            if let Bucket::Many(ids) = bucket {
                collision_payload = collision_payload
                    .saturating_add(ids.capacity().saturating_mul(size_of::<StringId>()));
            }
        }

        (strings_overhead + strings_payload + index_overhead + collision_payload) as u64
    }
}

fn hash_str(s: &str) -> u64 {
    let mut hasher = FxHasher::default();
    s.hash(&mut hasher);
    hasher.finish()
}
