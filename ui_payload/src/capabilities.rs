use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDefaults {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_memory_mb: Option<u32>,
    pub large_mode_threshold: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub engine_version: String,
    pub features: excel_diff::EngineFeatures,
    pub presets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_defaults: Option<HostDefaults>,
}

impl HostCapabilities {
    pub fn new(engine_version: String) -> Self {
        Self {
            engine_version,
            features: excel_diff::engine_features(),
            presets: vec![
                "fastest".to_string(),
                "balanced".to_string(),
                "most_precise".to_string(),
            ],
            host_defaults: None,
        }
    }

    pub fn with_defaults(mut self, defaults: HostDefaults) -> Self {
        self.host_defaults = Some(defaults);
        self
    }
}
