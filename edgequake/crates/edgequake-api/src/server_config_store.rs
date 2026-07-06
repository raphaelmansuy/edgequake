//! PostgreSQL `server_config` persistence for LLM defaults (SPEC-043).

use std::sync::{Arc, RwLock};

use edgequake_core::{install_server_config, ConfigPriorityMode, ServerLlmDefaults};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock as AsyncRwLock;

pub const KEY_LLM_DEFAULTS: &str = "llm_defaults";
pub const KEY_CONFIG_PRIORITY: &str = "config_priority";
pub const KEY_APP_ATTRIBUTION: &str = "app_attribution";

/// JSON shape stored under `server_config.key = 'app_attribution'`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerAppAttribution {
    pub app_id: Option<String>,
    pub app_name: Option<String>,
    pub app_url: Option<String>,
}

static APP_ATTRIBUTION: RwLock<ServerAppAttribution> = RwLock::new(ServerAppAttribution {
    app_id: None,
    app_name: None,
    app_url: None,
});

/// Install or replace server app attribution for this process (startup / PATCH).
pub fn install_app_attribution(attribution: ServerAppAttribution) {
    if let Ok(mut guard) = APP_ATTRIBUTION.write() {
        *guard = attribution;
    }
}

/// Current server-config app attribution (empty when unset or non-Postgres).
pub fn current_app_attribution() -> ServerAppAttribution {
    APP_ATTRIBUTION
        .read()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Snapshot used by explainability and GET handlers.
#[derive(Debug, Clone, Default)]
pub struct ServerConfigSnapshot {
    pub llm_defaults: ServerLlmDefaults,
    pub priority_mode: ConfigPriorityMode,
    pub app_attribution: ServerAppAttribution,
    pub postgres_available: bool,
}

#[derive(Debug, Clone, Default)]
struct Inner {
    llm_defaults: ServerLlmDefaults,
    priority_mode: ConfigPriorityMode,
    app_attribution: ServerAppAttribution,
}

/// Thread-safe cache of server-wide LLM configuration.
#[derive(Clone, Default)]
pub struct ServerConfigStore {
    inner: Arc<AsyncRwLock<Inner>>,
}

impl ServerConfigStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn snapshot(&self) -> ServerConfigSnapshot {
        let inner = self.inner.read().await;
        ServerConfigSnapshot {
            llm_defaults: inner.llm_defaults.clone(),
            priority_mode: inner.priority_mode,
            app_attribution: inner.app_attribution.clone(),
            postgres_available: false,
        }
    }

    pub async fn apply(&self, defaults: ServerLlmDefaults, priority: ConfigPriorityMode) {
        {
            let mut inner = self.inner.write().await;
            inner.llm_defaults = defaults.clone();
            inner.priority_mode = priority;
        }
        install_server_config(defaults, priority);
    }

    pub async fn apply_app_attribution(&self, attribution: ServerAppAttribution) {
        {
            let mut inner = self.inner.write().await;
            inner.app_attribution = attribution.clone();
        }
        install_app_attribution(attribution);
    }

    /// Load from PostgreSQL and sync into process-wide overrides.
    #[cfg(feature = "postgres")]
    pub async fn load_from_pool(&self, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
        let defaults = load_llm_defaults(pool).await?;
        let priority = load_priority_mode(pool).await?;
        let attribution = load_app_attribution(pool).await?;
        {
            let mut inner = self.inner.write().await;
            inner.llm_defaults = defaults.clone();
            inner.priority_mode = priority;
            inner.app_attribution = attribution.clone();
        }
        install_server_config(defaults, priority);
        install_app_attribution(attribution);
        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub async fn snapshot_with_postgres(
        &self,
        pool: Option<&sqlx::PgPool>,
    ) -> ServerConfigSnapshot {
        let inner = self.inner.read().await;
        ServerConfigSnapshot {
            llm_defaults: inner.llm_defaults.clone(),
            priority_mode: inner.priority_mode,
            app_attribution: inner.app_attribution.clone(),
            postgres_available: pool.is_some(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct LlmDefaultsJson {
    llm_provider: Option<String>,
    llm_model: Option<String>,
    embedding_provider: Option<String>,
    embedding_model: Option<String>,
    vision_provider: Option<String>,
    vision_model: Option<String>,
}

impl From<LlmDefaultsJson> for ServerLlmDefaults {
    fn from(v: LlmDefaultsJson) -> Self {
        Self {
            llm_provider: v.llm_provider.filter(|s| !s.trim().is_empty()),
            llm_model: v.llm_model.filter(|s| !s.trim().is_empty()),
            embedding_provider: v.embedding_provider.filter(|s| !s.trim().is_empty()),
            embedding_model: v.embedding_model.filter(|s| !s.trim().is_empty()),
            vision_provider: v.vision_provider.filter(|s| !s.trim().is_empty()),
            vision_model: v.vision_model.filter(|s| !s.trim().is_empty()),
        }
    }
}

impl From<ServerLlmDefaults> for LlmDefaultsJson {
    fn from(v: ServerLlmDefaults) -> Self {
        Self {
            llm_provider: v.llm_provider,
            llm_model: v.llm_model,
            embedding_provider: v.embedding_provider,
            embedding_model: v.embedding_model,
            vision_provider: v.vision_provider,
            vision_model: v.vision_model,
        }
    }
}

#[cfg(feature = "postgres")]
pub async fn load_llm_defaults(pool: &sqlx::PgPool) -> Result<ServerLlmDefaults, sqlx::Error> {
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT value FROM server_config WHERE key = $1")
            .bind(KEY_LLM_DEFAULTS)
            .fetch_optional(pool)
            .await?;

    Ok(row
        .and_then(|(v,)| serde_json::from_value::<LlmDefaultsJson>(v).ok())
        .map(ServerLlmDefaults::from)
        .unwrap_or_default())
}

#[cfg(feature = "postgres")]
pub async fn save_llm_defaults(
    pool: &sqlx::PgPool,
    defaults: &ServerLlmDefaults,
) -> Result<(), sqlx::Error> {
    let value = serde_json::to_value(LlmDefaultsJson::from(defaults.clone()))
        .unwrap_or(serde_json::json!({}));
    sqlx::query(
        r#"
        INSERT INTO server_config (key, value, updated_at)
        VALUES ($1, $2::jsonb, NOW())
        ON CONFLICT (key) DO UPDATE
          SET value = EXCLUDED.value,
              updated_at = NOW()
        "#,
    )
    .bind(KEY_LLM_DEFAULTS)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub async fn load_priority_mode(pool: &sqlx::PgPool) -> Result<ConfigPriorityMode, sqlx::Error> {
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT value FROM server_config WHERE key = $1")
            .bind(KEY_CONFIG_PRIORITY)
            .fetch_optional(pool)
            .await?;

    let mode = row
        .and_then(|(v,)| v.as_str().map(ConfigPriorityMode::parse))
        .unwrap_or_else(env_priority_mode);
    Ok(mode)
}

#[cfg(feature = "postgres")]
pub async fn save_priority_mode(
    pool: &sqlx::PgPool,
    mode: ConfigPriorityMode,
) -> Result<(), sqlx::Error> {
    let value = serde_json::json!(mode.as_str());
    sqlx::query(
        r#"
        INSERT INTO server_config (key, value, updated_at)
        VALUES ($1, $2::jsonb, NOW())
        ON CONFLICT (key) DO UPDATE
          SET value = EXCLUDED.value,
              updated_at = NOW()
        "#,
    )
    .bind(KEY_CONFIG_PRIORITY)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn env_priority_mode() -> ConfigPriorityMode {
    std::env::var("EDGEQUAKE_CONFIG_PRIORITY")
        .ok()
        .map(|v| ConfigPriorityMode::parse(&v))
        .unwrap_or_default()
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct AppAttributionJson {
    app_id: Option<String>,
    app_name: Option<String>,
    app_url: Option<String>,
}

impl From<AppAttributionJson> for ServerAppAttribution {
    fn from(v: AppAttributionJson) -> Self {
        Self {
            app_id: v.app_id.filter(|s| !s.trim().is_empty()),
            app_name: v.app_name.filter(|s| !s.trim().is_empty()),
            app_url: v.app_url.filter(|s| !s.trim().is_empty()),
        }
    }
}

impl From<ServerAppAttribution> for AppAttributionJson {
    fn from(v: ServerAppAttribution) -> Self {
        Self {
            app_id: v.app_id,
            app_name: v.app_name,
            app_url: v.app_url,
        }
    }
}

#[cfg(feature = "postgres")]
pub async fn load_app_attribution(
    pool: &sqlx::PgPool,
) -> Result<ServerAppAttribution, sqlx::Error> {
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT value FROM server_config WHERE key = $1")
            .bind(KEY_APP_ATTRIBUTION)
            .fetch_optional(pool)
            .await?;

    Ok(row
        .and_then(|(v,)| serde_json::from_value::<AppAttributionJson>(v).ok())
        .map(ServerAppAttribution::from)
        .unwrap_or_default())
}

#[cfg(feature = "postgres")]
pub async fn save_app_attribution(
    pool: &sqlx::PgPool,
    attribution: &ServerAppAttribution,
) -> Result<(), sqlx::Error> {
    let value = serde_json::to_value(AppAttributionJson::from(attribution.clone()))
        .unwrap_or(serde_json::json!({}));
    sqlx::query(
        r#"
        INSERT INTO server_config (key, value, updated_at)
        VALUES ($1, $2::jsonb, NOW())
        ON CONFLICT (key) DO UPDATE
          SET value = EXCLUDED.value,
              updated_at = NOW()
        "#,
    )
    .bind(KEY_APP_ATTRIBUTION)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_app_attribution_updates_process_store() {
        install_app_attribution(ServerAppAttribution {
            app_id: Some("eq-test".into()),
            app_name: Some("EdgeQuake Test".into()),
            app_url: None,
        });
        let current = current_app_attribution();
        assert_eq!(current.app_id.as_deref(), Some("eq-test"));
        install_app_attribution(ServerAppAttribution::default());
    }
}
