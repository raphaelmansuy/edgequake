//! Turn Lakebase OAuth (`EDGEQUAKE_DB_AUTH=lakebase_oauth`) into a Postgres URL.
//!
//! Databricks Apps inject `PGHOST` / `PGUSER` / `LAKEBASE_ENDPOINT` and a
//! service principal, not `DATABASE_URL`. The credential from
//! `POST /api/2.0/postgres/credentials` is the database password and expires,
//! so later connections must pick up a fresh password.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use tracing::info;
#[cfg(feature = "postgres")]
use tracing::warn;

/// Boot error for database URL resolution.
#[derive(Debug)]
pub enum DbUrlError {
    /// Neither `DATABASE_URL` nor Lakebase OAuth is configured.
    MissingDatabaseUrl,
    /// Lakebase OAuth was requested and a required setting or call failed.
    Message(String),
}

impl std::fmt::Display for DbUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDatabaseUrl => write!(
                f,
                "DATABASE_URL is required; start PostgreSQL and rerun make dev or make dev-auth"
            ),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for DbUrlError {}

/// `true` when this process should obtain a Lakebase database credential.
pub fn lakebase_oauth_enabled() -> bool {
    std::env::var("EDGEQUAKE_DB_AUTH")
        .map(|value| value.trim().eq_ignore_ascii_case("lakebase_oauth"))
        .unwrap_or(false)
}

/// Resolve the Postgres URL. `DATABASE_URL` wins when it is set.
pub async fn resolve_database_url() -> Result<String, DbUrlError> {
    if let Some(url) = nonempty("DATABASE_URL") {
        return Ok(url);
    }
    if !lakebase_oauth_enabled() {
        return Err(DbUrlError::MissingDatabaseUrl);
    }
    let credential = fetch_credential().await?;
    let url = postgres_url(
        &required("PGUSER")?,
        &credential.token,
        &required("PGHOST")?,
        pg_port()?,
        &required("PGDATABASE")?,
        &nonempty("PGSSLMODE").unwrap_or_else(|| "require".to_string()),
    );
    info!("Lakebase OAuth database URL is ready");
    remember_refresh_wait(credential.expire_time.as_deref());
    LAKEBASE_URL_ACTIVE.store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(url)
}

/// Refresh the Lakebase password for pools opened from the primary URL.
///
/// No-ops unless `EDGEQUAKE_DB_AUTH=lakebase_oauth`. Existing connections keep
/// the password they authenticated with; only future connections change.
#[cfg(feature = "postgres")]
pub fn spawn_credential_refresh(bundle: edgequake_storage::PgPoolBundle) {
    if !LAKEBASE_URL_ACTIVE.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let wait_secs = refresh_wait_secs();
    info!(
        wait_secs,
        "Lakebase credential applied to future connections"
    );
    tokio::spawn(async move {
        let mut wait = wait_secs;
        loop {
            tokio::time::sleep(Duration::from_secs(wait)).await;
            match fetch_credential().await {
                Ok(credential) => {
                    apply_password(&bundle, &credential.token);
                    wait = remember_refresh_wait(credential.expire_time.as_deref());
                    info!(
                        wait_secs = wait,
                        "Lakebase credential applied to future connections"
                    );
                }
                Err(error) => {
                    warn!(error = %error, "Lakebase credential refresh failed");
                    wait = 60;
                }
            }
        }
    });
}

#[cfg(not(feature = "postgres"))]
pub fn spawn_credential_refresh() {}

/// `postgresql://user:password@host:port/db?sslmode=...` with encoded secrets.
pub fn postgres_url(
    user: &str,
    password: &str,
    host: &str,
    port: u16,
    database: &str,
    sslmode: &str,
) -> String {
    format!(
        "postgresql://{}:{}@{}:{}/{}?sslmode={}",
        urlencoding::encode(user),
        urlencoding::encode(password),
        host,
        port,
        urlencoding::encode(database),
        urlencoding::encode(sslmode),
    )
}

struct Credential {
    token: String,
    expire_time: Option<String>,
}

async fn fetch_credential() -> Result<Credential, DbUrlError> {
    let access = workspace_access_token().await?;
    let endpoint = required("LAKEBASE_ENDPOINT")?;
    let url = format!("{}/api/2.0/postgres/credentials", workspace_host()?);
    let client = http_client()?;
    let response = client
        .post(url)
        .bearer_auth(access)
        .json(&serde_json::json!({ "endpoint": endpoint }))
        .send()
        .await
        .map_err(|error| DbUrlError::Message(format!("credential request failed: {error}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| DbUrlError::Message(format!("credential request failed: {error}")))?;
    if !status.is_success() {
        return Err(DbUrlError::Message(format!(
            "lakebase credential status {status}"
        )));
    }
    let parsed: CredBody = serde_json::from_str(&body)
        .map_err(|_| DbUrlError::Message("lakebase credential response has no token".into()))?;
    if parsed.token.trim().is_empty() {
        return Err(DbUrlError::Message(
            "lakebase credential token is empty".into(),
        ));
    }
    Ok(Credential {
        token: parsed.token,
        expire_time: parsed.expire_time,
    })
}

#[derive(Deserialize)]
struct CredBody {
    token: String,
    #[serde(default)]
    expire_time: Option<String>,
}

async fn workspace_access_token() -> Result<String, DbUrlError> {
    if let Some(token) = nonempty("DATABRICKS_TOKEN") {
        return Ok(token);
    }
    if std::env::var("DATABRICKS_TOKEN")
        .ok()
        .is_some_and(|value| value.trim().is_empty())
        && nonempty("DATABRICKS_CLIENT_ID").is_none()
    {
        return Err(DbUrlError::Message("DATABRICKS_TOKEN is empty".into()));
    }
    let Some(client_id) = nonempty("DATABRICKS_CLIENT_ID") else {
        return Err(DbUrlError::Message(
            "Set DATABRICKS_CLIENT_ID and DATABRICKS_CLIENT_SECRET, or DATABRICKS_TOKEN".into(),
        ));
    };
    let Some(client_secret) = nonempty("DATABRICKS_CLIENT_SECRET") else {
        return Err(DbUrlError::Message(
            "service principal credentials are not set".into(),
        ));
    };
    let basic = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        format!("{client_id}:{client_secret}"),
    );
    let url = format!("{}/oidc/v1/token", workspace_host()?);
    let client = http_client()?;
    let response = client
        .post(url)
        .header("Authorization", format!("Basic {basic}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body("grant_type=client_credentials&scope=all-apis")
        .send()
        .await
        .map_err(|error| DbUrlError::Message(format!("token request failed: {error}")))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| DbUrlError::Message(format!("token request failed: {error}")))?;
    if !status.is_success() {
        return Err(DbUrlError::Message(format!(
            "databricks oidc status {status}"
        )));
    }
    let parsed: OidcBody = serde_json::from_str(&body)
        .map_err(|_| DbUrlError::Message("databricks oidc response has no access_token".into()))?;
    if parsed.access_token.trim().is_empty() {
        return Err(DbUrlError::Message(
            "databricks oidc response has no access_token".into(),
        ));
    }
    Ok(parsed.access_token)
}

#[derive(Deserialize)]
struct OidcBody {
    access_token: String,
}

fn workspace_host() -> Result<String, DbUrlError> {
    let Some(raw) = std::env::var("DATABRICKS_HOST").ok() else {
        return Err(DbUrlError::Message("DATABRICKS_HOST is required".into()));
    };
    let raw = raw.trim().trim_end_matches('/').to_string();
    if raw.is_empty() {
        return Err(DbUrlError::Message("DATABRICKS_HOST is empty".into()));
    }
    if raw.starts_with("https://") || raw.starts_with("http://") {
        Ok(raw)
    } else {
        Ok(format!("https://{raw}"))
    }
}

fn required(name: &str) -> Result<String, DbUrlError> {
    nonempty(name).ok_or_else(|| DbUrlError::Message(format!("{name} is required for Lakebase")))
}

fn nonempty(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn pg_port() -> Result<u16, DbUrlError> {
    match nonempty("PGPORT") {
        None => Ok(5432),
        Some(value) => value
            .parse()
            .map_err(|_| DbUrlError::Message("PGPORT is required for Lakebase".into())),
    }
}

fn http_client() -> Result<reqwest::Client, DbUrlError> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| DbUrlError::Message(format!("credential request failed: {error}")))
}

#[cfg(feature = "postgres")]
fn apply_password(bundle: &edgequake_storage::PgPoolBundle, password: &str) {
    for pool in [&bundle.ingest, &bundle.admin, &bundle.queue] {
        let updated = (*pool.connect_options()).clone().password(password);
        pool.set_connect_options(updated);
    }
    if !bundle.query_uses_read_url {
        let updated = (*bundle.query.connect_options()).clone().password(password);
        bundle.query.set_connect_options(updated);
    }
}

fn remember_refresh_wait(expire_time: Option<&str>) -> u64 {
    let wait = expire_time.and_then(wait_until_refresh).unwrap_or(55 * 60);
    let mut slot = REFRESH_WAIT.lock().unwrap_or_else(|err| err.into_inner());
    *slot = wait;
    wait
}

#[cfg(feature = "postgres")]
fn refresh_wait_secs() -> u64 {
    let slot = REFRESH_WAIT.lock().unwrap_or_else(|err| err.into_inner());
    (*slot).max(60)
}

/// Seconds until five minutes before `expire_time` (RFC3339).
fn wait_until_refresh(expire_time: &str) -> Option<u64> {
    let expire = chrono::DateTime::parse_from_rfc3339(expire_time).ok()?;
    let expire_secs = expire.timestamp().max(0) as u64;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(
        expire_secs
            .saturating_sub(now)
            .saturating_sub(5 * 60)
            .max(60),
    )
}

static REFRESH_WAIT: std::sync::Mutex<u64> = std::sync::Mutex::new(55 * 60);
static LAKEBASE_URL_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_url_percent_encodes_password() {
        let url = postgres_url(
            "sp",
            "p@ss word",
            "db.example",
            5432,
            "databricks_postgres",
            "require",
        );
        assert_eq!(
            url,
            "postgresql://sp:p%40ss%20word@db.example:5432/databricks_postgres?sslmode=require"
        );
    }

    #[test]
    fn refresh_wait_is_five_minutes_before_expiry() {
        let expire = chrono::Utc::now() + chrono::Duration::seconds(3600);
        let wait = wait_until_refresh(&expire.to_rfc3339()).expect("parse");
        assert!((3000..=3600).contains(&wait), "{wait}");
    }
}
