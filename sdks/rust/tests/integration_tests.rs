#[cfg(test)]
mod tests {
    use edgequake_sdk::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, path_regex};
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
        let result = EdgeQuakeClient::builder()
            .base_url("not a url")
            .build();
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
            .and(path("/api/v1/documents/upload/text"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id":"doc-2","status":"processing","track_id":"trk-1"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let body = json!({"content":"hello world","title":"test"});
        let res = client.documents().upload_text(&body).await.unwrap();
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
            mode: None,
            top_k: Some(5),
            stream: None,
            only_need_context: None,
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
            mode: None,
            conversation_id: None,
            max_tokens: None,
            temperature: None,
            top_k: None,
            parent_id: None,
            provider: None,
            model: None,
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
            slug: None,
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
        };
        let msg = client.conversations().create_message("c1", &req).await.unwrap();
        assert_eq!(msg.id, "m1");
    }

    #[tokio::test]
    async fn test_conversations_share() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v1/conversations/c1/share"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "share_id":"sh-1","url":"https://app.co/share/sh-1"
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
                "deleted_count": 3
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let ids = vec!["c1".into(), "c2".into(), "c3".into()];
        let r = client.conversations().bulk_delete(&ids).await.unwrap();
        assert_eq!(r.deleted_count, 3);
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

    // ── Chunks ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_chunks_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/documents/doc-1/chunks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id":"ch-1","document_id":"doc-1","content":"chunk text","chunk_index":0}
            ])))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let chunks = client.chunks().list("doc-1").await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, "ch-1");
    }

    // ── Provenance ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_provenance_for_entity() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex("/api/v1/entities/.*/provenance"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"entity_name":"Alice","document_id":"d1","confidence":0.9}
            ])))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.provenance().for_entity("Alice").await.unwrap();
        assert_eq!(r.len(), 1);
        assert!((r[0].confidence.unwrap() - 0.9).abs() < 0.01);
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

    #[tokio::test]
    async fn test_models_set_provider() {
        let mock_server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/settings/provider"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "current_provider":"ollama","status":"active"
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(&mock_server).await;
        let r = client.models().set_provider("ollama").await.unwrap();
        assert_eq!(r.current_provider.as_deref(), Some("ollama"));
    }

    // ── Workspaces ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_workspaces_list() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/tenants/t1/workspaces"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                {"id":"w1","name":"default","tenant_id":"t1"}
            ])))
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
            slug: None,
            description: None,
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
}
