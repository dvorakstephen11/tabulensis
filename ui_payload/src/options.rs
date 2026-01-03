use excel_diff::{DiffConfig, LimitBehavior};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffPreset {
    Fastest,
    Balanced,
    MostPrecise,
}

impl DiffPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            DiffPreset::Fastest => "fastest",
            DiffPreset::Balanced => "balanced",
            DiffPreset::MostPrecise => "most_precise",
        }
    }

    pub fn to_config(self) -> DiffConfig {
        match self {
            DiffPreset::Fastest => DiffConfig::fastest(),
            DiffPreset::Balanced => DiffConfig::balanced(),
            DiffPreset::MostPrecise => DiffConfig::most_precise(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLimits {
    pub max_memory_mb: Option<u32>,
    pub timeout_ms: Option<u64>,
    pub max_ops: Option<usize>,
    pub on_limit_exceeded: Option<LimitBehavior>,
}

impl DiffLimits {
    pub fn apply_to(&self, cfg: &mut DiffConfig) {
        if let Some(value) = self.max_memory_mb {
            cfg.hardening.max_memory_mb = Some(value);
        }
        if let Some(value) = self.timeout_ms {
            let seconds = timeout_seconds_from_ms(value);
            cfg.hardening.timeout_seconds = Some(seconds);
        }
        if let Some(value) = self.max_ops {
            cfg.hardening.max_ops = Some(value);
        }
        if let Some(value) = self.on_limit_exceeded {
            cfg.hardening.on_limit_exceeded = value;
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<DiffPreset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<DiffLimits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trusted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_json: Option<String>,
}

impl DiffOptions {
    pub fn effective_config(&self, default_config: DiffConfig) -> Result<DiffConfig, String> {
        let mut cfg = if let Some(config_json) = self
            .config_json
            .as_ref()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
        {
            serde_json::from_str::<DiffConfig>(config_json)
                .map_err(|e| format!("Invalid configJson: {e}"))?
        } else if let Some(preset) = self.preset {
            preset.to_config()
        } else {
            default_config
        };

        if let Some(limits) = &self.limits {
            limits.apply_to(&mut cfg);
        }

        cfg.validate().map_err(|e| e.to_string())?;
        Ok(cfg)
    }
}

pub fn limits_from_config(cfg: &DiffConfig) -> DiffLimits {
    DiffLimits {
        max_memory_mb: cfg.hardening.max_memory_mb,
        timeout_ms: cfg
            .hardening
            .timeout_seconds
            .map(|v| u64::from(v).saturating_mul(1000)),
        max_ops: cfg.hardening.max_ops,
        on_limit_exceeded: Some(cfg.hardening.on_limit_exceeded),
    }
}

fn timeout_seconds_from_ms(value: u64) -> u32 {
    let seconds = value.saturating_add(999) / 1000;
    u32::try_from(seconds).unwrap_or(u32::MAX)
}
