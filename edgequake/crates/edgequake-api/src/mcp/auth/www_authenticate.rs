//! WWW-Authenticate builder for MCP OAuth (RFC 6750 + RFC 9728).

use crate::mcp::config::McpPublicConfig;

/// Build `WWW-Authenticate` header value for MCP 401 responses.
pub fn www_authenticate_bearer(headers: &axum::http::HeaderMap) -> String {
    let cfg = McpPublicConfig::resolve(headers);
    let prm_url = cfg.protected_resource_metadata_url();
    format!(r#"Bearer realm="edgequake-mcp", resource_metadata="{prm_url}""#)
}
