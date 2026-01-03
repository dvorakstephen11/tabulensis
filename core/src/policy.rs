use crate::config::DiffConfig;

pub const AUTO_STREAM_CELL_THRESHOLD: u64 = 1_000_000;

pub fn should_use_large_mode(estimated_cell_volume: u64, _config: &DiffConfig) -> bool {
    estimated_cell_volume >= AUTO_STREAM_CELL_THRESHOLD
}
