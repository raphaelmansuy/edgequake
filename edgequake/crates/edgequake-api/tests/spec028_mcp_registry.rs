//! SPEC-028 MCP Registry publish + manifest contract tests.

mod common;

use std::path::PathBuf;

use edgequake_api::mcp::{build_registry_manifest, REGISTRY_SERVER_NAME, SERVER_JSON_SCHEMA};
use serde_json::Value;

fn read_server_json() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../specs/028-edgequake-query-service/mcp/server.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read server.json at {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("parse server.json")
}

#[test]
fn spec028_mcp_registry_server_json_matches_rust_ssot() {
    let file = read_server_json();
    let ssot = build_registry_manifest(None);

    for key in ["name", "title", "description", "$schema"] {
        assert_eq!(file[key], ssot[key], "server.json drift on {key}");
    }
    assert_eq!(file["name"].as_str(), Some(REGISTRY_SERVER_NAME));
    assert_eq!(file["remotes"][0]["type"], "streamable-http");
    assert!(file["remotes"][0]["url"]
        .as_str()
        .unwrap()
        .contains("{host}"));
}

#[test]
fn spec028_mcp_registry_server_json_required_fields() {
    let file = read_server_json();
    assert_eq!(file["$schema"], SERVER_JSON_SCHEMA);
    assert!(file["version"].as_str().is_some_and(|v| !v.is_empty()));
    assert!(file["description"].as_str().is_some_and(|d| d.len() <= 100));
    assert!(file["repository"]["url"].as_str().is_some());
    assert_eq!(file["repository"]["source"], "github");
}

#[test]
fn spec028_mcp_registry_module_wired() {
    let routes =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/routes.rs"))
            .unwrap();
    assert!(routes.contains("/.well-known/mcp/server.json"));
    assert!(routes.contains("mcp_registry_server_json"));
}
