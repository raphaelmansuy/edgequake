//! Additional tests targeting error paths for every service method.
//!
//! WHY: The integration_tests.rs covers happy paths comprehensively.
//! These tests ensure every resource method properly propagates HTTP errors.

#[cfg(test)]
mod error_path_tests {
    use edgequake_sdk::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn client_no_retry(mock_server: &MockServer) -> EdgeQuakeClient {
        EdgeQuakeClient::builder()
            .base_url(mock_server.uri())
            .api_key("test-key")
            .max_retries(0)
            .build()
            .expect("failed to build client")
    }

    // ── Documents Error Paths ──────────────────────────────────────

    #[tokio::test]
    async fn test_documents_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let err = c.documents().list().await.unwrap_err();
        assert_eq!(err.status_code(), Some(500));
    }

    #[tokio::test]
    async fn test_documents_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({"message":"not found"})))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let err = c.documents().get("missing").await.unwrap_err();
        assert_eq!(err.status_code(), Some(404));
    }

    #[tokio::test]
    async fn test_documents_upload_text_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({"message":"bad"})))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::documents::UploadDocumentRequest::text("");
        let err = c.documents().upload(&req).await.unwrap_err();
        assert_eq!(err.status_code(), Some(400));
    }

    #[tokio::test]
    async fn test_documents_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/documents/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let err = c.documents().delete("missing").await.unwrap_err();
        assert_eq!(err.status_code(), Some(404));
    }

    #[tokio::test]
    async fn test_documents_track_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/track/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.documents().track("missing").await.is_err());
    }

    // ── Graph Error Paths ──────────────────────────────────────────

    #[tokio::test]
    async fn test_graph_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.graph().get().await.is_err());
    }

    #[tokio::test]
    async fn test_graph_search_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/graph/nodes/search.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.graph().search("q").await.is_err());
    }

    // ── Entities Error Paths ───────────────────────────────────────

    #[tokio::test]
    async fn test_entities_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.entities().list().await.is_err());
    }

    #[tokio::test]
    async fn test_entities_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/graph/entities/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.entities().get("MISSING").await.is_err());
    }

    #[tokio::test]
    async fn test_entities_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(422))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::graph::CreateEntityRequest {
            entity_name: "".into(),
            entity_type: "".into(),
            description: "".into(),
            source_id: "".into(),
            metadata: None,
        };
        assert!(c.entities().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_entities_merge_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities/merge"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.entities().merge("a", "b").await.is_err());
    }

    #[tokio::test]
    async fn test_entities_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path_regex("/api/v1/graph/entities/.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.entities().delete("X").await.is_err());
    }

    // ── Relationships Error Paths ──────────────────────────────────

    #[tokio::test]
    async fn test_relationships_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/relationships"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.relationships().list().await.is_err());
    }

    #[tokio::test]
    async fn test_relationships_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/relationships"))
            .respond_with(ResponseTemplate::new(422))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::graph::CreateRelationshipRequest {
            source: "a".into(),
            target: "b".into(),
            relationship_type: "x".into(),
            weight: None,
            description: None,
        };
        assert!(c.relationships().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_relationships_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path_regex("/api/v1/graph/relationships/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.relationships().delete("missing").await.is_err());
    }

    // ── Query Error Paths ──────────────────────────────────────────

    #[tokio::test]
    async fn test_query_execute_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/query"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::query::QueryRequest {
            query: "test".into(),
            ..Default::default()
        };
        assert!(c.query().execute(&req).await.is_err());
    }

    // ── Chat Error Paths ───────────────────────────────────────────

    #[tokio::test]
    async fn test_chat_completions_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::chat::ChatCompletionRequest {
            message: String::new(),
            stream: Some(false),
            ..Default::default()
        };
        assert!(c.chat().completions(&req).await.is_err());
    }

    // ── Auth Error Paths ───────────────────────────────────────────

    #[tokio::test]
    async fn test_auth_login_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/login"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::auth::LoginRequest {
            username: "bad".into(),
            password: "bad".into(),
        };
        let err = c.auth().login(&req).await.unwrap_err();
        assert_eq!(err.status_code(), Some(401));
    }

    #[tokio::test]
    async fn test_auth_me_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/auth/me"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.auth().me().await.is_err());
    }

    #[tokio::test]
    async fn test_auth_refresh_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/refresh"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::auth::RefreshRequest {
            refresh_token: "expired".into(),
        };
        assert!(c.auth().refresh(&req).await.is_err());
    }

    // ── Users Error Paths ──────────────────────────────────────────

    #[tokio::test]
    async fn test_users_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/users"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.users().list().await.is_err());
    }

    #[tokio::test]
    async fn test_users_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/users"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::auth::CreateUserRequest {
            username: "dup".into(),
            email: "dup@x.com".into(),
            password: "p".into(),
            role: None,
        };
        assert!(c.users().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_users_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/users/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.users().get("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_users_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/users/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.users().delete("missing").await.is_err());
    }

    // ── API Keys Error Paths ───────────────────────────────────────

    #[tokio::test]
    async fn test_api_keys_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/api-keys"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.api_keys().list().await.is_err());
    }

    #[tokio::test]
    async fn test_api_keys_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/api-keys"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.api_keys().create("dup").await.is_err());
    }

    #[tokio::test]
    async fn test_api_keys_revoke_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/api-keys/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.api_keys().revoke("missing").await.is_err());
    }

    // ── Tenants Error Paths ────────────────────────────────────────

    #[tokio::test]
    async fn test_tenants_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tenants().list().await.is_err());
    }

    #[tokio::test]
    async fn test_tenants_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tenants"))
            .respond_with(ResponseTemplate::new(409))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::auth::CreateTenantRequest {
            name: "dup".into(),
            ..Default::default()
        };
        assert!(c.tenants().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_tenants_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tenants().get("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_tenants_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/tenants/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tenants().delete("missing").await.is_err());
    }

    // ── Conversations Error Paths ──────────────────────────────────

    #[tokio::test]
    async fn test_conversations_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().list().await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::conversations::CreateConversationRequest {
            title: None,
            folder_id: None,
        };
        assert!(c.conversations().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().get("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/conversations/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().delete("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_create_message_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/c1/messages"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::conversations::CreateMessageRequest {
            role: "user".into(),
            content: "hi".into(),
            parent_id: None,
            stream: None,
        };
        assert!(c.conversations().create_message("c1", &req).await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_share_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/missing/share"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().share("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_bulk_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/bulk/delete"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().bulk_delete(&[]).await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_pin_error() {
        let ms = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/conversations/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().pin("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_conversations_unpin_error() {
        let ms = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/conversations/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.conversations().unpin("missing").await.is_err());
    }

    // ── Folders Error Paths ────────────────────────────────────────

    #[tokio::test]
    async fn test_folders_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/folders"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.folders().list().await.is_err());
    }

    #[tokio::test]
    async fn test_folders_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/folders"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::conversations::CreateFolderRequest {
            name: "".into(),
            parent_id: None,
        };
        assert!(c.folders().create(&req).await.is_err());
    }

    #[tokio::test]
    async fn test_folders_delete_error() {
        let ms = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/folders/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.folders().delete("missing").await.is_err());
    }

    // ── Tasks Error Paths ──────────────────────────────────────────

    #[tokio::test]
    async fn test_tasks_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tasks"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tasks().list().await.is_err());
    }

    #[tokio::test]
    async fn test_tasks_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tasks/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tasks().get("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_tasks_cancel_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tasks/missing/cancel"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.tasks().cancel("missing").await.is_err());
    }

    // ── Pipeline Error Paths ───────────────────────────────────────

    #[tokio::test]
    async fn test_pipeline_status_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/status"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.pipeline().status().await.is_err());
    }

    #[tokio::test]
    async fn test_pipeline_metrics_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/queue-metrics"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.pipeline().metrics().await.is_err());
    }

    // ── Costs Error Paths ──────────────────────────────────────────

    #[tokio::test]
    async fn test_costs_summary_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/costs/summary"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.costs().summary().await.is_err());
    }

    #[tokio::test]
    async fn test_costs_history_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/costs/history"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.costs().history().await.is_err());
    }

    #[tokio::test]
    async fn test_costs_budget_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/costs/budget"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.costs().budget().await.is_err());
    }

    // ── Chunks Error Paths ─────────────────────────────────────────

    #[tokio::test]
    async fn test_chunks_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/chunks/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.chunks().get("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_chunks_get_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/chunks/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.chunks().get("missing").await.is_err());
    }

    // ── Provenance Error Paths ─────────────────────────────────────

    #[tokio::test]
    async fn test_provenance_for_entity_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/entities/.*/provenance"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.provenance().for_entity("MISSING").await.is_err());
    }

    #[tokio::test]
    async fn test_provenance_lineage_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/lineage/entities/.*"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.provenance().lineage("MISSING").await.is_err());
    }

    // ── Models Error Paths ─────────────────────────────────────────

    #[tokio::test]
    async fn test_models_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.models().list().await.is_err());
    }

    #[tokio::test]
    async fn test_models_current_provider_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/settings/provider/status"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.models().current_provider().await.is_err());
    }

    #[tokio::test]
    async fn test_models_providers_health_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models/health"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.models().providers_health().await.is_err());
    }

    // ── Workspaces Error Paths ─────────────────────────────────────

    #[tokio::test]
    async fn test_workspaces_list_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants/t1/workspaces"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.workspaces().list("t1").await.is_err());
    }

    #[tokio::test]
    async fn test_workspaces_create_error() {
        let ms = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tenants/t1/workspaces"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        let req = types::workspaces::CreateWorkspaceRequest {
            name: "".into(),
            ..Default::default()
        };
        assert!(c.workspaces().create("t1", &req).await.is_err());
    }

    #[tokio::test]
    async fn test_workspaces_stats_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/workspaces/missing/stats"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.workspaces().stats("missing").await.is_err());
    }

    // ── PDF Error Paths ────────────────────────────────────────────

    #[tokio::test]
    async fn test_pdf_progress_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/pdf/progress/missing"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.pdf().progress("missing").await.is_err());
    }

    #[tokio::test]
    async fn test_pdf_content_error() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/pdf/missing/content"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&ms)
            .await;
        let c = client_no_retry(&ms).await;
        assert!(c.pdf().content("missing").await.is_err());
    }

    // ── Client Builder Edge Cases ──────────────────────────────────

    #[tokio::test]
    async fn test_builder_bearer_token() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ok"})))
            .mount(&ms)
            .await;
        let c = EdgeQuakeClient::builder()
            .base_url(ms.uri())
            .bearer_token("jwt-token")
            .max_retries(0)
            .build()
            .unwrap();
        let h = c.health().check().await.unwrap();
        assert_eq!(h.status, "ok");
    }

    #[tokio::test]
    async fn test_builder_timeout_and_connect_timeout() {
        let c = EdgeQuakeClient::builder()
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        assert_eq!(c.base_url(), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_builder_custom_user_agent() {
        let c = EdgeQuakeClient::builder()
            .user_agent("custom/1.0")
            .build()
            .unwrap();
        assert_eq!(c.base_url(), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_builder_workspace_and_tenant() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ok"})))
            .mount(&ms)
            .await;
        let c = EdgeQuakeClient::builder()
            .base_url(ms.uri())
            .tenant_id("t-123")
            .workspace_id("ws-456")
            .max_retries(0)
            .build()
            .unwrap();
        let h = c.health().check().await.unwrap();
        assert_eq!(h.status, "ok");
    }

    // ── Error Type Tests ───────────────────────────────────────────

    #[test]
    fn test_error_is_retryable_rate_limited() {
        let err = Error::RateLimited {
            message: "slow down".into(),
            retry_after: Some(std::time::Duration::from_secs(30)),
        };
        assert!(err.is_retryable());
        assert_eq!(err.status_code(), Some(429));
    }

    #[test]
    fn test_error_is_retryable_server_500() {
        let err = Error::Server {
            status: 500,
            message: "oops".into(),
            code: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_is_retryable_server_502() {
        let err = Error::Server {
            status: 502,
            message: "bad gw".into(),
            code: None,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_error_not_retryable_400() {
        let err = Error::BadRequest {
            message: "bad".into(),
            code: None,
            details: None,
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), Some(400));
    }

    #[test]
    fn test_error_not_retryable_403() {
        let err = Error::Forbidden {
            message: "denied".into(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), Some(403));
    }

    #[test]
    fn test_error_not_retryable_404() {
        let err = Error::NotFound {
            message: "gone".into(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), Some(404));
    }

    #[test]
    fn test_error_not_retryable_409() {
        let err = Error::Conflict {
            message: "conflict".into(),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), Some(409));
    }

    #[test]
    fn test_error_not_retryable_422() {
        let err = Error::Validation {
            message: "invalid".into(),
            details: None,
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), Some(422));
    }

    #[test]
    fn test_error_config_no_status() {
        let err = Error::Config("bad config".into());
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), None);
    }

    #[test]
    fn test_error_timeout_no_status() {
        let err = Error::Timeout {
            operation: "test".into(),
            duration: std::time::Duration::from_secs(30),
        };
        assert!(!err.is_retryable());
        assert_eq!(err.status_code(), None);
    }

    #[test]
    fn test_error_display() {
        let err = Error::NotFound {
            message: "doc not found".into(),
        };
        assert!(format!("{err}").contains("not found"));
    }

    // ── Retry Behavior Tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_retry_on_500_then_success() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&ms)
            .await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status":"ok"})))
            .mount(&ms)
            .await;
        let c = EdgeQuakeClient::builder()
            .base_url(ms.uri())
            .max_retries(2)
            .build()
            .unwrap();
        let h = c.health().check().await.unwrap();
        assert_eq!(h.status, "ok");
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&ms)
            .await;
        let c = EdgeQuakeClient::builder()
            .base_url(ms.uri())
            .max_retries(1)
            .build()
            .unwrap();
        // With max_retries=1, it does attempt 0 + attempt 1 = 2 total attempts
        // Both return 500, so the last 500 response is returned (not retried further)
        let err = c.health().check().await.unwrap_err();
        assert_eq!(err.status_code(), Some(500));
    }

    #[tokio::test]
    async fn test_no_retry_on_404() {
        let ms = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/missing"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&ms)
            .await;
        let c = EdgeQuakeClient::builder()
            .base_url(ms.uri())
            .max_retries(3)
            .build()
            .unwrap();
        let err = c.documents().get("missing").await.unwrap_err();
        assert_eq!(err.status_code(), Some(404));
    }

    // ── Client Debug impl ──────────────────────────────────────────

    #[test]
    fn test_client_debug() {
        let c = EdgeQuakeClient::builder().build().unwrap();
        let dbg = format!("{:?}", c);
        assert!(dbg.contains("EdgeQuakeClient"));
        assert!(dbg.contains("localhost:8080"));
    }

    // ── URL construction ───────────────────────────────────────────

    #[test]
    fn test_url_with_leading_slash() {
        let c = EdgeQuakeClient::builder()
            .base_url("http://localhost:8080")
            .build()
            .unwrap();
        assert_eq!(c.base_url(), "http://localhost:8080");
    }
}
