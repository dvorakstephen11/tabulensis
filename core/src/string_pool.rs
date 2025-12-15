use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StringId(pub u32);

impl std::fmt::Display for StringId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default)]
pub struct StringPool {
    strings: Vec<String>,
    index: FxHashMap<String, StringId>,
}

impl StringPool {
    pub fn new() -> Self {
        let mut pool = Self::default();
        pool.intern("");
        pool
    }

    pub fn intern(&mut self, s: &str) -> StringId {
        if let Some(&id) = self.index.get(s) {
            return id;
        }

        let id = StringId(self.strings.len() as u32);
        let owned = s.to_owned();
        self.strings.push(owned.clone());
        self.index.insert(owned, id);
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
}
