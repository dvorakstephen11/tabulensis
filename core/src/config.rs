//! Configuration for the diff engine.
//!
//! `DiffConfig` centralizes all algorithm thresholds and behavioral knobs
//! to avoid hardcoded constants scattered throughout the codebase.

#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
    pub max_align_cols: u32,
    pub max_block_gap: u32,
    pub max_hash_repeat: u32,
    pub fuzzy_similarity_threshold: f64,
    pub max_fuzzy_block_rows: u32,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_move_iterations: 10,
            max_align_rows: 2_000,
            max_align_cols: 64,
            max_block_gap: 32,
            max_hash_repeat: 8,
            fuzzy_similarity_threshold: 0.80,
            max_fuzzy_block_rows: 32,
        }
    }
}

impl DiffConfig {
    pub fn fastest() -> Self {
        Self {
            max_move_iterations: 3,
            max_block_gap: 16,
            ..Default::default()
        }
    }

    pub fn balanced() -> Self {
        Self::default()
    }

    pub fn most_precise() -> Self {
        Self {
            max_move_iterations: 20,
            max_block_gap: 64,
            fuzzy_similarity_threshold: 0.90,
            ..Default::default()
        }
    }
}
