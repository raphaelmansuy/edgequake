#[cfg(test)]
mod tests {
    use edgequake_sdk::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ── Helper ──────────────────────────────────────────────────────

    async fn test_client(mock_server: &MockServer) -> EdgeQuakeClient {
        EdgeQuakeClient::builder()
            .base_url(mock_server.uri())
            .api_key("test-key")
            .tenant_id("t1")
            .workspace_id("w1")
            .max_retries(0)
            .build()
            .expect("failed to build client")
    }

    // ── Client Builder ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_builder_default() {
        let client = EdgeQuakeClient::builder().build().unwrap();
        assert_eq!(client.base_url(), "http://localhost:8080");
    }

    #[tokio::test]
    async fn test_builder_custom_url() {
        let client = EdgeQuakeClient::builder()
            .base_url("https://api.example.com")
            .build()
            .unwrap();
        assert_eq!(client.base_url(), "https://api.example.com");
    }

    #[tokio::test]
    async fn test_builder_invalid_url() {
        let result = EdgeQuakeClient::builder().base_url("not a url").build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_api_key() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"status":"healthy","version":"0.1.0"})),
            )
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let health: types::common::HealthResponse = client.health().check().await.unwrap();
        assert_eq!(health.status, "healthy");
    }

    #[tokio::test]
    async fn test_client_is_clone_and_send() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        fn assert_clone<T: Clone>() {}
        assert_send::<EdgeQuakeClient>();
        assert_sync::<EdgeQuakeClient>();
        assert_clone::<EdgeQuakeClient>();
    }

    // ── Health ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_check() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "healthy",
                "version": "0.1.0",
                "storage_mode": "postgresql",
                "components": {"kv": true, "graph": true}
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let res = client.health().check().await.unwrap();
        assert_eq!(res.status, "healthy");
        assert_eq!(res.version.as_deref(), Some("0.1.0"));
    }

    // ── Documents ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_documents_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "documents": [{"id":"doc-1","file_name":"a.pdf","status":"completed"}]
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let res = client.documents().list().await.unwrap();
        assert_eq!(res.documents.len(), 1);
        assert_eq!(res.documents[0].id, "doc-1");
    }

    #[tokio::test]
    async fn test_documents_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"doc-1","file_name":"a.pdf","status":"completed"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let doc = client.documents().get("doc-1").await.unwrap();
        assert_eq!(doc.id, "doc-1");
    }

    #[tokio::test]
    async fn test_documents_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/documents/doc-1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        client.documents().delete("doc-1").await.unwrap();
    }

    #[tokio::test]
    async fn test_documents_upload_text() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "document_id":"doc-2","status":"processing","track_id":"trk-1"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::documents::UploadDocumentRequest {
            content: "hello world".into(),
            title: Some("test".into()),
            metadata: None,
            async_processing: false,
            track_id: None,
            enable_gleaning: true,
            max_gleaning: 1,
            use_llm_summarization: true,
        };
        let res = client.documents().upload(&req).await.unwrap();
        assert_eq!(res.id, "doc-2");
    }

    #[tokio::test]
    async fn test_documents_track() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/track/trk-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "track_id":"trk-1","status":"completed","progress":1.0
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let res = client.documents().track("trk-1").await.unwrap();
        assert_eq!(res.status, "completed");
    }

    // ── Graph ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_graph_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes":[{"id":"n1","label":"Alice"}],
                "edges":[{"source":"n1","target":"n2"}],
                "total_nodes":1,"total_edges":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let g = client.graph().get().await.unwrap();
        assert_eq!(g.nodes.len(), 1);
        assert_eq!(g.edges.len(), 1);
    }

    #[tokio::test]
    async fn test_graph_search() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/graph/nodes/search.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes":[{"id":"n1","label":"Alice"}],"edges":[],"total_matches":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.graph().search("Alice").await.unwrap();
        assert_eq!(r.total_matches.unwrap(), 1);
    }

    // ── Entities ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_entities_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items":[{"id":"ALICE","entity_name":"ALICE","entity_type":"person"}],
                "total":1,"page":1,"page_size":20,"total_pages":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let resp = client.entities().list().await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].entity_name, "ALICE");
        assert_eq!(resp.total, 1);
    }

    #[tokio::test]
    async fn test_entities_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "status":"success","message":"Entity created successfully",
                "entity":{"id":"BOB","entity_name":"BOB","entity_type":"person","description":"A person","source_id":"manual"}
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::graph::CreateEntityRequest {
            entity_name: "BOB".into(),
            entity_type: "person".into(),
            description: "A person".into(),
            source_id: "manual".into(),
            metadata: None,
        };
        let resp = client.entities().create(&req).await.unwrap();
        assert_eq!(resp.status, "success");
        assert_eq!(resp.entity.as_ref().unwrap().entity_name, "BOB");
    }

    #[tokio::test]
    async fn test_entities_merge() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities/merge"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "merged_count":2,"message":"merged"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.entities().merge("Alice", "ALICE").await.unwrap();
        assert_eq!(r.merged_count, 2);
    }

    #[tokio::test]
    async fn test_entities_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path_regex("/api/v1/graph/entities/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status":"success","message":"Entity deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        client.entities().delete("Alice").await.unwrap();
    }

    // ── Relationships ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_relationships_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items":[{"source":"Alice","target":"Bob","relationship_type":"knows"}],
                "total":1,"page":1,"page_size":20,"total_pages":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let resp = client.relationships().list().await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.total, 1);
    }

    #[tokio::test]
    async fn test_relationships_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/relationships"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "source":"Alice","target":"Bob","relationship_type":"knows"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::graph::CreateRelationshipRequest {
            source: "Alice".into(),
            target: "Bob".into(),
            relationship_type: "knows".into(),
            weight: None,
            description: None,
        };
        let rel = client.relationships().create(&req).await.unwrap();
        assert_eq!(rel.source, "Alice");
    }

    // ── Query ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_query_execute() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "answer":"42","sources":[{"document_id":"d1","score":0.95}],"mode":"hybrid"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::query::QueryRequest {
            query: "meaning of life".into(),
            top_k: Some(5),
            ..Default::default()
        };
        let r = client.query().execute(&req).await.unwrap();
        assert_eq!(r.answer.as_deref(), Some("42"));
        assert_eq!(r.sources.len(), 1);
    }

    // ── Chat ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_chat_completions() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "conversation_id": "conv-1",
                "user_message_id": "msg-1",
                "assistant_message_id": "msg-2",
                "content": "Hello!",
                "mode": "hybrid",
                "sources": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::chat::ChatCompletionRequest {
            message: "Hi".into(),
            stream: Some(false),
            ..Default::default()
        };
        let r = client.chat().completions(&req).await.unwrap();
        assert_eq!(r.content.as_deref(), Some("Hello!"));
        assert_eq!(r.conversation_id.as_deref(), Some("conv-1"));
    }

    // ── Auth ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_auth_login() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token":"tok-123","refresh_token":"ref-456"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::auth::LoginRequest {
            username: "admin".into(),
            password: "secret".into(),
        };
        let token = client.auth().login(&req).await.unwrap();
        assert_eq!(token.access_token, "tok-123");
    }

    #[tokio::test]
    async fn test_auth_me() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/auth/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"u1","username":"admin","email":"a@b.com","role":"admin"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let me = client.auth().me().await.unwrap();
        assert_eq!(me.id, "u1");
    }

    // ── Users ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_users_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/users"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"u2","username":"bob","email":"bob@x.com"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::auth::CreateUserRequest {
            username: "bob".into(),
            email: "bob@x.com".into(),
            password: "p@ss".into(),
            role: None,
        };
        let user = client.users().create(&req).await.unwrap();
        assert_eq!(user.id, "u2");
    }

    // ── API Keys ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_api_keys_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/api-keys"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"ak-1","key":"secret-key","name":"my key"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let key = client.api_keys().create("my key").await.unwrap();
        assert_eq!(key.id, "ak-1");
        assert_eq!(key.key, "secret-key");
    }

    #[tokio::test]
    async fn test_api_keys_revoke() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/api-keys/ak-1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        client.api_keys().revoke("ak-1").await.unwrap();
    }

    // ── Tenants ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tenants_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items":[{"id":"t1","name":"Acme","slug":"acme","plan":"free"}]
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let resp = client.tenants().list().await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].name, "Acme");
    }

    #[tokio::test]
    async fn test_tenants_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tenants"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"t2","name":"NewCo"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::auth::CreateTenantRequest {
            name: "NewCo".into(),
            ..Default::default()
        };
        let t = client.tenants().create(&req).await.unwrap();
        assert_eq!(t.id, "t2");
    }

    // ── Conversations ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_conversations_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"c1","title":"Test"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::conversations::CreateConversationRequest {
            title: Some("Test".into()),
            folder_id: None,
        };
        let c = client.conversations().create(&req).await.unwrap();
        assert_eq!(c.id, "c1");
    }

    #[tokio::test]
    async fn test_conversations_create_message() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/c1/messages"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"m1","role":"user","content":"Hello"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::conversations::CreateMessageRequest {
            role: "user".into(),
            content: "Hello".into(),
            parent_id: None,
            stream: None,
        };
        let msg = client
            .conversations()
            .create_message("c1", &req)
            .await
            .unwrap();
        assert_eq!(msg.id, "m1");
    }

    #[tokio::test]
    async fn test_conversations_share() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/c1/share"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "share_id":"sh-1","share_url":"/api/v1/shared/sh-1"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let share = client.conversations().share("c1").await.unwrap();
        assert_eq!(share.share_id, "sh-1");
    }

    #[tokio::test]
    async fn test_conversations_bulk_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/bulk/delete"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "affected": 3
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let ids = vec!["c1".into(), "c2".into(), "c3".into()];
        let r = client.conversations().bulk_delete(&ids).await.unwrap();
        assert_eq!(r.affected, 3);
    }

    // ── Folders ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_folders_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/folders"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"f1","name":"Work"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::conversations::CreateFolderRequest {
            name: "Work".into(),
            parent_id: None,
        };
        let f = client.folders().create(&req).await.unwrap();
        assert_eq!(f.id, "f1");
    }

    // ── Tasks ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_tasks_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tasks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tasks":[{"track_id":"trk-1","status":"completed"}],"total":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.tasks().list().await.unwrap();
        assert_eq!(r.tasks.len(), 1);
        assert_eq!(r.tasks[0].track_id, "trk-1");
    }

    #[tokio::test]
    async fn test_tasks_cancel() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tasks/trk-1/cancel"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        client.tasks().cancel("trk-1").await.unwrap();
    }

    // ── Pipeline ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_pipeline_status() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "is_busy":true,"pending_tasks":5,"processing_tasks":2,"completed_tasks":100,"failed_tasks":3
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.pipeline().status().await.unwrap();
        assert!(r.is_busy);
        assert_eq!(r.processing_tasks, 2);
    }

    #[tokio::test]
    async fn test_pipeline_metrics() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/queue-metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "queue_depth":10,"processing":2,"completed_last_hour":50,"failed_last_hour":1
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.pipeline().metrics().await.unwrap();
        assert_eq!(r.queue_depth, 10);
        assert_eq!(r.completed_last_hour, 50);
    }

    // ── Costs ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_costs_summary() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/costs/summary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_cost_usd":12.5,"total_tokens":100000,"document_count":50
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.costs().summary().await.unwrap();
        assert!((r.total_cost_usd - 12.5).abs() < 0.01);
    }

    // ── Chunks (see also test_chunks_get in later section) ───────────

    // ── Provenance ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_provenance_for_entity() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/entities/.*/provenance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "entity_id": "e1",
                "entity_name": "ALICE",
                "entity_type": "person",
                "sources": [{
                    "document_id": "d1",
                    "chunks": [],
                }],
                "total_extraction_count": 1,
                "related_entities": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.provenance().for_entity("Alice").await.unwrap();
        assert_eq!(r.entity_name, "ALICE");
        assert_eq!(r.sources.len(), 1);
    }

    // ── Models ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_models_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "providers":[{"name":"openai","display_name":"OpenAI","models":[{"name":"gpt-4","is_available":true}]}]
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let catalog = client.models().list().await.unwrap();
        assert_eq!(catalog.providers.len(), 1);
        assert_eq!(catalog.providers[0].name, "openai");
        assert_eq!(catalog.providers[0].models.len(), 1);
    }

    // ── Workspaces ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_workspaces_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants/t1/workspaces"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id":"w1","name":"default","tenant_id":"t1"}],
                "total": 1,
                "offset": 0,
                "limit": 20
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let ws = client.workspaces().list("t1").await.unwrap();
        assert_eq!(ws.len(), 1);
    }

    #[tokio::test]
    async fn test_workspaces_create() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tenants/t1/workspaces"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "id":"w2","name":"new-ws"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::workspaces::CreateWorkspaceRequest {
            name: "new-ws".into(),
            ..Default::default()
        };
        let ws = client.workspaces().create("t1", &req).await.unwrap();
        assert_eq!(ws.name, "new-ws");
    }

    #[tokio::test]
    async fn test_workspaces_stats() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/workspaces/w1/stats"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "workspace_id":"w1","document_count":50,"entity_count":200,"relationship_count":150
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let s = client.workspaces().stats("w1").await.unwrap();
        assert_eq!(s.document_count, 50);
    }

    // ── PDF ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_pdf_progress() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/pdf/progress/doc-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "track_id":"trk-1","status":"processing","progress":0.5
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.pdf().progress("doc-1").await.unwrap();
        assert_eq!(r.status, "processing");
    }

    #[tokio::test]
    async fn test_pdf_content() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/pdf/doc-1/content"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"doc-1","markdown":"# Hello\nWorld"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.pdf().content("doc-1").await.unwrap();
        assert_eq!(r.markdown.as_deref(), Some("# Hello\nWorld"));
    }

    // ── Error Handling ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_error_not_found() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(json!({
                "error":"not found","message":"document not found"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let result = client.documents().get("missing").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), Some(404));
    }

    #[tokio::test]
    async fn test_error_unauthorized() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({
                "error":"unauthorized"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let result = client.health().check().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status_code(), Some(401));
    }

    #[tokio::test]
    async fn test_error_validation() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(422).set_body_json(json!({
                "error":"validation error","details":"entity_name is required"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::graph::CreateEntityRequest {
            entity_name: String::new(),
            entity_type: "person".into(),
            description: "test".into(),
            source_id: "manual".into(),
            metadata: None,
        };
        let result = client.entities().create(&req).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().status_code(), Some(422));
    }

    #[tokio::test]
    async fn test_error_server_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let result = client.documents().list().await;
        assert!(result.is_err());
    }

    // ── Type Serialization ───────────────────────────────────────────

    #[test]
    fn test_query_mode_serialize() {
        let mode = types::query::QueryMode::Hybrid;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"hybrid\"");
    }

    #[test]
    fn test_health_response_deserialize() {
        let json = r#"{"status":"healthy","version":"0.1.0","storage_mode":"pg"}"#;
        let h: types::common::HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(h.status, "healthy");
    }

    #[test]
    fn test_entity_roundtrip() {
        let e = types::graph::Entity {
            id: "ALICE".into(),
            entity_name: "ALICE".into(),
            entity_type: Some("person".into()),
            description: Some("A character".into()),
            source_id: None,
            properties: None,
            degree: Some(5),
            created_at: None,
            updated_at: None,
            metadata: None,
        };
        let json = serde_json::to_string(&e).unwrap();
        let e2: types::graph::Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(e2.entity_name, "ALICE");
        assert_eq!(e2.degree, Some(5));
    }

    // ── Lineage & Metadata Tests ─────────────────────────────────────
    // WHY: The improve-lineage mission requires source_id, metadata,
    // and provenance fields to be properly tested across all SDKs.

    #[test]
    fn test_entity_source_id_field() {
        let e = types::graph::Entity {
            id: "ent-1".into(),
            entity_name: "ALICE".into(),
            entity_type: Some("person".into()),
            description: None,
            source_id: Some("doc-123".into()),
            properties: None,
            degree: None,
            created_at: None,
            updated_at: None,
            metadata: None,
        };
        assert_eq!(e.source_id, Some("doc-123".to_string()));
    }

    #[test]
    fn test_entity_metadata_field() {
        let meta = json!({"key": "value", "confidence": 0.95});
        let e = types::graph::Entity {
            id: "ent-2".into(),
            entity_name: "BOB".into(),
            entity_type: None,
            description: None,
            source_id: None,
            properties: None,
            degree: None,
            created_at: None,
            updated_at: None,
            metadata: Some(meta.clone()),
        };
        assert_eq!(e.metadata.unwrap()["key"], "value");
    }

    #[test]
    fn test_entity_timestamps() {
        let e = types::graph::Entity {
            id: "ent-3".into(),
            entity_name: "EVE".into(),
            entity_type: None,
            description: None,
            source_id: None,
            properties: None,
            degree: None,
            created_at: Some("2025-01-01T00:00:00Z".into()),
            updated_at: Some("2025-01-02T00:00:00Z".into()),
            metadata: None,
        };
        assert!(e.created_at.is_some());
        assert!(e.updated_at.is_some());
    }

    #[test]
    fn test_create_entity_request_source_id() {
        let req = types::graph::CreateEntityRequest {
            entity_name: "ALICE".into(),
            entity_type: "person".into(),
            description: "A researcher".into(),
            source_id: "doc-456".into(),
            metadata: None,
        };
        assert_eq!(req.source_id, "doc-456");
    }

    #[test]
    fn test_create_entity_request_with_metadata() {
        let meta = json!({"origin": "test", "confidence": 0.9});
        let req = types::graph::CreateEntityRequest {
            entity_name: "META_ENTITY".into(),
            entity_type: "concept".into(),
            description: "With metadata".into(),
            source_id: "src-m".into(),
            metadata: Some(meta),
        };
        let json_str = serde_json::to_string(&req).unwrap();
        assert!(json_str.contains("src-m"));
        assert!(json_str.contains("origin"));
    }

    #[test]
    fn test_entity_source_id_json_roundtrip() {
        let e = types::graph::Entity {
            id: "ent-rt".into(),
            entity_name: "ROUNDTRIP".into(),
            entity_type: Some("test".into()),
            description: None,
            source_id: Some("doc-rt-1".into()),
            properties: None,
            degree: None,
            created_at: None,
            updated_at: None,
            metadata: Some(json!({"origin": "test"})),
        };
        let json_str = serde_json::to_string(&e).unwrap();
        let e2: types::graph::Entity = serde_json::from_str(&json_str).unwrap();
        assert_eq!(e2.source_id, Some("doc-rt-1".to_string()));
        assert_eq!(e2.metadata.unwrap()["origin"], "test");
    }

    #[test]
    fn test_provenance_record_fields() {
        let json = r#"{
            "entity_id": "ent-1",
            "entity_name": "ALICE",
            "document_id": "doc-1",
            "chunk_id": "chunk-7",
            "extraction_method": "llm",
            "confidence": 0.92
        }"#;
        let pr: types::operations::ProvenanceRecord = serde_json::from_str(json).unwrap();
        assert_eq!(pr.document_id, Some("doc-1".to_string()));
        assert_eq!(pr.confidence, Some(0.92));
        assert_eq!(pr.extraction_method, Some("llm".to_string()));
    }

    #[test]
    fn test_lineage_graph_structure() {
        let json = r#"{
            "nodes": [
                {"id": "n1", "name": "ALICE", "node_type": "person"},
                {"id": "n2", "name": "BOB", "node_type": "person"}
            ],
            "edges": [
                {"source": "n1", "target": "n2", "relationship": "KNOWS"}
            ]
        }"#;
        let lg: types::operations::LineageGraph = serde_json::from_str(json).unwrap();
        assert_eq!(lg.nodes.len(), 2);
        assert_eq!(lg.edges[0].relationship, Some("KNOWS".to_string()));
    }

    #[test]
    fn test_lineage_node_fields() {
        let json = r#"{"id": "n1", "name": "ALICE", "node_type": "person"}"#;
        let n: types::operations::LineageNode = serde_json::from_str(json).unwrap();
        assert_eq!(n.id, "n1");
        assert_eq!(n.node_type, Some("person".to_string()));
    }

    #[test]
    fn test_lineage_edge_fields() {
        let json = r#"{"source": "A", "target": "B", "relationship": "COLLAB"}"#;
        let e: types::operations::LineageEdge = serde_json::from_str(json).unwrap();
        assert_eq!(e.source, "A");
        assert_eq!(e.relationship, Some("COLLAB".to_string()));
    }

    #[test]
    fn test_document_full_lineage() {
        let json = r#"{
            "document_id": "doc-1",
            "metadata": {"title": "Test Doc"},
            "lineage": {"entities": 5, "relationships": 3}
        }"#;
        let dfl: types::operations::DocumentFullLineage = serde_json::from_str(json).unwrap();
        assert_eq!(dfl.document_id, "doc-1");
        assert!(dfl.metadata.is_some());
        assert!(dfl.lineage.is_some());
    }

    #[test]
    fn test_chunk_lineage_info() {
        let json = r#"{
            "chunk_id": "chunk-1",
            "document_id": "doc-1",
            "document_name": "test.pdf",
            "index": 3,
            "start_line": 10,
            "end_line": 20,
            "entity_count": 5,
            "relationship_count": 2,
            "entity_names": ["ALICE", "BOB"]
        }"#;
        let cli: types::operations::ChunkLineageInfo = serde_json::from_str(json).unwrap();
        assert_eq!(cli.chunk_id, "chunk-1");
        assert_eq!(cli.document_id, Some("doc-1".to_string()));
        assert_eq!(cli.entity_count, Some(5));
        assert_eq!(cli.entity_names.len(), 2);
    }

    #[test]
    fn test_entity_statistics() {
        let json = r#"{
            "total_relationships": 10,
            "outgoing_count": 6,
            "incoming_count": 4,
            "document_references": 3
        }"#;
        let stats: types::graph::EntityStatistics = serde_json::from_str(json).unwrap();
        assert_eq!(stats.total_relationships, 10);
        assert_eq!(stats.document_references, 3);
    }

    #[tokio::test]
    async fn test_entity_create_sends_source_id() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "success",
                "message": "created"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let req = types::graph::CreateEntityRequest {
            entity_name: "LINEAGE_TEST".into(),
            entity_type: "person".into(),
            description: "Testing lineage".into(),
            source_id: "doc-lineage-test".into(),
            metadata: Some(json!({"test": true})),
        };
        let result = client.entities().create(&req).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lineage_via_provenance_service() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/lineage/entities/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [{"id": "n1", "name": "ALICE"}],
                "edges": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let result = client.provenance().lineage("ALICE").await;
        assert!(result.is_ok());
        let graph = result.unwrap();
        assert_eq!(graph.nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_provenance_for_entity_full_response() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/entities/.+/provenance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "entity_id": "e1",
                "entity_name": "ALICE",
                "entity_type": "person",
                "sources": [],
                "total_extraction_count": 0,
                "related_entities": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let result = client.provenance().for_entity("ALICE").await;
        assert!(result.is_ok());
        let prov = result.unwrap();
        assert_eq!(prov.entity_name, "ALICE");
    }

    // ── Lineage resource tests (OODA-31) ──

    #[tokio::test]
    async fn test_lineage_entity_lineage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/lineage/entities/ALICE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [
                    {"id": "n1", "name": "ALICE", "type": "entity"},
                    {"id": "n2", "name": "BOB", "type": "entity"}
                ],
                "edges": [
                    {"source": "n1", "target": "n2", "relationship": "KNOWS"}
                ],
                "root_id": "n1"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let graph = client.lineage().entity_lineage("ALICE").await.unwrap();
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.root_id, Some("n1".to_string()));
    }

    #[tokio::test]
    async fn test_lineage_entity_lineage_url_encodes_name() {
        let mock_server = MockServer::start().await;
        // Names with spaces should be URL-encoded
        Mock::given(method("GET"))
            .and(path("/api/v1/lineage/entities/SARAH%20CHEN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [{"id": "n1", "name": "SARAH CHEN"}],
                "edges": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let graph = client.lineage().entity_lineage("SARAH CHEN").await.unwrap();
        assert_eq!(graph.nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_lineage_document_lineage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/lineage/documents/doc-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [{"id": "d1", "name": "report.pdf", "type": "document"}],
                "edges": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let graph = client.lineage().document_lineage("doc-123").await.unwrap();
        assert_eq!(graph.nodes.len(), 1);
        assert!(graph.edges.is_empty());
    }

    #[tokio::test]
    async fn test_lineage_document_full_lineage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-456/lineage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "document_id": "doc-456",
                "metadata": {"title": "Test Doc"},
                "lineage": {
                    "nodes": [{"id": "e1", "name": "ALICE"}],
                    "edges": []
                }
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let full = client
            .lineage()
            .document_full_lineage("doc-456")
            .await
            .unwrap();
        assert_eq!(full.document_id, "doc-456");
        assert!(full.metadata.is_some());
        assert!(full.lineage.is_some());
    }

    #[tokio::test]
    async fn test_lineage_export_json() {
        let mock_server = MockServer::start().await;
        let export_body = r#"{"nodes":[],"edges":[]}"#;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-789/lineage/export"))
            .and(query_param("format", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(export_body.as_bytes().to_vec(), "application/json"),
            )
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let bytes = client
            .lineage()
            .export_lineage("doc-789", "json")
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("nodes"));
    }

    #[tokio::test]
    async fn test_lineage_export_csv() {
        let mock_server = MockServer::start().await;
        let csv_body = "source,target,relationship\nALICE,BOB,KNOWS\n";
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-789/lineage/export"))
            .and(query_param("format", "csv"))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(csv_body.as_bytes().to_vec(), "text/csv"),
            )
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let bytes = client
            .lineage()
            .export_lineage("doc-789", "csv")
            .await
            .unwrap();
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("ALICE"));
        assert!(text.contains("KNOWS"));
    }

    #[tokio::test]
    async fn test_lineage_empty_graph() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/lineage/entities/UNKNOWN"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "nodes": [],
                "edges": []
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let graph = client.lineage().entity_lineage("UNKNOWN").await.unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
        assert!(graph.root_id.is_none());
    }

    // ── Settings resource tests (OODA-31) ──

    #[tokio::test]
    async fn test_settings_provider_status() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/settings/provider/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "current_provider": "ollama",
                "current_model": "gemma3:latest",
                "status": "healthy"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let status = client.settings().provider_status().await.unwrap();
        assert_eq!(status.current_provider, Some("ollama".to_string()));
        assert_eq!(status.current_model, Some("gemma3:latest".to_string()));
        assert_eq!(status.status, Some("healthy".to_string()));
    }

    #[tokio::test]
    async fn test_settings_list_providers() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/settings/providers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"name": "ollama", "status": "available"},
                {"name": "openai", "status": "available"}
            ])))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let providers = client.settings().list_providers().await.unwrap();
        assert_eq!(providers.len(), 2);
    }

    #[tokio::test]
    async fn test_settings_provider_status_no_provider() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/settings/provider/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "status": "no_provider"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let status = client.settings().provider_status().await.unwrap();
        assert!(status.current_provider.is_none());
        assert!(status.current_model.is_none());
        assert_eq!(status.status, Some("no_provider".to_string()));
    }

    // ── OODA-32: Health extended endpoints ──

    #[tokio::test]
    async fn test_health_ready() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ready"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert_eq!(client.health().ready().await.unwrap(), "OK");
    }

    #[tokio::test]
    async fn test_health_live() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/live"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert_eq!(client.health().live().await.unwrap(), "OK");
    }

    #[tokio::test]
    async fn test_health_metrics() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/metrics"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "# HELP edgequake_http_requests_total counter\nedgequake_http_requests_total 42\n",
            ))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let text = client.health().metrics().await.unwrap();
        assert!(text.contains("edgequake_http_requests_total"));
    }

    // ── OODA-32: Auth logout ──

    #[tokio::test]
    async fn test_auth_logout() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/logout"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let result = client.auth().logout().await;
        assert!(result.is_ok());
    }

    // ── OODA-32: Document recovery endpoints ──

    #[tokio::test]
    async fn test_documents_delete_all() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": 5})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.documents().delete_all().await.unwrap();
        assert_eq!(val["deleted"], 5);
    }

    #[tokio::test]
    async fn test_documents_reprocess() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents/reprocess"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"reprocessed": 3})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.documents().reprocess().await.unwrap();
        assert_eq!(val["reprocessed"], 3);
    }

    #[tokio::test]
    async fn test_documents_recover_stuck() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents/recover-stuck"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"recovered": 2})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.documents().recover_stuck().await.unwrap();
        assert_eq!(val["recovered"], 2);
    }

    #[tokio::test]
    async fn test_documents_retry_chunks() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents/doc-1/retry-chunks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"retried": 4})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.documents().retry_chunks("doc-1").await.unwrap();
        assert_eq!(val["retried"], 4);
    }

    #[tokio::test]
    async fn test_documents_failed_chunks() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-1/failed-chunks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"chunk_id": "c1", "error": "timeout"}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let chunks = client.documents().failed_chunks("doc-1").await.unwrap();
        assert_eq!(chunks.len(), 1);
    }

    // ── OODA-32: Graph extended endpoints ──

    #[tokio::test]
    async fn test_graph_get_node() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/nodes/n123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "n123", "label": "PERSON", "name": "Alice"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let node = client.graph().get_node("n123").await.unwrap();
        assert_eq!(node["name"], "Alice");
    }

    #[tokio::test]
    async fn test_graph_search_labels() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/labels/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"label": "PERSON", "count": 42}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let labels = client.graph().search_labels("PERS").await.unwrap();
        assert_eq!(labels.len(), 1);
    }

    #[tokio::test]
    async fn test_graph_popular_labels() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/labels/popular"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"label": "ORGANIZATION", "count": 100},
                {"label": "PERSON", "count": 80}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let labels = client.graph().popular_labels().await.unwrap();
        assert_eq!(labels.len(), 2);
    }

    // ── OODA-32: Entity update ──

    #[tokio::test]
    async fn test_entities_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/graph/entities/ALICE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "ALICE", "entity_type": "PERSON", "description": "Updated"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"description": "Updated"});
        let val = client.entities().update("ALICE", &body).await.unwrap();
        assert_eq!(val["description"], "Updated");
    }

    // ── OODA-32: Relationship get/update ──

    #[tokio::test]
    async fn test_relationships_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/relationships/r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r1", "source": "ALICE", "target": "BOB",
                "keywords": ["KNOWS"], "weight": 0.9
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let rel = client.relationships().get("r1").await.unwrap();
        assert_eq!(rel.source, "ALICE");
    }

    #[tokio::test]
    async fn test_relationships_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/graph/relationships/r1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "r1", "weight": 1.0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"weight": 1.0});
        let val = client.relationships().update("r1", &body).await.unwrap();
        assert_eq!(val["weight"], 1.0);
    }

    // ── OODA-32: Task retry ──

    #[tokio::test]
    async fn test_tasks_retry() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/tasks/t1/retry"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "retrying"})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.tasks().retry("t1").await.unwrap();
        assert_eq!(val["status"], "retrying");
    }

    // ── OODA-32: Pipeline cancel ──

    #[tokio::test]
    async fn test_pipeline_cancel() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/pipeline/cancel"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.pipeline().cancel().await.is_ok());
    }

    // ── OODA-32: Costs extended ──

    #[tokio::test]
    async fn test_costs_pricing() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/costs/pricing"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{"name": "gpt-4o", "cost_per_1k_tokens": 0.03}]
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.costs().pricing().await.unwrap();
        assert!(val["models"].is_array());
    }

    #[tokio::test]
    async fn test_costs_estimate() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/pipeline/costs/estimate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "estimated_cost": 1.50, "token_count": 50000
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"document_count": 10});
        let val = client.costs().estimate(&body).await.unwrap();
        assert_eq!(val["estimated_cost"], 1.5);
    }

    #[tokio::test]
    async fn test_costs_update_budget() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/costs/budget"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "monthly_budget_usd": 100.0, "current_spend_usd": 10.0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"monthly_budget_usd": 100.0});
        let val = client.costs().update_budget(&body).await.unwrap();
        assert_eq!(val.monthly_budget_usd, Some(100.0));
    }

    // ── OODA-32: Tenant update ──

    #[tokio::test]
    async fn test_tenants_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/tenants/t1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "t1", "name": "Updated Tenant"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"name": "Updated Tenant"});
        let val = client.tenants().update("t1", &body).await.unwrap();
        assert_eq!(val.name, "Updated Tenant");
    }

    // ── OODA-32: Folder update ──

    #[tokio::test]
    async fn test_folders_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/folders/f1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "f1", "name": "Renamed Folder"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"name": "Renamed Folder"});
        let val = client.folders().update("f1", &body).await.unwrap();
        assert_eq!(val.name, "Renamed Folder");
    }

    // ── OODA-32: Conversation extended ──

    #[tokio::test]
    async fn test_conversations_import() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/import"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"imported": 3})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"conversations": []});
        let val = client.conversations().import(&body).await.unwrap();
        assert_eq!(val["imported"], 3);
    }

    #[tokio::test]
    async fn test_conversations_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/conversations/c1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "c1", "title": "New Title"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"title": "New Title"});
        let val = client.conversations().update("c1", &body).await.unwrap();
        assert_eq!(val.title, Some("New Title".to_string()));
    }

    #[tokio::test]
    async fn test_conversations_unshare() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/conversations/c1/share"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.conversations().unshare("c1").await.is_ok());
    }

    #[tokio::test]
    async fn test_conversations_bulk_archive() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/bulk/archive"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"affected": 2})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let ids = vec!["c1".to_string(), "c2".to_string()];
        let val = client.conversations().bulk_archive(&ids, true).await.unwrap();
        assert_eq!(val.affected, 2);
    }

    #[tokio::test]
    async fn test_conversations_bulk_move() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/bulk/move"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"affected": 2})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let ids = vec!["c1".to_string(), "c2".to_string()];
        let val = client
            .conversations()
            .bulk_move(&ids, Some("folder-1"))
            .await
            .unwrap();
        assert_eq!(val.affected, 2);
    }

    // ── OODA-32: Models extended ──

    #[tokio::test]
    async fn test_models_list_llm() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models/llm"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{"name": "gemma3"}]
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.models().list_llm().await.unwrap();
        assert!(val["models"].is_array());
    }

    #[tokio::test]
    async fn test_models_list_embedding() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models/embedding"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "models": [{"name": "nomic-embed-text"}]
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.models().list_embedding().await.unwrap();
        assert!(val["models"].is_array());
    }

    #[tokio::test]
    async fn test_models_get_provider() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models/ollama"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "ollama", "models": [{"name": "gemma3"}]
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.models().get_provider("ollama").await.unwrap();
        assert_eq!(val["name"], "ollama");
    }

    #[tokio::test]
    async fn test_models_get_model() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models/ollama/gemma3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "name": "gemma3", "provider": "ollama", "is_available": true
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.models().get_model("ollama", "gemma3").await.unwrap();
        assert_eq!(val["name"], "gemma3");
    }

    // ── OODA-32: Workspace extended ──

    #[tokio::test]
    async fn test_workspaces_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/workspaces/ws1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "ws1", "name": "Default Workspace"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let ws = client.workspaces().get("ws1").await.unwrap();
        assert_eq!(ws.name, "Default Workspace");
    }

    #[tokio::test]
    async fn test_workspaces_update() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/workspaces/ws1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "ws1", "name": "Renamed"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let body = json!({"name": "Renamed"});
        let ws = client.workspaces().update("ws1", &body).await.unwrap();
        assert_eq!(ws.name, "Renamed");
    }

    #[tokio::test]
    async fn test_workspaces_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/workspaces/ws1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.workspaces().delete("ws1").await.is_ok());
    }

    #[tokio::test]
    async fn test_workspaces_metrics_history() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/workspaces/ws1/metrics-history"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"timestamp": "2026-01-01", "document_count": 10}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let history = client.workspaces().metrics_history("ws1").await.unwrap();
        assert_eq!(history.len(), 1);
    }

    #[tokio::test]
    async fn test_workspaces_rebuild_embeddings() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/workspaces/ws1/rebuild-embeddings"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "started"})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client.workspaces().rebuild_embeddings("ws1").await.unwrap();
        assert_eq!(val["status"], "started");
    }

    #[tokio::test]
    async fn test_workspaces_rebuild_knowledge_graph() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/workspaces/ws1/rebuild-knowledge-graph"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"status": "started"})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client
            .workspaces()
            .rebuild_knowledge_graph("ws1")
            .await
            .unwrap();
        assert_eq!(val["status"], "started");
    }

    #[tokio::test]
    async fn test_workspaces_reprocess_documents() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/workspaces/ws1/reprocess-documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"reprocessed": 5})))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let val = client
            .workspaces()
            .reprocess_documents("ws1")
            .await
            .unwrap();
        assert_eq!(val["reprocessed"], 5);
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // OODA-41: Additional Test Coverage (22 new tests)
    // ═══════════════════════════════════════════════════════════════════════════

    // ── Users (missing: list, get, delete) ─────────────────────────────

    #[tokio::test]
    async fn test_users_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": "u1", "username": "alice", "email": "a@b.com", "role": "user"},
                {"id": "u2", "username": "bob", "email": "b@c.com", "role": "admin"}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let users = client.users().list().await.unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].username, Some("alice".to_string()));
    }

    #[tokio::test]
    async fn test_users_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/users/u1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "u1", "username": "alice", "email": "a@b.com", "role": "user"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let user = client.users().get("u1").await.unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(user.username, Some("alice".to_string()));
    }

    #[tokio::test]
    async fn test_users_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/users/u1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.users().delete("u1").await.is_ok());
    }

    // ── API Keys (missing: list) ───────────────────────────────────────

    #[tokio::test]
    async fn test_api_keys_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/api-keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": "ak1", "name": "prod-key", "created_at": "2026-01-01"},
                {"id": "ak2", "name": "dev-key", "created_at": "2026-01-02"}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let keys = client.api_keys().list().await.unwrap();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].name, Some("prod-key".to_string()));
    }

    // ── Tenants (missing: get, delete) ─────────────────────────────────

    #[tokio::test]
    async fn test_tenants_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants/t1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "t1", "name": "Acme Corp", "slug": "acme", "plan": "enterprise"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let tenant = client.tenants().get("t1").await.unwrap();
        assert_eq!(tenant.id, "t1");
        assert_eq!(tenant.name, "Acme Corp");
    }

    #[tokio::test]
    async fn test_tenants_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/tenants/t1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.tenants().delete("t1").await.is_ok());
    }

    // ── Folders (missing: list, delete) ────────────────────────────────

    #[tokio::test]
    async fn test_folders_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/folders"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id": "f1", "name": "Projects", "color": "#ff0000"},
                {"id": "f2", "name": "Archive", "color": "#00ff00"}
            ])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let folders = client.folders().list().await.unwrap();
        assert_eq!(folders.len(), 2);
        assert_eq!(folders[0].name, "Projects");
    }

    #[tokio::test]
    async fn test_folders_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/folders/f1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.folders().delete("f1").await.is_ok());
    }

    // ── Chunks (missing: get, get_lineage) ─────────────────────────────

    #[tokio::test]
    async fn test_chunks_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/chunks/ch1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "ch1", "document_id": "doc1", "index": 0, "content": "Hello world"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let chunk = client.chunks().get("ch1").await.unwrap();
        assert_eq!(chunk.id, "ch1");
        assert_eq!(chunk.content.as_deref(), Some("Hello world"));
    }

    #[tokio::test]
    async fn test_chunks_get_lineage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/chunks/ch1/lineage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "chunk_id": "ch1",
                "document_id": "doc1",
                "start_line": 10,
                "end_line": 20
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let lineage = client.chunks().get_lineage("ch1").await.unwrap();
        assert_eq!(lineage.chunk_id, "ch1");
        assert_eq!(lineage.document_id.as_deref(), Some("doc1"));
    }

    // ── Documents (missing: status, scan, deletion_impact, get_lineage, get_metadata) ─

    #[tokio::test]
    async fn test_documents_track_by_doc_flow() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/track/trk1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "track_id": "trk1", "status": "completed", "progress": 1.0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let status = client.documents().track("trk1").await.unwrap();
        assert_eq!(status.status, "completed");
    }

    #[tokio::test]
    async fn test_documents_scan() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/documents/scan"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "files_found": 5, "files_queued": 3, "files_skipped": 2
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let req = types::documents::ScanRequest {
            path: "/data/docs".into(),
            recursive: Some(true),
            extensions: None,
        };
        let resp = client.documents().scan(&req).await.unwrap();
        assert_eq!(resp.files_found, 5);
    }

    #[tokio::test]
    async fn test_documents_deletion_impact() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc1/deletion-impact"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "chunk_count": 10,
                "entity_count": 5,
                "relationship_count": 3
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let impact = client.documents().deletion_impact("doc1").await.unwrap();
        assert_eq!(impact.chunk_count, 10);
    }

    #[tokio::test]
    async fn test_documents_get_lineage() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc1/lineage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "document_id": "doc1",
                "chunks": [{"id": "ch1", "index": 0}],
                "entities": [{"name": "ALICE", "type": "person"}],
                "relationships": []
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let lineage = client.documents().get_lineage("doc1").await.unwrap();
        assert_eq!(lineage.document_id, "doc1");
    }

    #[tokio::test]
    async fn test_documents_get_metadata() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc1/metadata"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "author": "John Doe",
                "category": "research",
                "tags": ["AI", "ML"]
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let meta = client.documents().get_metadata("doc1").await.unwrap();
        assert_eq!(meta["author"], "John Doe");
    }

    // ── Conversations (missing: list, get, delete, list_messages, pin, unpin) ─

    #[tokio::test]
    async fn test_conversations_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "c1", "title": "Chat 1"},
                    {"id": "c2", "title": "Chat 2"}
                ],
                "pagination": {"has_more": false, "total": 2}
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let convos = client.conversations().list().await.unwrap();
        assert_eq!(convos.items.len(), 2);
        assert_eq!(convos.items[0].title, Some("Chat 1".to_string()));
    }

    #[tokio::test]
    async fn test_conversations_list_with_query() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations"))
            .and(query_param("limit", "5"))
            .and(query_param("filter[archived]", "false"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [{"id": "c1", "title": "A"}],
                "pagination": {"has_more": false, "total": 1}
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let q = types::conversations::ConversationListQuery {
            limit: Some(5),
            filter_archived: Some(false),
            ..Default::default()
        };
        let page = client.conversations().list_with_query(&q).await.unwrap();
        assert_eq!(page.items.len(), 1);
    }

    #[tokio::test]
    async fn test_conversations_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations/c1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "conversation": {
                    "id": "c1",
                    "title": "My Chat",
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z"
                },
                "messages": []
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let convo = client.conversations().get("c1").await.unwrap();
        assert_eq!(convo.conversation.id, "c1");
    }

    #[tokio::test]
    async fn test_conversations_delete() {
        let mock_server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/api/v1/conversations/c1"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.conversations().delete("c1").await.is_ok());
    }

    #[tokio::test]
    async fn test_conversations_list_messages() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/conversations/c1/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [
                    {"id": "m1", "role": "user", "content": "Hello"},
                    {"id": "m2", "role": "assistant", "content": "Hi!"}
                ],
                "pagination": {"has_more": false, "total": 2}
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let page = client.conversations().list_messages("c1").await.unwrap();
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_conversations_pin() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/conversations/c1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "c1", "title": "Chat", "is_pinned": true
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.conversations().pin("c1").await.is_ok());
    }

    #[tokio::test]
    async fn test_conversations_unpin() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/api/v1/conversations/c1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "c1", "title": "Chat", "is_pinned": false
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        assert!(client.conversations().unpin("c1").await.is_ok());
    }

    // ── Auth (missing: refresh) ────────────────────────────────────────

    #[tokio::test]
    async fn test_auth_refresh() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/auth/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "new-tok-123",
                "refresh_token": "new-ref-456"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let req = types::auth::RefreshRequest {
            refresh_token: "old-ref-token".into(),
        };
        let token = client.auth().refresh(&req).await.unwrap();
        assert_eq!(token.access_token, "new-tok-123");
    }

    // ── OODA-46: Additional Edge Case Tests ─────────────────────────────

    #[tokio::test]
    async fn test_documents_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "documents": [],
                "pagination": {"page": 1, "per_page": 10, "total": 0, "total_pages": 0}
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let docs = client.documents().list().await.unwrap();
        assert!(docs.documents.is_empty());
    }

    #[tokio::test]
    async fn test_entities_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/entities"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [],
                "total": 0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let entities = client.entities().list().await.unwrap();
        assert!(entities.items.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_status_idle() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "is_busy": false,
                "pending_tasks": 0,
                "processing_tasks": 0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let status = client.pipeline().status().await.unwrap();
        assert!(!status.is_busy);
        assert_eq!(status.pending_tasks, 0);
    }

    #[tokio::test]
    async fn test_pipeline_status_busy() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/pipeline/status"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "is_busy": true,
                "pending_tasks": 10,
                "processing_tasks": 3
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let status = client.pipeline().status().await.unwrap();
        assert!(status.is_busy);
        assert_eq!(status.pending_tasks, 10);
    }

    #[tokio::test]
    async fn test_relationships_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/graph/relationships"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": [],
                "total": 0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let rels = client.relationships().list().await.unwrap();
        assert!(rels.items.is_empty());
    }

    #[tokio::test]
    async fn test_tasks_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tasks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tasks": [],
                "total": 0
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let tasks = client.tasks().list().await.unwrap();
        assert_eq!(tasks.total, 0);
    }

    #[tokio::test]
    async fn test_tasks_get() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tasks/t-123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "track_id": "t-123",
                "status": "completed",
                "task_type": "document_processing"
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let task = client.tasks().get("t-123").await.unwrap();
        assert_eq!(task.status, "completed");
    }

    #[tokio::test]
    async fn test_models_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/models"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "providers": []
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let models = client.models().list().await.unwrap();
        assert!(models.providers.is_empty());
    }

    #[tokio::test]
    async fn test_costs_summary_ooda46() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/costs/summary"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "total_cost_usd": 150.75,
                "total_tokens": 50000,
                "total_input_tokens": 30000,
                "total_output_tokens": 20000,
                "document_count": 100,
                "query_count": 500
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let summary = client.costs().summary().await.unwrap();
        assert_eq!(summary.total_cost_usd, 150.75);
    }

    #[tokio::test]
    async fn test_tenants_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "items": []
            })))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let tenants = client.tenants().list().await.unwrap();
        assert!(tenants.items.is_empty());
    }

    #[tokio::test]
    async fn test_users_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let users = client.users().list().await.unwrap();
        assert!(users.is_empty());
    }

    #[tokio::test]
    async fn test_folders_list_empty() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/folders"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
            .mount(&mock_server)
            .await;
        let client = test_client(&mock_server).await;
        let folders = client.folders().list().await.unwrap();
        assert!(folders.is_empty());
    }
}
