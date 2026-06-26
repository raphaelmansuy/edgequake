//! E2E regression tests for SPEC-021 UX "0 documents" fix (P5-01).
//!
//! Validates hybrid read model: relational `documents` table primary,
//! KV metadata fallback, AGE graph for entities.
//!
//! Memory-mode tests run in CI without Postgres.
//! Postgres tests run when DATABASE_URL is set:
//!   cargo test -p edgequake-api --test e2e_zero_documents_spec021 --features postgres

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_core::{CreateWorkspaceRequest, Tenant, TenantPlan};
use edgequake_storage::kv_keys;
use serde_json::{json, Value};
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

async fn get_stats(app: &axum::Router, ws_id: Uuid) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{}/stats", ws_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    (resp.status(), json_body(resp).await)
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

// ============================================================================
// Memory mode — KV fallback path (CI-safe)
// ============================================================================

/// KV-only legacy uploads must still count when Postgres is unavailable.
#[tokio::test]
async fn test_kv_only_documents_still_counted_in_memory_mode() {
    let state = AppState::test_state();
    let (ws_id, _) = setup_workspace(&state, "kv-only").await;
    let doc_id = Uuid::new_v4().to_string();

    state
        .storage
        .kv_storage
        .upsert(&[(
            kv_keys::doc_metadata(&doc_id),
            json!({
                "id": doc_id,
                "title": "legacy-kv.md",
                "status": "completed",
                "workspace_id": ws_id.to_string(),
            }),
        )])
        .await
        .unwrap();

    let app = Server::new(test_config(), state).build_router();
    let (_, stats) = get_stats(&app, ws_id).await;

    assert_eq!(
        stats["document_count"], 1,
        "KV-only documents must count when Postgres unavailable"
    );
}

/// Entities still come from graph even when document count uses hybrid merge.
#[tokio::test]
async fn test_entity_count_still_from_graph_in_hybrid_mode() {
    let state = AppState::test_state();
    let (ws_id, _) = setup_workspace(&state, "hybrid-ent").await;
    let ws_str = ws_id.to_string();

    let mut props = std::collections::HashMap::new();
    props.insert("entity_type".into(), json!("ORG"));
    props.insert("workspace_id".into(), json!(ws_str));
    state
        .storage
        .graph_storage
        .upsert_node("HYBRID_ORG", props)
        .await
        .unwrap();

    let app = Server::new(test_config(), state).build_router();
    let (_, stats) = get_stats(&app, ws_id).await;

    assert_eq!(stats["entity_count"], 1);
    assert_eq!(stats["document_count"], 0);
}

// ============================================================================
// Postgres mode — relational primary path
// ============================================================================

#[cfg(feature = "postgres")]
mod postgres_tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;
    use std::env;

    fn database_url() -> Option<String> {
        env::var("DATABASE_URL").ok()
    }

    async fn postgres_pool() -> Option<sqlx::PgPool> {
        let url = database_url()?;
        PgPoolOptions::new()
            .max_connections(3)
            .connect(&url)
            .await
            .ok()
    }

    async fn postgres_app_state() -> Option<AppState> {
        let url = database_url()?;
        AppState::new_postgres(url, "").await.ok()
    }

    /// Core SPEC-021 regression: relational docs exist, KV metadata absent → count > 0.
    #[tokio::test]
    async fn test_relational_documents_counted_when_kv_missing() {
        let pool = match postgres_pool().await {
            Some(p) => p,
            None => {
                eprintln!("SKIP: DATABASE_URL not set");
                return;
            }
        };

        let state = match postgres_app_state().await {
            Some(s) => s,
            None => {
                eprintln!("SKIP: could not create postgres AppState");
                return;
            }
        };

        let (ws_id, tenant_id) = setup_workspace(&state, "pg-only").await;
        let doc_id = Uuid::new_v4();

        // Insert relational document WITHOUT KV metadata (simulates historical drift).
        sqlx::query(
            r#"
            INSERT INTO documents (id, tenant_id, workspace_id, title, content, status)
            VALUES ($1, $2, $3, $4, $5, 'indexed')
            "#,
        )
        .bind(doc_id)
        .bind(tenant_id)
        .bind(ws_id)
        .bind("relational-only.pdf")
        .bind("test content for spec 021")
        .execute(&pool)
        .await
        .expect("insert relational document");

        let app = Server::new(test_config(), state).build_router();

        let (_, stats) = get_stats(&app, ws_id).await;
        assert!(
            stats["document_count"].as_i64().unwrap_or(0) >= 1,
            "SPEC-021: relational document must appear in workspace stats when KV is empty"
        );

        let (_, list) = list_documents(&app, ws_id, tenant_id).await;
        assert!(
            list["total"].as_i64().unwrap_or(0) >= 1,
            "SPEC-021: relational document must appear in documents list"
        );

        // Cleanup
        let _ = sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(doc_id)
            .execute(&pool)
            .await;
    }

    /// Hybrid merge: max(pg, kv) when both stores have documents.
    #[tokio::test]
    async fn test_hybrid_merge_uses_max_of_pg_and_kv() {
        let pool = match postgres_pool().await {
            Some(p) => p,
            None => {
                eprintln!("SKIP: DATABASE_URL not set");
                return;
            }
        };

        let state = match postgres_app_state().await {
            Some(s) => s,
            None => return,
        };

        let (ws_id, tenant_id) = setup_workspace(&state, "hybrid-max").await;

        // 2 relational documents
        for i in 0..2 {
            let doc_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO documents (id, tenant_id, workspace_id, title, content, status) VALUES ($1,$2,$3,$4,$5,'indexed')",
            )
            .bind(doc_id)
            .bind(tenant_id)
            .bind(ws_id)
            .bind(format!("pg-{}", i))
            .bind("content")
            .execute(&pool)
            .await
            .unwrap();
        }

        // 3 KV-only documents (different ids, no relational rows)
        for i in 0..3 {
            let did = Uuid::new_v4().to_string();
            state
                .storage
                .kv_storage
                .upsert(&[(
                    kv_keys::doc_metadata(&did),
                    json!({
                        "id": did,
                        "title": format!("kv-{}", i),
                        "status": "completed",
                        "workspace_id": ws_id.to_string(),
                    }),
                )])
                .await
                .unwrap();
        }

        let app = Server::new(test_config(), state).build_router();
        let (_, stats) = get_stats(&app, ws_id).await;

        assert_eq!(
            stats["document_count"], 3,
            "Hybrid merge must use max(pg=2, kv=3) = 3"
        );

        let _ = sqlx::query("DELETE FROM documents WHERE workspace_id = $1")
            .bind(ws_id)
            .execute(&pool)
            .await;
    }
}
