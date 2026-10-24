//! Workspace bearer for Lakebase and the MLflow AI Gateway.
//!
//! An App uses `DATABRICKS_CLIENT_ID` and `DATABRICKS_CLIENT_SECRET`.
//! A local probe may set `DATABRICKS_TOKEN` instead.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DatabricksBearer {
    inner: Arc<Inner>,
}

struct Inner {
    host: String,
    client: reqwest::Client,
    cache: Mutex<Option<Cached>>,
    mode: Mode,
}

enum Mode {
    Personal(String),
    ServicePrincipal {
        client_id: String,
        client_secret: String,
    },
}

struct Cached {
    token: String,
    refresh_at: Instant,
}

impl DatabricksBearer {
    pub fn from_env() -> Result<Self, String> {
        let host = workspace_host()?;
        let mode = if let (Ok(id), Ok(secret)) = (
            std::env::var("DATABRICKS_CLIENT_ID"),
            std::env::var("DATABRICKS_CLIENT_SECRET"),
        ) {
            let id = id.trim().to_string();
            let secret = secret.trim().to_string();
            if !id.is_empty() && !secret.is_empty() {
                Mode::ServicePrincipal {
                    client_id: id,
                    client_secret: secret,
                }
            } else {
                personal_token()?
            }
        } else {
            personal_token()?
        };
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| format!("databricks http client: {e}"))?;
        Ok(Self {
            inner: Arc::new(Inner {
                host,
                client,
                cache: Mutex::new(None),
                mode,
            }),
        })
    }

    pub fn host(&self) -> &str {
        &self.inner.host
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.inner.client
    }

    /// Bearer for Databricks REST and the AI Gateway. Not a database password.
    pub async fn access_token(&self) -> Result<String, String> {
        if let Mode::Personal(token) = &self.inner.mode {
            return Ok(token.clone());
        }
        let mut guard = self.inner.cache.lock().await;
        if let Some(cached) = guard.as_ref() {
            if Instant::now() < cached.refresh_at {
                return Ok(cached.token.clone());
            }
        }
        let (token, ttl) = self.mint_service_token().await?;
        let refresh_at = Instant::now() + ttl.saturating_sub(Duration::from_secs(120));
        *guard = Some(Cached {
            token: token.clone(),
            refresh_at,
        });
        Ok(token)
    }

    async fn mint_service_token(&self) -> Result<(String, Duration), String> {
        let Mode::ServicePrincipal {
            client_id,
            client_secret,
        } = &self.inner.mode
        else {
            return Err("service principal credentials are not set".into());
        };
        let url = format!("{}/oidc/v1/token", self.inner.host);
        let response = self
            .inner
            .client
            .post(url)
            .basic_auth(client_id, Some(client_secret))
            .form(&[("grant_type", "client_credentials"), ("scope", "all-apis")])
            .send()
            .await
            .map_err(|e| format!("databricks oidc: {e}"))?;
        let status = response.status();
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("databricks oidc body: {e}"))?;
        if !status.is_success() {
            return Err(format!(
                "databricks oidc status {status}: {}",
                body.get("error_description")
                    .or_else(|| body.get("error"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("token request failed")
            ));
        }
        let token = body
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or("databricks oidc response has no access_token")?
            .to_string();
        let seconds = body
            .get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);
        Ok((token, Duration::from_secs(seconds)))
    }
}

pub fn workspace_host() -> Result<String, String> {
    let raw =
        std::env::var("DATABRICKS_HOST").map_err(|_| "DATABRICKS_HOST is required".to_string())?;
    let raw = raw.trim().trim_end_matches('/');
    if raw.is_empty() {
        return Err("DATABRICKS_HOST is empty".into());
    }
    let host = if raw.starts_with("https://") || raw.starts_with("http://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    Ok(host)
}

fn personal_token() -> Result<Mode, String> {
    let token = std::env::var("DATABRICKS_TOKEN").map_err(|_| {
        "Set DATABRICKS_CLIENT_ID and DATABRICKS_CLIENT_SECRET, or DATABRICKS_TOKEN".to_string()
    })?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err("DATABRICKS_TOKEN is empty".into());
    }
    Ok(Mode::Personal(token))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_adds_https() {
        let _guard = crate::databricks::env_lock();
        std::env::set_var("DATABRICKS_HOST", "dbc-example.cloud.databricks.com/");
        let host = workspace_host().expect("host");
        assert_eq!(host, "https://dbc-example.cloud.databricks.com");
        std::env::remove_var("DATABRICKS_HOST");
    }
}
