//! SPEC-180 W5a — the MCP surface of a `normal` workspace (Postgres).
//!
//! Covers T-180-mcp-entity-409, T-180-mcp-filters, T-180-mcp-instructions and
//! T-180-mcp-no-subgraph.

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::Request;
use common::spec180_harness::{create_workspace, harness, upload_scoped, Harness, USER_ID};
use serde_json::{json, Value};
use serial_test::serial;
use tower::ServiceExt;

async fn rpc(h: &Harness, tenant: &str, ws: &str, method: &str, params: Value) -> Value {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let response = h
        .app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/mcp")
                .header("Content-Type", "application/json")
                .header("X-Tenant-ID", tenant)
                .header("X-User-ID", USER_ID)
                .header("X-Workspace-ID", ws)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    common::spec028_mcp::parse_json(response).await
}

async fn call(h: &Harness, tenant: &str, ws: &str, tool: &str, args: Value) -> Value {
    let reply = rpc(
        h,
        tenant,
        ws,
        "tools/call",
        json!({"name": tool, "arguments": args}),
    )
    .await;
    common::spec028_mcp::tool_structured(&reply).clone()
}

#[test]
#[serial]
fn a_normal_workspace_has_chunks_only_on_the_mcp_surface() {
    common::spec180::block_on_big_stack(normal_surface());
}

async fn normal_surface() {
    let Some(h) = harness().await else {
        eprintln!("SKIP spec180 mcp: postgres not reachable");
        return;
    };
    let (tenant, ws) = create_workspace(&h, "normal").await;
    let (doc, _, _) = upload_scoped(&h, &tenant, &ws, "garden", "Tomatoes need warm soil.").await;

    // T-180-mcp-entity-409: every graph tool is refused with the REST code.
    for (tool, args) in [
        ("eq_entity_search", json!({"q": "tomato"})),
        ("eq_entity_get", json!({"entity_id": "TOMATO"})),
        ("eq_neighborhood", json!({"entity_id": "TOMATO"})),
        ("eq_graph_image", json!({"entity_id": "TOMATO"})),
    ] {
        let out = call(&h, &tenant, &ws, tool, args).await;
        assert_eq!(out["ok"], false, "{tool}: {out}");
        assert_eq!(
            out["error"]["code"], "profile_has_no_graph",
            "{tool}: {out}"
        );
    }

    // T-180-mcp-filters and T-180-mcp-no-subgraph: chunks come back, entities do not.
    let hit = call(
        &h,
        &tenant,
        &ws,
        "eq_search",
        json!({
            "query": "tomatoes", "doc_kinds": ["markdown"],
        }),
    )
    .await;
    assert_ne!(hit["ok"], false, "{hit}");
    let hits = hit["hits"].as_array().expect("hits");
    assert!(!hits.is_empty(), "{hit}");
    assert!(
        hits.iter().all(|x| x["kind"] != "entity"),
        "entity hit on a normal workspace: {hit}"
    );
    assert!(hits.iter().any(|x| x["document_id"] == json!(doc)), "{hit}");

    let none = call(
        &h,
        &tenant,
        &ws,
        "eq_search",
        json!({
            "query": "tomatoes", "document_filter": {"source_modalities": ["image"]},
        }),
    )
    .await;
    assert!(none["hits"].as_array().is_some_and(Vec::is_empty), "{none}");

    // T-180-mcp-instructions: the connection is told what it can and cannot do.
    let init = rpc(&h, &tenant, &ws, "initialize", json!({})).await;
    let text = init["result"]["instructions"]
        .as_str()
        .expect("instructions");
    assert!(text.contains("profile_has_no_graph"), "{text}");

    // A graph workspace keeps the plain text and its tools.
    let (gt, gw) = common::spec180_harness::create_workspace(&h, "graph").await;
    let init = rpc(&h, &gt, &gw, "initialize", json!({})).await;
    assert!(!init["result"]["instructions"]
        .as_str()
        .unwrap()
        .contains("profile_has_no_graph"));
    let out = call(&h, &gt, &gw, "eq_entity_search", json!({"q": "tomato"})).await;
    assert_ne!(out["error"]["code"], "profile_has_no_graph", "{out}");
}

#[test]
fn the_search_tools_declare_the_kind_filters() {
    let listing = edgequake_api::mcp::gateway::tools::tools_list_for(
        edgequake_api::mcp::project::McpProfile::Query,
    );
    for tool in listing["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        if matches!(
            name,
            "eq_search" | "eq_retrieve" | "edgequake_search" | "edgequake_retrieve"
        ) {
            let props = &tool["inputSchema"]["properties"];
            for field in ["doc_kinds", "mime_types", "source_modalities"] {
                assert!(props.get(field).is_some(), "{name} lacks {field}");
            }
        }
    }
}
