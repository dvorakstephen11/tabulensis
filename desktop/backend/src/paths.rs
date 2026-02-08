use std::path::PathBuf;

use directories::ProjectDirs;

use crate::DiffErrorPayload;

#[derive(Debug, Clone)]
pub struct BackendPaths {
    pub app_data_dir: PathBuf,
    pub store_db_path: PathBuf,
    pub recents_json_path: PathBuf,
}

pub fn resolve_paths(app_name: &str) -> Result<BackendPaths, DiffErrorPayload> {
    if let Ok(value) = std::env::var("EXCEL_DIFF_APP_DATA_DIR") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            let dir = PathBuf::from(trimmed);
            std::fs::create_dir_all(&dir).map_err(|e| {
                DiffErrorPayload::new(
                    "paths",
                    format!("Failed to create EXCEL_DIFF_APP_DATA_DIR directory: {e}"),
                    false,
                )
            })?;
            return Ok(BackendPaths {
                app_data_dir: dir.clone(),
                store_db_path: dir.join("diff_store.sqlite"),
                recents_json_path: dir.join("recents.json"),
            });
        }
    }

    let project_dirs = ProjectDirs::from("com", "tabulensis", app_name).ok_or_else(|| {
        DiffErrorPayload::new("paths", "Unable to resolve app data directory", false)
    })?;
    let dir = project_dirs.data_local_dir().to_path_buf();
    std::fs::create_dir_all(&dir).map_err(|e| {
        DiffErrorPayload::new(
            "paths",
            format!("Failed to create app data directory: {e}"),
            false,
        )
    })?;

    Ok(BackendPaths {
        app_data_dir: dir.clone(),
        store_db_path: dir.join("diff_store.sqlite"),
        recents_json_path: dir.join("recents.json"),
    })
}
