//! E2E contract tests for SPEC-021 storage write-path closure (Phase A/C).
//!
//! These tests pin the contracts that the "Completed / 0 entities" screenshot
//! violated (file 16). They run in memory mode (CI-safe) and exercise:
//!
//! - P-E2: writer/reader chunk-ID contract — the chunk IDs written by the
//!   pipeline must be exactly the IDs the read path queries.
//! - P-E3: ingest → delete → all-stores-empty — no orphan vectors / KV /
//!   graph nodes remain after a full delete.
//! - P-E4: per-row entity_count regression — the relational read model must
//!   report a non-zero entity_count for a completed document with chunks +
//!   entities (the exact screenshot scenario).
//!
//! Memory-mode tests use `AppState::test_state()`. Postgres-mode tests are
//! gated behind `DATABASE_URL` and the `postgres` feature.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use edgequake_storage::kv_keys;
use serde_json::{json, Value};
use std::collections::HashMap;
use tower::ServiceExt;
use uuid::Uuid;

fn test_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

async fn setup_workspace(state: &AppState, suffix: &str) -> (Uuid, Uuid) {
    let tenant = Tenant::new(format!("Tenant-{}", suffix), format!("tenant-{}", suffix))
        .with_plan(TenantPlan::Pro);
    let tenant = state.workspace_service.create_tenant(tenant).await.unwrap();
    let tenant_id = tenant.tenant_id;
    let ws = state
        .workspace_service
        .create_workspace(
            tenant_id,
            CreateWorkspaceRequest {
                name: format!("WS-{}", suffix),
                slug: None,
                description: None,
                max_documents: None,
                llm_model: None,
                llm_provider: None,
                embedding_model: None,
                embedding_provider: None,
                embedding_dimension: None,
                vision_llm_model: None,
                pdf_parser_backend: None,
                entity_types: None,
                vision_llm_provider: None,
                ..Default::default()
            },
        )
        .await
        .unwrap();
    (ws.workspace_id, tenant_id)
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

async fn list_documents(app: &axum::Router, ws_id: Uuid, tenant_id: Uuid) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Workspace-ID", ws_id.to_string())
                .header("X-Tenant-ID", tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    (resp.status(), json_body(resp).await)
}

/// Seed a completed document with N chunks and M entities into the memory
/// stores, mirroring what a successful ingestion would write.
async fn seed_completed_document(
    state: &AppState,
    ws_id: Uuid,
    tenant_id: Uuid,
    doc_id: &str,
    n_chunks: usize,
    entity_names: &[&str],
) {
    let ws_str = ws_id.to_string();
    let tenant_str = tenant_id.to_string();
    let chunk_prefix = kv_keys::doc_chunk_prefix(doc_id);

    // KV chunk entries + metadata.
    let mut kv_entries: Vec<(String, Value)> = Vec::new();
    for i in 0..n_chunks {
        let chunk_id = format!("{}{}", chunk_prefix, i);
        kv_entries.push((
            chunk_id.clone(),
            json!({
                "id": chunk_id,
                "document_id": doc_id,
                "content": format!("chunk {} content", i),
                "workspace_id": ws_str,
                "tenant_id": tenant_str,
            }),
        ));
    }
    // Metadata with the stats a real ingestion would write.
    kv_entries.push((
        kv_keys::doc_metadata(doc_id),
        json!({
            "id": doc_id,
            "title": format!("doc-{}", doc_id),
            "status": "completed",
            "workspace_id": ws_str,
            "tenant_id": tenant_str,
            "chunk_count": n_chunks,
            "entity_count": entity_names.len(),
            "relationship_count": 0,
        }),
    ));
    state.storage.kv_storage.upsert(&kv_entries).await.unwrap();

    // Graph nodes for entities, each carrying the doc's chunk prefix in
    // source_ids so `node_count_by_source_prefix` can find them (P-A3).
    for name in entity_names {
        let mut props = HashMap::new();
        props.insert("entity_type".into(), json!("ORG"));
        props.insert("workspace_id".into(), json!(ws_str));
        props.insert(
            "source_ids".into(),
            json!((0..n_chunks)
                .map(|i| format!("{}{}", chunk_prefix, i))
                .collect::<Vec<_>>()),
        );
        state
            .storage
            .graph_storage
            .upsert_node(name, props)
            .await
            .unwrap();
    }
}

// ============================================================================
// P-E2: writer/reader chunk-ID contract
// ============================================================================

/// The chunk IDs the pipeline writes (`{doc_id}-chunk-{i}`) must be exactly
/// the IDs the KV read path enumerates via the `doc_chunk_prefix`. A mismatch
/// here is the root cause of "0 chunks" displays.
#[tokio::test]
async fn pe2_chunk_id_writer_reader_contract() {
    let state = AppState::test_state();
    let (ws_id, _tenant_id) = setup_workspace(&state, "pe2").await;
    let doc_id = Uuid::new_v4().to_string();

    // Writer side: write 3 chunks using the canonical kv_keys helpers.
    let chunk_prefix = kv_keys::doc_chunk_prefix(&doc_id);
    let written_ids: Vec<String> = (0..3).map(|i| format!("{}{}", chunk_prefix, i)).collect();
    let mut kv_entries: Vec<(String, Value)> = written_ids
        .iter()
        .map(|id| {
            (
                id.clone(),
                json!({"id": id, "document_id": doc_id, "content": "x"}),
            )
        })
        .collect();
    kv_entries.push((
        kv_keys::doc_metadata(&doc_id),
        json!({"id": doc_id, "status": "completed", "workspace_id": ws_id.to_string(), "chunk_count": 3}),
    ));
    state.storage.kv_storage.upsert(&kv_entries).await.unwrap();

    // Reader side: enumerate keys with the same prefix.
    let keys = state.storage.kv_storage.keys().await.unwrap();
    let read_ids: Vec<String> = keys
        .iter()
        .filter(|k| k.starts_with(&chunk_prefix))
        .cloned()
        .collect();

    assert_eq!(
        read_ids.len(),
        written_ids.len(),
        "writer and reader must agree on chunk count via the shared kv_keys prefix"
    );
    for wid in &written_ids {
        assert!(
            read_ids.contains(wid),
            "reader must enumerate written chunk id {wid}"
        );
    }
}

// ============================================================================
// P-E3: ingest → delete → all-stores-empty
// ============================================================================

/// After deleting a document, no KV chunks, no metadata, no graph nodes, and
/// no entity vectors may remain for that document. This is the contract the
/// saga compensation + deletion coordinator (P-C1/P-C2/P-C3) must uphold.
///
/// Note: this test relies on the `edgequake-core/pipeline` dev-dependency
/// feature to bring `EdgeQuake` into scope.
#[tokio::test]
async fn pe3_ingest_delete_all_stores_empty() {
    let state = AppState::test_state();
    let (ws_id, tenant_id) = setup_workspace(&state, "pe3").await;
    let doc_id = Uuid::new_v4().to_string();
    let entities = vec!["PE3_ORG_A", "PE3_ORG_B"];

    // Seed (simulate a completed ingestion).
    seed_completed_document(&state, ws_id, tenant_id, &doc_id, 2, &entities).await;

    // Also seed entity vectors so we can assert they're gone after delete.
    // Use 1536-dim vectors to match the test_state's MemoryVectorStorage.
    let dim = state.storage.vector_storage.dimension();
    let entity_vec_ids: Vec<String> = entities.iter().map(|n| format!("entity:{n}")).collect();
    let mut vec_entries: Vec<(String, Vec<f32>, Value)> = Vec::new();
    for id in &entity_vec_ids {
        vec_entries.push((
            id.clone(),
            vec![0.1; dim],
            json!({"type": "entity", "entity_name": id}),
        ));
    }
    // Seed chunk vectors too (P-C2 deletes these).
    let chunk_prefix = kv_keys::doc_chunk_prefix(&doc_id);
    for i in 0..2 {
        let cid = format!("{}{}", chunk_prefix, i);
        vec_entries.push((
            cid.clone(),
            vec![0.5; dim],
            json!({"type": "chunk", "document_id": doc_id}),
        ));
    }
    state
        .storage
        .vector_storage
        .upsert(&vec_entries)
        .await
        .unwrap();

    // Delete via the orchestrator's delete_document (the single entry point).
    // Use the MockProvider so initialize() succeeds without real LLM access.
    use edgequake_llm::MockProvider;
    let mock = std::sync::Arc::new(MockProvider::new());
    let mut eq = edgequake_core::EdgeQuake::with_defaults()
        .with_storage_backends(
            state.storage.kv_storage.clone(),
            state.storage.vector_storage.clone(),
            state.storage.graph_storage.clone(),
        )
        .with_providers(mock.clone(), mock);
    eq.initialize().await.unwrap();
    let result = eq.delete_document(&doc_id).await.unwrap();
    assert!(
        result.chunks_deleted >= 2,
        "deletion must remove the chunks"
    );

    // Contract: no KV chunk keys remain.
    let keys = state.storage.kv_storage.keys().await.unwrap();
    let remaining_chunks = keys.iter().filter(|k| k.starts_with(&chunk_prefix)).count();
    assert_eq!(
        remaining_chunks, 0,
        "no KV chunk keys may remain after delete"
    );

    // Contract: no metadata remains.
    let md_key = kv_keys::doc_metadata(&doc_id);
    assert!(
        !keys.contains(&md_key),
        "document metadata must be deleted from KV"
    );

    // Contract: no graph nodes remain for the entity names.
    for name in &entities {
        let node = state.storage.graph_storage.get_node(name).await.unwrap();
        assert!(node.is_none(), "graph node {name} must be deleted");
    }

    // Contract: no entity vectors remain.
    for vid in &entity_vec_ids {
        let v = state.storage.vector_storage.get_by_id(vid).await.unwrap();
        assert!(v.is_none(), "entity vector {vid} must be deleted");
    }
}

// ============================================================================
// P-E4: per-row entity_count regression (the screenshot)
// ============================================================================

/// The Documents list must report a non-zero `entity_count` for a completed
/// document that has chunks and entities — the exact scenario the screenshot
/// showed as "0 entities". This pins the P-A1/P-A3 read-path fix.
#[tokio::test]
async fn pe4_completed_document_reports_nonzero_entity_count() {
    let state = AppState::test_state();
    let (ws_id, tenant_id) = setup_workspace(&state, "pe4").await;
    let doc_id = Uuid::new_v4().to_string();

    // Seed a completed document with 3 chunks and 2 entities.
    seed_completed_document(
        &state,
        ws_id,
        tenant_id,
        &doc_id,
        3,
        &["PE4_ORG_A", "PE4_ORG_B"],
    )
    .await;

    let app = Server::new(test_config(), state.clone()).build_router();
    let (status, body) = list_documents(&app, ws_id, tenant_id).await;
    assert_eq!(status, StatusCode::OK);

    let docs = body["documents"].as_array().expect("documents array");
    let our_doc = docs
        .iter()
        .find(|d| d["id"] == doc_id)
        .expect("seeded document appears in list");

    // The screenshot bug: entity_count was 0. Pin the fix.
    let entity_count = our_doc["entity_count"].as_u64().unwrap_or(0);
    assert!(
        entity_count >= 2,
        "completed document with entities must report entity_count >= 2, got {entity_count} (the screenshot bug)"
    );

    // Also pin chunk_count so the relational backfill can't regress it.
    let chunk_count = our_doc["chunk_count"].as_u64().unwrap_or(0);
    assert_eq!(
        chunk_count, 3,
        "chunk_count must reflect the 3 seeded chunks"
    );
}

/// P-B2 regression: a document with NULL/missing status must NOT be counted as
/// `completed` in the status_counts summary — it must land in `unknown`.
#[tokio::test]
async fn pe4b_null_status_not_counted_as_completed() {
    let state = AppState::test_state();
    let (ws_id, tenant_id) = setup_workspace(&state, "pe4b").await;
    let doc_id = Uuid::new_v4().to_string();

    // Seed a document with NO status field (simulates a relational backfill
    // row with NULL status — the P-B2 scenario).
    state
        .storage
        .kv_storage
        .upsert(&[(
            kv_keys::doc_metadata(&doc_id),
            json!({
                "id": doc_id,
                "title": "null-status.md",
                "workspace_id": ws_id.to_string(),
                "tenant_id": tenant_id.to_string(),
                // status intentionally omitted
            }),
        )])
        .await
        .unwrap();

    let app = Server::new(test_config(), state.clone()).build_router();
    let (status, body) = list_documents(&app, ws_id, tenant_id).await;
    assert_eq!(status, StatusCode::OK);

    let counts = &body["status_counts"];
    let completed = counts["completed"].as_u64().unwrap_or(0);
    let unknown = counts["unknown"].as_u64().unwrap_or(0);
    assert_eq!(
        completed, 0,
        "NULL status must NOT count as completed (P-B2)"
    );
    assert!(
        unknown >= 1,
        "NULL status must count as unknown (P-B2), got unknown={unknown}"
    );
}
