//! Real TCP/PostgreSQL regression for tenant guest bootstrap with forced RLS.
#![cfg(feature = "postgres")]
mod common;

use common::provider_access::{harness, http_harness};
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::test]
async fn identity_bootstrap_does_not_require_an_existing_active_workspace() {
    let Some(url) = harness::certification_database_url().expect("strict PG configuration") else {
        return;
    };
    let admin = sqlx::PgPool::connect(&url).await.unwrap();
    let database = format!("eq_identity_bootstrap_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    let mut scratch_url = reqwest::Url::parse(&url).unwrap();
    scratch_url.set_path(&database);
    let pool = sqlx::PgPool::connect(scratch_url.as_str()).await.unwrap();
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .unwrap();
    std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    // Post-hooks provision defaults too. Preserve their table and dependencies;
    // the isolated database's replacement reproduces an empty authority catalog.
    sqlx::raw_sql(
        "ALTER TABLE public.workspaces RENAME TO provisioned_workspaces; \
        CREATE TABLE public.workspaces (LIKE public.provisioned_workspaces INCLUDING ALL);",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql("GRANT USAGE ON SCHEMA public TO edgequake_tenant_access; \
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO edgequake_tenant_access; \
        GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO edgequake_tenant_access;")
        .execute(&pool).await.unwrap();
    let security = edgequake_api::state::ApiSecurityConfig::default();
    let workspace = edgequake_core::default_workspace_uuid();
    let absent: bool =
        sqlx::query_scalar("SELECT NOT EXISTS(SELECT 1 FROM workspaces WHERE workspace_id=$1)")
            .bind(workspace)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(absent, "fixture must reproduce missing default workspace");
    for _ in 0..2 {
        edgequake_api::services::identity_storage::ensure_default_tenant_workspace(
            &pool, &security,
        )
        .await
        .unwrap();
    }
    sqlx::query("UPDATE workspaces SET is_active=false WHERE workspace_id=$1")
        .bind(workspace)
        .execute(&pool)
        .await
        .unwrap();
    edgequake_api::services::identity_storage::ensure_default_tenant_workspace(&pool, &security)
        .await
        .unwrap();
    let scope = edgequake_api::services::tenant_isolation::PgIsolationScope::default_identity(None);
    edgequake_api::services::tenant_isolation::run_with_pg_rls(&pool, scope, |conn| {
        Box::pin(async move {
            let (role, workspace): (String, Option<Uuid>) =
                sqlx::query_as("SELECT current_user::text, public.current_workspace_id()")
                    .fetch_one(conn)
                    .await
                    .map_err(edgequake_storage::StorageError::from)?;
            assert_eq!(role, "edgequake_tenant_access");
            assert!(workspace.is_none());
            Ok(())
        })
    })
    .await
    .unwrap();
    let server = http_harness::boot(scratch_url.as_str()).await;
    let tenant = edgequake_api::middleware::default_tenant_uuid();
    let guest = edgequake_api::services::identity_storage::shared_guest_user_id(tenant);
    edgequake_api::services::identity_storage::ensure_shared_guest_user_in_postgres(
        &pool, &security, tenant, guest,
    )
    .await
    .unwrap();
    let refresh = format!("fixture-refresh-{}", Uuid::new_v4());
    use sha2::{Digest, Sha256};
    sqlx::query("INSERT INTO refresh_tokens(token_id,user_id,token_hash,expires_at,family_id,status,revoked) VALUES($1,$2,$3,NOW()+interval '1 hour',$4,'active',false)")
        .bind(Uuid::new_v4()).bind(guest).bind(format!("{:x}",Sha256::digest(refresh.as_bytes())))
        .bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    let response = server
        .client
        .post(format!("{}/api/v1/auth/refresh", server.base))
        .json(&json!({"refresh_token":refresh}))
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert!(serde_json::from_str::<Value>(&body).unwrap()["access_token"].is_string());
    let reused = server
        .client
        .post(format!("{}/api/v1/auth/refresh", server.base))
        .json(&json!({"refresh_token":refresh}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        reused.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "refresh reuse must still fail closed"
    );
    eprintln!("POSTGRES_IDENTITY_BOOTSTRAP_PASS");
    pool.close().await;
}

#[allow(clippy::too_many_arguments)]
async fn request(
    server: &http_harness::LiveServer,
    method: reqwest::Method,
    path: &str,
    tenant: Uuid,
    workspace: Uuid,
    browser: Uuid,
    body: Option<Value>,
) -> (reqwest::StatusCode, String) {
    let mut request = server
        .client
        .request(method, format!("{}{path}", server.base))
        .header("X-Tenant-ID", tenant.to_string())
        .header("X-Workspace-ID", workspace.to_string())
        .header("X-User-ID", browser.to_string());
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.expect("real HTTP request");
    (response.status(), response.text().await.expect("HTTP body"))
}

#[tokio::test]
async fn non_default_tenant_guest_conversations_and_folders() {
    let Some(url) = harness::certification_database_url().expect("strict PG configuration") else {
        return;
    };
    let server = http_harness::boot_anonymous(&url).await;
    assert!(server.state.security.pg_rls_enabled);
    let tenant = Uuid::new_v4();
    let workspace = Uuid::new_v4();
    http_harness::seed_scope(&server.pool, tenant, workspace, "anonymous").await;
    let writer = Uuid::new_v4();
    let reader = Uuid::new_v4();
    for (path, input, array_key) in [
        (
            "/api/v1/conversations",
            json!({"title":"guest conversation"}),
            Some("items"),
        ),
        ("/api/v1/folders", json!({"name":"guest folder"}), None),
    ] {
        let (status, body) = request(
            &server,
            reqwest::Method::POST,
            path,
            tenant,
            workspace,
            writer,
            Some(input),
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::CREATED, "{path}: {body}");
        let created: Value = serde_json::from_str(&body).unwrap();
        let (status, body) = request(
            &server,
            reqwest::Method::GET,
            path,
            tenant,
            workspace,
            reader,
            None,
        )
        .await;
        assert_eq!(status, reqwest::StatusCode::OK, "{path}: {body}");
        let listed: Value = serde_json::from_str(&body).unwrap();
        let items = array_key
            .map_or(&listed, |key| &listed[key])
            .as_array()
            .unwrap();
        assert!(
            items.iter().any(|item| item["id"] == created["id"]),
            "shared guest must retain browser history"
        );
    }
    let guest = edgequake_api::services::identity_storage::shared_guest_user_id(tenant);
    let users: Vec<Uuid> =
        sqlx::query_scalar("SELECT user_id FROM users WHERE tenant_id=$1 AND username='guest'")
            .bind(tenant)
            .fetch_all(&server.pool)
            .await
            .unwrap();
    assert_eq!(users, vec![guest], "one guest per tenant, no browser rows");

    let foreign = Uuid::new_v4();
    let foreign_workspace = Uuid::new_v4();
    http_harness::seed_scope(
        &server.pool,
        foreign,
        foreign_workspace,
        "anonymous-foreign",
    )
    .await;
    let (status, body) = request(
        &server,
        reqwest::Method::GET,
        "/api/v1/conversations",
        foreign,
        foreign_workspace,
        reader,
        None,
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "{body}");
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["items"],
        json!([])
    );
    for path in [
        "/api/v1/chat/completions",
        "/api/v1/chat/completions/stream",
    ] {
        let started = std::time::Instant::now();
        let (status, body) = request(&server, reqwest::Method::POST, path, tenant, workspace, writer,
            Some(json!({"message":"Say hello", "mode":"naive", "provider":"mock", "model":"mock-model"}))).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{path}: {body}");
        assert!(
            !body.contains("Tenant does not own") && !body.contains("\"type\":\"error\""),
            "{body}"
        );
        if path.ends_with("stream") {
            assert!(body.contains("\"type\":\"done\""), "{body}");
        } else {
            assert!(serde_json::from_str::<Value>(&body).unwrap()["conversation_id"].is_string());
        }
        eprintln!("GUEST_CHAT_PASS {path} {}ms", started.elapsed().as_millis());
    }
    let wrong_scope = request(
        &server,
        reqwest::Method::POST,
        "/api/v1/conversations",
        tenant,
        foreign_workspace,
        writer,
        Some(json!({"title":"forged"})),
    )
    .await;
    assert!(
        !wrong_scope.0.is_success(),
        "foreign workspace writes must remain denied"
    );
    eprintln!("POSTGRES_ANONYMOUS_GUEST_PASS");
}
