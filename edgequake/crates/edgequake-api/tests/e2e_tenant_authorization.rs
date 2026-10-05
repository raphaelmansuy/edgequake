//! Real TCP HTTP, persisted identities, memberships, and selected P0 providers.
#![cfg(feature = "postgres")]
mod common;
#[path = "common/tenant_security.rs"]
mod tenant_security;

use common::provider_access::{harness, http_harness};
use edgequake_auth::Role;
use reqwest::{Method, StatusCode};
use serde_json::json;
use tenant_security::{request, seed_user, token};
use uuid::Uuid;

#[tokio::test]
async fn real_http_principal_membership_role_and_scope_isolation() {
    let Some(url) = harness::certification_database_url().unwrap() else {
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    std::env::set_var("EDGEQUAKE_MIGRATE_CLI", "1");
    edgequake_api::state::migration_bootstrap::run_postgres_expandable_migrations(&pool)
        .await
        .unwrap();
    std::env::remove_var("EDGEQUAKE_MIGRATE_CLI");
    let server = http_harness::boot(&url).await;
    let tenant = Uuid::new_v4();
    let foreign = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    for (t, w, name) in [
        (tenant, a, "auth-a"),
        (tenant, b, "auth-b"),
        (foreign, c, "auth-c"),
    ] {
        http_harness::seed_scope(&server.pool, t, w, name).await;
    }
    let own = http_harness::commit_and_drain(
        &server.pool,
        &server.committer(),
        tenant,
        a,
        "AUTH_OWN_SENTINEL",
        "AUTH_OWN_NODE",
        true,
    )
    .await;
    let other = http_harness::commit_and_drain(
        &server.pool,
        &server.committer(),
        foreign,
        c,
        "AUTH_FOREIGN_SENTINEL",
        "AUTH_FOREIGN_NODE",
        true,
    )
    .await;
    let user = seed_user(&server, "user", tenant, a, "member").await;
    let scoped = token(&server, user, Role::User, Some((tenant, a)));
    let unscoped = token(&server, user, Role::User, None);
    let path = format!("/api/v1/documents/{}", own.document_id);
    let (status, body) = request(&server, &scoped, Method::GET, &path, None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("AUTH_OWN_SENTINEL"));
    assert_eq!(
        request(
            &server,
            &scoped,
            Method::GET,
            &path,
            Some((
                tenant.to_string().to_uppercase(),
                a.to_string().to_uppercase()
            )),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        tenant_security::websocket_status(&server, &scoped, None).await,
        StatusCode::SWITCHING_PROTOCOLS
    );
    assert_eq!(
        tenant_security::websocket_status(&server, &unscoped, None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        tenant_security::websocket_status(&server, &scoped, Some((foreign, c))).await,
        StatusCode::UNAUTHORIZED
    );
    let mut samples = Vec::new();
    for i in 0..26 {
        let start = std::time::Instant::now();
        let (status, body) = request(&server, &scoped, Method::GET, &path, None, None).await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.contains("AUTH_OWN_SENTINEL"));
        if i >= 5 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(f64::total_cmp);
    let report = json!({"samples":21,"warmup":5,"p50_ms":samples[10],"p95_ms":samples[19],"samples_ms":samples,"limits":"local real TCP document read, JWT verification, durable principal and membership checks; not a production SLA"});
    if let Ok(path) = std::env::var("EQ_TENANT_HTTP_REPORT") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
    for headers in [
        Some((tenant.to_string(), b.to_string())),
        Some((foreign.to_string(), c.to_string())),
    ] {
        assert_eq!(
            request(&server, &scoped, Method::GET, &path, headers, None)
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        request(&server, &unscoped, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &server,
            &unscoped,
            Method::GET,
            &path,
            Some((tenant.to_string(), "malformed".into())),
            None
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        request(
            &server,
            &unscoped,
            Method::GET,
            &path,
            Some((tenant.to_string(), a.to_string())),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    let (foreign_status, body) = request(
        &server,
        &scoped,
        Method::GET,
        &format!("/api/v1/documents/{}", other.document_id),
        None,
        None,
    )
    .await;
    assert_eq!(foreign_status, StatusCode::NOT_FOUND, "{body}");
    assert!(!body.contains("AUTH_FOREIGN_SENTINEL"));
    assert_eq!(
        request(
            &server,
            &scoped,
            Method::GET,
            &format!("/api/v1/workspaces/{b}"),
            None,
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (status, body) = request(
        &server,
        &scoped,
        Method::GET,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let page: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(page["total"], 1);
    assert_eq!(page["items"][0]["id"], a.to_string());
    assert_eq!(
        request(
            &server,
            &scoped,
            Method::GET,
            &format!("/api/v1/tenants/{foreign}/workspaces"),
            None,
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &server,
            &scoped,
            Method::POST,
            &format!("/api/v1/tenants/{foreign}/workspaces"),
            None,
            Some(json!({"name":"forged"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status,body)=request(&server,&scoped,Method::POST,"/api/v1/graph/entities",None,Some(json!({"entity_name":"Authorized Member Fixture","entity_type":"TEST","description":"authorized write","source_id":"manual_entry"}))).await;
    assert!(
        status.is_success(),
        "Member content write must succeed: {status} {body}"
    );
    let workspace_admin = seed_user(&server, "user", tenant, a, "admin").await;
    let workspace_admin_token = token(&server, workspace_admin, Role::User, Some((tenant, a)));
    assert!(server
        .state
        .workspace_service
        .get_user_role(workspace_admin, tenant)
        .await
        .unwrap()
        .is_none());
    assert_eq!(
        request(
            &server,
            &workspace_admin_token,
            Method::POST,
            &format!("/api/v1/tenants/{tenant}/workspaces"),
            None,
            Some(json!({"name":"workspace-admin-cannot-create"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    sqlx::query("UPDATE memberships SET workspace_id=NULL WHERE user_id=$1")
        .bind(workspace_admin)
        .execute(&server.pool)
        .await
        .unwrap();
    assert!(server
        .state
        .workspace_service
        .check_workspace_access(workspace_admin, b)
        .await
        .unwrap());
    let (status, body) = request(
        &server,
        &workspace_admin_token,
        Method::POST,
        &format!("/api/v1/tenants/{tenant}/workspaces"),
        None,
        Some(json!({"name":format!("tenant-admin-{workspace_admin}")})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    use edgequake_storage::contracts::{AccessScope, TenantId, WorkspaceId};
    let identity = server.state.operational_stores.identity.as_ref().unwrap();
    assert!(identity
        .membership_active(
            &AccessScope::new(TenantId::new(tenant), WorkspaceId::new(b)),
            workspace_admin
        )
        .await
        .unwrap());
    assert!(!identity
        .membership_active(
            &AccessScope::new(TenantId::new(foreign), WorkspaceId::new(b)),
            workspace_admin
        )
        .await
        .unwrap());
    // A token with a stale elevated role cannot retain privileges after a role reduction.
    let admin_claim = token(&server, user, Role::Admin, Some((tenant, a)));
    assert_eq!(
        request(
            &server,
            &admin_claim,
            Method::GET,
            "/api/v1/admin/config/defaults",
            None,
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let readonly = seed_user(&server, "user", tenant, a, "readonly").await;
    let ro = token(&server, readonly, Role::User, Some((tenant, a)));
    assert_eq!(
        request(&server, &ro, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&server, &ro, Method::DELETE, &path, None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &server,
            &ro,
            Method::POST,
            "/api/v1/graph/entities",
            None,
            Some(json!({"name":"forged","entity_type":"TEST"}))
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        server
            .client
            .post(format!("{}/api/chat", server.base))
            .json(&json!({"model":"mock","messages":[]}))
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );

    let master = server.state.auth.config.master_api_key.as_deref().unwrap();
    for method in [Method::GET, Method::POST] {
        assert_eq!(
            request(
                &server,
                master,
                method,
                "/api/v1/api-keys",
                None,
                Some(json!({}))
            )
            .await
            .0,
            StatusCode::FORBIDDEN,
            "ownerless break-glass keys cannot manage user keys"
        );
    }
    // Default user keys grant read/query only, including on REST.
    let (status, body) = request(
        &server,
        &scoped,
        Method::POST,
        "/api/v1/api-keys",
        None,
        Some(json!({"name":"tenancy-read-only"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    let key: serde_json::Value = serde_json::from_str(&body).unwrap();
    let key = key["api_key"].as_str().unwrap();
    assert_eq!(
        request(
            &server,
            key,
            Method::GET,
            &path,
            Some((tenant.to_string(), a.to_string())),
            None
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &server,
            key,
            Method::DELETE,
            &path,
            Some((tenant.to_string(), a.to_string())),
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let tools = json!({"jsonrpc":"2.0","id":1,"method":"tools/list"});
    let (status, body) = request(
        &server,
        key,
        Method::POST,
        "/mcp",
        Some((tenant.to_string(), a.to_string())),
        Some(tools.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("eq_search"));
    assert_eq!(
        request(
            &server,
            key,
            Method::POST,
            "/mcp",
            Some((foreign.to_string(), c.to_string())),
            Some(tools)
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let query = json!({"query":"fixture","mode":"naive","context_only":true,"enable_rerank":false,"llm_provider":"mock","llm_model":"mock-model","hl_keywords":["fixture"],"ll_keywords":["fixture"]});
    let (status, body) = request(
        &server,
        key,
        Method::POST,
        "/api/v1/query",
        Some((
            tenant.to_string().to_uppercase(),
            a.to_string().to_uppercase(),
        )),
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("AUTH_OWN_SENTINEL"));
    assert!(!body.contains("AUTH_FOREIGN_SENTINEL"));

    for (endpoint, body) in [
        (
            "/api/chat",
            json!({"model":"edgequake","stream":false,"messages":[{"role":"user","content":"/context fixture"}]}),
        ),
        (
            "/api/generate",
            json!({"model":"edgequake","stream":false,"prompt":"/context fixture"}),
        ),
        (
            "/api/chat",
            json!({"model":"edgequake","stream":true,"messages":[{"role":"user","content":"/context fixture"}]}),
        ),
        (
            "/api/generate",
            json!({"model":"edgequake","stream":true,"prompt":"/context fixture"}),
        ),
    ] {
        let (status, body) = request(
            &server,
            key,
            Method::POST,
            endpoint,
            Some((tenant.to_string(), a.to_string())),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{endpoint}: {body}");
        assert!(
            body.contains("AUTH_OWN_SENTINEL"),
            "{endpoint} must expose its own authorized context: {body}"
        );
        assert!(!body.contains("AUTH_FOREIGN_SENTINEL"));
    }
    sqlx::query("UPDATE workspaces SET is_active=false WHERE workspace_id=$1")
        .bind(a)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        request(&server, &scoped, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(!identity
        .membership_active(
            &AccessScope::new(TenantId::new(tenant), WorkspaceId::new(a)),
            workspace_admin
        )
        .await
        .unwrap());
    sqlx::query("UPDATE workspaces SET is_active=true WHERE workspace_id=$1")
        .bind(a)
        .execute(&server.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tenants SET is_active=false WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        request(&server, &scoped, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &server,
            &scoped,
            Method::GET,
            &format!("/api/v1/tenants/{tenant}"),
            None,
            None
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    assert!(!identity
        .membership_active(
            &AccessScope::new(TenantId::new(tenant), WorkspaceId::new(a)),
            workspace_admin
        )
        .await
        .unwrap());
    sqlx::query("UPDATE tenants SET is_active=true WHERE tenant_id=$1")
        .bind(tenant)
        .execute(&server.pool)
        .await
        .unwrap();
    sqlx::query("UPDATE memberships SET is_active=false WHERE user_id=$1")
        .bind(user)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        request(&server, &scoped, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::FORBIDDEN
    );
    assert!(!server
        .state
        .workspace_service
        .check_tenant_access(user, tenant)
        .await
        .unwrap());
    assert!(server
        .state
        .workspace_service
        .get_user_role(user, tenant)
        .await
        .unwrap()
        .is_none());
    assert!(!server
        .state
        .workspace_service
        .check_workspace_access(user, a)
        .await
        .unwrap());
    sqlx::query("UPDATE users SET is_active=false WHERE user_id=$1")
        .bind(user)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_eq!(
        request(&server, &scoped, Method::GET, &path, None, None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &server,
            key,
            Method::GET,
            &path,
            Some((tenant.to_string(), a.to_string())),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        tenant_security::websocket_status(&server, &scoped, None).await,
        StatusCode::UNAUTHORIZED
    );
    let (status, body) = server.get_text(&path, tenant, a).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(body.contains("AUTH_OWN_SENTINEL"));
}
