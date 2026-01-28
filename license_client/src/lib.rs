use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use directories::ProjectDirs;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use ureq::Agent;
use uuid::Uuid;

const DEFAULT_VENDOR: &str = "tabulensis";
const DEFAULT_APP: &str = "tabulensis";
const DEFAULT_BASE_URL: &str = "https://license.tabulensis.com";
const DEVICE_ID_FILENAME: &str = "device_id.txt";
const TOKEN_FILENAME: &str = "license_token.json";

#[derive(Debug, Error)]
pub enum LicenseError {
    #[error("license config error: {0}")]
    Config(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("http error: {0}")]
    Http(String),
    #[error("license error: {0}")]
    License(String),
    #[error("signature error: {0}")]
    Signature(String),
}

#[derive(Debug, Clone)]
pub struct LicenseConfig {
    pub base_url: String,
    pub vendor: String,
    pub app_name: String,
    pub public_key_b64: Option<String>,
    pub timeout: Duration,
}

impl LicenseConfig {
    pub fn from_env() -> Self {
        let base_url = std::env::var("TABULENSIS_LICENSE_BASE_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let vendor = std::env::var("TABULENSIS_LICENSE_VENDOR").unwrap_or_else(|_| DEFAULT_VENDOR.to_string());
        let app_name = std::env::var("TABULENSIS_LICENSE_APP").unwrap_or_else(|_| DEFAULT_APP.to_string());
        let public_key_b64 = std::env::var("TABULENSIS_LICENSE_PUBLIC_KEY").ok();
        let timeout = std::env::var("TABULENSIS_LICENSE_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(5));
        Self {
            base_url,
            vendor,
            app_name,
            public_key_b64,
            timeout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LicensePaths {
    pub root: PathBuf,
    pub device_id_path: PathBuf,
    pub token_path: PathBuf,
}

pub fn resolve_paths(cfg: &LicenseConfig) -> Result<LicensePaths, LicenseError> {
    if let Ok(dir) = std::env::var("TABULENSIS_LICENSE_DIR") {
        let root = PathBuf::from(dir);
        fs::create_dir_all(&root)?;
        return Ok(LicensePaths {
            device_id_path: root.join(DEVICE_ID_FILENAME),
            token_path: root.join(TOKEN_FILENAME),
            root,
        });
    }

    let project_dirs = ProjectDirs::from("com", &cfg.vendor, &cfg.app_name)
        .ok_or_else(|| LicenseError::Config("Unable to resolve app data directory".to_string()))?;
    let root = project_dirs.data_local_dir().to_path_buf();
    fs::create_dir_all(&root)?;

    Ok(LicensePaths {
        root: root.clone(),
        device_id_path: root.join(DEVICE_ID_FILENAME),
        token_path: root.join(TOKEN_FILENAME),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationTokenPayload {
    pub license_key: String,
    pub device_id: String,
    pub status: String,
    pub issued_at: i64,
    pub expires_at: i64,
    pub grace_until: Option<i64>,
    pub period_end: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationToken {
    pub payload: ActivationTokenPayload,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationInfo {
    pub device_id: String,
    pub device_label: Option<String>,
    pub activated_at: i64,
    pub last_seen_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub license_key: String,
    pub status: String,
    pub max_devices: u32,
    pub trial_end: Option<i64>,
    pub period_end: Option<i64>,
    pub activations: Vec<ActivationInfo>,
}

#[derive(Debug, Clone)]
pub struct LocalLicenseState {
    pub token: ActivationToken,
    pub verified: bool,
    pub expired: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum VerificationStatus {
    Verified,
    Unverified,
}

#[derive(Debug, Clone)]
pub struct TokenVerifier {
    verifying_key: Option<VerifyingKey>,
}

impl TokenVerifier {
    pub fn from_config(cfg: &LicenseConfig) -> Result<Self, LicenseError> {
        let verifying_key = if let Some(key_b64) = cfg.public_key_b64.as_ref() {
            let key_bytes = B64
                .decode(key_b64.trim())
                .map_err(|err| LicenseError::Signature(format!("Invalid public key: {err}")))?;
            let key_array: [u8; 32] = key_bytes
                .try_into()
                .map_err(|_| LicenseError::Signature("Public key must be 32 bytes".to_string()))?;
            Some(
                VerifyingKey::from_bytes(&key_array)
                    .map_err(|err| LicenseError::Signature(format!("Invalid public key: {err}")))?,
            )
        } else {
            None
        };

        Ok(Self { verifying_key })
    }

    pub fn verify(&self, token: &ActivationToken) -> Result<VerificationStatus, LicenseError> {
        let Some(verifying_key) = &self.verifying_key else {
            return Ok(VerificationStatus::Unverified);
        };
        let signature_bytes = B64
            .decode(token.signature.as_str())
            .map_err(|err| LicenseError::Signature(format!("Invalid signature: {err}")))?;
        let signature = Signature::try_from(signature_bytes.as_slice())
            .map_err(|err| LicenseError::Signature(format!("Invalid signature: {err}")))?;
        let payload_bytes = serde_json::to_vec(&token.payload)?;
        verifying_key
            .verify_strict(&payload_bytes, &signature)
            .map_err(|err| LicenseError::Signature(format!("Signature verification failed: {err}")))?;
        Ok(VerificationStatus::Verified)
    }
}

#[derive(Debug, Clone)]
pub struct LicenseClient {
    config: LicenseConfig,
    paths: LicensePaths,
    http: Agent,
    verifier: TokenVerifier,
}

impl LicenseClient {
    pub fn from_env() -> Result<Self, LicenseError> {
        let config = LicenseConfig::from_env();
        Self::new(config)
    }

    pub fn new(config: LicenseConfig) -> Result<Self, LicenseError> {
        let paths = resolve_paths(&config)?;
        let verifier = TokenVerifier::from_config(&config)?;
        let http = ureq::AgentBuilder::new()
            .timeout_read(config.timeout)
            .timeout_write(config.timeout)
            .timeout_connect(config.timeout)
            .build();
        Ok(Self {
            config,
            paths,
            http,
            verifier,
        })
    }

    pub fn paths(&self) -> &LicensePaths {
        &self.paths
    }

    pub fn load_local_state(&self) -> Result<Option<LocalLicenseState>, LicenseError> {
        if !self.paths.token_path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&self.paths.token_path)?;
        let token: ActivationToken = serde_json::from_str(&contents)?;
        let verification = self.verifier.verify(&token)?;
        let expired = token.payload.expires_at <= now_ts();
        Ok(Some(LocalLicenseState {
            token,
            verified: matches!(verification, VerificationStatus::Verified),
            expired,
        }))
    }

    pub fn ensure_valid_or_refresh(&self) -> Result<LicenseStatus, LicenseError> {
        if std::env::var("TABULENSIS_LICENSE_SKIP").ok().as_deref() == Some("1") {
            return Ok(LicenseStatus {
                license_key: "SKIPPED".to_string(),
                status: "skipped".to_string(),
                max_devices: 0,
                trial_end: None,
                period_end: None,
                activations: vec![],
            });
        }

        let local = self.load_local_state()?;
        if let Some(state) = &local {
            if !state.expired {
                return Ok(LicenseStatus {
                    license_key: state.token.payload.license_key.clone(),
                    status: state.token.payload.status.clone(),
                    max_devices: 0,
                    trial_end: None,
                    period_end: state.token.payload.period_end,
                    activations: vec![],
                });
            }
        }

        if std::env::var("TABULENSIS_LICENSE_OFFLINE").ok().as_deref() == Some("1") {
            return Err(LicenseError::License(
                "License token expired and offline mode is enabled".to_string(),
            ));
        }

        let license_key = local
            .as_ref()
            .map(|state| state.token.payload.license_key.clone())
            .ok_or_else(|| LicenseError::License("No local license found".to_string()))?;
        let result = self.activate(&license_key)?;
        Ok(result.status)
    }

    pub fn activate(&self, license_key: &str) -> Result<ActivateResult, LicenseError> {
        let device_id = load_or_create_device_id(&self.paths)?;
        let device_hash = hash_device_id(&device_id);
        let device_label = default_device_label();

        let url = format!("{}/license/activate", self.config.base_url.trim_end_matches('/'));
        let response = self
            .http
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(serde_json::json!({
                "license_key": license_key,
                "device_id": device_hash,
                "device_label": device_label,
            }))
            .map_err(|err| LicenseError::Http(err.to_string()))?;

        if response.status() >= 400 {
            let text = response.into_string().unwrap_or_else(|_| "".to_string());
            return Err(LicenseError::License(format!(
                "Activation failed: {}",
                text.trim()
            )));
        }

        let result: ActivateResult = response
            .into_json()
            .map_err(|err| LicenseError::Http(err.to_string()))?;
        persist_token(&self.paths.token_path, &result.token)?;
        Ok(result)
    }

    pub fn deactivate(&self, license_key: Option<&str>) -> Result<(), LicenseError> {
        let device_id = load_or_create_device_id(&self.paths)?;
        let device_hash = hash_device_id(&device_id);
        let license_key = match license_key {
            Some(value) => value.to_string(),
            None => self
                .load_local_state()?
                .map(|state| state.token.payload.license_key)
                .ok_or_else(|| LicenseError::License("No local license found".to_string()))?,
        };

        let url = format!("{}/license/deactivate", self.config.base_url.trim_end_matches('/'));
        let response = self
            .http
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(serde_json::json!({
                "license_key": license_key,
                "device_id": device_hash,
            }))
            .map_err(|err| LicenseError::Http(err.to_string()))?;

        if response.status() >= 400 {
            let text = response.into_string().unwrap_or_else(|_| "".to_string());
            return Err(LicenseError::License(format!(
                "Deactivate failed: {}",
                text.trim()
            )));
        }

        if self.paths.token_path.exists() {
            fs::remove_file(&self.paths.token_path)?;
        }
        Ok(())
    }

    pub fn status_remote(&self, license_key: Option<&str>) -> Result<LicenseStatus, LicenseError> {
        let device_id = load_or_create_device_id(&self.paths)?;
        let device_hash = hash_device_id(&device_id);
        let license_key = match license_key {
            Some(value) => value.to_string(),
            None => self
                .load_local_state()?
                .map(|state| state.token.payload.license_key)
                .ok_or_else(|| LicenseError::License("No local license found".to_string()))?,
        };

        let url = format!("{}/license/status", self.config.base_url.trim_end_matches('/'));
        let response = self
            .http
            .post(&url)
            .set("Content-Type", "application/json")
            .send_json(serde_json::json!({
                "license_key": license_key,
                "device_id": device_hash,
            }))
            .map_err(|err| LicenseError::Http(err.to_string()))?;

        if response.status() >= 400 {
            let text = response.into_string().unwrap_or_else(|_| "".to_string());
            return Err(LicenseError::License(format!(
                "Status failed: {}",
                text.trim()
            )));
        }

        let status: LicenseStatus = response
            .into_json()
            .map_err(|err| LicenseError::Http(err.to_string()))?;
        Ok(status)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivateResult {
    pub token: ActivationToken,
    pub status: LicenseStatus,
}

pub fn load_or_create_device_id(paths: &LicensePaths) -> Result<String, LicenseError> {
    if let Ok(value) = fs::read_to_string(&paths.device_id_path) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let new_id = Uuid::new_v4().to_string();
    fs::write(&paths.device_id_path, &new_id)?;
    Ok(new_id)
}

pub fn hash_device_id(device_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(device_id.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn default_device_label() -> String {
    let host = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown-host".to_string());
    let user = whoami::username();
    format!("{}@{}", user, host)
}

fn persist_token(path: &Path, token: &ActivationToken) -> Result<(), LicenseError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(token)?;
    fs::write(path, payload)?;
    Ok(())
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

impl fmt::Display for LicenseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.license_key, self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_hash_is_stable() {
        let id = "device-123";
        let hash1 = hash_device_id(id);
        let hash2 = hash_device_id(id);
        assert_eq!(hash1, hash2);
    }
}
