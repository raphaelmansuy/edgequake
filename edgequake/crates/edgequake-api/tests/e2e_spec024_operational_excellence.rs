//! SPEC-024 Phase 4 — operational excellence HTTP contracts.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_observability::{log_format_label, ObservabilityConfig};
use edgequake_query::{
    fusion::{mix_fusion_mode_from_env, mix_fusion_mode_label},
    hybrid_merge::{hybrid_fusion_mode_from_env, hybrid_fusion_mode_label},
};
use tower::ServiceExt;

fn test_server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

async fn extract_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json")
}

#[tokio::test]
async fn spec024_health_exposes_operational_snapshot() {
    let state = AppState::test_state();
    let app = Server::new(test_server_config(), state).build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;

    let operational = body
        .get("operational")
        .expect("health must include operational snapshot (Phase 4.3)");

    assert!(operational.get("task_queue").is_some());
    assert!(operational.get("query_engine").is_some());

    let observability = operational
        .get("observability")
        .expect("health must include observability snapshot (Phase 4.5)");
    let obs_cfg = ObservabilityConfig::from_env();
    assert_eq!(
        observability["log_format"].as_str(),
        Some(log_format_label(obs_cfg.log_format)),
        "health log_format must match EDGEQUAKE_LOG_FORMAT runtime config"
    );
    assert_eq!(
        observability["otel_enabled"].as_bool(),
        Some(obs_cfg.otel_enabled)
    );

    let read_model = operational
        .get("read_model")
        .expect("health must include read_model snapshot (Phase 4.6)");
    assert_eq!(
        read_model["merge_strategy"].as_str(),
        Some("max(postgresql, kv)")
    );
    assert_eq!(
        read_model["entity_count_graph_reconcile"].as_bool(),
        Some(true)
    );

    assert_eq!(
        operational["query_engine"]["default_mode"].as_str(),
        Some("mix"),
        "production default mode must be mix (Phase 2.1)"
    );
    assert_eq!(
        operational["query_engine"]["community_refresh_debounce_secs"].as_u64(),
        Some(300)
    );
    assert_eq!(
        operational["query_engine"]["hybrid_fusion"].as_str(),
        Some(hybrid_fusion_mode_label(hybrid_fusion_mode_from_env())),
        "health hybrid_fusion must match EDGEQUAKE_HYBRID_FUSION"
    );
    assert_eq!(
        operational["query_engine"]["mix_fusion"].as_str(),
        Some(mix_fusion_mode_label(mix_fusion_mode_from_env())),
        "health mix_fusion must match EDGEQUAKE_MIX_FUSION"
    );
    assert_eq!(
        operational["query_engine"]["community_refresh_scheduled_workspaces"].as_u64(),
        Some(0),
        "fresh test state should have no pending community refresh timers"
    );

    let ingestion = operational
        .get("ingestion")
        .expect("health must include ingestion snapshot (pass 12)");
    assert_eq!(ingestion["execution_model"].as_str(), Some("worker_queue"));
    assert_eq!(
        ingestion["persist_ssot"].as_str(),
        Some("IngestionPersister")
    );
    assert_eq!(
        ingestion["duplicate_reingest_enabled"].as_bool(),
        Some(true)
    );

    let storage = operational
        .get("storage")
        .expect("health must include storage snapshot (pass 12)");
    assert_eq!(storage["chunk_text_ssot"].as_str(), Some("kv"));
    assert_eq!(storage["vector_metadata_ref"].as_str(), Some("content_ref"));
    assert_eq!(storage["chunk_kv_in_persister"].as_bool(), Some(true));

    let task_queue = operational.get("task_queue").expect("task_queue");
    assert_eq!(task_queue["pressure"].as_str(), Some("normal"));
    assert!(task_queue.get("pending_warn_threshold").is_some());
    assert!(task_queue.get("pending_critical_threshold").is_some());
}

#[tokio::test]
async fn spec024_queue_metrics_endpoint_available() {
    let state = AppState::test_state();
    let app = Server::new(test_server_config(), state).build_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/pipeline/queue-metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert!(body.get("pending_count").is_some());
    assert!(body.get("processing_count").is_some());
    assert!(body.get("max_workers").is_some());
    assert_eq!(body["pressure"].as_str(), Some("normal"));
    assert!(body.get("pending_warn_threshold").is_some());
}

#[tokio::test]
async fn spec024_health_degrades_on_critical_queue_backlog() {
    use edgequake_tasks::{Task, TaskType};
    use uuid::Uuid;

    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        EnvGuard { key, prev }
    }

    let _warn = set_env("EDGEQUAKE_QUEUE_PENDING_WARN", "2");
    let _crit = set_env("EDGEQUAKE_QUEUE_PENDING_CRITICAL", "5");

    let state = AppState::test_state();
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    for _ in 0..5 {
        state
            .enqueue_task(Task::new(
                tenant_id,
                workspace_id,
                TaskType::Insert,
                serde_json::json!({"content": "backlog probe"}),
            ))
            .await
            .expect("enqueue backlog tasks");
    }

    let app = Server::new(test_server_config(), state).build_router();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = extract_json(response).await;
    assert_eq!(
        body["status"].as_str(),
        Some("degraded"),
        "critical backlog must degrade /health"
    );
    assert_eq!(
        body["operational"]["task_queue"]["pressure"].as_str(),
        Some("critical")
    );
    assert!(body["operational"]["task_queue"]["operator_action"]
        .as_str()
        .is_some());
}

#[test]
fn spec024_community_index_service_is_ssot_for_debounce() {
    let svc = include_str!("../../edgequake-storage/src/community_index_service.rs");
    assert!(svc.contains("CommunityRefreshScheduler"));
    assert!(svc.contains("schedule_community_index_refresh"));
}
