use axum::{
    Router,
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, Method, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json,
};
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use hmac::{Hmac, Mac};
use rand::{RngCore, rngs::OsRng};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Sha256;
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use tracing::{error, info, warn};
use tower_http::cors::{Any, CorsLayer};

type HmacSha256 = Hmac<Sha256>;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    init_logging();
    let config = Config::from_env()?;
    let signer = LicenseSigner::from_config(&config)?;
    let db = Db::new(&config.db_path)?;
    db.migrate()?;
    let http = reqwest::Client::new();

    let state = AppState {
        config,
        signer,
        db,
        http,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let app = Router::new()
        .route("/health", get(health))
        .route("/public_key", get(public_key))
        .route("/api/checkout/start", post(checkout_start))
        .route("/api/checkout/status", get(checkout_status))
        .route("/stripe/webhook", post(stripe_webhook))
        .route("/license/activate", post(license_activate))
        .route("/license/deactivate", post(license_deactivate))
        .route("/license/status", post(license_status))
        .route("/license/resend", post(license_resend))
        .route("/license/reset", post(license_reset))
        .route("/portal/session", post(portal_session))
        .with_state(state.clone())
        .layer(cors);

    let addr: SocketAddr = state
        .config
        .bind_addr
        .parse()
        .map_err(|err| AppError::Config(format!("Invalid bind addr: {err}")))?;
    info!("license service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|err| AppError::Http(format!("Bind error: {err}")))?;
    axum::serve(listener, app)
        .await
        .map_err(|err| AppError::Http(format!("Server error: {err}")))?;

    Ok(())
}

#[derive(Clone)]
struct AppState {
    config: Config,
    signer: LicenseSigner,
    db: Db,
    http: reqwest::Client,
}

#[derive(Clone)]
struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    fn new(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).map_err(AppError::Db)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn
            .lock()
            .map_err(|_| AppError::Config("DB lock poisoned".to_string()))
    }

    fn migrate(&self) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute_batch(
            "\
            CREATE TABLE IF NOT EXISTS licenses (
                license_key TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                customer_id TEXT,
                subscription_id TEXT,
                email TEXT,
                trial_end INTEGER,
                period_end INTEGER,
                max_devices INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS activations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                license_key TEXT NOT NULL,
                device_id TEXT NOT NULL,
                device_label TEXT,
                activated_at INTEGER NOT NULL,
                last_seen_at INTEGER,
                revoked_at INTEGER,
                UNIQUE(license_key, device_id)
            );
            CREATE TABLE IF NOT EXISTS checkout_sessions (
                session_id TEXT PRIMARY KEY,
                license_key TEXT NOT NULL,
                email TEXT,
                created_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_activations_license ON activations(license_key);
            ",
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn save_checkout_session(&self, session_id: &str, license_key: &str, email: Option<&str>) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT OR REPLACE INTO checkout_sessions (session_id, license_key, email, created_at) VALUES (?, ?, ?, ?)",
            params![session_id, license_key, email, now_ts()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn checkout_license_key(&self, session_id: &str) -> Result<Option<String>, AppError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare("SELECT license_key FROM checkout_sessions WHERE session_id = ?")
            .map_err(AppError::Db)?;
        let mut rows = stmt.query(params![session_id]).map_err(AppError::Db)?;
        if let Some(row) = rows.next().map_err(AppError::Db)? {
            let key: String = row.get(0).map_err(AppError::Db)?;
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    fn upsert_license(&self, record: &LicenseRecord) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO licenses (license_key, status, customer_id, subscription_id, email, trial_end, period_end, max_devices, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(license_key) DO UPDATE SET
                status=excluded.status,
                customer_id=excluded.customer_id,
                subscription_id=excluded.subscription_id,
                email=excluded.email,
                trial_end=excluded.trial_end,
                period_end=excluded.period_end,
                max_devices=excluded.max_devices,
                updated_at=excluded.updated_at",
            params![
                record.license_key,
                record.status,
                record.customer_id,
                record.subscription_id,
                record.email,
                record.trial_end,
                record.period_end,
                record.max_devices,
                record.created_at,
                record.updated_at,
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn get_license(&self, license_key: &str) -> Result<Option<LicenseRecord>, AppError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT license_key, status, customer_id, subscription_id, email, trial_end, period_end, max_devices, created_at, updated_at
                 FROM licenses WHERE license_key = ?",
            )
            .map_err(AppError::Db)?;
        let mut rows = stmt.query(params![license_key]).map_err(AppError::Db)?;
        if let Some(row) = rows.next().map_err(AppError::Db)? {
            Ok(Some(LicenseRecord {
                license_key: row.get(0).map_err(AppError::Db)?,
                status: row.get(1).map_err(AppError::Db)?,
                customer_id: row.get(2).map_err(AppError::Db)?,
                subscription_id: row.get(3).map_err(AppError::Db)?,
                email: row.get(4).map_err(AppError::Db)?,
                trial_end: row.get(5).map_err(AppError::Db)?,
                period_end: row.get(6).map_err(AppError::Db)?,
                max_devices: row.get::<_, i64>(7).map_err(AppError::Db)? as u32,
                created_at: row.get(8).map_err(AppError::Db)?,
                updated_at: row.get(9).map_err(AppError::Db)?,
            }))
        } else {
            Ok(None)
        }
    }

    fn update_license_by_subscription(
        &self,
        subscription_id: &str,
        status: Option<&str>,
        trial_end: Option<i64>,
        period_end: Option<i64>,
        customer_id: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE licenses SET
                status = COALESCE(?, status),
                trial_end = COALESCE(?, trial_end),
                period_end = COALESCE(?, period_end),
                customer_id = COALESCE(?, customer_id),
                updated_at = ?
             WHERE subscription_id = ?",
            params![status, trial_end, period_end, customer_id, now_ts(), subscription_id],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn update_license_by_customer(
        &self,
        customer_id: &str,
        status: Option<&str>,
        trial_end: Option<i64>,
        period_end: Option<i64>,
        subscription_id: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE licenses SET
                status = COALESCE(?, status),
                trial_end = COALESCE(?, trial_end),
                period_end = COALESCE(?, period_end),
                subscription_id = COALESCE(?, subscription_id),
                updated_at = ?
             WHERE customer_id = ?",
            params![status, trial_end, period_end, subscription_id, now_ts(), customer_id],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn upsert_activation(
        &self,
        license_key: &str,
        device_id: &str,
        device_label: Option<&str>,
        now: i64,
    ) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO activations (license_key, device_id, device_label, activated_at, last_seen_at, revoked_at)
             VALUES (?, ?, ?, ?, ?, NULL)
             ON CONFLICT(license_key, device_id) DO UPDATE SET
                device_label=COALESCE(excluded.device_label, activations.device_label),
                last_seen_at=excluded.last_seen_at,
                revoked_at=NULL",
            params![license_key, device_id, device_label, now, now],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn is_device_active(&self, license_key: &str, device_id: &str) -> Result<bool, AppError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(1) FROM activations WHERE license_key = ? AND device_id = ? AND revoked_at IS NULL",
            )
            .map_err(AppError::Db)?;
        let count: i64 = stmt
            .query_row(params![license_key, device_id], |row| row.get(0))
            .map_err(AppError::Db)?;
        Ok(count > 0)
    }

    fn count_active_activations(&self, license_key: &str) -> Result<i64, AppError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT COUNT(1) FROM activations WHERE license_key = ? AND revoked_at IS NULL",
            )
            .map_err(AppError::Db)?;
        let count: i64 = stmt
            .query_row(params![license_key], |row| row.get(0))
            .map_err(AppError::Db)?;
        Ok(count)
    }

    fn list_activations(&self, license_key: &str) -> Result<Vec<ActivationInfo>, AppError> {
        let conn = self.lock()?;
        let mut stmt = conn
            .prepare(
                "SELECT device_id, device_label, activated_at, last_seen_at FROM activations
                 WHERE license_key = ? AND revoked_at IS NULL ORDER BY activated_at DESC",
            )
            .map_err(AppError::Db)?;
        let rows = stmt
            .query_map(params![license_key], |row| {
                Ok(ActivationInfo {
                    device_id: row.get(0)?,
                    device_label: row.get(1)?,
                    activated_at: row.get(2)?,
                    last_seen_at: row.get(3)?,
                })
            })
            .map_err(AppError::Db)?;
        let mut activations = Vec::new();
        for row in rows {
            activations.push(row.map_err(AppError::Db)?);
        }
        Ok(activations)
    }

    fn deactivate_device(&self, license_key: &str, device_id: &str) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE activations SET revoked_at = ? WHERE license_key = ? AND device_id = ?",
            params![now_ts(), license_key, device_id],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    fn reset_activations(&self, license_key: &str) -> Result<(), AppError> {
        let conn = self.lock()?;
        conn.execute(
            "UPDATE activations SET revoked_at = ? WHERE license_key = ?",
            params![now_ts(), license_key],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }
}

#[derive(Clone)]
struct LicenseSigner {
    signing_key: SigningKey,
    public_key_b64: String,
}

impl LicenseSigner {
    fn from_config(config: &Config) -> Result<Self, AppError> {
        let signing_key = if let Some(key_b64) = config.signing_key_b64.as_ref() {
            let key_bytes = B64
                .decode(key_b64.trim())
                .map_err(|err| AppError::Config(format!("Invalid LICENSE_SIGNING_KEY_B64: {err}")))?;
            let key_array: [u8; 32] = key_bytes
                .try_into()
                .map_err(|_| AppError::Config("Signing key must be 32 bytes".to_string()))?;
            SigningKey::from_bytes(&key_array)
        } else {
            warn!("LICENSE_SIGNING_KEY_B64 not set; generating ephemeral key (dev only)");
            let mut seed = [0u8; 32];
            OsRng.fill_bytes(&mut seed);
            SigningKey::from_bytes(&seed)
        };
        let verifying_key = signing_key.verifying_key();
        let public_key_b64 = B64.encode(verifying_key.to_bytes());
        Ok(Self {
            signing_key,
            public_key_b64,
        })
    }

    fn sign(&self, payload: &ActivationTokenPayload) -> Result<ActivationToken, AppError> {
        let payload_bytes = serde_json::to_vec(payload)?;
        let signature = self.signing_key.sign(&payload_bytes);
        Ok(ActivationToken {
            payload: payload.clone(),
            signature: B64.encode(signature.to_bytes()),
        })
    }
}

#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
    db_path: PathBuf,
    stripe_secret_key: Option<String>,
    stripe_webhook_secret: Option<String>,
    stripe_price_id: Option<String>,
    stripe_success_url: String,
    stripe_cancel_url: String,
    stripe_portal_return_url: String,
    trial_days: i64,
    token_ttl_days: i64,
    past_due_grace_days: i64,
    max_devices: u32,
    signing_key_b64: Option<String>,
    admin_token: Option<String>,
    mock_stripe: bool,
}

impl Config {
    fn from_env() -> Result<Self, AppError> {
        let bind_addr = env_or("LICENSE_BIND", "0.0.0.0:8080");
        let db_path = PathBuf::from(env_or("LICENSE_DB_PATH", "license_service/data/licenses.sqlite"));
        let stripe_secret_key = std::env::var("STRIPE_SECRET_KEY").ok();
        let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").ok();
        let stripe_price_id = std::env::var("STRIPE_PRICE_ID").ok();
        let stripe_success_url = env_or("STRIPE_SUCCESS_URL", "https://tabulensis.com/download/success");
        let stripe_cancel_url = env_or("STRIPE_CANCEL_URL", "https://tabulensis.com/download");
        let stripe_portal_return_url = env_or("STRIPE_PORTAL_RETURN_URL", "https://tabulensis.com/support/billing");
        let trial_days = env_or_int("STRIPE_TRIAL_DAYS", 30)?;
        let token_ttl_days = env_or_int("LICENSE_TOKEN_TTL_DAYS", 14)?;
        let past_due_grace_days = env_or_int("LICENSE_PAST_DUE_GRACE_DAYS", 3)?;
        let max_devices = env_or_int("LICENSE_MAX_DEVICES", 2)? as u32;
        let signing_key_b64 = std::env::var("LICENSE_SIGNING_KEY_B64").ok();
        let admin_token = std::env::var("LICENSE_ADMIN_TOKEN").ok();
        let mock_stripe = std::env::var("LICENSE_MOCK_STRIPE")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(stripe_secret_key.is_none());

        Ok(Self {
            bind_addr,
            db_path,
            stripe_secret_key,
            stripe_webhook_secret,
            stripe_price_id,
            stripe_success_url,
            stripe_cancel_url,
            stripe_portal_return_url,
            trial_days,
            token_ttl_days,
            past_due_grace_days,
            max_devices,
            signing_key_b64,
            admin_token,
            mock_stripe,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivationTokenPayload {
    license_key: String,
    device_id: String,
    status: String,
    issued_at: i64,
    expires_at: i64,
    grace_until: Option<i64>,
    period_end: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivationToken {
    payload: ActivationTokenPayload,
    signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivationInfo {
    device_id: String,
    device_label: Option<String>,
    activated_at: i64,
    last_seen_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LicenseStatusResponse {
    license_key: String,
    status: String,
    max_devices: u32,
    trial_end: Option<i64>,
    period_end: Option<i64>,
    activations: Vec<ActivationInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivateResult {
    token: ActivationToken,
    status: LicenseStatusResponse,
}

#[derive(Debug, Clone)]
struct LicenseRecord {
    license_key: String,
    status: String,
    customer_id: Option<String>,
    subscription_id: Option<String>,
    email: Option<String>,
    trial_end: Option<i64>,
    period_end: Option<i64>,
    max_devices: u32,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct CheckoutStartRequest {
    email: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckoutStartResponse {
    checkout_url: String,
    session_id: String,
    license_key: String,
}

#[derive(Debug, Deserialize)]
struct CheckoutStatusQuery {
    session_id: String,
}

#[derive(Debug, Serialize)]
struct CheckoutStatusResponse {
    session_id: String,
    license_key: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseActivateRequest {
    license_key: String,
    device_id: String,
    device_label: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseDeactivateRequest {
    license_key: String,
    device_id: String,
}

#[derive(Debug, Deserialize)]
struct LicenseStatusRequest {
    license_key: String,
    device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseResendRequest {
    email: Option<String>,
    license_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LicenseResetRequest {
    license_key: String,
}

#[derive(Debug, Deserialize)]
struct PortalSessionRequest {
    license_key: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Serialize)]
struct PortalSessionResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct StripeEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: StripeEventData,
}

#[derive(Debug, Deserialize)]
struct StripeEventData {
    object: Value,
}

async fn health(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    state.db.migrate()?;
    Ok((StatusCode::OK, "ok"))
}

async fn public_key(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(serde_json::json!({
        "public_key_b64": state.signer.public_key_b64,
    })))
}

async fn checkout_start(
    State(state): State<AppState>,
    Json(req): Json<CheckoutStartRequest>,
) -> Result<impl IntoResponse, AppError> {
    let license_key = generate_license_key();
    if state.config.mock_stripe {
        let now = now_ts();
        let record = LicenseRecord {
            license_key: license_key.clone(),
            status: "trialing".to_string(),
            customer_id: None,
            subscription_id: None,
            email: req.email.clone(),
            trial_end: Some(now + days_to_seconds(state.config.trial_days)),
            period_end: Some(now + days_to_seconds(state.config.trial_days)),
            max_devices: state.config.max_devices,
            created_at: now,
            updated_at: now,
        };
        state.db.upsert_license(&record)?;
        let session_id = format!("mock_{}", generate_short_id());
        state
            .db
            .save_checkout_session(&session_id, &license_key, req.email.as_deref())?;
        let checkout_url = format!(
            "{}?license_key={}",
            state.config.stripe_success_url, license_key
        );
        return Ok(Json(CheckoutStartResponse {
            checkout_url,
            session_id,
            license_key,
        }));
    }

    let price_id = state
        .config
        .stripe_price_id
        .as_ref()
        .ok_or_else(|| AppError::Config("STRIPE_PRICE_ID not set".to_string()))?;
    let secret_key = state
        .config
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Config("STRIPE_SECRET_KEY not set".to_string()))?;

    let mut form = vec![
        ("mode", "subscription".to_string()),
        ("success_url", format!("{}?session_id={{CHECKOUT_SESSION_ID}}", state.config.stripe_success_url)),
        ("cancel_url", state.config.stripe_cancel_url.clone()),
        ("line_items[0][price]", price_id.clone()),
        ("line_items[0][quantity]", "1".to_string()),
        ("metadata[license_key]", license_key.clone()),
    ];
    if state.config.trial_days > 0 {
        form.push((
            "subscription_data[trial_period_days]",
            state.config.trial_days.to_string(),
        ));
    }
    if let Some(email) = req.email.as_ref() {
        form.push(("customer_email", email.clone()));
    }

    let response = state
        .http
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(secret_key, Some(""))
        .form(&form)
        .send()
        .await
        .map_err(|err| AppError::Http(format!("Stripe error: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Http(format!("Stripe error {status}: {body}")));
    }

    let payload: Value = response.json().await.map_err(|err| AppError::Http(err.to_string()))?;
    let session_id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Http("Stripe response missing session id".to_string()))?
        .to_string();
    let checkout_url = payload
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Http("Stripe response missing checkout url".to_string()))?
        .to_string();

    state
        .db
        .save_checkout_session(&session_id, &license_key, req.email.as_deref())?;

    let record = LicenseRecord {
        license_key: license_key.clone(),
        status: "pending".to_string(),
        customer_id: None,
        subscription_id: None,
        email: req.email.clone(),
        trial_end: None,
        period_end: None,
        max_devices: state.config.max_devices,
        created_at: now_ts(),
        updated_at: now_ts(),
    };
    state.db.upsert_license(&record)?;

    Ok(Json(CheckoutStartResponse {
        checkout_url,
        session_id,
        license_key,
    }))
}

async fn checkout_status(
    State(state): State<AppState>,
    Query(query): Query<CheckoutStatusQuery>,
) -> Result<impl IntoResponse, AppError> {
    let license_key = state.db.checkout_license_key(&query.session_id)?;
    let mut status = None;
    if let Some(ref key) = license_key {
        status = state.db.get_license(key)?.map(|record| record.status);
    }
    Ok(Json(CheckoutStatusResponse {
        session_id: query.session_id,
        license_key,
        status,
    }))
}

async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    if let Some(secret) = state.config.stripe_webhook_secret.as_ref() {
        let signature_header = headers
            .get("stripe-signature")
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AppError::Auth("Missing Stripe-Signature header".to_string()))?;
        if !verify_stripe_signature(secret, signature_header, &body) {
            warn!("stripe webhook signature verification failed");
            return Err(AppError::Auth("Invalid signature".to_string()));
        }
    } else {
        warn!("STRIPE_WEBHOOK_SECRET not set; skipping signature verification");
    }

    let event: StripeEvent = serde_json::from_slice(&body)?;
    handle_stripe_event(&state, event).await?;
    Ok(StatusCode::OK)
}

async fn handle_stripe_event(state: &AppState, event: StripeEvent) -> Result<(), AppError> {
    match event.event_type.as_str() {
        "checkout.session.completed" => handle_checkout_completed(state, &event.data.object).await?,
        "invoice.paid" => handle_invoice_paid(state, &event.data.object).await?,
        "invoice.payment_failed" => handle_invoice_failed(state, &event.data.object).await?,
        "customer.subscription.updated" => handle_subscription_updated(state, &event.data.object).await?,
        "customer.subscription.deleted" => handle_subscription_deleted(state, &event.data.object).await?,
        _ => {
            info!("ignored stripe event: {}", event.event_type);
        }
    }
    Ok(())
}

async fn handle_checkout_completed(state: &AppState, obj: &Value) -> Result<(), AppError> {
    let session_id = get_str(obj, "id").unwrap_or_default();
    let customer_id = get_str(obj, "customer");
    let subscription_id = get_str(obj, "subscription");
    let email = obj
        .get("customer_details")
        .and_then(|v| v.get("email"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let metadata_license = obj
        .get("metadata")
        .and_then(|v| v.get("license_key"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut license_key = metadata_license
        .or_else(|| state.db.checkout_license_key(&session_id).ok().flatten())
        .unwrap_or_else(generate_license_key);

    if license_key.is_empty() {
        license_key = generate_license_key();
    }

    let now = now_ts();
    let record = LicenseRecord {
        license_key: license_key.clone(),
        status: "trialing".to_string(),
        customer_id,
        subscription_id,
        email,
        trial_end: None,
        period_end: None,
        max_devices: state.config.max_devices,
        created_at: now,
        updated_at: now,
    };
    state.db.upsert_license(&record)?;
    Ok(())
}

async fn handle_invoice_paid(state: &AppState, obj: &Value) -> Result<(), AppError> {
    let subscription_id = get_str(obj, "subscription");
    let customer_id = get_str(obj, "customer");
    let period_end = obj
        .get("lines")
        .and_then(|v| v.get("data"))
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("period"))
        .and_then(|v| v.get("end"))
        .and_then(|v| v.as_i64());

    if let Some(sub_id) = subscription_id.as_ref() {
        state
            .db
            .update_license_by_subscription(sub_id, Some("active"), None, period_end, customer_id.as_deref())?;
    } else if let Some(cust_id) = customer_id.as_ref() {
        state
            .db
            .update_license_by_customer(cust_id, Some("active"), None, period_end, None)?;
    }
    Ok(())
}

async fn handle_invoice_failed(state: &AppState, obj: &Value) -> Result<(), AppError> {
    let subscription_id = get_str(obj, "subscription");
    let customer_id = get_str(obj, "customer");
    if let Some(sub_id) = subscription_id.as_ref() {
        state
            .db
            .update_license_by_subscription(sub_id, Some("past_due"), None, None, customer_id.as_deref())?;
    } else if let Some(cust_id) = customer_id.as_ref() {
        state
            .db
            .update_license_by_customer(cust_id, Some("past_due"), None, None, None)?;
    }
    Ok(())
}

async fn handle_subscription_updated(state: &AppState, obj: &Value) -> Result<(), AppError> {
    let subscription_id = get_str(obj, "id");
    let customer_id = get_str(obj, "customer");
    let status = get_str(obj, "status");
    let trial_end = obj.get("trial_end").and_then(|v| v.as_i64());
    let period_end = obj.get("current_period_end").and_then(|v| v.as_i64());

    if let Some(sub_id) = subscription_id.as_ref() {
        state.db.update_license_by_subscription(
            sub_id,
            status.as_deref(),
            trial_end,
            period_end,
            customer_id.as_deref(),
        )?;
    }
    Ok(())
}

async fn handle_subscription_deleted(state: &AppState, obj: &Value) -> Result<(), AppError> {
    let subscription_id = get_str(obj, "id");
    let customer_id = get_str(obj, "customer");
    if let Some(sub_id) = subscription_id.as_ref() {
        state.db.update_license_by_subscription(
            sub_id,
            Some("canceled"),
            None,
            None,
            customer_id.as_deref(),
        )?;
    }
    Ok(())
}

async fn license_activate(
    State(state): State<AppState>,
    Json(req): Json<LicenseActivateRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .db
        .get_license(&req.license_key)?
        .ok_or_else(|| AppError::NotFound("License not found".to_string()))?;

    if !status_allows_activation(&record.status) {
        return Err(AppError::Forbidden(format!(
            "License status does not allow activation: {}",
            record.status
        )));
    }

    let already_active = state
        .db
        .is_device_active(&req.license_key, &req.device_id)?;
    let active_count = state.db.count_active_activations(&req.license_key)? as u32;
    if !already_active && active_count >= record.max_devices {
        return Err(AppError::Forbidden("Device limit reached".to_string()));
    }

    let now = now_ts();
    state.db.upsert_activation(
        &req.license_key,
        &req.device_id,
        req.device_label.as_deref(),
        now,
    )?;

    let expires_at = if record.status == "past_due" {
        now + days_to_seconds(state.config.past_due_grace_days)
    } else {
        now + days_to_seconds(state.config.token_ttl_days)
    };

    let payload = ActivationTokenPayload {
        license_key: record.license_key.clone(),
        device_id: req.device_id.clone(),
        status: record.status.clone(),
        issued_at: now,
        expires_at,
        grace_until: Some(expires_at),
        period_end: record.period_end,
    };
    let token = state.signer.sign(&payload)?;

    let status = LicenseStatusResponse {
        license_key: record.license_key.clone(),
        status: record.status.clone(),
        max_devices: record.max_devices,
        trial_end: record.trial_end,
        period_end: record.period_end,
        activations: state.db.list_activations(&req.license_key)?,
    };

    Ok(Json(ActivateResult { token, status }))
}

async fn license_deactivate(
    State(state): State<AppState>,
    Json(req): Json<LicenseDeactivateRequest>,
) -> Result<impl IntoResponse, AppError> {
    state
        .db
        .deactivate_device(&req.license_key, &req.device_id)?;
    Ok(StatusCode::OK)
}

async fn license_status(
    State(state): State<AppState>,
    Json(req): Json<LicenseStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .db
        .get_license(&req.license_key)?
        .ok_or_else(|| AppError::NotFound("License not found".to_string()))?;

    if let Some(device_id) = req.device_id.as_ref() {
        if state
            .db
            .is_device_active(&record.license_key, device_id)?
        {
            state
                .db
                .upsert_activation(&record.license_key, device_id, None, now_ts())?;
        }
    }

    let status = LicenseStatusResponse {
        license_key: record.license_key.clone(),
        status: record.status.clone(),
        max_devices: record.max_devices,
        trial_end: record.trial_end,
        period_end: record.period_end,
        activations: state.db.list_activations(&record.license_key)?,
    };

    Ok(Json(status))
}

async fn license_resend(
    State(state): State<AppState>,
    Json(req): Json<LicenseResendRequest>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(key) = req.license_key.as_ref() {
        let record = state.db.get_license(key)?;
        if record.is_none() {
            return Err(AppError::NotFound("License not found".to_string()));
        }
    }
    info!("resend requested: {:?}", req.email);
    Ok(Json(serde_json::json!({"status":"queued"})))
}

async fn license_reset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LicenseResetRequest>,
) -> Result<impl IntoResponse, AppError> {
    let Some(expected) = state.config.admin_token.as_ref() else {
        return Err(AppError::Forbidden("Admin token not configured".to_string()));
    };
    let provided = headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Auth("Missing admin token".to_string()))?;
    if provided != expected {
        return Err(AppError::Auth("Invalid admin token".to_string()));
    }
    state.db.reset_activations(&req.license_key)?;
    Ok(Json(serde_json::json!({"status":"reset"})))
}

async fn portal_session(
    State(state): State<AppState>,
    Json(req): Json<PortalSessionRequest>,
) -> Result<impl IntoResponse, AppError> {
    if state.config.mock_stripe {
        return Ok(Json(PortalSessionResponse {
            url: state.config.stripe_portal_return_url.clone(),
        }));
    }

    let customer_id = if let Some(license_key) = req.license_key.as_ref() {
        state
            .db
            .get_license(license_key)?
            .and_then(|record| record.customer_id)
    } else if let Some(email) = req.email.as_ref() {
        let conn = state.db.lock()?;
        let mut stmt = conn
            .prepare("SELECT customer_id FROM licenses WHERE email = ? LIMIT 1")
            .map_err(AppError::Db)?;
        let mut rows = stmt.query(params![email]).map_err(AppError::Db)?;
        rows.next()
            .map_err(AppError::Db)?
            .and_then(|row| row.get(0).ok())
    } else {
        None
    };

    let Some(customer_id) = customer_id else {
        return Err(AppError::NotFound("Customer not found".to_string()));
    };

    let secret_key = state
        .config
        .stripe_secret_key
        .as_ref()
        .ok_or_else(|| AppError::Config("STRIPE_SECRET_KEY not set".to_string()))?;

    let response = state
        .http
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .basic_auth(secret_key, Some(""))
        .form(&[
            ("customer", customer_id.clone()),
            ("return_url", state.config.stripe_portal_return_url.clone()),
        ])
        .send()
        .await
        .map_err(|err| AppError::Http(format!("Stripe error: {err}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Http(format!("Stripe error {status}: {body}")));
    }

    let payload: Value = response.json().await.map_err(|err| AppError::Http(err.to_string()))?;
    let url = payload
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Http("Stripe response missing portal url".to_string()))?
        .to_string();

    Ok(Json(PortalSessionResponse { url }))
}

fn status_allows_activation(status: &str) -> bool {
    matches!(status, "trialing" | "active" | "past_due")
}

fn verify_stripe_signature(secret: &str, header: &str, payload: &[u8]) -> bool {
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in header.split(',') {
        let mut iter = part.splitn(2, '=');
        let key = iter.next().unwrap_or("");
        let value = iter.next().unwrap_or("");
        match key {
            "t" => timestamp = value.parse::<i64>().ok(),
            "v1" => signatures.push(value),
            _ => {}
        }
    }

    let Some(timestamp) = timestamp else {
        return false;
    };

    let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(payload));
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(signed_payload.as_bytes());
    let result = mac.finalize();
    let expected = hex::encode(result.into_bytes());

    signatures.iter().any(|sig| sig == &expected)
}

fn get_str(obj: &Value, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn generate_license_key() -> String {
    let mut bytes = [0u8; 12];
    OsRng.fill_bytes(&mut bytes);
    let encoded = base32_no_pad(&bytes);
    format!("TABU-{}-{}-{}", &encoded[0..4], &encoded[4..8], &encoded[8..12])
}

fn generate_short_id() -> String {
    let mut bytes = [0u8; 8];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn base32_no_pad(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::new();
    let mut buffer = 0u32;
    let mut bits_left = 0u8;
    for &byte in bytes {
        buffer = (buffer << 8) | byte as u32;
        bits_left += 8;
        while bits_left >= 5 {
            let index = ((buffer >> (bits_left - 5)) & 0x1f) as usize;
            output.push(ALPHABET[index] as char);
            bits_left -= 5;
        }
    }
    if bits_left > 0 {
        let index = ((buffer << (5 - bits_left)) & 0x1f) as usize;
        output.push(ALPHABET[index] as char);
    }
    output
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

fn days_to_seconds(days: i64) -> i64 {
    days.saturating_mul(86_400)
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn env_or_int(key: &str, default: i64) -> Result<i64, AppError> {
    match std::env::var(key) {
        Ok(value) => value
            .parse::<i64>()
            .map_err(|_| AppError::Config(format!("Invalid integer for {key}"))),
        Err(_) => Ok(default),
    }
}

fn init_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();
}

#[derive(Debug, Error)]
enum AppError {
    #[error("config error: {0}")]
    Config(String),
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("http error: {0}")]
    Http(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("forbidden: {0}")]
    Forbidden(String),
    #[error("auth error: {0}")]
    Auth(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::Config(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            AppError::Db(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::Json(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            AppError::Io(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            AppError::Http(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            AppError::Auth(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
        };
        error!("request error: {}", message);
        (status, Json(serde_json::json!({"error": message}))).into_response()
    }
}
