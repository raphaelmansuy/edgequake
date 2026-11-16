//! Provider connection types (SPEC-163).

use serde::{Deserialize, Serialize};

/// A stored or env-backed provider connection. The API key is never returned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub slug: String,
    pub display_name: String,
    pub api_shape: String,
    pub locality: String,
    pub base_url: String,
    pub auth_scheme: String,
    #[serde(default)]
    pub key_fingerprint: Option<String>,
    pub key_configured: bool,
    pub timeout_secs: i32,
    pub allow_private_network: bool,
    pub source: String,
    #[serde(default)]
    pub last_test_ok: Option<bool>,
    #[serde(default)]
    pub last_test_error: Option<String>,
    /// Google Cloud project saved on a Vertex connection.
    #[serde(default)]
    pub google_project: Option<String>,
}

/// Body for POST and PUT /api/v1/connections.
#[derive(Debug, Clone, Serialize)]
pub struct UpsertConnection {
    pub slug: String,
    pub display_name: String,
    pub api_shape: String,
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_scheme: Option<String>,
    /// Write-only. Never returned on [`Connection`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_private_network: Option<bool>,
    /// Google Cloud project. Saved on a Vertex connection and used for queries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub google_project: Option<String>,
}

/// Body for POST /api/v1/providers/test.
#[derive(Debug, Clone, Serialize)]
pub struct ProviderTestRequest {
    pub shape: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_private_network: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_dimension: Option<u32>,
}

/// Result of a provider probe.
#[derive(Debug, Clone, Deserialize)]
pub struct ProbeResult {
    pub ok: bool,
    pub kind: String,
    pub latency_ms: u64,
    pub message: String,
    #[serde(default)]
    pub models: Vec<String>,
    #[serde(default)]
    pub embedding_dimension: Option<u32>,
    pub chat_ok: bool,
    pub embed_ok: bool,
    pub list_ok: bool,
}
