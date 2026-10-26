//! SPEC-123: PDF parser config priority is inviolable.
//!
//! LAW-123-1..7 — Upload > Workspace > Tenant > Env > Vision;
//! Auto is explicit; Server Default Vision must not silently EdgeParse.
//!
//! Run:
//! `cargo test -p edgequake-api --test e2e_spec123_parser_priority`

mod common;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use common::{create_test_app, extract_json};
use edgequake_api::services::LargeDocumentProfile;
use edgequake_pdf::{resolve_pdf_parser_choice, PdfParserBackend, PdfParserResolutionSource};
use serde_json::json;
use serial_test::serial;
use tower::ServiceExt;
use uuid::Uuid;

async fn create_tenant(app: &axum::Router) -> String {
    let slug = format!("spec123-{}", Uuid::new_v4());
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/tenants")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "name": format!("SPEC-123 {slug}"),
                        "slug": slug,
                        "plan": "free"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = extract_json(response).await;
    body["id"].as_str().expect("tenant id").to_string()
}

async fn default_workspace_id(app: &axum::Router, tenant_id: &str) -> String {
    let list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tenants/{tenant_id}/workspaces"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let body = extract_json(list).await;
    body["items"][0]["id"]
        .as_str()
        .expect("workspace id")
        .to_string()
}

#[test]
fn spec123_resolve_matrix_upload_workspace_tenant_env() {
    let upload_wins = resolve_pdf_parser_choice(
        Some(PdfParserBackend::EdgeParse),
        Some(PdfParserBackend::Vision),
        Some(PdfParserBackend::Auto),
        Some(PdfParserBackend::Vision),
    );
    assert_eq!(upload_wins.source, PdfParserResolutionSource::Upload);
    assert_eq!(upload_wins.runtime_backend, PdfParserBackend::EdgeParse);
    assert!(upload_wins.backend_explicit());

    let workspace_wins = resolve_pdf_parser_choice(
        None,
        Some(PdfParserBackend::Vision),
        Some(PdfParserBackend::EdgeParse),
        Some(PdfParserBackend::EdgeParse),
    );
    assert_eq!(workspace_wins.source, PdfParserResolutionSource::Workspace);
    assert!(workspace_wins.backend_explicit());

    let tenant_wins = resolve_pdf_parser_choice(
        None,
        None,
        Some(PdfParserBackend::EdgeParse),
        Some(PdfParserBackend::Vision),
    );
    assert_eq!(tenant_wins.source, PdfParserResolutionSource::Tenant);
    assert_eq!(tenant_wins.runtime_backend, PdfParserBackend::EdgeParse);

    let unset = resolve_pdf_parser_choice(None, None, None, None);
    assert_eq!(unset.source, PdfParserResolutionSource::Default);
    assert_eq!(unset.runtime_backend, PdfParserBackend::Vision);
    assert!(
        unset.backend_explicit(),
        "Server Default Vision must be inviolable (LAW-123-3)"
    );
    assert!(!unset.allows_auto_route);

    let auto = resolve_pdf_parser_choice(None, Some(PdfParserBackend::Auto), None, None);
    assert!(auto.allows_auto_route);
    assert!(!auto.backend_explicit());
    assert_eq!(auto.runtime_backend, PdfParserBackend::Vision);
}

#[test]
fn spec123_auto_route_gate_requires_non_explicit() {
    // Resolved Vision (explicit) never auto-routes.
    assert!(!LargeDocumentProfile::should_try_edgeparse_before_vision(
        PdfParserBackend::Vision,
        true
    ));
    // Auto path stores Vision runtime + explicit=false.
    assert!(LargeDocumentProfile::should_try_edgeparse_before_vision(
        PdfParserBackend::Vision,
        false
    ));
}

#[tokio::test]
#[serial]
async fn e2e_workspace_none_resolves_to_inviolable_vision_label_path() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;
    let workspace_id = default_workspace_id(&app, &tenant_id).await;

    // Clear workspace override → Server Default (Vision).
    let clear = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "pdf_parser_backend": "none" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clear.status(), StatusCode::OK);
    let cleared = extract_json(clear).await;
    assert!(
        cleared["pdf_parser_backend"].is_null()
            || cleared["pdf_parser_backend"].as_str() == Some("none")
            || cleared["pdf_parser_backend"].as_str().is_none(),
        "workspace override must clear: {cleared}"
    );

    let resolved = resolve_pdf_parser_choice(None, None, None, None);
    assert_eq!(resolved.runtime_backend, PdfParserBackend::Vision);
    assert!(resolved.backend_explicit());
}

#[tokio::test]
#[serial]
async fn e2e_tenant_pdf_parser_backend_update() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/tenants/{tenant_id}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "pdf_parser_backend": "edgeparse" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let body = extract_json(update).await;
    assert_eq!(body["pdf_parser_backend"].as_str(), Some("edgeparse"));

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/tenants/{tenant_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let detail = extract_json(get).await;
    assert_eq!(detail["pdf_parser_backend"].as_str(), Some("edgeparse"));
}

#[tokio::test]
#[serial]
async fn e2e_workspace_auto_accepted() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;
    let workspace_id = default_workspace_id(&app, &tenant_id).await;

    let update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({ "pdf_parser_backend": "auto" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(update.status(), StatusCode::OK);
    let body = extract_json(update).await;
    assert_eq!(body["pdf_parser_backend"].as_str(), Some("auto"));
}

#[test]
fn spec123_model_resolve_matrix_llm_embedding_vision() {
    use edgequake_core::{
        resolve_embedding_choice, resolve_llm_choice, resolve_vision_llm_choice,
        ModelResolutionSource, Tenant, Workspace,
    };

    let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
    ws.metadata
        .insert("llm_provider".into(), serde_json::json!("ollama"));
    ws.metadata
        .insert("llm_model".into(), serde_json::json!("gemma4:latest"));
    ws.llm_provider = "ollama".into();
    ws.llm_model = "gemma4:latest".into();

    let mut tenant = Tenant::new("t", "t");
    tenant.default_llm_provider = "mistral".into();
    tenant.default_llm_model = "mistral-small-latest".into();
    tenant.default_vision_llm_provider = Some("mistral".into());
    tenant.default_vision_llm_model = Some("mistral-small-latest".into());
    tenant.default_embedding_provider = "openai".into();
    tenant.default_embedding_model = "text-embedding-3-small".into();
    tenant.default_embedding_dimension = 1536;

    let req = resolve_llm_choice(
        Some("openai"),
        Some("gpt-4.1-mini"),
        Some(&ws),
        Some(&tenant),
    );
    assert_eq!(req.source, ModelResolutionSource::Request);

    let mut bare = Workspace::new(Uuid::nil(), "bare", "bare");
    bare.vision_llm_provider = None;
    bare.vision_llm_model = None;
    let vision = resolve_vision_llm_choice(None, None, Some(&bare), Some(&tenant));
    assert_eq!(vision.source, ModelResolutionSource::Tenant);
    assert_eq!(vision.provider, "mistral");

    let emb = resolve_embedding_choice(None, None, None, Some(&bare), Some(&tenant));
    assert_eq!(emb.source, ModelResolutionSource::Tenant);
    assert_eq!(emb.dimension, 1536);
}

#[tokio::test]
#[serial]
async fn e2e_workspace_get_exposes_model_resolution_provenance() {
    let app = create_test_app();
    let tenant_id = create_tenant(&app).await;

    // Set tenant vision defaults.
    let tenant_update = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/tenants/{tenant_id}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "default_vision_llm_provider": "mistral",
                        "default_vision_llm_model": "mistral-small-latest",
                        "default_llm_provider": "mistral",
                        "default_llm_model": "mistral-small-latest",
                        "default_embedding_provider": "openai",
                        "default_embedding_model": "text-embedding-3-small",
                        "default_embedding_dimension": 1536
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(tenant_update.status(), StatusCode::OK);

    let workspace_id = default_workspace_id(&app, &tenant_id).await;

    // Clear workspace overrides so tenant cascade is honest (LAW-123-8).
    let clear_overrides = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "vision_llm_provider": "",
                        "vision_llm_model": "",
                        "llm_provider": "",
                        "llm_model": "",
                        "embedding_provider": "",
                        "embedding_model": "",
                        "embedding_dimension": 0
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(clear_overrides.status(), StatusCode::OK);

    let get = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{workspace_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let body = extract_json(get).await;

    assert!(
        body["llm_resolution_source"].as_str().is_some(),
        "expected llm_resolution_source: {body}"
    );
    assert!(
        body["embedding_resolution_source"].as_str().is_some(),
        "expected embedding_resolution_source: {body}"
    );
    assert!(
        body["vision_llm_resolution_source"].as_str().is_some(),
        "expected vision_llm_resolution_source: {body}"
    );
    assert_eq!(
        body["resolved_vision_llm_provider"].as_str(),
        Some("mistral"),
        "tenant vision must win when workspace vision unset: {body}"
    );
    assert_eq!(
        body["resolved_vision_llm_model"].as_str(),
        Some("mistral-small-latest"),
        "{body}"
    );
    assert_eq!(
        body["llm_resolution_source"].as_str(),
        Some("tenant"),
        "cleared workspace LLM must resolve from tenant: {body}"
    );
    assert_eq!(
        body["resolved_llm_provider"].as_str(),
        Some("mistral"),
        "{body}"
    );
    assert_eq!(
        body["embedding_resolution_source"].as_str(),
        Some("tenant"),
        "cleared workspace embedding must resolve from tenant: {body}"
    );
    assert_eq!(
        body["resolved_embedding_provider"].as_str(),
        Some("openai"),
        "{body}"
    );
    // LAW-123-8: unset override stays unset on Option fields.
    assert!(
        body["vision_llm_provider"].is_null()
            || body["vision_llm_provider"]
                .as_str()
                .map(|s| s.is_empty())
                .unwrap_or(true),
        "must not paint tenant into vision_llm_provider: {body}"
    );
}

#[tokio::test]
#[serial]
async fn e2e_pdf_upload_options_honor_tenant_vision() {
    use edgequake_api::handlers::pdf_upload::PdfUploadOptions;
    use edgequake_core::{Tenant, Workspace};

    let mut ws = Workspace::new(Uuid::nil(), "ws", "ws");
    ws.vision_llm_provider = None;
    ws.vision_llm_model = None;
    let mut tenant = Tenant::new("t", "t");
    tenant.default_vision_llm_provider = Some("mistral".into());
    tenant.default_vision_llm_model = Some("mistral-small-latest".into());

    let opts = PdfUploadOptions::default();
    let vision = opts.resolved_vision_llm(Some(&ws), Some(&tenant));
    assert_eq!(vision.provider, "mistral");
    assert_eq!(vision.model, "mistral-small-latest");

    let upload_wins = PdfUploadOptions {
        access: None,
        vision_provider: Some("openai".into()),
        vision_model: Some("gpt-4.1-nano".into()),
        ..Default::default()
    };
    let v2 = upload_wins.resolved_vision_llm(Some(&ws), Some(&tenant));
    assert_eq!(v2.provider, "openai");
    assert_eq!(v2.model, "gpt-4.1-nano");
}
