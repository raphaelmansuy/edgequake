//! PostgreSQL `server_config` persistence for LLM defaults (SPEC-043).

use std::sync::Arc;

use edgequake_core::{install_server_config, ConfigPriorityMode, ServerLlmDefaults};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub const KEY_LLM_DEFAULTS: &str = "llm_defaults";
pub const KEY_CONFIG_PRIORITY: &str = "config_priority";

/// Snapshot used by explainability and GET handlers.
#[derive(Debug, Clone, Default)]
pub struct ServerConfigSnapshot {
    pub llm_defaults: ServerLlmDefaults,
    pub priority_mode: ConfigPriorityMode,
    pub postgres_available: bool,
}

#[derive(Debug, Clone, Default)]
struct Inner {
    llm_defaults: ServerLlmDefaults,
    priority_mode: ConfigPriorityMode,
}

/// Thread-safe cache of server-wide LLM configuration.
#[derive(Clone, Default)]
pub struct ServerConfigStore {
    inner: Arc<RwLock<Inner>>,
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

    /// Load from PostgreSQL and sync into process-wide overrides.
    #[cfg(feature = "postgres")]
    pub async fn load_from_pool(&self, pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
        let defaults = load_llm_defaults(pool).await?;
        let priority = load_priority_mode(pool).await?;
        self.apply(defaults, priority).await;
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
