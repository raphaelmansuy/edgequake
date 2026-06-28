//! MCP Registry `server.json` SSOT (official registry remote metadata).

use serde_json::{json, Value};

/// Official MCP Registry server name (`io.github.{owner}/{server}`).
pub const REGISTRY_SERVER_NAME: &str = "io.github.raphaelmansuy/edgequake";

pub const SERVER_JSON_SCHEMA: &str =
    "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json";

pub const REGISTRY_REPOSITORY_URL: &str = "https://github.com/raphaelmansuy/edgequake";

pub const REGISTRY_DOCUMENTATION_URL: &str =
    "https://github.com/raphaelmansuy/edgequake/tree/main/specs/028-edgequake-query-service/mcp";

/// Build MCP Registry manifest. When `public_base` is `Some`, remotes use a resolved URL;
/// otherwise remotes expose `{host}` for self-hosted deployments (publish artifact).
pub fn build_registry_manifest(public_base: Option<&str>) -> Value {
    let version = env!("CARGO_PKG_VERSION");
    let remote = remote_transport(public_base);

    json!({
        "$schema": SERVER_JSON_SCHEMA,
        "name": REGISTRY_SERVER_NAME,
        "title": "EdgeQuake",
        "description": "Graph RAG MCP: search, fetch, and retrieve knowledge-graph context over Streamable HTTP.",
        "version": version,
        "websiteUrl": REGISTRY_DOCUMENTATION_URL,
        "repository": {
            "url": REGISTRY_REPOSITORY_URL,
            "source": "github",
            "subfolder": "edgequake/crates/edgequake-api/src/mcp"
        },
        "remotes": [remote],
        "_meta": {
            "io.modelcontextprotocol.registry/publisher-provided": {
                "tool": "edgequake-api",
                "protocolVersions": ["2026-07-28", "2025-11-25"],
                "oauthProtectedResource": "/.well-known/oauth-protected-resource",
                "streamingRetrieve": "Mcp-Stream: true on edgequake_retrieve"
            }
        }
    })
}

fn remote_transport(public_base: Option<&str>) -> Value {
    let (url, variables) = match public_base {
        Some(base) => {
            let base = base.trim().trim_end_matches('/');
            (format!("{base}/mcp"), None)
        }
        None => (
            "https://{host}/mcp".to_string(),
            Some(json!({
                "host": {
                    "description": "EdgeQuake API host (HTTPS in production, include port if needed)",
                    "isRequired": true,
                    "format": "string",
                    "placeholder": "api.example.com"
                },
                "accessToken": {
                    "description": "OAuth 2.1 access token or EdgeQuake API key",
                    "isRequired": true,
                    "format": "string",
                    "isSecret": true
                },
                "tenantId": {
                    "description": "Optional tenant scope (X-Tenant-ID)",
                    "isRequired": false,
                    "format": "string",
                    "placeholder": "default"
                }
            })),
        ),
    };

    let mut remote = json!({
        "type": "streamable-http",
        "url": url,
        "headers": streamable_http_headers(public_base.is_some())
    });

    if let Some(vars) = variables {
        remote["variables"] = vars;
    }

    remote
}

fn streamable_http_headers(resolved: bool) -> Value {
    if resolved {
        json!([
            {
                "name": "Accept",
                "description": "Streamable HTTP — JSON and SSE",
                "isRequired": true,
                "format": "string",
                "value": "application/json, text/event-stream"
            },
            {
                "name": "MCP-Protocol-Version",
                "description": "MCP protocol version",
                "isRequired": false,
                "format": "string",
                "value": "2026-07-28"
            }
        ])
    } else {
        json!([
            {
                "name": "Accept",
                "description": "Streamable HTTP — JSON and SSE",
                "isRequired": true,
                "format": "string",
                "value": "application/json, text/event-stream"
            },
            {
                "name": "Authorization",
                "description": "OAuth 2.1 Bearer token or API key",
                "isRequired": true,
                "format": "string",
                "isSecret": true,
                "value": "Bearer {accessToken}"
            },
            {
                "name": "X-Tenant-ID",
                "description": "Tenant scope for multi-tenant deployments",
                "isRequired": false,
                "format": "string",
                "value": "{tenantId}"
            },
            {
                "name": "MCP-Protocol-Version",
                "description": "MCP protocol version (modern clients)",
                "isRequired": false,
                "format": "string",
                "value": "2026-07-28"
            }
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_manifest_uses_template_host() {
        let manifest = build_registry_manifest(None);
        assert_eq!(manifest["name"], REGISTRY_SERVER_NAME);
        assert!(manifest["remotes"][0]["url"]
            .as_str()
            .unwrap()
            .contains("{host}"));
    }

    #[test]
    fn live_manifest_resolves_public_url() {
        let manifest = build_registry_manifest(Some("https://api.edgequake.example"));
        assert_eq!(
            manifest["remotes"][0]["url"],
            "https://api.edgequake.example/mcp"
        );
    }
}
