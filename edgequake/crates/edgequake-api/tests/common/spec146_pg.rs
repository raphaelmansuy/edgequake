//! SPEC-146 shared PG harness — auth on + ABAC on (LAW-146-20).
//!
//! Mutates AppState after build (no process-wide `EDGEQUAKE_DOC_ABAC` race).
//! Skip when DATABASE_URL is unset.

#![cfg(feature = "postgres")]
#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use edgequake_api::services::spec146_authz::build_allow_set_provider;
use edgequake_api::{AppState, Server, ServerConfig};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use tower::ServiceExt;
use uuid::Uuid;

use super::spec013_postgres;
use super::test_db;

pub const SECRET_TOKEN: &str = "SECRET_TOKEN_SPEC146_UNAUTHORIZED";
pub const DOCA_TITLE: &str = "Public Handbook ENTITY_X";
pub const DOCB_TITLE: &str = "Secret Notebook ENTITY_X";
pub const MASTER_API_KEY: &str = "spec146-master-key";

pub struct Spec146Harness {
    pub pool: PgPool,
    pub state: AppState,
    pub tenant_id: String,
    pub workspace_id: String,
    pub owner_user_id: String,
    pub owner_token: String,
    pub peer_user_id: String,
    pub peer_token: String,
    pub doc_a: Uuid,
    pub doc_b: Uuid,
}

fn server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    }
}

pub fn build_router(state: AppState) -> axum::Router {
    Server::new(server_config(), state).build_router()
}

pub async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap_or(json!({}))
}

/// Build ABAC-on + auth-on AppState on isolated `{db}_test`. Returns None when no DB.
pub async fn try_create_harness() -> Option<Spec146Harness> {
    let base = spec013_postgres::try_database_url()?;
    let url = test_db::isolated_test_url(&base);

    // Avoid process-wide DOC_ABAC env; mutate state after construction.
    env::set_var("EDGEQUAKE_AUTH_ENABLED", "true");
    env::set_var("EDGEQUAKE_DOC_ABAC", "false");
    env::set_var("EDGEQUAKE_LLM_PROVIDER", "mock");
    env::set_var("EDGEQUAKE_EMBEDDING_PROVIDER", "mock");
    env::set_var("EDGEQUAKE_ALLOW_MOCK_PROVIDER", "1");
    env::set_var("EDGEQUAKE_DEV_MODE", "false");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .map_err(|e| eprintln!("SKIP SPEC-146 PG harness: connect failed: {e}"))
        .ok()?;

    // Stale `{db}_test` schemas may predate SPEC-027 lockout columns (support/048).
    for stmt in [
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS failed_login_attempts INT NOT NULL DEFAULT 0",
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ",
    ] {
        if let Err(e) = sqlx::query(stmt).execute(&pool).await {
            eprintln!("SPEC-146 schema repair warn ({stmt}): {e}");
        }
    }

    let mut state = AppState::new_postgres(url, "")
        .await
        .map_err(|e| eprintln!("SKIP SPEC-146 PG harness: AppState failed: {e}"))
        .ok()?;

    state.security.doc_abac = true;
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.auth.config.api_keys = vec![MASTER_API_KEY.to_string()];
    state.allow_set_provider = build_allow_set_provider(state.pg_pool.as_ref(), true);

    // Defaults may already exist on shared `_test` DB — ignore duplicate-key noise.
    if let Err(e) = state.initialize_defaults().await {
        eprintln!("SPEC-146: initialize_defaults warn (continuing): {e}");
    }

    let mut harness = Spec146Harness {
        pool: state.pg_pool.clone().unwrap_or(pool),
        state,
        tenant_id: String::new(),
        workspace_id: String::new(),
        owner_user_id: String::new(),
        owner_token: String::new(),
        peer_user_id: String::new(),
        peer_token: String::new(),
        doc_a: Uuid::new_v4(),
        doc_b: Uuid::new_v4(),
    };

    harness.bootstrap_identity().await?;
    harness.seed_documents().await?;
    Some(harness)
}

impl Spec146Harness {
    pub fn router(&self) -> axum::Router {
        build_router(self.state.clone())
    }

    /// Same stack with ABAC off (G-146-90).
    pub fn router_abac_off(&self) -> axum::Router {
        let mut state = self.state.clone();
        state.security.doc_abac = false;
        state.allow_set_provider = None;
        build_router(state)
    }

    async fn bootstrap_identity(&mut self) -> Option<()> {
        let owner_name = format!("owner_{}", &Uuid::new_v4().to_string()[..8]);
        let peer_name = format!("peer_{}", &Uuid::new_v4().to_string()[..8]);

        let app = self.router();
        let owner = create_and_login(&app, &owner_name).await?;
        let peer = create_and_login(&app, &peer_name).await?;
        self.owner_user_id = owner.0;
        self.owner_token = owner.1;
        self.peer_user_id = peer.0;
        self.peer_token = peer.1;

        let tenant_name = format!("t146_{}", &Uuid::new_v4().to_string()[..8]);
        let create_tenant = self
            .router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/tenants")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {}", self.owner_token))
                    .body(Body::from(
                        json!({ "name": tenant_name, "slug": tenant_name }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .ok()?;
        if create_tenant.status() != StatusCode::CREATED
            && create_tenant.status() != StatusCode::OK
        {
            eprintln!(
                "SKIP SPEC-146: create tenant status {}",
                create_tenant.status()
            );
            return None;
        }
        let tbody = parse_json(create_tenant).await;
        self.tenant_id = tbody
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if self.tenant_id.is_empty() {
            eprintln!("SKIP SPEC-146: no tenant id in response {tbody}");
            return None;
        }

        let ws_name = format!("ws146_{}", &Uuid::new_v4().to_string()[..8]);
        let create_ws = self
            .router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/v1/tenants/{}/workspaces", self.tenant_id))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {}", self.owner_token))
                    .header("X-Tenant-ID", &self.tenant_id)
                    .body(Body::from(
                        json!({
                            "name": ws_name,
                            "llm_provider": "mock",
                            "llm_model": "mock",
                            "embedding_provider": "mock",
                            "embedding_model": "mock",
                            "embedding_dimension": 768
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .ok()?;
        if create_ws.status() != StatusCode::CREATED && create_ws.status() != StatusCode::OK {
            eprintln!(
                "SKIP SPEC-146: create workspace status {}",
                create_ws.status()
            );
            return None;
        }
        let wbody = parse_json(create_ws).await;
        self.workspace_id = wbody
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if self.workspace_id.is_empty() {
            eprintln!("SKIP SPEC-146: no workspace_id in {wbody}");
            return None;
        }
        Some(())
    }

    pub async fn create_user_with_role(&self, username: &str, role: &str) -> Option<(String, String)> {
        create_and_login_with_role(&self.router(), username, role).await
    }

    pub async fn seed_graph_fixture(&self) -> Option<()> {
        use std::collections::HashMap;
        // Shared hub: Public + Secret fragments (G-146-30/31 TARGET 0).
        let mut hub = HashMap::new();
        hub.insert("entity_type".into(), json!("CONCEPT"));
        hub.insert(
            "description".into(),
            json!(format!(
                "public handbook ENTITY_X<SEP>secret notebook {SECRET_TOKEN}"
            )),
        );
        hub.insert(
            "source_ids".into(),
            json!([self.doc_a.to_string(), self.doc_b.to_string()]),
        );
        hub.insert("tenant_id".into(), json!(self.tenant_id));
        hub.insert("workspace_id".into(), json!(self.workspace_id));

        // Secret-only node (must 404 for peer).
        let mut secret = HashMap::new();
        secret.insert("entity_type".into(), json!("CONCEPT"));
        secret.insert(
            "description".into(),
            json!(format!("secret-only {SECRET_TOKEN}")),
        );
        secret.insert(
            "source_ids".into(),
            json!([self.doc_b.to_string()]),
        );
        secret.insert("tenant_id".into(), json!(self.tenant_id));
        secret.insert("workspace_id".into(), json!(self.workspace_id));

        self.state
            .storage
            .graph_storage
            .upsert_node("ENTITY_X", hub)
            .await
            .ok()?;
        self.state
            .storage
            .graph_storage
            .upsert_node("ENTITY_X_SECRET", secret)
            .await
            .ok()?;

        let mut edge_props = HashMap::new();
        edge_props.insert("relation_type".into(), json!("RELATED_TO"));
        edge_props.insert(
            "source_ids".into(),
            json!([self.doc_a.to_string(), self.doc_b.to_string()]),
        );
        edge_props.insert("tenant_id".into(), json!(self.tenant_id));
        edge_props.insert("workspace_id".into(), json!(self.workspace_id));
        let _ = self
            .state
            .storage
            .graph_storage
            .upsert_edge("ENTITY_X", "ENTITY_X_SECRET", edge_props)
            .await;

        Some(())
    }

    /// Seed DocB chunk text into KV so retrieval can surface SECRET_TOKEN if allow leaks.
    pub async fn seed_secret_chunk_kv(&self) -> Option<()> {
        let chunk_id = format!("{}-chunk-0", self.doc_b);
        let rows = vec![(
            chunk_id.clone(),
            json!({
                "id": chunk_id,
                "document_id": self.doc_b.to_string(),
                "content": format!("ENTITY_X secret chunk {SECRET_TOKEN}"),
                "tokens": 8,
                "tenant_id": self.tenant_id,
                "workspace_id": self.workspace_id,
            }),
        )];
        self.state
            .storage
            .kv_storage
            .upsert(&rows)
            .await
            .map_err(|e| eprintln!("SKIP secret chunk KV: {e}"))
            .ok()?;
        Some(())
    }

    /// Stamp track_id on DocA+DocB KV for track_status PEP (G-146-11).
    pub async fn seed_track_on_docs(&self, track_id: &str) -> Option<()> {
        let mut rows = Vec::new();
        for (id, title, class, share) in [
            (self.doc_a, DOCA_TITLE, "internal", "workspace"),
            (self.doc_b, DOCB_TITLE, "secret", "owner_only"),
        ] {
            let key = format!("{id}-metadata");
            rows.push((
                key,
                json!({
                    "id": id.to_string(),
                    "title": title,
                    "status": "completed",
                    "classification": class,
                    "share_mode": share,
                    "security_status": "ok",
                    "track_id": track_id,
                    "tenant_id": self.tenant_id,
                    "workspace_id": self.workspace_id,
                    "created_at": "2026-01-01T00:00:00Z",
                }),
            ));
        }
        self.state
            .storage
            .kv_storage
            .upsert(&rows)
            .await
            .ok()?;
        Some(())
    }

    async fn seed_documents(&mut self) -> Option<()> {
        let tenant = Uuid::parse_str(&self.tenant_id).ok()?;
        let workspace = Uuid::parse_str(&self.workspace_id).ok()?;

        // DocA: workspace-visible
        sqlx::query(
            r#"
            INSERT INTO documents (
              id, tenant_id, workspace_id, title, content, status,
              classification, share_mode, security_status,
              owner_principal_kind, owner_principal_id, content_hash
            ) VALUES (
              $1, $2, $3, $4, $5, 'completed',
              'internal', 'workspace', 'ok',
              'user', $6, $7
            )
            "#,
        )
        .bind(self.doc_a)
        .bind(tenant)
        .bind(workspace)
        .bind(DOCA_TITLE)
        .bind("ENTITY_X public handbook content")
        .bind(&self.owner_user_id)
        .bind(format!("hash-a-{}", self.doc_a))
        .execute(&self.pool)
        .await
        .map_err(|e| eprintln!("SKIP seed DocA: {e}"))
        .ok()?;

        // DocB: owner_only with SECRET_TOKEN — peer must not see
        sqlx::query(
            r#"
            INSERT INTO documents (
              id, tenant_id, workspace_id, title, content, status,
              classification, share_mode, security_status,
              owner_principal_kind, owner_principal_id, content_hash
            ) VALUES (
              $1, $2, $3, $4, $5, 'completed',
              'secret', 'owner_only', 'ok',
              'user', $6, $7
            )
            "#,
        )
        .bind(self.doc_b)
        .bind(tenant)
        .bind(workspace)
        .bind(DOCB_TITLE)
        .bind(format!("ENTITY_X secret payload {SECRET_TOKEN}"))
        .bind(&self.owner_user_id)
        .bind(format!("hash-b-{}", self.doc_b))
        .execute(&self.pool)
        .await
        .map_err(|e| eprintln!("SKIP seed DocB: {e}"))
        .ok()?;

        // Dual-write KV metadata so list PEP can see titles when KV is primary.
        let mut kv_rows = Vec::new();
        for (id, title, class, share) in [
            (self.doc_a, DOCA_TITLE, "internal", "workspace"),
            (self.doc_b, DOCB_TITLE, "secret", "owner_only"),
        ] {
            let key = format!("{id}-metadata");
            let meta = json!({
                "id": id.to_string(),
                "title": title,
                "status": "completed",
                "classification": class,
                "share_mode": share,
                "security_status": "ok",
                "tenant_id": self.tenant_id,
                "workspace_id": self.workspace_id,
                "created_at": "2026-01-01T00:00:00Z",
            });
            kv_rows.push((key, meta));
        }
        self.state
            .storage
            .kv_storage
            .upsert(&kv_rows)
            .await
            .map_err(|e| eprintln!("SKIP KV upsert: {e}"))
            .ok()?;

        Some(())
    }

    pub fn auth_headers_peer(&self) -> Vec<(String, String)> {
        vec![
            (
                header::AUTHORIZATION.to_string(),
                format!("Bearer {}", self.peer_token),
            ),
            ("X-Tenant-ID".into(), self.tenant_id.clone()),
            ("X-Workspace-ID".into(), self.workspace_id.clone()),
        ]
    }

    pub fn auth_headers_owner(&self) -> Vec<(String, String)> {
        vec![
            (
                header::AUTHORIZATION.to_string(),
                format!("Bearer {}", self.owner_token),
            ),
            ("X-Tenant-ID".into(), self.tenant_id.clone()),
            ("X-Workspace-ID".into(), self.workspace_id.clone()),
        ]
    }
}

async fn create_and_login(app: &axum::Router, username: &str) -> Option<(String, String)> {
    create_and_login_with_role(app, username, "admin").await
}

async fn create_and_login_with_role(
    app: &axum::Router,
    username: &str,
    role: &str,
) -> Option<(String, String)> {
    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", MASTER_API_KEY)
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": role
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .ok()?;
    let create_status = create.status();
    if create_status != StatusCode::CREATED && create_status != StatusCode::OK {
        let body = parse_json(create).await;
        eprintln!("SKIP create user {username}: {create_status} body={body}");
        return None;
    }

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": username,
                        "password": "SecurePass123!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .ok()?;
    if login.status() != StatusCode::OK {
        eprintln!("SKIP login {}: {}", username, login.status());
        return None;
    }
    let body = parse_json(login).await;
    let token = body.get("access_token")?.as_str()?.to_string();
    let user_id = body
        .pointer("/user/user_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if user_id.is_empty() {
        return None;
    }
    Some((user_id, token))
}

pub async fn get_documents(app: axum::Router, headers: &[(String, String)]) -> (StatusCode, Value) {
    let mut req = Request::builder().method("GET").uri("/api/v1/documents");
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    (status, parse_json(resp).await)
}

pub async fn get_document(
    app: axum::Router,
    headers: &[(String, String)],
    doc_id: Uuid,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/documents/{doc_id}"));
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = app.oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    (status, parse_json(resp).await)
}

pub async fn post_query(
    app: axum::Router,
    headers: &[(String, String)],
    query: &str,
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/query")
        .header(header::CONTENT_TYPE, "application/json");
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = app
        .oneshot(
            req.body(Body::from(
                json!({
                    "query": query,
                    "mode": "mix",
                    "include_references": true
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    (status, parse_json(resp).await)
}

pub async fn post_query_filtered(
    app: axum::Router,
    headers: &[(String, String)],
    query: &str,
    document_ids: &[&str],
) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/api/v1/query")
        .header(header::CONTENT_TYPE, "application/json");
    for (k, v) in headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp = app
        .oneshot(
            req.body(Body::from(
                json!({
                    "query": query,
                    "mode": "mix",
                    "include_references": true,
                    "document_filter": { "document_ids": document_ids }
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    (status, parse_json(resp).await)
}

pub async fn request_json(
    app: axum::Router,
    method: &str,
    uri: &str,
    headers: &[(String, String)],
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");
    for (k, v) in headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    let payload = body
        .map(|v| Body::from(v.to_string()))
        .unwrap_or_else(Body::empty);
    let resp = app.oneshot(builder.body(payload).unwrap()).await.unwrap();
    let status = resp.status();
    (status, parse_json(resp).await)
}
