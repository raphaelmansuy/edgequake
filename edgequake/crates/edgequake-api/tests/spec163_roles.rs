//! SPEC-163: each workspace role's `connection_id` talks to the saved host.

#![cfg(feature = "postgres")]

use std::sync::Arc;

use edgequake_api::providers::connection_factory::{llm_from_connection, ConnectionSpec};
use edgequake_api::providers::connection_store::{
    embedding_from_pool, llm_from_workspace_role, scope_saved_connection_pool,
};
use edgequake_api::providers::{LlmResolutionRequest, WorkspaceProviderResolver};
use edgequake_api::services::vlm_provider_resolver::{
    resolve_extract_provider_for_workspace, resolve_vlm_provider,
};
use edgequake_core::{LlmRole, UpdateWorkspaceRequest};
use edgequake_fake_llm::{spawn_ephemeral, FakeLlmState};
use edgequake_llm::traits::{EmbeddingProvider, LLMProvider};
use uuid::Uuid;

async fn insert_ollama_connection(pool: &sqlx::PgPool, base: &str) -> (Uuid, String) {
    let slug = format!("spec163-role-{}", Uuid::new_v4().simple());
    let id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO provider_connections (
                slug, display_name, api_shape, locality, base_url, auth_scheme,
                allow_private_network
           ) VALUES ($1, $2, 'ollama', 'local', $3, 'none', true)
           RETURNING id"#,
    )
    .bind(&slug)
    .bind(format!("Role proof {slug}"))
    .bind(base)
    .fetch_one(pool)
    .await
    .expect("insert connection");
    (id, slug)
}

#[tokio::test]
async fn roles_complete_against_saved_host() {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .expect("postgres");
    let (addr, _h) = spawn_ephemeral(FakeLlmState {
        embedding_dimension: 8,
        ..Default::default()
    })
    .await
    .unwrap();
    let base = format!("http://{addr}");
    let (conn_id, slug) = insert_ollama_connection(&pool, &base).await;
    let conn_id_str = conn_id.to_string();

    let result = async {
        let state = edgequake_api::AppState::test_state_with_pg_pool(pool.clone());
        state.workspace_service.seed_default_workspace().await;
        let ws_id = edgequake_api::middleware::default_workspace_uuid();
        let role_body = serde_json::json!({
            "provider": "ollama",
            "model": "fake-chat",
            "connection_id": conn_id_str,
        });
        state
            .workspace_service
            .update_workspace(
                ws_id,
                UpdateWorkspaceRequest {
                    llm_provider: Some("ollama".into()),
                    llm_model: Some("fake-chat".into()),
                    embedding_provider: Some("ollama".into()),
                    embedding_model: Some("fake-embed".into()),
                    embedding_dimension: Some(8),
                    vision_llm_provider: Some("ollama".into()),
                    vision_llm_model: Some("fake-chat".into()),
                    llm_roles: Some(serde_json::json!({
                        "extract": role_body.clone(),
                        "query": role_body.clone(),
                        "keyword": role_body.clone(),
                        "summary": role_body.clone(),
                        "vlm": role_body,
                        "embedding": { "connection_id": conn_id_str },
                    })),
                    ..Default::default()
                },
            )
            .await
            .expect("update workspace roles");

        let ws = state
            .workspace_service
            .get_workspace(ws_id)
            .await
            .expect("get")
            .expect("workspace");

        for role in [
            LlmRole::Extract,
            LlmRole::Query,
            LlmRole::Keyword,
            LlmRole::Summary,
            LlmRole::Vlm,
        ] {
            let provider = llm_from_workspace_role(&pool, &ws, role)
                .await
                .unwrap_or_else(|| panic!("{role:?} connection missing"));
            let chat = LLMProvider::complete(provider.as_ref(), "ping")
                .await
                .unwrap_or_else(|e| panic!("{role:?} complete: {e}"));
            assert!(
                chat.content.contains("echo:ping"),
                "{role:?}: {}",
                chat.content
            );
        }

        let embed = embedding_from_pool(&pool, &conn_id_str, "fake-embed", 8)
            .await
            .expect("embedding connection");
        let vectors = EmbeddingProvider::embed(embed.as_ref(), &["hello".into()])
            .await
            .expect("embed");
        assert_eq!(vectors[0].len(), 8);

        let resolver = WorkspaceProviderResolver::from_app_state(&state);
        let query = resolver
            .resolve_llm_provider_for_workspace(
                Some(&ws),
                &LlmResolutionRequest {
                    provider: None,
                    model: None,
                    extra_headers: None,
                },
            )
            .await
            .expect("resolve query")
            .expect("query provider");
        let chat = LLMProvider::complete(query.provider.as_ref(), "query-ping")
            .await
            .expect("query complete");
        assert!(chat.content.contains("echo:query-ping"), "{}", chat.content);

        let emb = resolver
            .resolve_embedding_provider(&ws_id.to_string())
            .await
            .expect("resolve embedding");
        let vectors = EmbeddingProvider::embed(emb.provider.as_ref(), &["hello".into()])
            .await
            .expect("resolver embed");
        assert_eq!(vectors[0].len(), 8);

        let fallback = Arc::clone(&state.query.llm_provider);
        let extract = scope_saved_connection_pool(
            Some(pool.clone()),
            resolve_extract_provider_for_workspace(
                Some(&state.workspace_service),
                ws_id,
                Arc::clone(&fallback),
            ),
        )
        .await;
        let chat = LLMProvider::complete(extract.as_ref(), "extract-ping")
            .await
            .expect("extract complete");
        assert!(
            chat.content.contains("echo:extract-ping"),
            "{}",
            chat.content
        );

        let vlm = resolve_vlm_provider(&state, Some(ws_id)).await;
        let chat = LLMProvider::complete(vlm.as_ref(), "vlm-ping")
            .await
            .expect("vlm complete");
        assert!(chat.content.contains("echo:vlm-ping"), "{}", chat.content);
        Ok::<(), String>(())
    }
    .await;

    let _ = sqlx::query("DELETE FROM provider_connections WHERE slug = $1")
        .bind(&slug)
        .execute(&pool)
        .await;
    result.expect("role e2e");
}

#[tokio::test]
async fn anthropic_saved_connection_uses_x_api_key_path() {
    let (addr, _h) = spawn_ephemeral(FakeLlmState {
        require_key: Some("secret".into()),
        ..Default::default()
    })
    .await
    .unwrap();
    let base = format!("http://{addr}");
    let provider = llm_from_connection(&ConnectionSpec {
        shape: "anthropic_messages".into(),
        base_url: base,
        api_key: Some("secret".into()),
        model: "fake-chat".into(),
        embedding_model: None,
        embedding_dimension: None,
    })
    .expect("build anthropic");
    let chat = LLMProvider::complete(provider.as_ref(), "anth-ping")
        .await
        .expect("anthropic complete");
    assert!(chat.content.contains("echo:anth-ping"), "{}", chat.content);
}
