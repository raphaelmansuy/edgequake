//! Shared P0 HTTP boot + F0 seed helpers for PROVIDER-ACCESS-E2E02+.

use std::sync::Arc;
use std::time::{Duration, Instant};

use edgequake_api::{AppState, Server, ServerConfig};
use edgequake_storage::contracts::{
    AccessScope, DocumentId, PreparedIngestionBatch, PreparedRecord, TenantId, WorkspaceId,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

use super::assertions::assert_contains_no_forbidden_sentinel;
use super::fixtures;

pub const API_KEY: &str = "provider-access-e2e-key";
pub const EMBED_MODEL: &str = "text-embedding-3-small";
// Upstream MockProvider emits 1536 dimensions regardless of the requested size.
pub const EMBED_DIM: usize = 1536;

pub struct LiveServer {
    pub state: AppState,
    pub base: String,
    pub pool: sqlx::PgPool,
    pub client: reqwest::Client,
    pub _server: tokio::task::JoinHandle<()>,
}

pub fn prepare_env() {
    std::env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    std::env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    std::env::set_var("EDGEQUAKE_EMBEDDING_MODEL", EMBED_MODEL);
    std::env::set_var("EDGEQUAKE_EMBEDDING_DIMENSION", EMBED_DIM.to_string());
    // Certification must not call a developer OpenAI key mid-suite.
    std::env::remove_var("OPENAI_API_KEY");
    std::env::remove_var("OPENAI_BASE_URL");
}

pub async fn boot(database_url: &str) -> LiveServer {
    boot_with_providers(database_url, Default::default()).await
}

pub async fn boot_with_providers(
    database_url: &str,
    providers: edgequake_api::state::data_access_providers::ProviderOverrides,
) -> LiveServer {
    prepare_env();
    let mut state = AppState::new_postgres_with_providers(database_url, "", providers)
        .await
        .expect("AppState::new_postgres must succeed");
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec![API_KEY.to_string()];
    let pool = state.pg_pool.as_ref().expect("P0 pool").clone();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let address = listener.local_addr().expect("addr");
    let app = Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: false,
            enable_compression: false,
            enable_swagger: false,
        },
        state.clone(),
    )
    .build_router();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve");
    });
    LiveServer {
        state,
        base: format!("http://{address}"),
        pool,
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("bounded HTTP client"),
        _server: server,
    }
}

impl Drop for LiveServer {
    fn drop(&mut self) {
        self._server.abort();
    }
}

pub fn prepared_record(id: Uuid, payload: serde_json::Value) -> PreparedRecord {
    let payload = serde_json::to_vec(&payload).expect("encode");
    PreparedRecord {
        id,
        revision: 1,
        digest: Sha256::digest(&payload).into(),
        payload,
    }
}

pub async fn seed_scope(pool: &sqlx::PgPool, tenant_id: Uuid, workspace_id: Uuid, label: &str) {
    let suffix = tenant_id.as_simple().to_string();
    sqlx::query(
        "INSERT INTO tenants (tenant_id, name, slug) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(tenant_id)
    .bind(format!("PA {label}"))
    .bind(format!("pa-{label}-{suffix}"))
    .execute(pool)
    .await
    .expect("seed tenant");
    sqlx::query(
        "INSERT INTO workspaces (workspace_id, tenant_id, name, slug) VALUES ($1, $2, $3, $4) \
         ON CONFLICT DO NOTHING",
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(format!("PA ws {label}"))
    .bind(format!("pa-ws-{label}-{suffix}"))
    .execute(pool)
    .await
    .expect("seed workspace");
    sqlx::query("UPDATE workspaces SET metadata = metadata || jsonb_build_object('embedding_dimension', $2::int, 'embedding_provider', 'mock', 'embedding_model', $3::text, 'llm_provider', 'mock', 'llm_model', 'mock-model') WHERE workspace_id = $1")
        .bind(workspace_id).bind(EMBED_DIM as i32).bind(EMBED_MODEL)
        .execute(pool).await.expect("explicit mock workspace configuration");
}

pub struct SeededDoc {
    pub document_id: Uuid,
    pub chunk_id: Uuid,
    pub node_logical: String,
}

/// Commit one document with a distinguishing graph node + embedding, then wait for drain.
pub async fn commit_and_drain(
    state_pool: &sqlx::PgPool,
    committer: &Arc<dyn edgequake_storage::contracts::IngestionCommitter>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    content: &str,
    node_logical: &str,
    mark_ready: bool,
) -> SeededDoc {
    commit_and_drain_with_embedding(
        state_pool,
        committer,
        tenant_id,
        workspace_id,
        content,
        node_logical,
        mark_ready,
        vec![0.1; EMBED_DIM],
    )
    .await
}

/// Same durable fixture with explicit dimensions for real query-provider parity.
#[allow(clippy::too_many_arguments)]
pub async fn commit_and_drain_with_embedding(
    state_pool: &sqlx::PgPool,
    committer: &Arc<dyn edgequake_storage::contracts::IngestionCommitter>,
    tenant_id: Uuid,
    workspace_id: Uuid,
    content: &str,
    node_logical: &str,
    mark_ready: bool,
    vector: Vec<f32>,
) -> SeededDoc {
    let document_id = Uuid::new_v4();
    let chunk_id = Uuid::new_v4();
    let fact_id = Uuid::new_v4();
    let fact = serde_json::json!({
        "schema": "edgequake.graph.fact.v1",
        "kind": "node",
        "node_id": node_logical,
        "properties": {
            "entity_type": "TEST",
            "description": content,
            "source_document_id": document_id,
            "tenant_id": tenant_id,
            "workspace_id": workspace_id,
        }
    });
    let embedding = serde_json::json!({
        "schema": "edgequake.embedding.v1",
        "family": "chunk",
        "subject_id": chunk_id,
        "workspace_id": workspace_id,
        "model_id": EMBED_MODEL,
        "dimensions": vector.len(),
        "embedding": vector,
        "legacy_vector_id": format!("{document_id}-chunk-0"),
    });
    let command = PreparedIngestionBatch {
        scope: AccessScope::new(TenantId::new(tenant_id), WorkspaceId::new(workspace_id)),
        document_id: DocumentId::new(document_id),
        ingest_generation: 1,
        batch_ordinal: 0,
        expected_revision: Some(0),
        idempotency_key: format!("pa-http-{document_id}"),
        schema_version: 1,
        canonical_digest: Sha256::digest(format!("pa-http:{document_id}:{content}").as_bytes())
            .into(),
        chunks: vec![prepared_record(
            chunk_id,
            serde_json::json!({
                "chunk_index": 0,
                "content": content,
                "metadata": {"legacy_chunk_key": format!("{document_id}-chunk-0")}
            }),
        )],
        facts: vec![prepared_record(fact_id, fact.clone())],
        contributions: vec![prepared_record(fact_id, fact)],
        embeddings: vec![prepared_record(chunk_id, embedding)],
    };
    committer
        .commit_batch(&command)
        .await
        .expect("commit_batch");
    wait_applied(state_pool, document_id).await;
    if mark_ready {
        mark_chunk_ready(state_pool, chunk_id).await;
    }
    let _ = sqlx::query(
        "UPDATE documents SET title = $2, status = 'indexed', content = $3 WHERE id = $1",
    )
    .bind(document_id)
    .bind(content.chars().take(48).collect::<String>())
    .bind(content)
    .execute(state_pool)
    .await;
    SeededDoc {
        document_id,
        chunk_id,
        node_logical: node_logical.to_string(),
    }
}

pub async fn wait_applied(pool: &sqlx::PgPool, document_id: Uuid) {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let applied: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM projection_deliveries d \
             JOIN projection_events e USING (event_id) \
             WHERE e.object_id = $1 AND d.state = 'applied'",
        )
        .bind(document_id)
        .fetch_one(pool)
        .await
        .expect("count applied");
        if applied >= 2 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "projection drain timed out for {document_id}"
        );
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
}

pub async fn mark_chunk_ready(pool: &sqlx::PgPool, chunk_id: Uuid) {
    sqlx::query(
        "INSERT INTO public.chunk_serving_state (chunk_id, state) VALUES ($1, 'ready') \
         ON CONFLICT (chunk_id) DO UPDATE SET state = 'ready'",
    )
    .bind(chunk_id)
    .execute(pool)
    .await
    .expect("mark chunk ready");
}

impl LiveServer {
    pub fn committer(&self) -> Arc<dyn edgequake_storage::contracts::IngestionCommitter> {
        self.state
            .ingestion_committer
            .as_ref()
            .expect("selected relational committer")
            .clone()
    }

    fn authed_get(&self, path: &str, tenant: Uuid, workspace: Uuid) -> reqwest::RequestBuilder {
        self.client
            .get(format!("{}{path}", self.base))
            .header("X-API-Key", API_KEY)
            .header("X-Tenant-ID", tenant.to_string())
            .header("X-Workspace-ID", workspace.to_string())
    }

    pub async fn get_text(
        &self,
        path: &str,
        tenant: Uuid,
        workspace: Uuid,
    ) -> (reqwest::StatusCode, String) {
        let response = self
            .authed_get(path, tenant, workspace)
            .send()
            .await
            .expect("GET");
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        (status, body)
    }

    pub async fn delete_document(
        &self,
        document_id: Uuid,
        tenant: Uuid,
        workspace: Uuid,
    ) -> reqwest::StatusCode {
        self.client
            .delete(format!("{}/api/v1/documents/{document_id}", self.base))
            .header("X-API-Key", API_KEY)
            .header("X-Tenant-ID", tenant.to_string())
            .header("X-Workspace-ID", workspace.to_string())
            .send()
            .await
            .expect("DELETE")
            .status()
    }

    pub async fn post_json(
        &self,
        path: &str,
        tenant: Uuid,
        workspace: Uuid,
        body: serde_json::Value,
    ) -> (reqwest::StatusCode, String) {
        let response = self
            .client
            .post(format!("{}{path}", self.base))
            .header("X-API-Key", API_KEY)
            .header("X-Tenant-ID", tenant.to_string())
            .header("X-Workspace-ID", workspace.to_string())
            .json(&body)
            .send()
            .await
            .expect("POST");
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        (status, text)
    }

    pub async fn query_naive(
        &self,
        tenant: Uuid,
        workspace: Uuid,
        query: &str,
        document_ids: Option<Vec<String>>,
        max_results: Option<usize>,
    ) -> (reqwest::StatusCode, String) {
        let mut body = serde_json::json!({
            "query": query,
            "mode": "naive",
            "context_only": true,
            "enable_rerank": false,
            "llm_provider": "mock",
            "llm_model": "mock-model",
            "hl_keywords": ["fixture"],
            "ll_keywords": ["fixture"],
        });
        if let Some(ids) = document_ids {
            body["document_filter"] = serde_json::json!({ "document_ids": ids });
        }
        if let Some(k) = max_results {
            body["max_results"] = serde_json::json!(k);
        }
        self.post_json("/api/v1/query", tenant, workspace, body)
            .await
    }

    pub fn assert_no_secrets(body: &str) {
        assert_contains_no_forbidden_sentinel(
            body,
            &[
                fixtures::BETA_SECRET,
                fixtures::PENDING_SECRET,
                fixtures::WORKSPACE_SECRET,
                fixtures::ALPHA_ONLY,
            ],
        );
    }
}

pub fn pass(test_id: &str) {
    eprintln!(
        r#"{{"event":"provider_access_pass","test_id":"{test_id}","profile":"P0","certification_successes":1}}"#
    );
}
