//! SPEC-163: probe OpenAI / Anthropic / Ollama shapes against the fake LLM.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use edgequake_api::providers::connection_factory::{
    embedding_from_connection, llm_from_connection, ConnectionSpec,
};
use edgequake_api::providers::probe::{probe_provider, ProbeErrorKind, ProbeRequest};
use edgequake_fake_llm::{spawn_ephemeral, FakeLlmState};
use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use tower::ServiceExt;

#[tokio::test]
async fn openai_shape_ok_and_401() {
    let state = FakeLlmState {
        require_key: Some("secret".into()),
        embedding_dimension: 8,
        ..Default::default()
    };
    let (addr, _h) = spawn_ephemeral(state).await.unwrap();
    let base = format!("http://{addr}");

    let ok = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some(base.clone()),
        model: Some("fake-chat".into()),
        embedding_model: Some("fake-embed".into()),
        api_key: Some("secret".into()),
        auth_scheme: Some("bearer".into()),
        allow_private_network: Some(true),
        expected_dimension: Some(8),
    })
    .await;
    assert!(ok.ok, "{ok:?}");
    assert_eq!(ok.embedding_dimension, Some(8));

    let unauth = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some(base.clone()),
        model: None,
        embedding_model: None,
        api_key: Some("wrong".into()),
        auth_scheme: Some("bearer".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert_eq!(unauth.kind, ProbeErrorKind::Unauthorized);

    let anth = probe_provider(ProbeRequest {
        shape: "anthropic_messages".into(),
        base_url: Some(base.clone()),
        model: Some("fake-chat".into()),
        embedding_model: None,
        api_key: Some("secret".into()),
        auth_scheme: Some("x_api_key".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert!(anth.chat_ok, "{anth:?}");

    let ollama = probe_provider(ProbeRequest {
        shape: "ollama".into(),
        base_url: Some(base),
        model: Some("fake-chat".into()),
        embedding_model: None,
        api_key: None,
        auth_scheme: Some("none".into()),
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert!(ollama.list_ok, "{ollama:?}");
}

#[tokio::test]
async fn ssrf_blocks_metadata() {
    let r = probe_provider(ProbeRequest {
        shape: "openai_chat".into(),
        base_url: Some("http://169.254.169.254/".into()),
        model: None,
        embedding_model: None,
        api_key: None,
        auth_scheme: None,
        allow_private_network: Some(true),
        expected_dimension: None,
    })
    .await;
    assert_eq!(r.kind, ProbeErrorKind::SsrfDenied);
}

fn spec(shape: &str, base: &str) -> ConnectionSpec {
    ConnectionSpec {
        shape: shape.into(),
        base_url: base.into(),
        api_key: Some("secret".into()),
        model: "fake-chat".into(),
        embedding_model: Some("fake-embed".into()),
        embedding_dimension: Some(8),
    }
}

/// A saved connection uses its own host for chat and embeddings.
#[tokio::test]
async fn saved_connection_talks_to_its_own_host() {
    let (addr, _h) = spawn_ephemeral(FakeLlmState {
        embedding_dimension: 8,
        ..Default::default()
    })
    .await
    .unwrap();
    let root = format!("http://{addr}");

    let ollama = llm_from_connection(&spec("ollama", &root)).unwrap();
    let chat = LLMProvider::complete(ollama.as_ref(), "ping-ollama")
        .await
        .expect("ollama complete");
    assert!(
        chat.content.contains("echo:ping-ollama"),
        "{}",
        chat.content
    );

    let ollama_embed = embedding_from_connection(&spec("ollama", &root)).unwrap();
    let vectors = EmbeddingProvider::embed(ollama_embed.as_ref(), &["hello".into()])
        .await
        .expect("ollama embed");
    assert_eq!(vectors[0].len(), 8);

    let openai_base = format!("{root}/v1");
    let openai = llm_from_connection(&spec("openai_chat", &openai_base)).unwrap();
    let chat = LLMProvider::complete(openai.as_ref(), "ping-openai")
        .await
        .expect("openai complete");
    assert!(
        chat.content.contains("echo:ping-openai"),
        "{}",
        chat.content
    );
    let openai_embed = embedding_from_connection(&spec("openai_chat", &openai_base)).unwrap();
    let vectors = EmbeddingProvider::embed(openai_embed.as_ref(), &["hello".into()])
        .await
        .expect("openai embed");
    assert_eq!(vectors[0].len(), 8);
}

/// Router mounts the probe and the connection CRUD routes.
#[tokio::test]
async fn http_provider_test_and_connection_routes() {
    let (addr, _h) = spawn_ephemeral(FakeLlmState::default()).await.unwrap();
    let base = format!("http://{addr}");
    let app = edgequake_api::create_router(edgequake_api::AppState::test_state());

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/providers/test")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "shape": "openai_chat",
                        "base_url": base,
                        "model": "fake-chat",
                        "embedding_model": "fake-embed",
                        "allow_private_network": true,
                        "expected_dimension": 8
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let probe: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(probe["ok"], true, "{probe}");

    let app = edgequake_api::create_router(edgequake_api::AppState::test_state());
    let listed = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/connections")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listed.status(), StatusCode::OK);

    let app = edgequake_api::create_router(edgequake_api::AppState::test_state());
    let created = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/connections")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "slug": "local-fake",
                        "display_name": "Local fake",
                        "api_shape": "ollama",
                        "base_url": base,
                        "allow_private_network": true
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    // Memory mode has no connection table. The route is mounted and refuses
    // the write instead of 404.
    assert_eq!(created.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Persist a connection and probe it through the HTTP routes.
#[cfg(feature = "postgres")]
#[tokio::test]
async fn http_saved_connection_roundtrip() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("postgres");
    let (addr, _h) = spawn_ephemeral(FakeLlmState::default()).await.unwrap();
    let base = format!("http://{addr}");
    let slug = format!("spec163-{}", uuid::Uuid::new_v4().simple());

    let result = async {
        let app = edgequake_api::create_router(edgequake_api::AppState::test_state_with_pg_pool(
            pool.clone(),
        ));
        let created = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/connections")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "slug": slug,
                            "display_name": "SPEC-163 fake",
                            "api_shape": "ollama",
                            "base_url": base,
                            "allow_private_network": true
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        if created.status() != StatusCode::CREATED {
            return Err(format!("create status {}", created.status()));
        }
        let bytes = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap();
        let view: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if view["locality"] != "local" {
            return Err(format!("locality {:?}", view["locality"]));
        }
        if view["source"] != "db" {
            return Err(format!("source {:?}", view["source"]));
        }
        if view["key_configured"] != false {
            return Err("key_configured should be false".into());
        }
        if view.get("api_key").is_some() {
            return Err("api_key must not be echoed".into());
        }
        let id = view["id"]
            .as_str()
            .ok_or_else(|| "missing id".to_string())?
            .to_string();

        let app = edgequake_api::create_router(edgequake_api::AppState::test_state_with_pg_pool(
            pool.clone(),
        ));
        let listed = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if listed.status() != StatusCode::OK {
            return Err(format!("list status {}", listed.status()));
        }
        let bytes = axum::body::to_bytes(listed.into_body(), usize::MAX)
            .await
            .unwrap();
        let rows: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if !rows
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["slug"] == slug)
        {
            return Err(format!("slug {slug} missing from {rows}"));
        }

        let app = edgequake_api::create_router(edgequake_api::AppState::test_state_with_pg_pool(
            pool.clone(),
        ));
        let tested = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/connections/{id}/test"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if tested.status() != StatusCode::OK {
            return Err(format!("test status {}", tested.status()));
        }
        let bytes = axum::body::to_bytes(tested.into_body(), usize::MAX)
            .await
            .unwrap();
        let probe: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if probe["ok"] != true {
            return Err(format!("probe not ok: {probe}"));
        }

        let mut locked = edgequake_api::AppState::test_state_with_pg_pool(pool.clone());
        locked.auth.config.auth_enabled = true;
        locked.auth.config.dev_mode = false;
        let app = edgequake_api::create_router(locked);
        let denied = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/connections")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        if denied.status() != StatusCode::UNAUTHORIZED {
            return Err(format!("expected 401, got {}", denied.status()));
        }
        Ok(())
    }
    .await;

    let _ = sqlx::query("DELETE FROM provider_connections WHERE slug = $1")
        .bind(&slug)
        .execute(&pool)
        .await;
    result.expect("postgres connection roundtrip");
}
