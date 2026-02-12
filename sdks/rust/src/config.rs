//! Configuration types for the EdgeQuake SDK.

use std::time::Duration;

/// Client configuration with sensible defaults.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub base_url: String,
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub max_retries: u32,
    pub retry_backoff: Duration,
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(5),
            max_retries: 3,
            retry_backoff: Duration::from_millis(500),
            user_agent: format!("edgequake-rust/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Authentication method.
#[derive(Clone, Debug, Default)]
pub enum Auth {
    /// No authentication.
    #[default]
    None,
    /// API key (sent as `Authorization: Bearer <key>`).
    ApiKey(String),
    /// JWT bearer token.
    Bearer(String),
}

/// Multi-tenant context.
#[derive(Clone, Debug, Default)]
pub struct TenantContext {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub workspace_id: Option<String>,
}
