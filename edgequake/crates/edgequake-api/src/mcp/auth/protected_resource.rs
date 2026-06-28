//! RFC 9728 Protected Resource Metadata for MCP.

use axum::{extract::State, http::HeaderMap, Json};
use serde::Serialize;

use crate::mcp::config::McpPublicConfig;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub bearer_methods_supported: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_documentation: Option<String>,
}

pub fn protected_resource_metadata(headers: &HeaderMap) -> ProtectedResourceMetadata {
    let cfg = McpPublicConfig::resolve(headers);
    ProtectedResourceMetadata {
        resource: cfg.resource_url.clone(),
        authorization_servers: vec![cfg.authorization_server.clone()],
        scopes_supported: vec![
            "edgequake:read".to_string(),
            "edgequake:query".to_string(),
            "openid".to_string(),
            "profile".to_string(),
        ],
        bearer_methods_supported: vec!["header"],
        resource_documentation: Some(
            "https://github.com/edgequake/edgequake/blob/main/specs/028-edgequake-query-service/mcp/000-index.md"
                .to_string(),
        ),
    }
}

pub async fn mcp_oauth_protected_resource(
    State(_state): State<AppState>,
    headers: HeaderMap,
) -> Json<ProtectedResourceMetadata> {
    Json(protected_resource_metadata(&headers))
}
