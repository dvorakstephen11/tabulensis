use std::path::{Path, PathBuf};

use super::diff::hash_text;
use super::normalize::normalize_doc_text;
use super::types::{
    PbipDocRecord, PbipDocSnapshot, PbipDocType, PbipNormalizationProfile, PbipProjectSnapshot,
};

#[derive(Debug, Clone)]
pub struct PbipScanConfig {
    /// Directory names to skip entirely during traversal.
    pub ignore_dir_names: Vec<String>,
    /// Maximum file size to read into memory (bytes).
    pub max_file_bytes: u64,
}

impl Default for PbipScanConfig {
    fn default() -> Self {
        Self {
            ignore_dir_names: vec![
                ".git".to_string(),
                ".pbi".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".venv".to_string(),
                ".idea".to_string(),
                ".vscode".to_string(),
            ],
            max_file_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

#[derive(Debug, Clone)]
pub struct PbipScanError {
    pub message: String,
}

impl std::fmt::Display for PbipScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PbipScanError {}

pub fn snapshot_project_from_fs(
    root: &Path,
    profile: PbipNormalizationProfile,
    scan: PbipScanConfig,
) -> Result<PbipProjectSnapshot, PbipScanError> {
    if !root.exists() {
        return Err(PbipScanError {
            message: format!("PBIP root not found: {root:?}"),
        });
    }
    if !root.is_dir() {
        return Err(PbipScanError {
            message: format!("PBIP root must be a directory: {root:?}"),
        });
    }

    let mut docs: Vec<PbipDocRecord> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| PbipScanError {
            message: format!("Failed to read directory {dir:?}: {e}"),
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| PbipScanError {
                message: format!("Failed to read directory entry in {dir:?}: {e}"),
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|e| PbipScanError {
                message: format!("Failed to stat {path:?}: {e}"),
            })?;
            if file_type.is_dir() {
                if should_ignore_dir_name(&path, &scan.ignore_dir_names) {
                    continue;
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let doc_type = doc_type_for_path(&path);
            if !matches!(doc_type, PbipDocType::Pbir | PbipDocType::Tmdl) {
                continue;
            }

            let rel = rel_path(root, &path);

            let meta = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    docs.push(PbipDocRecord {
                        path: rel,
                        doc_type,
                        snapshot: PbipDocSnapshot {
                            doc_type,
                            normalized_text: String::new(),
                            hash: 0,
                            error: Some(format!("Failed to read metadata for {path:?}: {e}")),
                            normalization_applied: None,
                        },
                    });
                    continue;
                }
            };
            if meta.len() > scan.max_file_bytes {
                docs.push(PbipDocRecord {
                    path: rel,
                    doc_type,
                    snapshot: PbipDocSnapshot {
                        doc_type,
                        normalized_text: String::new(),
                        hash: 0,
                        error: Some(format!(
                            "File too large to read ({} bytes > {} bytes cap).",
                            meta.len(),
                            scan.max_file_bytes
                        )),
                        normalization_applied: None,
                    },
                });
                continue;
            }

            let raw = match std::fs::read_to_string(&path) {
                Ok(value) => value,
                Err(e) => {
                    docs.push(PbipDocRecord {
                        path: rel,
                        doc_type,
                        snapshot: PbipDocSnapshot {
                            doc_type,
                            normalized_text: String::new(),
                            hash: 0,
                            error: Some(format!("Failed to read {path:?} as UTF-8 text: {e}")),
                            normalization_applied: None,
                        },
                    });
                    continue;
                }
            };

            let (normalized_text, applied, err) = match normalize_doc_text(doc_type, &raw, profile)
            {
                Ok((text, applied)) => (text, Some(applied), None),
                Err(e) => (normalize_text_minimal(&raw), None, Some(e.to_string())),
            };
            let hash = if normalized_text.is_empty() { 0 } else { hash_text(&normalized_text) };
            docs.push(PbipDocRecord {
                path: rel,
                doc_type,
                snapshot: PbipDocSnapshot {
                    doc_type,
                    normalized_text,
                    hash,
                    error: err,
                    normalization_applied: applied.as_ref().map(|a| a.summary.clone()),
                },
            });
        }
    }

    docs.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(PbipProjectSnapshot {
        docs,
        profile,
        profile_summary: profile_summary(profile),
    })
}

fn should_ignore_dir_name(path: &Path, ignore: &[String]) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return false;
    };
    ignore.iter().any(|value| value == name)
}

fn doc_type_for_path(path: &Path) -> PbipDocType {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "pbir" => PbipDocType::Pbir,
        "tmdl" => PbipDocType::Tmdl,
        _ => PbipDocType::Other,
    }
}

fn rel_path(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    // Store as forward slashes for stability across platforms.
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_text_minimal(raw: &str) -> String {
    // Mirror normalize::normalize_text_minimal without exposing it publicly.
    let raw = raw.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(raw.len() + 1);
    for (idx, line) in raw.split('\n').enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(line.trim_end_matches(|c: char| c == ' ' || c == '\t'));
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn profile_summary(profile: PbipNormalizationProfile) -> String {
    match profile {
        PbipNormalizationProfile::Strict => {
            "Strict: minimal text normalization (no JSON parsing, no GUID normalization).".to_string()
        }
        PbipNormalizationProfile::Balanced => {
            "Balanced: canonical JSON (sorted keys); GUID normalization only for id-like keys.".to_string()
        }
        PbipNormalizationProfile::Aggressive => {
            "Aggressive: canonical JSON (sorted keys) + GUID normalization for all GUID-like strings.".to_string()
        }
    }
}
