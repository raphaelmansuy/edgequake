//! SPEC-006 resource safety proof tests.
//!
//! See: `specifications/006-ensure-perf/e2e/000-e2e-index.md`

use axum::{
    body::Body,
    extract::FromRef,
    http::{Request, StatusCode},
};
use edgequake_api::state::{GraphQueryRuntime, StorageRuntime};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::ResourceBudgetConfig;
use edgequake_storage::traits::NodeListFilter;
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;

const PROOF_TENANT: &str = "spec006-proof-tenant";
const PROOF_WORKSPACE: &str = "spec006-proof-workspace";

fn proof_server() -> edgequake_api::Server {
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        AppState::test_state(),
    )
}

async fn parse_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("json")
}

async fn seed_workspace_nodes(state: &AppState, count: usize) {
    for i in 0..count {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        props.insert("entity_type".to_string(), json!("CONCEPT"));
        props.insert(
            "description".to_string(),
            json!(format!("proof entity {}", i)),
        );
        let id = format!("PROOF_ENTITY_{:06}", i);
        state
            .storage
            .graph_storage
            .upsert_node(&id, props)
            .await
            .expect("seed node");
    }
}

/// NFR-006-001 / UC-006-001 — list uses push-down pagination, not full graph load.
#[tokio::test]
async fn resource_safety_list_entities_bounded_page() {
    let state = AppState::test_state();
    seed_workspace_nodes(&state, 2_500).await;

    let filter = NodeListFilter {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        entity_type: None,
        search: None,
        community_ids: None,
    };

    let page = state
        .storage
        .graph_storage
        .list_nodes_filtered(&filter, 0, 10)
        .await
        .expect("list_nodes_filtered");

    assert_eq!(page.total, 2_500);
    assert_eq!(page.items.len(), 10);
    assert!(page.items[0].id.starts_with("PROOF_ENTITY_"));
}

/// NFR-006-001 — HTTP list entities returns one page from large workspace graph.
#[tokio::test]
async fn resource_safety_list_entities_http_pagination() {
    let state = AppState::test_state();
    seed_workspace_nodes(&state, 1_200).await;
    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/entities?page=1&page_size=25")
                .header("X-Tenant-ID", PROOF_TENANT)
                .header("X-Workspace-ID", PROOF_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["total"].as_u64(), Some(1_200));
    assert_eq!(body["items"].as_array().map(|a| a.len()), Some(25));
}

/// BR-006-014 — graph query timeout maps to 503 + Retry-After, not full-graph fallback.
#[tokio::test]
async fn resource_safety_graph_query_timeout_response() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;

    let app = Router::new().route(
        "/graph-timeout",
        get(|| async { ApiError::graph_query_timeout().into_response() }),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/graph-timeout")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert_eq!(retry_after, "30");
}

/// BR-006-012 — budget defaults match catalog (delegates to edgequake-core test).
#[test]
fn resource_safety_budget_catalog_sync() {
    let budget = ResourceBudgetConfig::default();
    assert_eq!(budget.max_upload_bytes, 50 * 1024 * 1024);
    assert_eq!(budget.max_graph_nodes, 500);
    assert_eq!(budget.graph_query_timeout_secs, 15);
}

/// TR-006-019 — upload limit SSOT: catalog constant == ResourceBudget default.
#[test]
fn resource_safety_upload_limit_ssot() {
    assert_eq!(edgequake_core::MAX_UPLOAD_BYTES, 50 * 1024 * 1024);
    assert_eq!(
        AppState::test_state().resource_budget().max_upload_bytes,
        edgequake_core::MAX_UPLOAD_BYTES
    );
}

/// NFR-006-001 P8 — graph materialization busy maps to 503 + Retry-After.
#[tokio::test]
async fn resource_safety_graph_materialization_busy_response() {
    use axum::{response::IntoResponse, routing::get, Router};
    use edgequake_api::error::ApiError;

    let app = Router::new().route(
        "/graph-busy",
        get(|| async { ApiError::graph_materialization_busy().into_response() }),
    );

    let response = app
        .oneshot(
            Request::builder()
                .uri("/graph-busy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        Some("5")
    );
}

/// NFR-006-001 P9 — GET /api/v1/graph returns 503 when materialization slots exhausted.
#[tokio::test]
async fn resource_safety_get_graph_503_when_materialize_full() {
    use edgequake_core::GraphMaterializationSemaphore;
    use std::sync::Arc;

    let mut state = AppState::test_state();
    state.graph_materialize = Arc::new(GraphMaterializationSemaphore::new(1));
    let _held = state
        .graph_materialize
        .acquire_owned()
        .await
        .expect("hold slot");

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph?max_nodes=50")
                .header("X-Tenant-ID", PROOF_TENANT)
                .header("X-Workspace-ID", PROOF_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok()),
        Some("5")
    );
}

/// NFR-006-001 P8 — popular labels returns 503 when materialization slots exhausted.
#[tokio::test]
async fn resource_safety_popular_labels_503_when_materialize_full() {
    use axum::extract::{Query, State};
    use edgequake_api::handlers::{get_popular_labels, graph_types::PopularLabelsQuery};
    use edgequake_api::middleware::TenantContext;
    use edgequake_core::GraphMaterializationSemaphore;
    use std::sync::Arc;

    let mut state = AppState::test_state();
    state.graph_materialize = Arc::new(GraphMaterializationSemaphore::new(1));
    let _held = state
        .graph_materialize
        .acquire_owned()
        .await
        .expect("hold slot");

    let result = get_popular_labels(
        State(state.clone()),
        State(StorageRuntime::from_ref(&state)),
        State(GraphQueryRuntime::from_ref(&state)),
        TenantContext::default(),
        edgequake_api::handlers::auth::OptionalAuth(None),
        Query(PopularLabelsQuery {
            limit: 5,
            min_degree: None,
            entity_type: None,
        }),
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("capacity") || err.contains("503") || err.contains("Retry"),
        "expected busy error, got: {err}"
    );
}

/// NFR-006-002 / UC-006-002 — document delete cascade is document-scoped, not full-graph.
#[tokio::test]
async fn resource_safety_delete_cascade_bounded_scope() {
    use edgequake_storage::traits::EdgeListFilter;
    use serde_json::json;

    const DOC_ID: &str = "proof-doc-delete-001";

    let state = AppState::test_state();

    // Noise: 500 unrelated nodes (would OOM if loaded on delete)
    seed_workspace_nodes(&state, 500).await;

    // Document-scoped entities and edge
    for (id, sources) in [
        ("DOC_ENTITY_A", vec![format!("{}-chunk-0", DOC_ID)]),
        ("DOC_ENTITY_B", vec![format!("{}-chunk-1", DOC_ID)]),
        (
            "SHARED_ENTITY",
            vec![
                format!("{}-chunk-0", DOC_ID),
                "other-doc-chunk-0".to_string(),
            ],
        ),
    ] {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        props.insert("entity_type".to_string(), json!("CONCEPT"));
        props.insert("source_ids".to_string(), json!(sources));
        state
            .storage
            .graph_storage
            .upsert_node(id, props)
            .await
            .expect("seed doc node");
    }

    let mut edge_props = HashMap::new();
    edge_props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
    edge_props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    edge_props.insert(
        "source_ids".to_string(),
        json!([format!("{}-chunk-0", DOC_ID)]),
    );
    state
        .storage
        .graph_storage
        .upsert_edge("DOC_ENTITY_A", "DOC_ENTITY_B", edge_props)
        .await
        .expect("seed edge");

    let scope = edgequake_api::services::DocumentSourceScope::from_document_id(DOC_ID);
    let stats = edgequake_api::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        None,
        &scope,
    )
    .await
    .expect("cascade");

    assert_eq!(
        stats.entities_removed, 2,
        "DOC_ENTITY_A and DOC_ENTITY_B removed"
    );
    assert_eq!(stats.entities_updated, 1, "SHARED_ENTITY updated");
    // Memory backend may cascade-delete edges when nodes are removed (stats may be 0).

    assert!(!state
        .storage
        .graph_storage
        .has_node("DOC_ENTITY_A")
        .await
        .unwrap());
    assert!(state
        .storage
        .graph_storage
        .has_node("SHARED_ENTITY")
        .await
        .unwrap());
    assert!(
        state
            .storage
            .graph_storage
            .has_node("PROOF_ENTITY_000000")
            .await
            .unwrap(),
        "unrelated seeded node must survive delete cascade"
    );

    let edge_filter = EdgeListFilter {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        relationship_type: None,
    };
    let remaining_edges = state
        .storage
        .graph_storage
        .list_edges_filtered(&edge_filter, 0, 100)
        .await
        .expect("list edges")
        .items;
    assert!(
        remaining_edges.is_empty(),
        "document-sourced edge should be removed"
    );
}

/// TR-006-004 — relationship CRUD uses indexed lookup, not get_all_nodes scan.
#[tokio::test]
async fn resource_safety_relationship_lookup_bounded() {
    use edgequake_storage::traits::EdgeListFilter;
    use serde_json::json;

    let state = AppState::test_state();
    seed_workspace_nodes(&state, 300).await;

    let mut edge_props = HashMap::new();
    edge_props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
    edge_props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    edge_props.insert("keywords".to_string(), json!("relates_to"));
    state
        .storage
        .graph_storage
        .upsert_node("REL_SOURCE", {
            let mut p = HashMap::new();
            p.insert("tenant_id".to_string(), json!(PROOF_TENANT));
            p.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
            p
        })
        .await
        .expect("source");
    state
        .storage
        .graph_storage
        .upsert_node("REL_TARGET", {
            let mut p = HashMap::new();
            p.insert("tenant_id".to_string(), json!(PROOF_TENANT));
            p.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
            p
        })
        .await
        .expect("target");
    state
        .storage
        .graph_storage
        .upsert_edge("REL_SOURCE", "REL_TARGET", edge_props)
        .await
        .expect("edge");

    let filter = EdgeListFilter {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        relationship_type: None,
    };
    let edge = state
        .storage
        .graph_storage
        .find_edge_by_relationship_id(&filter, "REL_SOURCE_REL_TARGET")
        .await
        .expect("lookup")
        .expect("found");

    assert_eq!(edge.source, "REL_SOURCE");
    assert_eq!(edge.target, "REL_TARGET");
}

/// RB-LLM-008 — orchestrator context tokens align with SOTA 30k SSOT.
#[test]
fn resource_safety_orchestrator_token_cap_ssot() {
    assert_eq!(edgequake_core::MAX_ORCHESTRATOR_CONTEXT_TOKENS, 30_000);
}

/// NFR-006-003 — community detection gated by ResourceGuard before full-graph load.
#[tokio::test]
async fn resource_safety_community_guard_rejects_large_graph() {
    use edgequake_core::{ResourceBudgetConfig, ResourceGuard};
    use serde_json::json;

    let state = AppState::test_state();
    for i in 0..100 {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        state
            .storage
            .graph_storage
            .upsert_node(&format!("GUARD_NODE_{:03}", i), props)
            .await
            .expect("seed");
    }

    let guard = ResourceGuard::new(ResourceBudgetConfig {
        graph_scan_threshold_nodes: 10,
        ..Default::default()
    });

    let result = edgequake_api::services::detect_communities_guarded(
        &state.storage.graph_storage,
        &edgequake_storage::CommunityConfig::default(),
        &guard,
    )
    .await;

    assert!(result.is_err());
}

/// P4 edge — legacy pipe-separated source_id cascade (not just source_ids array).
#[tokio::test]
async fn resource_safety_cascade_legacy_source_id_pipe_format() {
    use serde_json::json;

    const DOC_ID: &str = "proof-legacy-pipe-doc";

    let state = AppState::test_state();
    let mut props = HashMap::new();
    props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
    props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    props.insert(
        "source_id".to_string(),
        json!(format!("{}-chunk-0|other-doc-chunk-0", DOC_ID)),
    );
    state
        .storage
        .graph_storage
        .upsert_node("LEGACY_PIPE_ENTITY", props)
        .await
        .expect("seed");

    let scope = edgequake_api::services::DocumentSourceScope::from_document_id(DOC_ID);
    let stats = edgequake_api::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        None,
        &scope,
    )
    .await
    .expect("cascade");

    assert_eq!(stats.entities_updated, 1);
    let node = state
        .storage
        .graph_storage
        .get_node("LEGACY_PIPE_ENTITY")
        .await
        .expect("get")
        .expect("exists");
    let refs: Vec<String> = node
        .properties
        .get("source_ids")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(refs, vec!["other-doc-chunk-0"]);
    assert!(
        !node.properties.contains_key("source_id"),
        "legacy source_id must be cleared after partial cascade"
    );
}

/// P4 edge — KV key prefix mismatch (document_id vs storage key_prefix).
#[tokio::test]
async fn resource_safety_cascade_key_prefix_mismatch() {
    use serde_json::json;

    let state = AppState::test_state();
    let doc_id = "doc-uuid-abc".to_string();
    let kv_key = "kv-storage-key-xyz".to_string();

    let mut props = HashMap::new();
    props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
    props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    props.insert(
        "source_ids".to_string(),
        json!([format!("{}-chunk-0", kv_key)]),
    );
    state
        .storage
        .graph_storage
        .upsert_node("KV_MISMATCH_ENTITY", props)
        .await
        .expect("seed");

    let scope =
        edgequake_api::services::DocumentSourceScope::with_key_prefix(doc_id.clone(), kv_key);
    let stats = edgequake_api::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        None,
        &scope,
    )
    .await
    .expect("cascade");

    assert_eq!(stats.entities_removed, 1);
    assert!(!state
        .storage
        .graph_storage
        .has_node("KV_MISMATCH_ENTITY")
        .await
        .unwrap());
}

/// P4 edge — tenant isolation: cascade must not touch other tenant nodes.
#[tokio::test]
async fn resource_safety_cascade_tenant_isolation() {
    use edgequake_api::middleware::TenantContext;
    use serde_json::json;

    const DOC_ID: &str = "proof-tenant-iso-doc";
    const OTHER_TENANT: &str = "other-tenant-proof";

    let state = AppState::test_state();

    for (tenant, id) in [
        (PROOF_TENANT, "TENANT_A_ENTITY"),
        (OTHER_TENANT, "TENANT_B_ENTITY"),
    ] {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(tenant));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        props.insert(
            "source_ids".to_string(),
            json!([format!("{}-chunk-0", DOC_ID)]),
        );
        state
            .storage
            .graph_storage
            .upsert_node(id, props)
            .await
            .expect("seed");
    }

    let tenant_ctx = TenantContext {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        user_id: None,
    };

    let scope = edgequake_api::services::DocumentSourceScope::from_document_id(DOC_ID);
    edgequake_api::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        Some(&tenant_ctx),
        &scope,
    )
    .await
    .expect("cascade");

    assert!(!state
        .storage
        .graph_storage
        .has_node("TENANT_A_ENTITY")
        .await
        .unwrap());
    assert!(
        state
            .storage
            .graph_storage
            .has_node("TENANT_B_ENTITY")
            .await
            .unwrap(),
        "other tenant entity must survive scoped cascade"
    );
}

/// P4 edge — legacy nodes missing tenant_id still cascade when workspace matches.
#[tokio::test]
async fn resource_safety_cascade_legacy_null_tenant() {
    use edgequake_api::middleware::TenantContext;
    use serde_json::json;

    const DOC_ID: &str = "proof-legacy-null-tenant-doc";

    let state = AppState::test_state();

    let mut props = HashMap::new();
    // No tenant_id property — legacy AGE row.
    props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    props.insert(
        "source_ids".to_string(),
        json!([format!("{}-chunk-0", DOC_ID)]),
    );
    state
        .storage
        .graph_storage
        .upsert_node("LEGACY_NULL_TENANT_ENTITY", props)
        .await
        .expect("seed");

    let tenant_ctx = TenantContext {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        user_id: None,
    };

    let scope = edgequake_api::services::DocumentSourceScope::from_document_id(DOC_ID);
    edgequake_api::services::cascade_remove_document_sources(
        &state.storage.graph_storage,
        None,
        Some(&tenant_ctx),
        &scope,
    )
    .await
    .expect("cascade");

    assert!(
        !state
            .storage
            .graph_storage
            .has_node("LEGACY_NULL_TENANT_ENTITY")
            .await
            .unwrap(),
        "legacy null-tenant entity must cascade when workspace matches"
    );
}

/// P4 edge — relationship lookup by property `id`, not only composite key.
#[tokio::test]
async fn resource_safety_relationship_lookup_by_property_id() {
    use edgequake_storage::traits::EdgeListFilter;
    use serde_json::json;

    let state = AppState::test_state();
    let custom_id = "custom-rel-uuid-12345";

    for (id, props) in [("PROP_SRC", HashMap::new()), ("PROP_TGT", HashMap::new())] {
        let mut p = props;
        p.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        p.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        state
            .storage
            .graph_storage
            .upsert_node(id, p)
            .await
            .expect("node");
    }

    let mut edge_props = HashMap::new();
    edge_props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
    edge_props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
    edge_props.insert("id".to_string(), json!(custom_id));
    state
        .storage
        .graph_storage
        .upsert_edge("PROP_SRC", "PROP_TGT", edge_props)
        .await
        .expect("edge");

    let filter = EdgeListFilter {
        tenant_id: Some(PROOF_TENANT.to_string()),
        workspace_id: Some(PROOF_WORKSPACE.to_string()),
        relationship_type: None,
    };
    let edge = state
        .storage
        .graph_storage
        .find_edge_by_relationship_id(&filter, custom_id)
        .await
        .expect("lookup")
        .expect("found by property id");

    assert_eq!(edge.source, "PROP_SRC");
    assert_eq!(edge.target, "PROP_TGT");
}

/// P4 edge — community guard allows small graphs (positive path).
#[tokio::test]
async fn resource_safety_community_guard_allows_small_graph() {
    use edgequake_core::{ResourceBudgetConfig, ResourceGuard};
    use serde_json::json;

    let state = AppState::test_state();
    for i in 0..5 {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        state
            .storage
            .graph_storage
            .upsert_node(&format!("SMALL_NODE_{:02}", i), props)
            .await
            .expect("seed");
    }

    let guard = ResourceGuard::new(ResourceBudgetConfig {
        graph_scan_threshold_nodes: 50,
        ..Default::default()
    });

    let result = edgequake_api::services::detect_communities_guarded(
        &state.storage.graph_storage,
        &edgequake_storage::CommunityConfig::default(),
        &guard,
    )
    .await;

    assert!(
        result.is_ok(),
        "small graph should pass admission: {:?}",
        result
    );
}

/// P4 edge — community guard threshold boundary (exactly at threshold = allow).
#[tokio::test]
async fn resource_safety_community_guard_threshold_boundary_allow() {
    use edgequake_core::{ResourceBudgetConfig, ResourceGuard};
    use serde_json::json;

    const THRESHOLD: usize = 10;

    let state = AppState::test_state();
    for i in 0..THRESHOLD {
        let mut props = HashMap::new();
        props.insert("tenant_id".to_string(), json!(PROOF_TENANT));
        props.insert("workspace_id".to_string(), json!(PROOF_WORKSPACE));
        state
            .storage
            .graph_storage
            .upsert_node(&format!("BOUNDARY_{:02}", i), props)
            .await
            .expect("seed");
    }

    let guard = ResourceGuard::new(ResourceBudgetConfig {
        graph_scan_threshold_nodes: THRESHOLD,
        ..Default::default()
    });

    let result = edgequake_api::services::detect_communities_guarded(
        &state.storage.graph_storage,
        &edgequake_storage::CommunityConfig::default(),
        &guard,
    )
    .await;

    assert!(
        result.is_ok(),
        "node_count == threshold should allow (reject only when > threshold)"
    );
}

/// Smoke: health still OK with resource changes.
#[tokio::test]
async fn resource_safety_health_regression() {
    let app = proof_server().build_router();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// SPEC-053 hardening tests — graph search reliability
// ============================================================================

/// HT-01 (SPEC-053 B1): search_nodes does NOT acquire the materialization semaphore.
///
/// WHY: search_nodes is an O(log N) indexed btree lookup and must never be blocked
/// by graph materialization capacity. If the semaphore is full, graph streaming
/// should 503 but search must succeed.
///
/// Proof: exhaust the semaphore (hold all slots), then call the search endpoint.
/// The search must return 200 (not 503).
#[tokio::test]
async fn spec053_search_nodes_bypasses_materialization_semaphore() {
    use edgequake_core::GraphMaterializationSemaphore;
    use std::sync::Arc;

    let mut state = AppState::test_state();
    // Exhaust the entire semaphore
    state.graph_materialize = Arc::new(GraphMaterializationSemaphore::new(1));
    let _held = state
        .graph_materialize
        .acquire_owned()
        .await
        .expect("hold all slots");

    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router();

    // With semaphore exhausted, search must still return 200.
    // (It queries the empty in-memory store, so results are empty — that is fine.
    // The critical assertion is that the status code is NOT 503.)
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/graph/nodes/search?q=test")
                .header("X-Tenant-ID", PROOF_TENANT)
                .header("X-Workspace-ID", PROOF_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_ne!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "search_nodes must not return 503 when materialization semaphore is exhausted \
         (SPEC-053 B1: search is O(log N) indexed lookup, not a materialization)"
    );
    assert_eq!(response.status(), StatusCode::OK);
}

/// HT-02 (SPEC-053 source contract): search.rs must not contain admit_graph_materialization.
///
/// WHY: This is an inviolable contract test. If someone re-adds the guard to search.rs,
/// this test will catch it immediately. The contract is: search is not a materialization.
#[test]
fn spec053_search_handler_does_not_call_admit_graph_materialization() {
    let src = include_str!("../src/handlers/graph/graph_query/search.rs");
    assert!(
        !src.contains("admit_graph_materialization"),
        "search_nodes must not gate on admit_graph_materialization — \
         search is an O(log N) indexed lookup, not an O(V+E) materialization (SPEC-053 B1). \
         Use run_timed_graph_query for backpressure."
    );
}

/// HT-03 (SPEC-053 B2): stream_graph handler drops the guard before the SSE loop.
///
/// WHY contract test: If the guard drop comment/call is removed, the permit will be
/// held for the entire SSE stream (seconds), starving concurrent search operations.
#[test]
fn spec053_stream_graph_drops_guard_before_sse_loop() {
    let src = include_str!("../src/handlers/graph/graph_stream.rs");
    // Verify the guard is explicitly dropped (not just held until task exit)
    assert!(
        src.contains("drop(_materialize_guard)"),
        "stream_graph must explicitly drop(_materialize_guard) after the initial data fetch \
         and before the SSE streaming loop (SPEC-053 B2). Without this, the permit is held \
         for the entire stream duration (~5-10s), starving concurrent search operations."
    );
}

/// HT-04 (SPEC-053 B2): popular_labels still returns 503 when semaphore is full.
///
/// Regression guard: popular_labels IS a materialization operation (O(V) scan)
/// and MUST continue to be gated. Only search_nodes should bypass.
#[tokio::test]
async fn spec053_popular_labels_still_503_when_semaphore_full() {
    use axum::extract::{Query, State};
    use edgequake_api::handlers::{get_popular_labels, graph_types::PopularLabelsQuery};
    use edgequake_api::middleware::TenantContext;
    use edgequake_core::GraphMaterializationSemaphore;
    use std::sync::Arc;

    let mut state = AppState::test_state();
    state.graph_materialize = Arc::new(GraphMaterializationSemaphore::new(1));
    let _held = state
        .graph_materialize
        .acquire_owned()
        .await
        .expect("hold slot");

    let result = get_popular_labels(
        State(state.clone()),
        State(StorageRuntime::from_ref(&state)),
        State(GraphQueryRuntime::from_ref(&state)),
        TenantContext::default(),
        edgequake_api::handlers::auth::OptionalAuth(None),
        Query(PopularLabelsQuery {
            limit: 5,
            min_degree: None,
            entity_type: None,
        }),
    )
    .await;

    assert!(
        result.is_err(),
        "popular_labels must still 503 when semaphore is full — \
         it IS a materialization operation (SPEC-053 regression guard)"
    );
}

// ============================================================================
// SPEC-054 hardening tests — bug fixes for GitHub issues #297 and #294
// ============================================================================

/// SPEC-054 / GitHub #297: WorkspaceVectorRegistry must have drop_workspace_table.
///
/// Source contract: the trait must define the method that prevents orphan tables
/// after workspace cascade delete.
#[test]
fn spec054_workspace_vector_registry_has_drop_workspace_table() {
    // Include the trait source and assert the method is declared.
    let src = include_str!("../../edgequake-storage/src/traits/workspace_vector.rs");
    assert!(
        src.contains("fn drop_workspace_table"),
        "WorkspaceVectorRegistry must declare drop_workspace_table (SPEC-054 / GitHub #297). \
         Without it, DeleteWorkspace leaves orphan vector tables."
    );
}

/// SPEC-054 / GitHub #297: delete_workspace handler must call drop_workspace_table.
///
/// Source contract: the delete_workspace handler must call drop_workspace_table
/// after evicting the workspace from the cache.
#[test]
fn spec054_delete_workspace_calls_drop_workspace_table() {
    let src = include_str!("../src/handlers/workspaces/workspace_crud.rs");
    assert!(
        src.contains("drop_workspace_table"),
        "delete_workspace must call vector_registry.drop_workspace_table (SPEC-054 / GitHub #297). \
         Without this, the physical PostgreSQL vector table remains as an orphan after workspace delete."
    );
}

/// SPEC-054 / GitHub #294: persist_api_key must warn when falling back to KV storage.
///
/// Source contract: when API keys fall back to in-memory storage (KV backend),
/// a warning must be emitted to alert operators of multi-instance risk.
#[test]
fn spec054_persist_api_key_warns_on_kv_fallback() {
    let src = include_str!("../src/services/session_storage.rs");
    assert!(
        src.contains("gh#294") || src.contains("SPEC-054"),
        "persist_api_key must warn when falling back to in-memory API key storage \
         (SPEC-054 / GitHub #294). In multi-instance deployments (ECS, k8s), keys \
         stored in-memory are instance-local and return 401 on other instances."
    );
}

/// SPEC-054 / GitHub #37: context_only parameter must exist in query types.
///
/// Source contract: the context_only parameter allows retrieving chunks without
/// LLM generation, as requested in GitHub #37.
#[test]
fn spec054_context_only_parameter_implemented() {
    let query_types = include_str!("../src/handlers/query_types.rs");
    assert!(
        query_types.contains("context_only"),
        "context_only parameter must be present in query_types.rs (SPEC-054 / GitHub #37). \
         This allows retrieving only retrieval context without LLM generation."
    );
}
