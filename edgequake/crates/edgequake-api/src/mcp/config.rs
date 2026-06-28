//! MCP public URL and resource identity (RFC 9728).

use axum::http::HeaderMap;

/// Public MCP resource configuration derived from env or request Host.
#[derive(Debug, Clone)]
pub struct McpPublicConfig {
    pub resource_url: String,
    pub authorization_server: String,
}

impl McpPublicConfig {
    /// Resolve MCP resource URL and authorization server base.
    pub fn resolve(headers: &HeaderMap) -> Self {
        let public_base = std::env::var("EDGEQUAKE_PUBLIC_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| s.starts_with("http"))
            .or_else(|| host_header_base(headers))
            .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

        let auth_base = std::env::var("EDGEQUAKE_OAUTH_ISSUER_URL")
            .ok()
            .map(|s| s.trim().trim_end_matches('/').to_string())
            .filter(|s| s.starts_with("http"))
            .unwrap_or_else(|| format!("{public_base}/api/v1/auth/oidc"));

        Self {
            resource_url: format!("{public_base}/mcp"),
            authorization_server: auth_base,
        }
    }

    pub fn protected_resource_metadata_url(&self) -> String {
        let base = self
            .resource_url
            .trim_end_matches("/mcp")
            .trim_end_matches('/');
        format!("{base}/.well-known/oauth-protected-resource")
    }

    /// Public base URL without `/mcp` suffix.
    pub fn public_base_url(&self) -> String {
        self.resource_url
            .trim_end_matches("/mcp")
            .trim_end_matches('/')
            .to_string()
    }
}

fn host_header_base(headers: &HeaderMap) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?;
    if host.starts_with("http") {
        return Some(host.trim_end_matches('/').to_string());
    }
    Some(format!("http://{host}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_public_url_wins() {
        std::env::set_var("EDGEQUAKE_PUBLIC_URL", "https://api.example.com");
        let cfg = McpPublicConfig::resolve(&HeaderMap::new());
        assert_eq!(cfg.resource_url, "https://api.example.com/mcp");
        std::env::remove_var("EDGEQUAKE_PUBLIC_URL");
    }
}
