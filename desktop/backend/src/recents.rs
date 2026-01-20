use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::DiffErrorPayload;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecentComparison {
    pub old_path: String,
    pub new_path: String,
    pub old_name: String,
    pub new_name: String,
    pub last_run_iso: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

pub fn load_recents(path: &Path) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
    Ok(load_recents_from_disk(path))
}

pub fn save_recent(path: &Path, entry: RecentComparison) -> Result<Vec<RecentComparison>, DiffErrorPayload> {
    let current = load_recents_from_disk(path);
    let updated = update_recents(current, entry);
    save_recents_to_disk(path, &updated)?;
    Ok(updated)
}

fn load_recents_from_disk(path: &Path) -> Vec<RecentComparison> {
    let data = std::fs::read_to_string(path).unwrap_or_default();
    if data.trim().is_empty() {
        return Vec::new();
    }
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_recents_to_disk(path: &Path, entries: &[RecentComparison]) -> Result<(), DiffErrorPayload> {
    let data = serde_json::to_string_pretty(entries)
        .map_err(|e| DiffErrorPayload::new("recents", format!("Failed to serialize recents: {e}"), false))?;
    std::fs::write(path, data)
        .map_err(|e| DiffErrorPayload::new("recents", format!("Failed to write recents: {e}"), false))
}

fn update_recents(mut entries: Vec<RecentComparison>, entry: RecentComparison) -> Vec<RecentComparison> {
    entries.retain(|item| !(item.old_path == entry.old_path && item.new_path == entry.new_path));
    entries.insert(0, entry);
    entries.truncate(20);
    entries
}
