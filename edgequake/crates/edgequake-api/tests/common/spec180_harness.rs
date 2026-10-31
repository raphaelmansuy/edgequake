//! SPEC-180 e2e harness: Postgres app, counting providers, workspace and upload helpers.

#![cfg(feature = "postgres")]
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

pub const USER_ID: &str = "00000000-0000-0000-0000-000000000001";

pub struct Harness {
    pub app: axum::Router,
    pub pool: PgPool,
    pub mock: Arc<super::spec180::CountingProvider>,
    pub workspaces: Arc<dyn edgequake_core::WorkspaceService>,
}

pub async fn harness() -> Option<Harness> {
    super::clear_provider_detection_env();
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_ALLOW_TEST_PROVIDER_OVERRIDE", "1");
    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    // The bundled catalog, not a stale ~/.edgequake/models.toml on the developer's machine.
    std::env::set_var(
        "EDGEQUAKE_MODELS_CONFIG",
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../models.toml"),
    );
    // Answer and keyword caches would hide the LLM calls these tests count.
    std::env::set_var("EDGEQUAKE_LLM_CACHE", "0");
    if std::env::var("EDGEQUAKE_AUTH_ENABLED").is_err() {
        std::env::set_var("EDGEQUAKE_AUTH_ENABLED", "false");
    }
    let url = super::spec013_postgres::try_database_url()?;
    let pool = tokio::time::timeout(Duration::from_secs(3), PgPool::connect(&url))
        .await
        .ok()?
        .ok()?;

    // A normal workspace must never call the chat model: the double counts every call.
    let mock = super::spec180::CountingProvider::shared();
    edgequake_api::safety_limits::set_test_provider_override(
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::LLMProvider>,
        Arc::clone(&mock) as Arc<dyn edgequake_llm::traits::EmbeddingProvider>,
    );
    let mut state = edgequake_api::AppState::new_postgres(&url, "")
        .await
        .expect("postgres app state");
    let workspaces = Arc::clone(&state.workspace_service);
    super::spec013_postgres::start_worker_pool(&mut state).await;
    let config = edgequake_api::ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    };
    let app = edgequake_api::Server::new(config, state).build_router();
    super::spec013_postgres::wait_until_app_ready(&app).await;
    Some(Harness {
        app,
        pool,
        mock,
        workspaces,
    })
}

/// A tenant plus a mock-provider workspace of the given profile.
///
/// The workspace is created through the service so its provider pair is `mock`; the API
/// route judges every explicit pair against the live model catalog, which has no mock.
/// API-level create rules are covered by `workspace_create_validates_profile_words`.
pub async fn create_workspace(h: &Harness, profile: &str) -> (String, String) {
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let (st, tenant) = super::post_json(
        &h.app,
        "/api/v1/tenants",
        &json!({"name": format!("spec180 {suffix}")}),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "tenant: {tenant}");
    let tenant_id = tenant["id"].as_str().unwrap().to_string();
    let request = edgequake_core::CreateWorkspaceRequest::new(format!("ws {suffix}"))
        .with_llm_config("mock-llm", "mock")
        .with_embedding_config("mock-embedding", "mock", super::spec180::DIM)
        .with_rag_profile(
            edgequake_core::rag_profile::RagProfile::parse(profile).expect("profile"),
        );
    let ws = h
        .workspaces
        .create_workspace(uuid::Uuid::parse_str(&tenant_id).unwrap(), request)
        .await
        .expect("create workspace");
    (tenant_id, ws.workspace_id.to_string())
}

/// Upload text and wait for a terminal state. A failure prints the stored document so a
/// broken ingest is diagnosable from the test log.
pub async fn upload_scoped(
    h: &Harness,
    tenant: &str,
    ws: &str,
    title: &str,
    text: &str,
) -> (String, String, String) {
    let (st, body) = super::post_json_with_tenant(
        &h.app,
        "/api/v1/documents",
        &json!({"content": text, "title": title}),
        tenant,
        USER_ID,
        ws,
    )
    .await;
    assert!(st.is_success(), "upload: {st} {body}");
    let doc_id = body["document_id"]
        .as_str()
        .expect("document_id")
        .to_string();
    let track_id = body["track_id"].as_str().expect("track_id").to_string();
    let (status, doc) = wait_terminal(h, tenant, ws, &doc_id).await;
    assert_eq!(status, "completed", "document ended {status}: {doc}");
    (doc_id, track_id, status)
}

/// Poll a document until it is terminal (or 90 s), returning its status and body.
pub async fn wait_terminal(
    h: &Harness,
    tenant: &str,
    ws: &str,
    doc_id: &str,
) -> (String, serde_json::Value) {
    let deadline = std::time::Instant::now() + Duration::from_secs(90);
    loop {
        let (_, doc) = super::get_with_tenant(
            &h.app,
            &format!("/api/v1/documents/{doc_id}"),
            tenant,
            USER_ID,
            ws,
        )
        .await;
        let status = doc["status"].as_str().unwrap_or("unknown").to_string();
        let terminal = matches!(
            status.as_str(),
            "completed" | "failed" | "partial_failure" | "cancelled"
        );
        if terminal || std::time::Instant::now() >= deadline {
            return (status, doc);
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

pub async fn count(pool: &PgPool, sql: &str, ws: &str) -> i64 {
    let ws = uuid::Uuid::parse_str(ws).unwrap();
    sqlx::query_scalar::<_, i64>(sql)
        .bind(ws)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("{sql}: {e}"))
}
