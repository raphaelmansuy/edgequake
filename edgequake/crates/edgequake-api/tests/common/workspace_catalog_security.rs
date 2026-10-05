//! Real HTTP catalog visibility parity, including MCP aliases and pagination.

use super::common::provider_access::http_harness::LiveServer;
use super::tenant_security::{request, seed_user, token};
use edgequake_auth::Role;
use reqwest::{Method, StatusCode};
use serde_json::{json, Value};
use uuid::Uuid;

type Scope = (Uuid, Uuid);

async fn issue_key(server: &LiveServer, user: Uuid, scope: Scope) -> String {
    issue_scoped_key(server, user, scope, None).await
}

async fn issue_scoped_key(
    server: &LiveServer,
    user: Uuid,
    scope: Scope,
    scopes: Option<&[&str]>,
) -> String {
    let credential = token(server, user, Role::User, Some(scope));
    let mut payload = json!({"name":format!("catalog-{user}")});
    if let Some(scopes) = scopes {
        payload["scopes"] = json!(scopes);
    }
    let (status, body) = request(
        server,
        &credential,
        Method::POST,
        "/api/v1/api-keys",
        None,
        Some(payload),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{body}");
    serde_json::from_str::<Value>(&body).unwrap()["api_key"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn mcp_page(
    server: &LiveServer,
    credential: &str,
    scope: Scope,
    path: &str,
    args: Value,
    spoofed_user: Option<Uuid>,
) -> Value {
    let mut call = server
        .client
        .post(format!("{}{path}", server.base))
        .bearer_auth(credential)
        .header("X-Tenant-ID", scope.0.to_string())
        .header("X-Workspace-ID", scope.1.to_string())
        .json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"eq_workspace_list","arguments":args}}));
    if let Some(user) = spoofed_user {
        call = call.header("X-User-ID", user.to_string());
    }
    let response = call.send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    let rpc: Value = serde_json::from_str(&body).unwrap();
    assert_ne!(rpc["result"]["isError"], true, "{body}");
    let page = rpc["result"]["structuredContent"].clone();
    assert_eq!(page["ok"], true, "{body}");
    page
}

fn assert_items(page: &Value, expected: &[Uuid], total: usize) {
    assert_eq!(page["total"], total, "{page}");
    let mut ids: Vec<_> = page["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| Uuid::parse_str(item["id"].as_str().unwrap()).unwrap())
        .collect();
    let mut expected = expected.to_vec();
    ids.sort();
    expected.sort();
    assert_eq!(ids, expected, "{page}");
}

async fn assert_transport_parity(
    server: &LiveServer,
    credential: &str,
    scope: Scope,
    expected: &[Uuid],
) {
    for path in ["/mcp", "/api/v1/mcp"] {
        let page = mcp_page(server, credential, scope, path, json!({"limit":100}), None).await;
        assert_items(&page, expected, expected.len());
        assert!(page.get("next_cursor").is_none(), "{page}");
    }
    let (status, body) = request(
        server,
        credential,
        Method::GET,
        &format!("/api/v1/tenants/{}/workspaces?limit=100", scope.0),
        Some((scope.0.to_string(), scope.1.to_string())),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_items(
        &serde_json::from_str(&body).unwrap(),
        expected,
        expected.len(),
    );
}

async fn verify_filtered_pages(server: &LiveServer, key: &str, scope: Scope, sibling: Uuid) {
    assert_transport_parity(server, key, scope, &[scope.1]).await;
    for path in ["/mcp", "/api/v1/mcp"] {
        for cursor in ["1".to_string(), "2".to_string(), usize::MAX.to_string()] {
            let page = mcp_page(
                server,
                key,
                scope,
                path,
                json!({"limit":100,"cursor":cursor}),
                None,
            )
            .await;
            let expected = if cursor == "1" { vec![scope.1] } else { vec![] };
            assert_items(&page, &expected, 1);
            assert!(page.get("next_cursor").is_none(), "{page}");
            assert!(!page.to_string().contains(&sibling.to_string()), "{page}");
        }
    }
    let (status, body) = request(
        server,
        key,
        Method::GET,
        &format!("/api/v1/tenants/{}/workspaces?limit=1&offset=1", scope.0),
        Some((scope.0.to_string(), scope.1.to_string())),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_items(&serde_json::from_str(&body).unwrap(), &[], 1);
}

async fn verify_role_visibility(
    server: &LiveServer,
    member_key: &str,
    scope: Scope,
    sibling: Uuid,
) {
    for (role, tenant_wide) in [("readonly", false), ("admin", false), ("member", true)] {
        let user = seed_user(server, "user", scope.0, scope.1, role).await;
        if tenant_wide {
            sqlx::query("UPDATE memberships SET workspace_id=NULL WHERE user_id=$1")
                .bind(user)
                .execute(&server.pool)
                .await
                .unwrap();
        }
        let key = issue_key(server, user, scope).await;
        let expected = if tenant_wide {
            vec![scope.1, sibling]
        } else {
            vec![scope.1]
        };
        assert_transport_parity(server, &key, scope, &expected).await;
        if tenant_wide {
            verify_two_pages(server, &key, scope).await;
            let page = mcp_page(server, member_key, scope, "/mcp", json!({}), Some(user)).await;
            assert_items(&page, &[scope.1], 1);
            sqlx::query("UPDATE workspaces SET is_active=false WHERE workspace_id=$1")
                .bind(sibling)
                .execute(&server.pool)
                .await
                .unwrap();
            assert_transport_parity(server, &key, scope, &[scope.1]).await;
            sqlx::query("UPDATE workspaces SET is_active=true WHERE workspace_id=$1")
                .bind(sibling)
                .execute(&server.pool)
                .await
                .unwrap();
        }
    }
}

async fn verify_admin_pagination(server: &LiveServer, scope: Scope, sibling: Uuid) {
    let master = server.state.auth.config.master_api_key.as_deref().unwrap();
    assert_transport_parity(server, master, scope, &[scope.1, sibling]).await;
    verify_two_pages(server, master, scope).await;
    let user = seed_user(server, "admin", scope.0, scope.1, "member").await;
    let readonly_key = issue_key(server, user, scope).await;
    assert_transport_parity(server, &readonly_key, scope, &[scope.1]).await;
    let key = issue_scoped_key(server, user, scope, Some(&["*"])).await;
    assert_transport_parity(server, &key, scope, &[scope.1, sibling]).await;
    sqlx::query("UPDATE users SET role='user' WHERE user_id=$1")
        .bind(user)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_transport_parity(server, &key, scope, &[scope.1]).await;
}

async fn verify_two_pages(server: &LiveServer, key: &str, scope: Scope) {
    for path in ["/mcp", "/api/v1/mcp"] {
        let first = mcp_page(server, key, scope, path, json!({"limit":1}), None).await;
        assert_eq!(first["total"], 2);
        assert_eq!(first["items"].as_array().unwrap().len(), 1);
        let second = mcp_page(
            server,
            key,
            scope,
            path,
            json!({"limit":1,"cursor":first["next_cursor"].as_str().unwrap()}),
            None,
        )
        .await;
        assert_eq!(second["total"], 2);
        assert_eq!(second["items"].as_array().unwrap().len(), 1);
        assert_ne!(first["items"][0]["id"], second["items"][0]["id"]);
        assert!(second.get("next_cursor").is_none());
    }
    let mut ids = Vec::new();
    for offset in 0..=2 {
        let (status, body) = request(
            server,
            key,
            Method::GET,
            &format!(
                "/api/v1/tenants/{}/workspaces?limit=1&offset={offset}",
                scope.0
            ),
            Some((scope.0.to_string(), scope.1.to_string())),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let page: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(page["total"], 2);
        assert_eq!(
            page["items"].as_array().unwrap().len(),
            usize::from(offset < 2)
        );
        if offset < 2 {
            ids.push(page["items"][0]["id"].clone());
        }
    }
    assert_ne!(ids[0], ids[1]);
}

pub async fn verify_extreme_admin_cursors(
    server: &LiveServer,
    tenant: Uuid,
    own: Uuid,
    sibling: Uuid,
) {
    let scope = (tenant, own);
    let master = server.state.auth.config.master_api_key.as_deref().unwrap();
    assert_transport_parity(server, master, scope, &[own, sibling]).await;
    for path in ["/mcp", "/api/v1/mcp"] {
        let page = mcp_page(
            server,
            master,
            scope,
            path,
            json!({"limit":100,"cursor":usize::MAX.to_string()}),
            None,
        )
        .await;
        assert_items(&page, &[], 2);
        assert!(page.get("next_cursor").is_none(), "{page}");
    }
    let (status, body) = request(
        server,
        master,
        Method::GET,
        &format!(
            "/api/v1/tenants/{tenant}/workspaces?limit=100&offset={}",
            usize::MAX
        ),
        Some((tenant.to_string(), own.to_string())),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_items(&serde_json::from_str(&body).unwrap(), &[], 2);
}

async fn measure_catalog(server: &LiveServer, key: &str, scope: Scope) {
    let mut samples = Vec::new();
    for i in 0..26 {
        let start = std::time::Instant::now();
        let page = mcp_page(server, key, scope, "/mcp", json!({"limit":100}), None).await;
        assert_items(&page, &[scope.1], 1);
        if i >= 5 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(f64::total_cmp);
    if let Ok(path) = std::env::var("EQ_TENANT_CATALOG_REPORT") {
        let report = json!({"samples":21,"warmup":5,"p50_ms":samples[10],
            "p95_ms":samples[19],"samples_ms":samples,
            "fixture":"real TCP MCP catalog; member sees one of two tenant workspaces",
            "limits":"unoptimized test build; includes API-key password-hash verification and TCP/auth/database round trips; concurrent build load may affect samples; not isolated DAL latency, a scale proof, or a production SLA"});
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}

async fn verify_revocation(server: &LiveServer, key: &str, scope: Scope, user: Uuid) {
    sqlx::query("UPDATE memberships SET is_active=false WHERE user_id=$1 AND workspace_id=$2")
        .bind(user)
        .bind(scope.1)
        .execute(&server.pool)
        .await
        .unwrap();
    for path in ["/mcp", "/api/v1/mcp"] {
        let (status, body) = request(
            server,
            key,
            Method::POST,
            path,
            Some((scope.0.to_string(), scope.1.to_string())),
            Some(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
                "params":{"name":"eq_workspace_list","arguments":{}}})),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    }
    let (status, body) = request(
        server,
        key,
        Method::GET,
        &format!("/api/v1/tenants/{}/workspaces", scope.0),
        Some((scope.0.to_string(), scope.1.to_string())),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    sqlx::query("UPDATE memberships SET is_active=true WHERE user_id=$1 AND workspace_id=$2")
        .bind(user)
        .bind(scope.1)
        .execute(&server.pool)
        .await
        .unwrap();
    assert_transport_parity(server, key, scope, &[scope.1]).await;
}

pub async fn verify(server: &LiveServer, tenant: Uuid, own: Uuid, sibling: Uuid, user: Uuid) {
    let scope = (tenant, own);
    let key = issue_key(server, user, scope).await;
    verify_filtered_pages(server, &key, scope, sibling).await;
    verify_role_visibility(server, &key, scope, sibling).await;
    verify_admin_pagination(server, scope, sibling).await;
    sqlx::query("INSERT INTO memberships(membership_id,tenant_id,workspace_id,user_id,role,is_active) VALUES($1,$2,$3,$4,'member',false)")
        .bind(Uuid::new_v4()).bind(tenant).bind(sibling).bind(user)
        .execute(&server.pool).await.unwrap();
    assert_transport_parity(server, &key, scope, &[own]).await;
    verify_revocation(server, &key, scope, user).await;
    measure_catalog(server, &key, scope).await;
}
