//! Configuration for the diff engine.
//!
//! `DiffConfig` centralizes all algorithm thresholds and behavioral knobs
//! to avoid hardcoded constants scattered throughout the codebase.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitBehavior {
    FallbackToPositional,
    ReturnPartialResult,
    ReturnError,
}

#[derive(Debug, Clone)]
pub struct DiffConfig {
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
    pub max_align_cols: u32,
    pub max_block_gap: u32,
    pub max_hash_repeat: u32,
    pub fuzzy_similarity_threshold: f64,
    pub max_fuzzy_block_rows: u32,
    pub rare_threshold: u32,
    pub low_info_threshold: u32,
    pub small_gap_threshold: u32,
    pub recursive_align_threshold: u32,
    pub max_recursion_depth: u32,
    pub on_limit_exceeded: LimitBehavior,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_move_iterations: 20,
            max_align_rows: 10_000,
            max_align_cols: 16_384,
            max_block_gap: 10_000,
            max_hash_repeat: 8,
            fuzzy_similarity_threshold: 0.80,
            max_fuzzy_block_rows: 32,
            rare_threshold: 5,
            low_info_threshold: 1,
            small_gap_threshold: 50,
            recursive_align_threshold: 200,
            max_recursion_depth: 10,
            on_limit_exceeded: LimitBehavior::FallbackToPositional,
        }
    }
}

impl DiffConfig {
    pub fn fastest() -> Self {
        Self {
            max_move_iterations: 5,
            max_block_gap: 1_000,
            small_gap_threshold: 20,
            recursive_align_threshold: 80,
            ..Default::default()
        }
    }

    pub fn balanced() -> Self {
        Self::default()
    }

    pub fn most_precise() -> Self {
        Self {
            max_move_iterations: 30,
            max_block_gap: 20_000,
            fuzzy_similarity_threshold: 0.90,
            small_gap_threshold: 80,
            recursive_align_threshold: 400,
            ..Default::default()
        }
    }
}
