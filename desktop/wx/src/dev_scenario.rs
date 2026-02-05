use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiScenarioFile {
    name: Option<String>,
    description: Option<String>,
    old_path: String,
    new_path: String,
    auto_run_diff: Option<bool>,
    stable_wait_ms: Option<u64>,
    expect_mode: Option<String>,
    focus_panel: Option<String>,
    preset: Option<String>,
    trusted_files: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct UiScenario {
    pub name: String,
    pub description: Option<String>,
    pub old_path: PathBuf,
    pub new_path: PathBuf,
    pub auto_run_diff: bool,
    pub stable_wait_ms: u64,
    pub expect_mode: Option<String>,
    pub focus_panel: Option<String>,
    pub preset: Option<String>,
    pub trusted_files: Option<bool>,
}

pub fn load_from_env() -> Result<Option<UiScenario>, String> {
    let Ok(name) = env::var("EXCEL_DIFF_DEV_SCENARIO") else {
        return Ok(None);
    };
    let name = name.trim().to_string();
    if name.is_empty() {
        return Ok(None);
    }
    let scenario = load_by_name(&name)?;
    Ok(Some(scenario))
}

fn load_by_name(name: &str) -> Result<UiScenario, String> {
    let dir = resolve_scenario_dir(name).ok_or_else(|| {
        format!(
            "Unable to locate UI scenario '{name}'. Set EXCEL_DIFF_UI_SCENARIOS_ROOT or run from the repo root."
        )
    })?;
    let path = dir.join("scenario.json");
    let contents = std::fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {path:?}: {err}"))?;
    let file: UiScenarioFile = serde_json::from_str(&contents)
        .map_err(|err| format!("Failed to parse {path:?}: {err}"))?;

    let old_path = resolve_scenario_path(&dir, &file.old_path)
        .ok_or_else(|| format!("Scenario '{name}' old_path missing: {}", file.old_path))?;
    let new_path = resolve_scenario_path(&dir, &file.new_path)
        .ok_or_else(|| format!("Scenario '{name}' new_path missing: {}", file.new_path))?;

    if !old_path.exists() {
        return Err(format!("Scenario '{name}' old_path not found: {old_path:?}"));
    }
    if !new_path.exists() {
        return Err(format!("Scenario '{name}' new_path not found: {new_path:?}"));
    }

    Ok(UiScenario {
        name: file.name.unwrap_or_else(|| name.to_string()),
        description: file.description,
        old_path,
        new_path,
        auto_run_diff: file.auto_run_diff.unwrap_or(true),
        stable_wait_ms: file.stable_wait_ms.unwrap_or(800),
        expect_mode: file.expect_mode,
        focus_panel: file.focus_panel,
        preset: file.preset,
        trusted_files: file.trusted_files,
    })
}

fn resolve_scenario_path(root: &Path, value: &str) -> Option<PathBuf> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        return Some(candidate);
    }
    Some(root.join(candidate))
}

fn resolve_scenario_dir(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(root) = env::var("EXCEL_DIFF_UI_SCENARIOS_ROOT") {
        if !root.trim().is_empty() {
            candidates.push(PathBuf::from(root));
        }
    }

    if let Ok(current) = env::current_dir() {
        candidates.push(current.join("desktop").join("ui_scenarios"));
    }

    candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../ui_scenarios"));

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("ui_scenarios"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("ui_scenarios"));
                if let Some(grand) = parent.parent() {
                    candidates.push(grand.join("ui_scenarios"));
                }
            }
        }
    }

    candidates
        .into_iter()
        .map(|root| root.join(name))
        .find(|path| path.join("scenario.json").exists())
}
