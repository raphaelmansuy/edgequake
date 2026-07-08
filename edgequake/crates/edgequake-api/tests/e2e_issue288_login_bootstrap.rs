//! GitHub #288 — login 401 on v0.15 when auth is enabled but no PG users exist.
//!
//! Requires: `DATABASE_URL` and `--features postgres`
//!
//! Run:
//!   EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD='SecurePass123!' \
//!     cargo test -p edgequake-api e2e_issue288_login_bootstrap --features postgres

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

use edgequake_api::services::auth_bootstrap::bootstrap_auth_identity_if_needed;
use edgequake_api::state::migration_bootstrap::run_postgres_migrations;
use edgequake_api::{AppState, Server, ServerConfig};

fn server_config() -> ServerConfig {
    ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 0,
        enable_cors: false,
        enable_compression: false,
        enable_swagger: true,
    }
}

fn build_app(state: AppState) -> axum::Router {
    Server::new(server_config(), state).build_router()
}

fn auth_enabled_pg_state(pool: sqlx::PgPool) -> AppState {
    let mut state = AppState::test_state_with_pg_pool(pool);
    state.auth.config.auth_enabled = true;
    state.auth.config.dev_mode = false;
    state.security.kv_identity_mirror = false;
    state
}

async fn connect_and_bootstrap() -> Option<sqlx::PgPool> {
    let database_url = common::spec013_postgres::try_database_url()?;
    let pool = match PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            eprintln!("SKIP issue288 postgres E2E: connect failed: {error}");
            return None;
        }
    };
    if run_postgres_migrations(&pool).await.is_err() {
        eprintln!("SKIP issue288 postgres E2E: migration bootstrap failed");
        return None;
    }
    Some(pool)
}

#[tokio::test]
async fn issue288_bootstrap_admin_allows_login_when_auth_enabled() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP issue288_bootstrap_admin_allows_login_when_auth_enabled: DATABASE_URL not set");
            return;
        }
    };

    let username = format!("issue288_{}", &uuid::Uuid::new_v4().to_string()[..8]);

    std::env::set_var("EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME", &username);
    std::env::set_var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD", "SecurePass123!");
    std::env::set_var(
        "EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL",
        format!("{username}@example.com"),
    );

    let state = auth_enabled_pg_state(pool);
    state.initialize_defaults().await.expect("defaults");
    bootstrap_auth_identity_if_needed(&state)
        .await
        .expect("bootstrap");

    let app = build_app(state);

    let response = app
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
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "login should succeed after bootstrap admin creation (GitHub #288)"
    );

    std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_USERNAME");
    std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD");
    std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_EMAIL");
}

#[tokio::test]
async fn issue288_login_returns_401_without_bootstrap_user() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP issue288_login_returns_401_without_bootstrap_user: DATABASE_URL not set"
            );
            return;
        }
    };

    std::env::remove_var("EDGEQUAKE_BOOTSTRAP_ADMIN_PASSWORD");

    let state = auth_enabled_pg_state(pool);
    state.initialize_defaults().await.expect("defaults");

    let app = build_app(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "username": "nobody",
                        "password": "SecurePass123!"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "login must 401 when no login-capable users exist (root cause of #288)"
    );
}
