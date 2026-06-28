//! SPEC-027 end-to-end HTTP tests — security hardening with ascending compatibility.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use edgequake_api::{AppState, Server, ServerConfig};

fn server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn build_app(state: AppState) -> axum::Router {
    Server::new(server_config(), state).build_router()
}

async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&body) }))
}

fn auth_enabled_state() -> AppState {
    let mut state = AppState::test_state();
    state.auth.config.auth_enabled = true;
    state.auth.config.api_keys = vec!["master-test-key".to_string()];
    state
}

#[tokio::test]
async fn spec027_stored_api_key_authenticates_when_auth_enabled() {
    let state = auth_enabled_state();
    let app = build_app(state.clone());

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({ "name": "spec027-roundtrip" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create.status(), StatusCode::CREATED);
    let created = parse_json(create).await;
    let api_key = created["api_key"]
        .as_str()
        .expect("api_key in create response");
    assert!(api_key.starts_with("eq_"));

    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("x-api-key", api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert!(body.get("documents").is_some());
}

#[tokio::test]
async fn spec027_non_admin_receives_403_on_admin_endpoint() {
    let state = auth_enabled_state();
    let app = build_app(state);

    let _ = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": "spec027-admin-deny",
                        "email": "spec027-admin-deny@example.com",
                        "password": "SecurePass123!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": "spec027-admin-deny",
                        "password": "SecurePass123!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(login.status(), StatusCode::OK);
    let login_json = parse_json(login).await;
    let token = login_json["access_token"].as_str().unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/config/defaults")
                .header(header::AUTHORIZATION, format!("Bearer {}", token))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn spec027_ollama_compat_disabled_returns_503() {
    let mut state = AppState::test_state();
    state.security.enable_ollama_compat = false;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = parse_json(response).await;
    assert_eq!(body["code"], "SERVICE_UNAVAILABLE");
}

#[tokio::test]
async fn spec027_bulk_delete_requires_confirm_when_enforced() {
    let mut state = AppState::test_state();
    state.security.require_delete_all_confirm = true;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn spec027_bulk_delete_succeeds_with_confirm_header() {
    let mut state = AppState::test_state();
    state.security.require_delete_all_confirm = true;
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/documents")
                .header("x-edgequake-confirm", "delete-all-documents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn spec027_websocket_auth_rejects_missing_token_when_auth_enabled() {
    let mut state = auth_enabled_state();
    state.auth.config.auth_enabled = true;

    assert!(
        !edgequake_api::middleware::ws_validate_token(&state, None).await,
        "missing token must fail when auth enabled"
    );

    assert!(
        edgequake_api::middleware::ws_validate_token(&state, Some("master-test-key")).await,
        "valid static API key must pass ws auth gate"
    );
}

#[tokio::test]
async fn spec027_health_includes_api_capabilities() {
    let app = build_app(AppState::test_state());
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
    let body = parse_json(response).await;
    let caps = body["capabilities"].as_object().expect("capabilities");
    assert_eq!(caps["openapi_url"], "/api-docs/openapi.json");
    assert_eq!(caps["asyncapi_url"], "/api-docs/asyncapi.json");
    assert_eq!(caps["admin_api_prefix"], "/api/v1/admin");
}

#[tokio::test]
async fn spec027_health_reports_storage_component_probes() {
    let app = build_app(AppState::test_state());
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
    let body = parse_json(response).await;
    let components = body["components"].as_object().expect("components");
    assert!(components.contains_key("kv_storage"));
    assert!(components.contains_key("vector_storage"));
    assert!(components.contains_key("graph_storage"));
    assert!(components["kv_storage"].as_bool().unwrap_or(false));
}

#[tokio::test]
async fn spec027_openapi_json_endpoint_serves_valid_document() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["openapi"], "3.1.0");
    assert_eq!(body["info"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["paths"].as_object().map(|p| p.len()).unwrap_or(0) >= 100);
    assert!(body["servers"].as_array().map(|s| !s.is_empty()).unwrap_or(false));
    assert!(body["x-edgequake-asyncapi"]["channels"].is_object());
}

#[tokio::test]
async fn spec027_asyncapi_json_endpoint_serves_standalone_document() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api-docs/asyncapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["asyncapi"], "2.6.0");
    assert_eq!(body["info"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["channels"]["/ws/pipeline/progress"]["subscribe"].is_object());
    assert!(body["servers"]["local"]["url"].as_str().unwrap().starts_with("ws://"));
}

#[tokio::test]
async fn spec027_error_json_includes_problem_details_fields() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents/nonexistent-doc-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = parse_json(response).await;
    assert_eq!(body["code"], "NOT_FOUND");
    assert!(body["type"].as_str().unwrap().contains("not-found"));
    assert_eq!(body["title"], "Not Found");
    assert_eq!(body["status"], 404);
}

const SPEC027_TENANT: &str = "aaaaaaaa-0027-0027-0027-aaaaaaaaaaaa";
const SPEC027_USER: &str = "bbbbbbbb-0027-0027-0027-bbbbbbbbbbbb";
const SPEC027_WORKSPACE: &str = "cccccccc-0027-0027-0027-cccccccccccc";

#[tokio::test]
async fn spec027_v2_job_create_and_get_roundtrip() {
    let state = AppState::test_state();
    let app = build_app(state.clone());
    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/jobs")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .header("X-User-ID", SPEC027_USER)
                .body(Body::from(
                    json!({ "job_type": "insert", "payload": { "source": "spec027" } }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::ACCEPTED);
    let created = parse_json(create).await;
    let job_id = created["job_id"].as_str().expect("job_id");
    assert_eq!(
        created["links"]["v1_task"],
        format!("/api/v1/tasks/{job_id}")
    );

    let app = build_app(state);
    let get = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v2/jobs/{job_id}"))
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body = parse_json(get).await;
    assert_eq!(body["job_id"], job_id);
    assert_eq!(body["job_type"], "insert");
}

#[tokio::test]
async fn spec027_v2_job_list_scopes_by_workspace() {
    let state = AppState::test_state();
    let app = build_app(state.clone());

    for (idx, job_type) in ["insert", "scan"].into_iter().enumerate() {
        let create = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v2/jobs")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header("X-Tenant-ID", SPEC027_TENANT)
                    .header("X-Workspace-ID", SPEC027_WORKSPACE)
                    .header("X-User-ID", SPEC027_USER)
                    .body(Body::from(
                        json!({ "job_type": job_type, "payload": { "n": idx } }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create.status(), StatusCode::ACCEPTED);
    }

    let other_workspace = uuid::Uuid::new_v4().to_string();
    let create_other = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/jobs")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", other_workspace)
                .header("X-User-ID", SPEC027_USER)
                .body(Body::from(
                    json!({ "job_type": "insert", "payload": { "hidden": true } }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_other.status(), StatusCode::ACCEPTED);

    let app = build_app(state);
    let list = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v2/jobs")
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list.status(), StatusCode::OK);
    let body = parse_json(list).await;
    let jobs = body["jobs"].as_array().expect("jobs array");
    assert_eq!(jobs.len(), 2);
    assert_eq!(body["pagination"]["total"], 2);
}

#[tokio::test]
async fn spec027_v2_job_cancel_roundtrip() {
    let state = AppState::test_state();
    let app = build_app(state.clone());
    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/jobs")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .header("X-User-ID", SPEC027_USER)
                .body(Body::from(
                    json!({ "job_type": "insert", "payload": { "cancel_me": true } }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::ACCEPTED);
    let created = parse_json(create).await;
    let job_id = created["job_id"].as_str().expect("job_id");

    let app = build_app(state);
    let cancel = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v2/jobs/{job_id}/cancel"))
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cancel.status(), StatusCode::OK);
    let body = parse_json(cancel).await;
    assert_eq!(body["job_id"], job_id);
    assert_eq!(body["status"], "cancelled");
}

#[tokio::test]
async fn spec027_share_url_matches_v1_route() {
    let state = AppState::test_state();
    let app = build_app(state.clone());
    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/conversations")
                .header(header::CONTENT_TYPE, "application/json")
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .header("X-User-ID", SPEC027_USER)
                .body(Body::from(json!({ "title": "spec027 share" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let conv = parse_json(create).await;
    let conv_id = conv["id"].as_str().expect("conversation id");

    let app = build_app(state);
    let share = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/conversations/{conv_id}/share"))
                .header("X-Tenant-ID", SPEC027_TENANT)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .header("X-User-ID", SPEC027_USER)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(share.status(), StatusCode::OK);
    let body = parse_json(share).await;
    let share_url = body["share_url"].as_str().expect("share_url");
    assert!(
        share_url.starts_with("/api/v1/shared/"),
        "share_url must match route: {share_url}"
    );
}

#[tokio::test]
async fn spec027_cost_summary_scopes_metadata_by_tenant_workspace() {
    use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid};

    let state = AppState::test_state();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-visible-metadata".to_string(),
            json!({
                "tenant_id": "default",
                "workspace_id": "default",
                "status": "completed",
                "cost_usd": 2.5,
                "input_tokens": 100,
                "output_tokens": 50,
            }),
        )])
        .await
        .expect("seed visible metadata");

    let other_workspace = uuid::Uuid::new_v4().to_string();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-hidden-metadata".to_string(),
            json!({
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": other_workspace,
                "status": "completed",
                "cost_usd": 99.0,
                "input_tokens": 1000,
                "output_tokens": 500,
            }),
        )])
        .await
        .expect("seed hidden metadata");

    let app = build_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/costs/summary")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["document_count"], 1);
    let total_cost = body["total_cost"].as_f64().expect("total_cost");
    assert!(
        (total_cost - 2.5).abs() < f64::EPSILON,
        "expected scoped cost 2.5, got {total_cost}"
    );

    // UUID-stored metadata remains visible under default alias (legacy compat).
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-uuid-alias-metadata".to_string(),
            json!({
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": default_workspace_uuid().to_string(),
                "status": "indexed",
                "cost_usd": 1.0,
                "input_tokens": 10,
                "output_tokens": 5,
            }),
        )])
        .await
        .expect("seed uuid metadata");

    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/costs/summary")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = parse_json(response).await;
    assert_eq!(body["document_count"], 2);
    assert!((body["total_cost"].as_f64().unwrap() - 3.5).abs() < f64::EPSILON);
}

#[tokio::test]
async fn spec027_document_list_scopes_metadata_by_tenant_workspace() {
    use edgequake_api::middleware::{default_tenant_uuid, default_workspace_uuid};

    let state = AppState::test_state();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-list-visible-metadata".to_string(),
            json!({
                "id": "doc-spec027-list-visible",
                "title": "Visible Doc",
                "status": "completed",
                "tenant_id": "default",
                "workspace_id": "default",
            }),
        )])
        .await
        .expect("seed visible metadata");

    let other_workspace = uuid::Uuid::new_v4().to_string();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-list-hidden-metadata".to_string(),
            json!({
                "id": "doc-spec027-list-hidden",
                "title": "Hidden Doc",
                "status": "completed",
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": other_workspace,
            }),
        )])
        .await
        .expect("seed hidden metadata");

    let app = build_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    let documents = body["documents"].as_array().expect("documents array");
    let ids: Vec<&str> = documents.iter().filter_map(|d| d["id"].as_str()).collect();
    assert!(ids.contains(&"doc-spec027-list-visible"));
    assert!(!ids.contains(&"doc-spec027-list-hidden"));

    // UUID-stored metadata visible under default alias.
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-list-uuid-metadata".to_string(),
            json!({
                "id": "doc-spec027-list-uuid",
                "title": "UUID Doc",
                "status": "completed",
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": default_workspace_uuid().to_string(),
            }),
        )])
        .await
        .expect("seed uuid metadata");

    let app = build_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = parse_json(response).await;
    let documents = body["documents"].as_array().expect("documents array");
    let ids: Vec<&str> = documents.iter().filter_map(|d| d["id"].as_str()).collect();
    assert!(ids.contains(&"doc-spec027-list-uuid"));
}

#[tokio::test]
async fn spec027_cancel_task_scopes_document_status_update() {
    use edgequake_api::middleware::default_tenant_uuid;

    let state = AppState::test_state();
    let track_id = "spec027-cancel-shared-track";

    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-cancel-visible-metadata".to_string(),
            json!({
                "id": "doc-spec027-cancel-visible",
                "track_id": track_id,
                "status": "processing",
                "tenant_id": "default",
                "workspace_id": "default",
            }),
        )])
        .await
        .expect("seed visible doc");

    let other_workspace = uuid::Uuid::new_v4().to_string();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-cancel-hidden-metadata".to_string(),
            json!({
                "id": "doc-spec027-cancel-hidden",
                "track_id": track_id,
                "status": "processing",
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": other_workspace,
            }),
        )])
        .await
        .expect("seed hidden doc");

    let app = build_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v1/tasks/{track_id}/cancel"))
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let visible = state
        .storage
        .kv_storage
        .get_by_id("doc-spec027-cancel-visible-metadata")
        .await
        .expect("read visible")
        .expect("visible metadata");
    assert_eq!(visible["status"], "cancelled");

    let hidden = state
        .storage
        .kv_storage
        .get_by_id("doc-spec027-cancel-hidden-metadata")
        .await
        .expect("read hidden")
        .expect("hidden metadata");
    assert_eq!(
        hidden["status"], "processing",
        "cross-workspace document must not be cancelled"
    );
}

#[tokio::test]
async fn spec027_document_list_pagination_respects_page_params() {
    let state = AppState::test_state();

    for (idx, id) in [
        "doc-spec027-page-a",
        "doc-spec027-page-b",
        "doc-spec027-page-c",
    ]
    .into_iter()
    .enumerate()
    {
        state
            .storage
            .kv_storage
            .upsert(&[(
                format!("{id}-metadata"),
                json!({
                    "id": id,
                    "title": format!("Doc {idx}"),
                    "status": "completed",
                    "tenant_id": "default",
                    "workspace_id": "default",
                    "created_at": format!("2026-01-0{}T00:00:00Z", idx + 1),
                }),
            )])
            .await
            .expect("seed doc");
    }

    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents?page=2&page_size=1")
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["page"], 2);
    assert_eq!(body["page_size"], 1);
    assert_eq!(body["total"], 3);
    assert_eq!(body["total_pages"], 3);
    assert!(body["has_more"].as_bool().unwrap());
    let documents = body["documents"].as_array().expect("documents");
    assert_eq!(documents.len(), 1);
    let status_counts = &body["status_counts"];
    assert_eq!(status_counts["completed"], 3);
}

#[tokio::test]
async fn spec027_track_status_scopes_by_tenant_workspace() {
    use edgequake_api::middleware::default_tenant_uuid;

    let state = AppState::test_state();
    let track_id = "spec027-track-shared";

    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-track-visible-metadata".to_string(),
            json!({
                "id": "doc-spec027-track-visible",
                "track_id": track_id,
                "status": "completed",
                "tenant_id": "default",
                "workspace_id": "default",
                "chunk_count": 2,
            }),
        )])
        .await
        .expect("seed visible");

    let other_workspace = uuid::Uuid::new_v4().to_string();
    state
        .storage
        .kv_storage
        .upsert(&[(
            "doc-spec027-track-hidden-metadata".to_string(),
            json!({
                "id": "doc-spec027-track-hidden",
                "track_id": track_id,
                "status": "completed",
                "tenant_id": default_tenant_uuid().to_string(),
                "workspace_id": other_workspace,
                "chunk_count": 5,
            }),
        )])
        .await
        .expect("seed hidden");

    let app = build_app(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/documents/track/{track_id}"))
                .header("X-Tenant-ID", "default")
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = parse_json(response).await;
    assert_eq!(body["total_count"], 1);
    let documents = body["documents"].as_array().expect("documents");
    let ids: Vec<&str> = documents.iter().filter_map(|d| d["id"].as_str()).collect();
    assert!(ids.contains(&"doc-spec027-track-visible"));
    assert!(!ids.contains(&"doc-spec027-track-hidden"));
}

#[tokio::test]
async fn spec027_admin_accessible_without_auth_when_auth_disabled() {
    let state = AppState::test_state();
    assert!(
        !state.auth.config.auth_enabled,
        "default test state must have auth disabled (SEC-003 baseline)"
    );
    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/admin/config/defaults")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "auth off grants synthetic Admin — documents opt-in hardening requirement"
    );
}

#[tokio::test]
async fn spec027_rate_limit_returns_429_when_enabled() {
    use edgequake_rate_limiter::{RateLimitConfig as TokenBucketConfig, RateLimiter};

    let mut state = AppState::test_state();
    state.security.rate_limit_enabled = true;
    state.rate_limiter = RateLimiter::new(TokenBucketConfig::strict(1, 60));

    let app = build_app(state);
    let tenant = "spec027-rate-limit-tenant";

    for _ in 0..1 {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/v1/documents")
                    .header("X-Tenant-ID", tenant)
                    .header("X-Workspace-ID", "default")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("X-Tenant-ID", tenant)
                .header("X-Workspace-ID", "default")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = parse_json(response).await;
    assert_eq!(body["error_code"], "RATE_LIMITED");
}

#[tokio::test]
async fn spec027_error_response_content_type_is_problem_json() {
    let app = build_app(AppState::test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents/nonexistent-doc-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        content_type.starts_with("application/problem+json"),
        "expected problem+json, got {content_type}"
    );
}

#[tokio::test]
async fn spec027_workspace_delete_scopes_kv_documents() {
    use edgequake_core::{CreateWorkspaceRequest, Tenant};

    let state = AppState::test_state();
    let tenant = state
        .workspace_service
        .create_tenant(Tenant::new(
            "spec027 ws delete",
            format!("spec027-ws-{}", uuid::Uuid::new_v4()),
        ))
        .await
        .expect("create tenant");

    let ws_a = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "workspace-a".to_string(),
                slug: Some(format!("ws-a-{}", uuid::Uuid::new_v4())),
                ..Default::default()
            },
        )
        .await
        .expect("create workspace a");

    let ws_b = state
        .workspace_service
        .create_workspace(
            tenant.tenant_id,
            CreateWorkspaceRequest {
                name: "workspace-b".to_string(),
                slug: Some(format!("ws-b-{}", uuid::Uuid::new_v4())),
                ..Default::default()
            },
        )
        .await
        .expect("create workspace b");

    state
        .storage
        .kv_storage
        .upsert(&[
            (
                "doc-spec027-wsdel-a-metadata".to_string(),
                json!({
                    "id": "doc-spec027-wsdel-a",
                    "workspace_id": ws_a.workspace_id.to_string(),
                    "tenant_id": tenant.tenant_id.to_string(),
                }),
            ),
            (
                "doc-spec027-wsdel-a-content".to_string(),
                json!({ "content": "workspace a" }),
            ),
            (
                "doc-spec027-wsdel-b-metadata".to_string(),
                json!({
                    "id": "doc-spec027-wsdel-b",
                    "workspace_id": ws_b.workspace_id.to_string(),
                    "tenant_id": tenant.tenant_id.to_string(),
                }),
            ),
            (
                "doc-spec027-wsdel-b-content".to_string(),
                json!({ "content": "workspace b" }),
            ),
        ])
        .await
        .expect("seed documents");

    let app = build_app(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/workspaces/{}", ws_a.workspace_id))
                .header("X-Tenant-ID", tenant.tenant_id.to_string())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    assert!(
        state
            .storage
            .kv_storage
            .get_by_id("doc-spec027-wsdel-a-metadata")
            .await
            .expect("read a metadata")
            .is_none(),
        "workspace A document metadata must be deleted"
    );
    assert!(
        state
            .storage
            .kv_storage
            .get_by_id("doc-spec027-wsdel-b-metadata")
            .await
            .expect("read b metadata")
            .is_some(),
        "workspace B document metadata must remain"
    );
}

#[tokio::test]
async fn spec027_strict_tenant_bind_rejects_header_jwt_mismatch() {
    use edgequake_auth::{Claims, Role};

    let mut state = auth_enabled_state();
    state.security.strict_tenant_bind = true;

    let user_id = uuid::Uuid::new_v4();
    let claims = Claims::new(user_id, Role::User, 3600).with_tenant_id(SPEC027_TENANT);
    let token = state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .expect("sign jwt");

    let app = build_app(state);
    let mismatched_tenant = uuid::Uuid::new_v4().to_string();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header("X-Tenant-ID", mismatched_tenant)
                .header("X-Workspace-ID", SPEC027_WORKSPACE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "strict tenant bind must reject header/JWT mismatch (SEC-004 opt-in)"
    );
}

#[tokio::test]
async fn spec027_websocket_auth_allows_missing_token_when_auth_disabled() {
    let state = AppState::test_state();
    assert!(
        !state.auth.config.auth_enabled,
        "default test state must have auth disabled (SEC-007 baseline)"
    );

    assert!(
        edgequake_api::middleware::ws_validate_token(&state, None).await,
        "websocket must be open when auth disabled — opt-in hardening required"
    );
}
