use crate::string_pool::StringPool;

/// Holds shared diffing state such as the string pool.
pub struct DiffSession {
    pub strings: StringPool,
}

impl DiffSession {
    pub fn new() -> Self {
        Self {
            strings: StringPool::new(),
        }
    }

    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    pub fn strings_mut(&mut self) -> &mut StringPool {
        &mut self.strings
    }
}
