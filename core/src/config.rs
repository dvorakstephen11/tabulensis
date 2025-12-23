//! Configuration for the diff engine.
//!
//! `DiffConfig` centralizes all algorithm thresholds and behavioral knobs
//! to avoid hardcoded constants scattered throughout the codebase.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitBehavior {
    FallbackToPositional,
    ReturnPartialResult,
    ReturnError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticNoisePolicy {
    SuppressFormattingOnly,
    ReportFormattingOnly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    /// Maximum number of masked move-detection iterations per sheet.
    /// Set to 0 to disable move detection and represent moves as insert/delete.
    pub max_move_iterations: u32,
    pub max_align_rows: u32,
    pub max_align_cols: u32,
    pub max_block_gap: u32,
    pub max_hash_repeat: u32,
    pub fuzzy_similarity_threshold: f64,
    pub max_fuzzy_block_rows: u32,
    #[serde(alias = "rare_frequency_threshold")]
    pub rare_threshold: u32,
    #[serde(alias = "low_info_cell_threshold")]
    pub low_info_threshold: u32,
    /// Row-count threshold for recursive gap alignment. Does not gate masked move detection.
    #[serde(alias = "recursive_threshold")]
    pub recursive_align_threshold: u32,
    pub small_gap_threshold: u32,
    pub max_recursion_depth: u32,
    pub on_limit_exceeded: LimitBehavior,
    pub enable_fuzzy_moves: bool,
    pub enable_m_semantic_diff: bool,
    pub enable_formula_semantic_diff: bool,
    /// Policy for handling formatting-only M changes when semantic diff is enabled.
    pub semantic_noise_policy: SemanticNoisePolicy,
    /// When true, emits CellEdited ops even when values are unchanged (diagnostic);
    /// downstream consumers should treat edits as semantic only if from != to.
    pub include_unchanged_cells: bool,
    /// Ratio of differing cells required to emit dense row/rect replacement ops.
    /// Set to 0.0 to disable dense replacement.
    pub dense_row_replace_ratio: f64,
    /// Minimum column count to consider dense row replacement.
    pub dense_row_replace_min_cols: u32,
    /// Minimum consecutive replaced rows to emit a RectReplaced op.
    pub dense_rect_replace_min_rows: u32,
    pub max_context_rows: u32,
    pub min_block_size_for_move: u32,
    pub max_lcs_gap_size: u32,
    pub lcs_dp_work_limit: usize,
    pub move_extraction_max_slice_len: u32,
    pub move_extraction_max_candidates_per_sig: u32,
    pub context_anchor_k1: u32,
    pub context_anchor_k2: u32,
    /// Masked move detection runs only when max(old.nrows, new.nrows) <= this.
    pub max_move_detection_rows: u32,
    /// Masked move detection runs only when max(old.ncols, new.ncols) <= this.
    pub max_move_detection_cols: u32,
    /// Preflight: minimum row count to consider short-circuit bailouts.
    /// Grids smaller than this always run full move detection/alignment.
    pub preflight_min_rows: u32,
    /// Preflight: maximum number of in-order row mismatches to trigger near-identical bailout.
    pub preflight_in_order_mismatch_max: u32,
    /// Preflight: minimum ratio of in-order matching rows (0.0..=1.0) for near-identical bailout.
    pub preflight_in_order_match_ratio_min: f64,
    /// Preflight: Jaccard similarity threshold below which grids are considered dissimilar
    /// and move detection/alignment are skipped.
    pub bailout_similarity_threshold: f64,
    /// Optional soft cap on estimated memory usage (in MB) for advanced strategies.
    ///
    /// When the estimate exceeds this cap, the engine falls back to positional diff for the
    /// affected sheet and marks the overall diff as incomplete with a warning.
    pub max_memory_mb: Option<u32>,
    /// Optional timeout (in seconds) for the diff engine.
    ///
    /// When exceeded, the engine aborts early, preserving any already-emitted ops, and marks the
    /// result as incomplete with a warning.
    pub timeout_seconds: Option<u32>,
    /// Optional maximum number of operations to emit.
    ///
    /// When the limit is reached, the engine stops emitting further ops and marks the result
    /// as incomplete with a warning. This bounds both time and memory for pathological "everything
    /// changed" cases.
    pub max_ops: Option<usize>,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            max_move_iterations: 20,
            max_align_rows: 500_000,
            max_align_cols: 16_384,
            max_block_gap: 10_000,
            max_hash_repeat: 8,
            fuzzy_similarity_threshold: 0.80,
            max_fuzzy_block_rows: 32,
            rare_threshold: 5,
            low_info_threshold: 2,
            small_gap_threshold: 50,
            recursive_align_threshold: 200,
            max_recursion_depth: 10,
            on_limit_exceeded: LimitBehavior::FallbackToPositional,
            enable_fuzzy_moves: true,
            enable_m_semantic_diff: true,
            enable_formula_semantic_diff: false,
            semantic_noise_policy: SemanticNoisePolicy::ReportFormattingOnly,
            include_unchanged_cells: false,
            dense_row_replace_ratio: 0.90,
            dense_row_replace_min_cols: 64,
            dense_rect_replace_min_rows: 4,
            max_context_rows: 3,
            min_block_size_for_move: 3,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            context_anchor_k1: 4,
            context_anchor_k2: 8,
            max_move_detection_rows: 200,
            max_move_detection_cols: 256,
            preflight_min_rows: 5000,
            preflight_in_order_mismatch_max: 32,
            preflight_in_order_match_ratio_min: 0.995,
            bailout_similarity_threshold: 0.05,
            max_memory_mb: None,
            timeout_seconds: None,
            max_ops: None,
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
            max_move_detection_rows: 80,
            enable_fuzzy_moves: false,
            enable_m_semantic_diff: false,
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
            fuzzy_similarity_threshold: 0.95,
            small_gap_threshold: 80,
            recursive_align_threshold: 400,
            enable_formula_semantic_diff: true,
            max_lcs_gap_size: 1_500,
            lcs_dp_work_limit: 20_000,
            move_extraction_max_slice_len: 10_000,
            move_extraction_max_candidates_per_sig: 16,
            max_move_detection_rows: 400,
            max_move_detection_cols: 256,
            ..Default::default()
        }
    }

    pub fn builder() -> DiffConfigBuilder {
        DiffConfigBuilder {
            inner: DiffConfig::default(),
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.fuzzy_similarity_threshold.is_finite()
            || self.fuzzy_similarity_threshold < 0.0
            || self.fuzzy_similarity_threshold > 1.0
        {
            return Err(ConfigError::InvalidFuzzySimilarity {
                value: self.fuzzy_similarity_threshold,
            });
        }

        ensure_non_zero_u32(self.max_align_rows, "max_align_rows")?;
        ensure_non_zero_u32(self.max_align_cols, "max_align_cols")?;
        ensure_non_zero_u32(self.max_lcs_gap_size, "max_lcs_gap_size")?;
        ensure_non_zero_u32(
            self.move_extraction_max_slice_len,
            "move_extraction_max_slice_len",
        )?;
        ensure_non_zero_u32(
            self.move_extraction_max_candidates_per_sig,
            "move_extraction_max_candidates_per_sig",
        )?;
        ensure_non_zero_u32(self.context_anchor_k1, "context_anchor_k1")?;
        ensure_non_zero_u32(self.context_anchor_k2, "context_anchor_k2")?;
        ensure_non_zero_u32(self.max_move_detection_rows, "max_move_detection_rows")?;
        ensure_non_zero_u32(self.max_move_detection_cols, "max_move_detection_cols")?;
        ensure_non_zero_u32(self.max_context_rows, "max_context_rows")?;
        ensure_non_zero_u32(self.min_block_size_for_move, "min_block_size_for_move")?;

        if self.lcs_dp_work_limit == 0 {
            return Err(ConfigError::NonPositiveLimit {
                field: "lcs_dp_work_limit",
                value: 0,
            });
        }

        if !self.preflight_in_order_match_ratio_min.is_finite()
            || self.preflight_in_order_match_ratio_min < 0.0
            || self.preflight_in_order_match_ratio_min > 1.0
        {
            return Err(ConfigError::InvalidPreflightRatio {
                value: self.preflight_in_order_match_ratio_min,
            });
        }

        if !self.dense_row_replace_ratio.is_finite()
            || self.dense_row_replace_ratio < 0.0
            || self.dense_row_replace_ratio > 1.0
        {
            return Err(ConfigError::InvalidDenseRowReplaceRatio {
                value: self.dense_row_replace_ratio,
            });
        }

        if !self.bailout_similarity_threshold.is_finite()
            || self.bailout_similarity_threshold < 0.0
            || self.bailout_similarity_threshold > 1.0
        {
            return Err(ConfigError::InvalidBailoutSimilarity {
                value: self.bailout_similarity_threshold,
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigError {
    #[error("fuzzy_similarity_threshold must be in [0.0, 1.0] and finite (got {value})")]
    InvalidFuzzySimilarity { value: f64 },
    #[error("{field} must be greater than zero (got {value})")]
    NonPositiveLimit { field: &'static str, value: u64 },
    #[error("preflight_in_order_match_ratio_min must be in [0.0, 1.0] and finite (got {value})")]
    InvalidPreflightRatio { value: f64 },
    #[error("dense_row_replace_ratio must be in [0.0, 1.0] and finite (got {value})")]
    InvalidDenseRowReplaceRatio { value: f64 },
    #[error("bailout_similarity_threshold must be in [0.0, 1.0] and finite (got {value})")]
    InvalidBailoutSimilarity { value: f64 },
}

fn ensure_non_zero_u32(value: u32, field: &'static str) -> Result<(), ConfigError> {
    if value == 0 {
        return Err(ConfigError::NonPositiveLimit {
            field,
            value: value as u64,
        });
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct DiffConfigBuilder {
    inner: DiffConfig,
}

impl Default for DiffConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffConfigBuilder {
    pub fn new() -> Self {
        DiffConfig::builder()
    }

    pub fn max_move_iterations(mut self, value: u32) -> Self {
        self.inner.max_move_iterations = value;
        self
    }

    pub fn max_align_rows(mut self, value: u32) -> Self {
        self.inner.max_align_rows = value;
        self
    }

    pub fn max_align_cols(mut self, value: u32) -> Self {
        self.inner.max_align_cols = value;
        self
    }

    pub fn max_block_gap(mut self, value: u32) -> Self {
        self.inner.max_block_gap = value;
        self
    }

    pub fn max_hash_repeat(mut self, value: u32) -> Self {
        self.inner.max_hash_repeat = value;
        self
    }

    pub fn fuzzy_similarity_threshold(mut self, value: f64) -> Self {
        self.inner.fuzzy_similarity_threshold = value;
        self
    }

    pub fn max_fuzzy_block_rows(mut self, value: u32) -> Self {
        self.inner.max_fuzzy_block_rows = value;
        self
    }

    pub fn rare_threshold(mut self, value: u32) -> Self {
        self.inner.rare_threshold = value;
        self
    }

    pub fn low_info_threshold(mut self, value: u32) -> Self {
        self.inner.low_info_threshold = value;
        self
    }

    pub fn recursive_align_threshold(mut self, value: u32) -> Self {
        self.inner.recursive_align_threshold = value;
        self
    }

    pub fn small_gap_threshold(mut self, value: u32) -> Self {
        self.inner.small_gap_threshold = value;
        self
    }

    pub fn max_recursion_depth(mut self, value: u32) -> Self {
        self.inner.max_recursion_depth = value;
        self
    }

    pub fn on_limit_exceeded(mut self, value: LimitBehavior) -> Self {
        self.inner.on_limit_exceeded = value;
        self
    }

    pub fn enable_fuzzy_moves(mut self, value: bool) -> Self {
        self.inner.enable_fuzzy_moves = value;
        self
    }

    pub fn enable_m_semantic_diff(mut self, value: bool) -> Self {
        self.inner.enable_m_semantic_diff = value;
        self
    }

    pub fn enable_formula_semantic_diff(mut self, value: bool) -> Self {
        self.inner.enable_formula_semantic_diff = value;
        self
    }

    pub fn semantic_noise_policy(mut self, value: SemanticNoisePolicy) -> Self {
        self.inner.semantic_noise_policy = value;
        self
    }

    pub fn include_unchanged_cells(mut self, value: bool) -> Self {
        self.inner.include_unchanged_cells = value;
        self
    }

    pub fn dense_row_replace_ratio(mut self, value: f64) -> Self {
        self.inner.dense_row_replace_ratio = value;
        self
    }

    pub fn dense_row_replace_min_cols(mut self, value: u32) -> Self {
        self.inner.dense_row_replace_min_cols = value;
        self
    }

    pub fn dense_rect_replace_min_rows(mut self, value: u32) -> Self {
        self.inner.dense_rect_replace_min_rows = value;
        self
    }

    pub fn max_context_rows(mut self, value: u32) -> Self {
        self.inner.max_context_rows = value;
        self
    }

    pub fn min_block_size_for_move(mut self, value: u32) -> Self {
        self.inner.min_block_size_for_move = value;
        self
    }

    pub fn max_lcs_gap_size(mut self, value: u32) -> Self {
        self.inner.max_lcs_gap_size = value;
        self
    }

    pub fn lcs_dp_work_limit(mut self, value: usize) -> Self {
        self.inner.lcs_dp_work_limit = value;
        self
    }

    pub fn move_extraction_max_slice_len(mut self, value: u32) -> Self {
        self.inner.move_extraction_max_slice_len = value;
        self
    }

    pub fn move_extraction_max_candidates_per_sig(mut self, value: u32) -> Self {
        self.inner.move_extraction_max_candidates_per_sig = value;
        self
    }

    pub fn context_anchor_k1(mut self, value: u32) -> Self {
        self.inner.context_anchor_k1 = value;
        self
    }

    pub fn context_anchor_k2(mut self, value: u32) -> Self {
        self.inner.context_anchor_k2 = value;
        self
    }

    pub fn max_move_detection_rows(mut self, value: u32) -> Self {
        self.inner.max_move_detection_rows = value;
        self
    }

    pub fn max_move_detection_cols(mut self, value: u32) -> Self {
        self.inner.max_move_detection_cols = value;
        self
    }

    pub fn preflight_min_rows(mut self, value: u32) -> Self {
        self.inner.preflight_min_rows = value;
        self
    }

    pub fn preflight_in_order_mismatch_max(mut self, value: u32) -> Self {
        self.inner.preflight_in_order_mismatch_max = value;
        self
    }

    pub fn preflight_in_order_match_ratio_min(mut self, value: f64) -> Self {
        self.inner.preflight_in_order_match_ratio_min = value;
        self
    }

    pub fn bailout_similarity_threshold(mut self, value: f64) -> Self {
        self.inner.bailout_similarity_threshold = value;
        self
    }

    pub fn max_memory_mb(mut self, value: Option<u32>) -> Self {
        self.inner.max_memory_mb = value;
        self
    }

    pub fn timeout_seconds(mut self, value: Option<u32>) -> Self {
        self.inner.timeout_seconds = value;
        self
    }

    pub fn max_ops(mut self, value: Option<usize>) -> Self {
        self.inner.max_ops = value;
        self
    }

    pub fn build(self) -> Result<DiffConfig, ConfigError> {
        self.inner.validate()?;
        Ok(self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_limit_spec() {
        let cfg = DiffConfig::default();

        assert_eq!(cfg.max_align_rows, 500_000);
        assert_eq!(cfg.max_align_cols, 16_384);
        assert_eq!(cfg.max_recursion_depth, 10);
        assert!(matches!(
            cfg.on_limit_exceeded,
            LimitBehavior::FallbackToPositional
        ));

        assert_eq!(cfg.fuzzy_similarity_threshold, 0.80);
        assert_eq!(cfg.min_block_size_for_move, 3);
        assert_eq!(cfg.max_move_iterations, 20);

        assert_eq!(cfg.recursive_align_threshold, 200);
        assert_eq!(cfg.small_gap_threshold, 50);
        assert_eq!(cfg.low_info_threshold, 2);
        assert_eq!(cfg.rare_threshold, 5);
        assert_eq!(cfg.max_block_gap, 10_000);

        assert_eq!(cfg.max_move_detection_rows, 200);
        assert_eq!(cfg.max_move_detection_cols, 256);

        assert_eq!(cfg.preflight_min_rows, 5000);
        assert_eq!(cfg.preflight_in_order_mismatch_max, 32);
        assert!((cfg.preflight_in_order_match_ratio_min - 0.995).abs() < f64::EPSILON);
        assert!((cfg.bailout_similarity_threshold - 0.05).abs() < f64::EPSILON);

        assert_eq!(cfg.max_memory_mb, None);
        assert_eq!(cfg.timeout_seconds, None);

        assert!(!cfg.include_unchanged_cells);
        assert!((cfg.dense_row_replace_ratio - 0.90).abs() < f64::EPSILON);
        assert_eq!(cfg.dense_row_replace_min_cols, 64);
        assert_eq!(cfg.dense_rect_replace_min_rows, 4);
        assert_eq!(cfg.max_context_rows, 3);

        assert!(cfg.enable_fuzzy_moves);
        assert!(cfg.enable_m_semantic_diff);
        assert!(!cfg.enable_formula_semantic_diff);
        assert!(matches!(
            cfg.semantic_noise_policy,
            SemanticNoisePolicy::ReportFormattingOnly
        ));
    }

    #[test]
    fn serde_roundtrip_preserves_defaults() {
        let cfg = DiffConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize default config");
        let parsed: DiffConfig = serde_json::from_str(&json).expect("deserialize default config");
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn serde_aliases_populate_fields() {
        let json = r#"{
            "rare_frequency_threshold": 9,
            "low_info_cell_threshold": 3,
            "recursive_threshold": 123
        }"#;
        let cfg: DiffConfig = serde_json::from_str(json).expect("deserialize with aliases");
        assert_eq!(cfg.rare_threshold, 9);
        assert_eq!(cfg.low_info_threshold, 3);
        assert_eq!(cfg.recursive_align_threshold, 123);
    }

    #[test]
    fn builder_rejects_invalid_similarity_threshold() {
        let err = DiffConfig::builder()
            .fuzzy_similarity_threshold(2.0)
            .build()
            .expect_err("builder should reject invalid probability");
        assert!(matches!(
            err,
            ConfigError::InvalidFuzzySimilarity { value } if (value - 2.0).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn presets_differ_in_expected_directions() {
        let fastest = DiffConfig::fastest();
        let balanced = DiffConfig::balanced();
        let precise = DiffConfig::most_precise();

        assert!(!fastest.enable_fuzzy_moves);
        assert!(!fastest.enable_m_semantic_diff);
        assert!(precise.max_move_iterations >= balanced.max_move_iterations);
        assert!(precise.max_block_gap >= balanced.max_block_gap);
        assert!(precise.fuzzy_similarity_threshold >= balanced.fuzzy_similarity_threshold);
    }

    #[test]
    fn most_precise_matches_sprint_plan_values() {
        let cfg = DiffConfig::most_precise();
        assert_eq!(cfg.fuzzy_similarity_threshold, 0.95);
        assert!(cfg.enable_formula_semantic_diff);
    }

    #[test]
    fn builder_rejects_invalid_preflight_ratio() {
        let err = DiffConfig::builder()
            .preflight_in_order_match_ratio_min(1.5)
            .build()
            .expect_err("builder should reject invalid preflight ratio");
        assert!(matches!(
            err,
            ConfigError::InvalidPreflightRatio { value } if (value - 1.5).abs() < f64::EPSILON
        ));

        let err = DiffConfig::builder()
            .preflight_in_order_match_ratio_min(-0.1)
            .build()
            .expect_err("builder should reject negative preflight ratio");
        assert!(matches!(err, ConfigError::InvalidPreflightRatio { .. }));
    }

    #[test]
    fn builder_rejects_invalid_dense_row_replace_ratio() {
        let err = DiffConfig::builder()
            .dense_row_replace_ratio(2.0)
            .build()
            .expect_err("builder should reject invalid dense row replace ratio");
        assert!(matches!(
            err,
            ConfigError::InvalidDenseRowReplaceRatio { value } if (value - 2.0).abs() < f64::EPSILON
        ));

        let err = DiffConfig::builder()
            .dense_row_replace_ratio(-0.5)
            .build()
            .expect_err("builder should reject negative dense row replace ratio");
        assert!(matches!(err, ConfigError::InvalidDenseRowReplaceRatio { .. }));
    }

    #[test]
    fn builder_rejects_invalid_bailout_similarity() {
        let err = DiffConfig::builder()
            .bailout_similarity_threshold(2.0)
            .build()
            .expect_err("builder should reject invalid bailout similarity");
        assert!(matches!(
            err,
            ConfigError::InvalidBailoutSimilarity { value } if (value - 2.0).abs() < f64::EPSILON
        ));

        let err = DiffConfig::builder()
            .bailout_similarity_threshold(-0.5)
            .build()
            .expect_err("builder should reject negative bailout similarity");
        assert!(matches!(err, ConfigError::InvalidBailoutSimilarity { .. }));
    }

    #[test]
    fn preflight_config_builder_setters_work() {
        let cfg = DiffConfig::builder()
            .preflight_min_rows(10000)
            .preflight_in_order_mismatch_max(64)
            .preflight_in_order_match_ratio_min(0.99)
            .bailout_similarity_threshold(0.10)
            .max_memory_mb(Some(64))
            .timeout_seconds(Some(5))
            .dense_row_replace_ratio(0.75)
            .dense_row_replace_min_cols(16)
            .dense_rect_replace_min_rows(2)
            .semantic_noise_policy(SemanticNoisePolicy::SuppressFormattingOnly)
            .build()
            .expect("valid config should build");

        assert_eq!(cfg.preflight_min_rows, 10000);
        assert_eq!(cfg.preflight_in_order_mismatch_max, 64);
        assert!((cfg.preflight_in_order_match_ratio_min - 0.99).abs() < f64::EPSILON);
        assert!((cfg.bailout_similarity_threshold - 0.10).abs() < f64::EPSILON);
        assert_eq!(cfg.max_memory_mb, Some(64));
        assert_eq!(cfg.timeout_seconds, Some(5));
        assert!((cfg.dense_row_replace_ratio - 0.75).abs() < f64::EPSILON);
        assert_eq!(cfg.dense_row_replace_min_cols, 16);
        assert_eq!(cfg.dense_rect_replace_min_rows, 2);
        assert!(matches!(
            cfg.semantic_noise_policy,
            SemanticNoisePolicy::SuppressFormattingOnly
        ));
    }
}
