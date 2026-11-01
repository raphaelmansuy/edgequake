//! SPEC-181 — tenancy: ownership, quota, slugs, purge (PostgreSQL, real HTTP).
//!
//! These are the gates from `specs/181-admin/14-e2e-test-matrix.md` whose names start with
//! `R-`. They need a PostgreSQL database; set `EDGEQUAKE_SPEC181_PG_URL` or reuse the
//! certification URL the other PostgreSQL suites use. Without one the module is a no-op, which
//! matches how `e2e_tenant_authorization.rs` behaves.
#![cfg(feature = "postgres")]

mod common;

use common::provider_access::harness;
use common::provider_access::http_harness::{self, LiveServer};
use edgequake_auth::Role;
use reqwest::{Method, StatusCode};
// These tests share one database and each boots its own server; running them concurrently makes
// the quota race and the owner trigger interfere through connection-pool pressure. The project
// already uses `serial_test` for the same reason in the multimodal contract suites.
use serde_json::{json, Value};
use serial_test::serial;
use uuid::Uuid;

async fn boot() -> Option<LiveServer> {
    let url = harness::certification_database_url().unwrap()?;
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .unwrap();
    std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    Some(http_harness::boot(&url).await)
}

/// A platform admin who exists in the `users` table and holds the `Admin` role.
async fn platform_admin(server: &LiveServer) -> (Uuid, String) {
    let user = Uuid::new_v4();
    let name = format!("spec181-admin-{user}");
    let hash = server
        .state
        .auth
        .password
        .hash_password("Spec181Test123!")
        .unwrap();
    sqlx::query(
        "INSERT INTO users(user_id,tenant_id,username,email,password_hash,role,is_active) \
         VALUES($1,$2,$3,$4,$5,'admin',true)",
    )
    .bind(user)
    .bind(edgequake_api::middleware::default_tenant_uuid())
    .bind(&name)
    .bind(format!("{name}@example.test"))
    .bind(hash)
    .execute(&server.pool)
    .await
    .unwrap();

    let mut claims = edgequake_auth::Claims::new(user, Role::Admin, 900);
    claims = claims.with_tenant_id(edgequake_api::middleware::default_tenant_uuid().to_string());
    let token = server
        .state
        .auth
        .jwt
        .generate_token_with_claims(claims)
        .unwrap();
    (user, token)
}

async fn call(
    server: &LiveServer,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = server
        .client
        .request(method, format!("{}{path}", server.base))
        .bearer_auth(token);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let value = serde_json::from_str(&text).unwrap_or(Value::Null);
    (status, value)
}

/// Like [`call`], but sends the scope headers `bind_request` checks.
async fn call_scoped(
    server: &LiveServer,
    token: &str,
    method: Method,
    path: &str,
    body: Option<Value>,
    tenant: Uuid,
    workspace: Uuid,
) -> (StatusCode, Value) {
    let mut request = server
        .client
        .request(method, format!("{}{path}", server.base))
        .bearer_auth(token)
        .header("X-Tenant-ID", tenant.to_string())
        .header("X-Workspace-ID", workspace.to_string());
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

fn unique_slug(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4().simple())
}

// ===========================================================================
// R-create-rollback / R-create-owner: the F-181-01 regression gates
// ===========================================================================

#[tokio::test]
#[serial]
async fn create_tenant_grants_owner_and_default_workspace() {
    let Some(server) = boot().await else { return };
    let (admin_id, token) = platform_admin(&server).await;
    let slug = unique_slug("spec181-owner");

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Owner Gate Ltd", "slug": slug, "plan": "free" })),
    )
    .await;

    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant_id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    assert_eq!(body["plan"], "free");
    // SPEC-181: the quota comes from the plan catalog, not a hard-coded 5.
    assert_eq!(body["max_workspaces"], 10);
    assert_eq!(body["max_users"], 3);

    // The creator owns it — this is the row the pre-181 handler never wrote.
    let owner_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM memberships \
         WHERE tenant_id=$1 AND user_id=$2 AND workspace_id IS NULL AND role='owner' AND is_active",
    )
    .bind(tenant_id)
    .bind(admin_id)
    .fetch_one(&server.pool)
    .await
    .unwrap();
    assert_eq!(owner_rows, 1, "creator must hold a tenant-wide owner grant");

    // A default workspace exists, and the creator administers it.
    let workspaces = call(
        &server,
        &token,
        Method::GET,
        &format!("/api/v1/tenants/{tenant_id}/workspaces"),
        None,
    )
    .await;
    assert_eq!(workspaces.0, StatusCode::OK, "body={:?}", workspaces.1);
    assert_eq!(
        workspaces.1["total"], 1,
        "exactly one default workspace: {:?}",
        workspaces.1
    );

    // The organization is now governable without a platform admin: the members directory
    // answers for the owner instead of 403.
    let members = call(
        &server,
        &token,
        Method::GET,
        &format!("/api/v1/tenants/{tenant_id}/members"),
        None,
    )
    .await;
    assert_eq!(
        members.0,
        StatusCode::OK,
        "owner can read the directory: {:?}",
        members.1
    );

    // And `me/access` agrees, which is what the UI renders from.
    let access = call(
        &server,
        &token,
        Method::GET,
        &format!("/api/v1/me/access?tenant_id={tenant_id}"),
        None,
    )
    .await;
    assert_eq!(access.0, StatusCode::OK);
    assert_eq!(access.1["organization"]["role"], "owner");
    assert_eq!(access.1["can_create"]["workspace"], true);
    assert_eq!(access.1["can_delete"]["organization"], true);

    cleanup(&server, tenant_id).await;
}

#[tokio::test]
#[serial]
async fn create_tenant_without_a_plan_still_gets_the_plan_quota() {
    // F-181-06: `Tenant::new` hard-coded 5/10 while `TenantPlan::Free` said 10/3.
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "No Plan Ltd", "slug": unique_slug("spec181-noplan") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    assert_eq!(body["max_workspaces"], 10);
    assert_eq!(body["max_users"], 3);
    cleanup(
        &server,
        Uuid::parse_str(body["id"].as_str().unwrap()).unwrap(),
    )
    .await;
}

#[tokio::test]
#[serial]
async fn create_tenant_twice_conflicts_with_the_existing_slug() {
    // LAW-181-8: create is not upsert.
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;
    let slug = unique_slug("spec181-dup");

    let first = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Dup A", "slug": slug })),
    )
    .await;
    assert_eq!(first.0, StatusCode::CREATED, "body={:?}", first.1);
    let id = Uuid::parse_str(first.1["id"].as_str().unwrap()).unwrap();

    let second = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Dup B", "slug": slug })),
    )
    .await;
    assert_eq!(
        second.0,
        StatusCode::CONFLICT,
        "a second create must not silently return the first: {:?}",
        second.1
    );
    let reason = second.1.to_string();
    assert!(
        reason.contains("tenant_slug_taken"),
        "reason must be machine-readable: {reason}"
    );

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM tenants WHERE slug=$1")
        .bind(&slug)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(count, 1, "no second row");
    cleanup(&server, id).await;
}

// ===========================================================================
// R-quota-*
// ===========================================================================

#[tokio::test]
#[serial]
async fn concurrent_workspace_creates_respect_the_quota() {
    // LAW-181-6: the pre-181 check was COUNT(*) then INSERT with no lock, so two concurrent
    // creates at limit-1 both succeeded. Migration 226 serializes on the tenant row.
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Quota Ltd", "slug": unique_slug("spec181-quota") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    // Free plan allows 10; one default workspace exists, so 9 free slots. Pin the limit to 2 to
    // make the race cheap and deterministic.
    sqlx::query("UPDATE tenants SET max_workspaces=2 WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await
        .unwrap();
    // The organization has 1 workspace, so exactly one more fits.

    let mk = |suffix: &'static str| {
        let server = &server;
        let token = token.clone();
        async move {
            call(
                server,
                &token,
                Method::POST,
                &format!("/api/v1/tenants/{tenant}/workspaces"),
                Some(json!({ "name": format!("Race {suffix}"), "slug": format!("race-{suffix}") })),
            )
            .await
        }
    };

    let (a, b) = tokio::join!(mk("a"), mk("b"));
    let created = [a.0, b.0].iter().filter(|s| s.is_success()).count();
    let rejected = [a.0, b.0].iter().filter(|s| s.as_u16() == 409).count();
    assert_eq!(
        created, 1,
        "exactly one create may win at the limit: a={:?} b={:?}",
        a.0, b.0
    );
    assert_eq!(
        rejected, 1,
        "the loser gets a 409, not a 500 or a silent success: a={:?} {:?} b={:?} {:?}",
        a.0, a.1, b.0, b.1
    );

    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workspaces WHERE tenant_id=$1 AND archived_at IS NULL",
    )
    .bind(tenant)
    .fetch_one(&server.pool)
    .await
    .unwrap();
    assert_eq!(live, 2, "the plan limit holds at the database");

    cleanup(&server, tenant).await;
}

// ===========================================================================
// R-tenant-list-scoped
// ===========================================================================

#[tokio::test]
#[serial]
async fn a_member_sees_only_the_organizations_they_belong_to() {
    let Some(server) = boot().await else { return };
    let (admin_id, admin_token) = platform_admin(&server).await;

    let mut created = Vec::new();
    for n in ["list-a", "list-b"] {
        let (status, body) = call(
            &server,
            &admin_token,
            Method::POST,
            "/api/v1/tenants",
            Some(json!({ "name": format!("List {n}"), "slug": unique_slug(n) })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "body={body}");
        created.push(Uuid::parse_str(body["id"].as_str().unwrap()).unwrap());
    }

    // A plain user who is a member of only the first organization.
    let user = Uuid::new_v4();
    let name = format!("spec181-member-{user}");
    let hash = server
        .state
        .auth
        .password
        .hash_password("Spec181Test123!")
        .unwrap();
    sqlx::query(
        "INSERT INTO users(user_id,tenant_id,username,email,password_hash,role,is_active) \
         VALUES($1,$2,$3,$4,$5,'user',true)",
    )
    .bind(user)
    .bind(edgequake_api::middleware::default_tenant_uuid())
    .bind(&name)
    .bind(format!("{name}@example.test"))
    .bind(hash)
    .execute(&server.pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO memberships(membership_id,tenant_id,workspace_id,user_id,role,is_active) \
         VALUES($1,$2,NULL,$3,'member',true)",
    )
    .bind(Uuid::new_v4())
    .bind(created[0])
    .bind(user)
    .execute(&server.pool)
    .await
    .unwrap();

    let token = {
        let claims = edgequake_auth::Claims::new(user, Role::User, 900)
            .with_tenant_id(created[0].to_string());
        server
            .state
            .auth
            .jwt
            .generate_token_with_claims(claims)
            .unwrap()
    };

    let (status, body) = call(&server, &token, Method::GET, "/api/v1/tenants", None).await;
    assert_eq!(status, StatusCode::OK);
    let names: Vec<&str> = body["items"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|r| r["id"].as_str()).collect())
        .unwrap_or_default();
    assert!(
        names.contains(&created[0].to_string().as_str()),
        "member sees their own organization"
    );
    assert!(
        !names.contains(&created[1].to_string().as_str()),
        "member must not see an organization they do not belong to: {names:?}"
    );

    sqlx::query("DELETE FROM memberships WHERE user_id=$1")
        .bind(user)
        .execute(&server.pool)
        .await
        .ok();
    for t in created {
        cleanup(&server, t).await;
    }
    sqlx::query("DELETE FROM users WHERE user_id=$1")
        .bind(admin_id)
        .execute(&server.pool)
        .await
        .ok();
}

// ===========================================================================
// R-last-owner-trigger — the database agrees with the application guard
// ===========================================================================

#[tokio::test]
#[serial]
async fn the_database_refuses_to_orphan_an_organization() {
    let Some(server) = boot().await else { return };
    let (admin_id, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Guard Ltd", "slug": unique_slug("spec181-guard") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    // Bypass the API entirely: a script deleting the last owner must not leave an orphan.
    let mut tx = server.pool.begin().await.unwrap();
    sqlx::query(
        "DELETE FROM memberships WHERE tenant_id=$1 AND workspace_id IS NULL AND role='owner'",
    )
    .bind(tenant)
    .execute(&mut *tx)
    .await
    .unwrap();
    let commit = tx.commit().await;
    assert!(
        commit.is_err(),
        "the deferred trigger must reject an ownerless live organization"
    );

    cleanup(&server, tenant).await;
    sqlx::query("DELETE FROM users WHERE user_id=$1")
        .bind(admin_id)
        .execute(&server.pool)
        .await
        .ok();
}

// ===========================================================================
// helper
// ===========================================================================

/// Archive first (the owner trigger ignores archived tenants), then remove the rows.
async fn cleanup(server: &LiveServer, tenant: Uuid) {
    let _ = sqlx::query("UPDATE tenants SET archived_at=NOW(), is_active=false WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await;
    let _ = sqlx::query("DELETE FROM memberships WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await;
    let _ = sqlx::query("DELETE FROM workspaces WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await;
    let _ = sqlx::query("DELETE FROM tenants WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await;
}

// ===========================================================================
// R-purge-*: the critical plane-coverage gates (F-181-04)
// ===========================================================================

/// Assert that nothing belonging to `tenant` survives anywhere.
async fn orphan_sweep(server: &LiveServer, tenant: Uuid) -> Vec<String> {
    let mut left = Vec::new();
    for table in ["tenants", "workspaces", "memberships"] {
        let col = match table {
            "tenants" => "tenant_id",
            _ => "tenant_id",
        };
        let n: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE {col}=$1"))
            .bind(tenant)
            .fetch_one(&server.pool)
            .await
            .unwrap();
        if n > 0 {
            left.push(format!("{table}={n}"));
        }
    }
    // Orphan vector tables created per workspace (`eq_..._ws_{uuid}_vectors`).
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename FROM pg_tables WHERE schemaname='public' AND tablename LIKE 'eq\\_%\\_vectors'",
    )
    .fetch_all(&server.pool)
    .await
    .unwrap();
    for t in tables {
        if t.contains(&tenant.to_string()) {
            left.push(format!("vector_table:{t}"));
        }
    }
    left
}

#[tokio::test]
#[serial]
async fn deleting_an_organization_purges_every_plane() {
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Purge Ltd", "slug": unique_slug("spec181-purge") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    // A second workspace, archived, must be purged too (EC-181-35).
    let (ws_status, ws_body) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        Some(json!({ "name": "Archived One", "slug": "archived-one" })),
    )
    .await;
    assert_eq!(ws_status, StatusCode::CREATED, "body={ws_body}");
    let ws = Uuid::parse_str(ws_body["id"].as_str().unwrap()).unwrap();
    sqlx::query("UPDATE workspaces SET archived_at=NOW() WHERE workspace_id=$1")
        .bind(ws)
        .execute(&server.pool)
        .await
        .unwrap();

    // Preview states what will be destroyed.
    let (preview_status, preview) = call(
        &server,
        &token,
        Method::GET,
        &format!("/api/v1/tenants/{tenant}/purge-preview"),
        None,
    )
    .await;
    assert_eq!(preview_status, StatusCode::OK, "body={preview}");
    assert_eq!(
        preview["workspaces"], 2,
        "both live and archived workspaces count"
    );

    let (status, receipt) = call(
        &server,
        &token,
        Method::DELETE,
        &format!("/api/v1/tenants/{tenant}"),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED, "body={receipt}");
    assert_eq!(receipt["status"], "done", "receipt={receipt}");
    assert_eq!(receipt["planes_total"], 9);
    assert_eq!(receipt["planes_done"], 9);

    let leftovers = orphan_sweep(&server, tenant).await;
    assert!(
        leftovers.is_empty(),
        "organization delete must leave nothing behind: {leftovers:?}"
    );
}

#[tokio::test]
#[serial]
async fn deleting_a_workspace_purges_its_planes_and_returns_a_receipt() {
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;
    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Ws Purge Ltd", "slug": unique_slug("spec181-wspurge") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    let (ws_status, ws_body) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        Some(json!({ "name": "Doomed", "slug": "doomed" })),
    )
    .await;
    assert_eq!(ws_status, StatusCode::CREATED, "body={ws_body}");
    let ws = Uuid::parse_str(ws_body["id"].as_str().unwrap()).unwrap();

    // `bind_request` requires the selected workspace header to match the path target, so the
    // caller deletes a workspace in the scope they are acting in. That is the real UI contract.
    let (del_status, receipt) = call_scoped(
        &server,
        &token,
        Method::DELETE,
        &format!("/api/v1/workspaces/{ws}"),
        None,
        tenant,
        ws,
    )
    .await;
    assert_eq!(del_status, StatusCode::ACCEPTED, "body={receipt}");
    assert_eq!(receipt["scope_kind"], "workspace");
    assert_eq!(receipt["status"], "done");

    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM workspaces WHERE workspace_id=$1")
        .bind(ws)
        .fetch_one(&server.pool)
        .await
        .unwrap();
    assert_eq!(rows, 0, "the workspace row is gone");

    cleanup(&server, tenant).await;
}

// ===========================================================================
// SPEC-181: grants + archive (the write paths the pack promises)
// ===========================================================================

#[tokio::test]
#[serial]
async fn archiving_frees_a_quota_slot_and_restore_respects_the_limit() {
    let Some(server) = boot().await else { return };
    let (_, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Archive Ltd", "slug": unique_slug("spec181-arch") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();

    // Pin the limit to the one default workspace, so the next create must be refused.
    sqlx::query("UPDATE tenants SET max_workspaces=1 WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await
        .unwrap();

    let (blocked, _) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        Some(json!({ "name": "Nope", "slug": "nope" })),
    )
    .await;
    assert_eq!(blocked, StatusCode::CONFLICT, "at the limit");

    let ws: Uuid =
        sqlx::query_scalar("SELECT workspace_id FROM workspaces WHERE tenant_id=$1 LIMIT 1")
            .bind(tenant)
            .fetch_one(&server.pool)
            .await
            .unwrap();

    // Archive: the slot frees and the row leaves the live list (EC-181-16).
    let (arch, archived) = call_scoped(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/workspaces/{ws}/archive"),
        Some(json!({ "reason": "quota" })),
        tenant,
        ws,
    )
    .await;
    assert_eq!(arch, StatusCode::OK, "body={archived}");
    assert!(
        archived["archived_at"].is_string(),
        "archived_at set: {archived}"
    );

    let live: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workspaces WHERE tenant_id=$1 AND archived_at IS NULL",
    )
    .bind(tenant)
    .fetch_one(&server.pool)
    .await
    .unwrap();
    assert_eq!(live, 0, "archiving frees the slot");

    // The freed slot is usable again.
    let (created, created_body) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        Some(json!({ "name": "After archive", "slug": "after-archive" })),
    )
    .await;
    assert_eq!(created, StatusCode::CREATED, "body={created_body}");

    // Restoring the archived one must now fail: the plan is full again.
    let (restore_status, restore_body) = call_scoped(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/workspaces/{ws}/restore"),
        None,
        tenant,
        ws,
    )
    .await;
    assert_eq!(
        restore_status,
        StatusCode::CONFLICT,
        "restore consumes a slot like a create: {restore_body}"
    );

    cleanup(&server, tenant).await;
}

#[tokio::test]
#[serial]
async fn a_workspace_grant_is_writable_and_scoped() {
    let Some(server) = boot().await else { return };
    let (admin_id, token) = platform_admin(&server).await;

    let (status, body) = call(
        &server,
        &token,
        Method::POST,
        "/api/v1/tenants",
        Some(json!({ "name": "Grants Ltd", "slug": unique_slug("spec181-grants") })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "body={body}");
    let tenant = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
    let ws: Uuid =
        sqlx::query_scalar("SELECT workspace_id FROM workspaces WHERE tenant_id=$1 LIMIT 1")
            .bind(tenant)
            .fetch_one(&server.pool)
            .await
            .unwrap();

    // A second person who holds no grant yet.
    let user = Uuid::new_v4();
    let name = format!("spec181-grantee-{user}");
    sqlx::query(
        "INSERT INTO users(user_id,tenant_id,username,email,password_hash,role,is_active) \
         VALUES($1,$2,$3,$4,NULL,'user',true)",
    )
    .bind(user)
    .bind(edgequake_api::middleware::default_tenant_uuid())
    .bind(&name)
    .bind(format!("{name}@example.test"))
    .execute(&server.pool)
    .await
    .unwrap();

    // F-181-05: this row could not be created through any route before SPEC-181.
    let (created, grant) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/grants"),
        Some(json!({ "user_id": user, "workspace_id": ws, "role": "admin" })),
    )
    .await;
    assert_eq!(created, StatusCode::CREATED, "body={grant}");
    assert_eq!(grant["scope"], "workspace");
    assert_eq!(grant["role"], "admin");
    assert_eq!(grant["workspace_id"], ws.to_string());

    // The same grant twice is a conflict the UI can offer to change instead.
    let (dup, dup_body) = call(
        &server,
        &token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/grants"),
        Some(json!({ "user_id": user, "workspace_id": ws, "role": "admin" })),
    )
    .await;
    assert_eq!(dup, StatusCode::CONFLICT, "body={dup_body}");

    // A workspace from another tenant must be refused (EC-181-24).
    let foreign = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tenants(tenant_id,name,slug,is_active,plan,max_workspaces,max_users,created_at,updated_at) \
         VALUES($1,'Foreign','spec181-foreign-x',true,'free',10,3,now(),now())",
    )
    .bind(foreign)
    .execute(&server.pool)
    .await
    .ok();
    sqlx::query(
        "INSERT INTO workspaces(workspace_id,tenant_id,name,slug,is_active,created_at,updated_at) \
         VALUES($1,$2,'Foreign WS','foreign-ws',true,now(),now())",
    )
    .bind(Uuid::new_v4())
    .bind(foreign)
    .execute(&server.pool)
    .await
    .ok();
    let foreign_ws: Option<Uuid> =
        sqlx::query_scalar("SELECT workspace_id FROM workspaces WHERE tenant_id=$1 LIMIT 1")
            .bind(foreign)
            .fetch_optional(&server.pool)
            .await
            .unwrap_or(None);
    if let Some(foreign_ws) = foreign_ws {
        let (bad, bad_body) = call(
            &server,
            &token,
            Method::POST,
            &format!("/api/v1/tenants/{tenant}/grants"),
            Some(json!({ "user_id": user, "workspace_id": foreign_ws, "role": "member" })),
        )
        .await;
        assert_eq!(bad, StatusCode::BAD_REQUEST, "body={bad_body}");
        assert!(
            bad_body.to_string().contains("grant_scope_mismatch"),
            "reason must be machine-readable: {bad_body}"
        );
    }

    // Listing returns the grant with its workspace name resolved.
    let (listed, rows) = call(
        &server,
        &token,
        Method::GET,
        &format!("/api/v1/tenants/{tenant}/grants?user_id={user}"),
        None,
    )
    .await;
    assert_eq!(listed, StatusCode::OK);
    let arr = rows.as_array().cloned().unwrap_or_default();
    assert_eq!(arr.len(), 1, "one grant: {rows}");
    assert!(
        arr[0]["workspace_name"].is_string(),
        "name resolved: {rows}"
    );

    // Removing it returns 204 and the row is deactivated.
    let membership_id = arr[0]["membership_id"].as_str().unwrap();
    let (deleted, _) = call(
        &server,
        &token,
        Method::DELETE,
        &format!("/api/v1/tenants/{tenant}/grants/{membership_id}"),
        None,
    )
    .await;
    assert_eq!(deleted, StatusCode::NO_CONTENT);

    sqlx::query("DELETE FROM users WHERE user_id=$1")
        .bind(user)
        .execute(&server.pool)
        .await
        .ok();
    sqlx::query("UPDATE tenants SET archived_at=NOW(), is_active=false WHERE tenant_id=$1")
        .bind(foreign)
        .execute(&server.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM workspaces WHERE tenant_id=$1")
        .bind(foreign)
        .execute(&server.pool)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE tenant_id=$1")
        .bind(foreign)
        .execute(&server.pool)
        .await
        .ok();
    cleanup(&server, tenant).await;
    sqlx::query("DELETE FROM users WHERE user_id=$1")
        .bind(admin_id)
        .execute(&server.pool)
        .await
        .ok();
}
