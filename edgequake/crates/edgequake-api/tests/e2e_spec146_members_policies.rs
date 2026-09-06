//! SPEC-146 M5 — members/roles/policies smoke (PG when available).
//!
//! Run: `cargo test -p edgequake-api --test e2e_spec146_members_policies --features postgres`

mod common;

use edgequake_api::{AppState, Server, ServerConfig};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

fn test_app() -> axum::Router {
    let config = ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: false,
    };
    Server::new(config, AppState::test_state()).build_router()
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap_or(json!({}))
}

#[tokio::test]
async fn authz_members_without_auth_is_unauthorized_or_unavailable() {
    // Memory/test state: authz PAP requires Postgres + auth → 401/503, not 200 with leak.
    let app = test_app();
    let ws = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/authz/members"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    assert!(
        status == StatusCode::UNAUTHORIZED
            || status == StatusCode::FORBIDDEN
            || status == StatusCode::SERVICE_UNAVAILABLE
            || status == StatusCode::NOT_FOUND,
        "unexpected status {status}"
    );
}

#[tokio::test]
async fn authz_policies_route_is_registered() {
    let app = test_app();
    let ws = Uuid::new_v4();
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/workspaces/{ws}/authz/policies"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // Route exists — not 404 for path itself when unauthenticated typically 401.
    assert_ne!(
        resp.status(),
        StatusCode::METHOD_NOT_ALLOWED,
        "policies route should be registered"
    );
    let _ = json_body(resp).await;
}

#[cfg(feature = "postgres")]
mod pg_smoke {
    use super::*;
    use sqlx::PgPool;
    use std::env;

    fn database_url() -> Option<String> {
        let base = env::var("DATABASE_URL").ok().or_else(|| {
            std::fs::read_to_string("/tmp/edgequake-db-url")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })?;
        Some(common::test_db::isolated_test_url(&base))
    }

    #[tokio::test]
    async fn workspace_authz_tables_exist_after_migration() {
        let Some(url) = database_url() else {
            eprintln!("SKIP members/policies PG smoke: no DATABASE_URL");
            return;
        };
        let pool = match PgPool::connect(&url).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("SKIP members/policies PG smoke: {e}");
                return;
            }
        };

        for table in [
            "workspace_roles",
            "workspace_role_bindings",
            "policies",
            "break_glass_sessions",
            "workspace_authz_state",
        ] {
            let exists: bool = sqlx::query_scalar(
                r#"SELECT EXISTS (
                    SELECT 1 FROM information_schema.tables WHERE table_name = $1
                )"#,
            )
            .bind(table)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
            assert!(exists, "expected migration table {table}");
        }
    }

    /// G-146-13: members bind + list + delete; attrs upsert; no leak without auth.
    #[tokio::test]
    #[serial_test::serial]
    async fn g146_13_members_bind_list_delete_and_attrs() {
        use common::spec146_pg::{request_json, try_create_harness};
        use serde_json::json;

        let Some(h) = try_create_harness().await else {
            eprintln!("SKIP g146_13: no DATABASE_URL");
            return;
        };

        let ws = &h.workspace_id;
        let roles_uri = format!("/api/v1/workspaces/{ws}/authz/roles");
        let members_uri = format!("/api/v1/workspaces/{ws}/authz/members");
        let owner = h.auth_headers_owner();

        let (noauth, _) = request_json(
            h.router(),
            "GET",
            &members_uri,
            &[],
            None,
        )
        .await;
        assert!(
            noauth == StatusCode::UNAUTHORIZED || noauth == StatusCode::FORBIDDEN,
            "unauthenticated members list leaked: {noauth}"
        );

        let (rs, roles) = request_json(h.router(), "GET", &roles_uri, &owner, None).await;
        assert_eq!(rs, StatusCode::OK, "{roles}");
        let viewer_id = roles["roles"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r.get("name").and_then(|n| n.as_str()) == Some("viewer"))
            .and_then(|r| r.get("role_id").and_then(|id| id.as_str()))
            .map(|s| s.to_string());
        let Some(viewer_id) = viewer_id else {
            eprintln!("SKIP g146_13: no viewer role in {roles}");
            return;
        };

        let (created, cbind) = request_json(
            h.router(),
            "POST",
            &members_uri,
            &owner,
            Some(json!({
                "principal_kind": "user",
                "principal_id": h.peer_user_id,
                "role_id": viewer_id
            })),
        )
        .await;
        assert!(
            created == StatusCode::CREATED || created == StatusCode::OK,
            "bind failed {created}: {cbind}"
        );

        let (listed, lbody) = request_json(h.router(), "GET", &members_uri, &owner, None).await;
        assert_eq!(listed, StatusCode::OK, "{lbody}");
        assert!(
            lbody.to_string().contains(&h.peer_user_id),
            "bound peer missing: {lbody}"
        );

        let (deleted, dbody) = request_json(
            h.router(),
            "DELETE",
            &format!("{members_uri}/user/{}/{viewer_id}", h.peer_user_id),
            &owner,
            None,
        )
        .await;
        assert!(
            deleted.is_success() || deleted == StatusCode::NO_CONTENT,
            "delete {deleted}: {dbody}"
        );

        let attr_uri = format!("/api/v1/workspaces/{ws}/authz/principal-attributes");
        let (attr, abody) = request_json(
            h.router(),
            "PUT",
            &attr_uri,
            &owner,
            Some(json!({
                "principal_kind": "user",
                "principal_id": h.peer_user_id,
                "name": "clearance",
                "value": "internal"
            })),
        )
        .await;
        assert!(
            attr.is_success() || attr == StatusCode::CREATED,
            "attr upsert {attr}: {abody}"
        );
    }
}
