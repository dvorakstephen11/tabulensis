use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineFeatures {
    pub vba: bool,
    pub model_diff: bool,
    pub parallel: bool,
    pub std_fs: bool,
}

pub fn engine_features() -> EngineFeatures {
    EngineFeatures {
        vba: cfg!(feature = "vba"),
        model_diff: cfg!(feature = "model-diff"),
        parallel: cfg!(feature = "parallel"),
        std_fs: cfg!(feature = "std-fs"),
    }
}
