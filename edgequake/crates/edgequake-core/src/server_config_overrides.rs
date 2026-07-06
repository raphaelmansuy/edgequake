//! Server-wide LLM defaults from `server_config` table (SPEC-043).
//!
//! Installed at API startup from PostgreSQL; used by [`crate::Workspace`] resolution
//! and explainability endpoints.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// JSON shape stored under `server_config.key = 'llm_defaults'`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLlmDefaults {
    pub llm_provider: Option<String>,
    pub llm_model: Option<String>,
    pub embedding_provider: Option<String>,
    pub embedding_model: Option<String>,
    pub vision_provider: Option<String>,
    pub vision_model: Option<String>,
}

/// Which source wins when both env vars and server DB define a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigPriorityMode {
    /// `server_config` overrides environment (first-principles default).
    #[default]
    ServerFirst,
    /// Environment variables override `server_config`.
    EnvFirst,
}

impl ConfigPriorityMode {
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "env" | "environment" => Self::EnvFirst,
            _ => Self::ServerFirst,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ServerFirst => "server",
            Self::EnvFirst => "env",
        }
    }
}

#[derive(Clone, Default)]
struct StoredConfig {
    defaults: ServerLlmDefaults,
    priority: ConfigPriorityMode,
}

static STORE: RwLock<StoredConfig> = RwLock::new(StoredConfig {
    defaults: ServerLlmDefaults {
        llm_provider: None,
        llm_model: None,
        embedding_provider: None,
        embedding_model: None,
        vision_provider: None,
        vision_model: None,
    },
    priority: ConfigPriorityMode::ServerFirst,
});

/// Install or replace server defaults for this process (startup / PATCH).
pub fn install_server_config(defaults: ServerLlmDefaults, priority: ConfigPriorityMode) {
    if let Ok(mut guard) = STORE.write() {
        guard.defaults = defaults;
        guard.priority = priority;
    }
}

pub fn current_defaults() -> ServerLlmDefaults {
    STORE.read().map(|g| g.defaults.clone()).unwrap_or_default()
}

pub fn current_priority_mode() -> ConfigPriorityMode {
    std::env::var("EDGEQUAKE_CONFIG_PRIORITY")
        .ok()
        .map(|v| ConfigPriorityMode::parse(&v))
        .or_else(|| STORE.read().ok().map(|g| g.priority))
        .unwrap_or_default()
}

/// Merge env-resolved value with server DB value according to priority mode.
pub fn merge_config_field(
    env_value: Option<String>,
    server_value: Option<String>,
    fallback: String,
    priority: ConfigPriorityMode,
) -> String {
    match priority {
        ConfigPriorityMode::ServerFirst => server_value.or(env_value).unwrap_or(fallback),
        ConfigPriorityMode::EnvFirst => env_value.or(server_value).unwrap_or(fallback),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_server_first_prefers_db() {
        assert_eq!(
            merge_config_field(
                Some("env".into()),
                Some("db".into()),
                "fallback".into(),
                ConfigPriorityMode::ServerFirst,
            ),
            "db"
        );
    }

    #[test]
    fn merge_env_first_prefers_env() {
        assert_eq!(
            merge_config_field(
                Some("env".into()),
                Some("db".into()),
                "fallback".into(),
                ConfigPriorityMode::EnvFirst,
            ),
            "env"
        );
    }
}
