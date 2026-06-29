//! SPEC-027 PostgreSQL auth E2E — proves PG SSOT for identity + session artifacts.
//!
//! Requires: `DATABASE_URL` and `--features postgres`
//!
//! Run:
//!   cargo test -p edgequake-api spec027_pg_auth --features postgres

#![cfg(feature = "postgres")]

mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use edgequake_api::state::migration_bootstrap::run_postgres_migrations;
use edgequake_api::{AppState, Server, ServerConfig};

async fn parse_json(response: axum::response::Response) -> Value {
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body).unwrap()
}

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
    state.auth.config.api_keys = vec!["master-test-key".to_string()];
    state.security.kv_identity_mirror = false;
    state
}

async fn connect_and_bootstrap() -> Option<sqlx::PgPool> {
    let database_url = common::spec013_postgres::try_database_url()?;
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .expect("connect postgres");
    run_postgres_migrations(&pool)
        .await
        .expect("bootstrap migrations");
    Some(pool)
}

#[tokio::test]
async fn spec027_pg_auth_login_refresh_stored_in_postgres() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP spec027_pg_auth_login_refresh_stored_in_postgres: DATABASE_URL not set"
            );
            return;
        }
    };

    let state = auth_enabled_pg_state(pool.clone());
    state.initialize_defaults().await.expect("defaults");
    let kv_storage = std::sync::Arc::clone(&state.storage.kv_storage);

    let username = format!("pg_auth_{}", &Uuid::new_v4().to_string()[..8]);
    let app = build_app(state);

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": "user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let login_app = build_app(auth_enabled_pg_state(pool.clone()));
    let login = login_app
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
    assert_eq!(login.status(), StatusCode::OK);
    let login_body = parse_json(login).await;
    let refresh_token = login_body["refresh_token"].as_str().expect("refresh_token");
    let user_id = login_body["user"]["user_id"].as_str().expect("user_id");

    let pg_user_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM users WHERE username = $1 AND tenant_id IS NOT NULL",
    )
    .bind(&username)
    .fetch_one(&pool)
    .await
    .expect("count users");
    assert_eq!(pg_user_count, 1, "user must exist in PostgreSQL");

    let pg_token_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM refresh_tokens rt
        INNER JOIN users u ON rt.user_id = u.user_id
        WHERE u.username = $1 AND rt.revoked = false
        "#,
    )
    .bind(&username)
    .fetch_one(&pool)
    .await
    .expect("count refresh tokens");
    assert_eq!(pg_token_count, 1, "refresh token must be in PostgreSQL");

    let kv_user_keys = kv_storage
        .keys_with_prefix("auth:user:")
        .await
        .expect("kv keys");
    assert!(
        !kv_user_keys.iter().any(|k| k.contains(user_id)),
        "PG-primary auth must not mirror user to KV when kv_identity_mirror=false"
    );

    let refresh_app = build_app(auth_enabled_pg_state(pool.clone()));
    let refresh = refresh_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh_token }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh.status(), StatusCode::OK);
}

#[tokio::test]
async fn spec027_pg_auth_logout_revokes_refresh_token_in_postgres() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!(
                "SKIP spec027_pg_auth_logout_revokes_refresh_token_in_postgres: DATABASE_URL not set"
            );
            return;
        }
    };

    let state = auth_enabled_pg_state(pool.clone());
    state.initialize_defaults().await.expect("defaults");

    let username = format!("pg_logout_{}", &Uuid::new_v4().to_string()[..8]);

    let create_app = build_app(auth_enabled_pg_state(pool.clone()));
    let create = create_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": "user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let login_app = build_app(auth_enabled_pg_state(pool.clone()));
    let login = login_app
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
    assert_eq!(login.status(), StatusCode::OK);
    let login_body = parse_json(login).await;
    let refresh_token = login_body["refresh_token"].as_str().expect("refresh_token");

    let logout_app = build_app(auth_enabled_pg_state(pool.clone()));
    let logout = logout_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh_token }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(logout.status(), StatusCode::NO_CONTENT);

    let revoked_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM refresh_tokens rt
        INNER JOIN users u ON rt.user_id = u.user_id
        WHERE u.username = $1 AND rt.revoked = true
        "#,
    )
    .bind(&username)
    .fetch_one(&pool)
    .await
    .expect("count revoked tokens");
    assert_eq!(
        revoked_count, 1,
        "logout must revoke refresh token in PostgreSQL"
    );

    let refresh_app = build_app(auth_enabled_pg_state(pool.clone()));
    let refresh = refresh_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refresh_token": refresh_token }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        refresh.status(),
        StatusCode::UNAUTHORIZED,
        "revoked refresh token must not mint new access token"
    );
}

#[tokio::test]
async fn spec027_pg_auth_api_key_roundtrip_postgres() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP spec027_pg_auth_api_key_roundtrip_postgres: DATABASE_URL not set");
            return;
        }
    };

    let state = auth_enabled_pg_state(pool.clone());
    state.initialize_defaults().await.expect("defaults");
    let kv_storage = std::sync::Arc::clone(&state.storage.kv_storage);
    let app = build_app(state);

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/api-keys")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({ "name": "spec027-pg-roundtrip" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let created = parse_json(create).await;
    let api_key = created["api_key"].as_str().expect("api_key");
    let key_id = created["key_id"].as_str().expect("key_id");
    assert!(api_key.starts_with("eq_"));

    let pg_key_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM api_keys WHERE key_id = $1::uuid AND is_active = true",
    )
    .bind(key_id)
    .fetch_one(&pool)
    .await
    .expect("count api keys");
    assert_eq!(pg_key_count, 1, "API key must be stored in PostgreSQL");

    let kv_api_keys = kv_storage
        .keys_with_prefix("auth:api_key:")
        .await
        .expect("kv api keys");
    assert!(
        !kv_api_keys.iter().any(|k| k.ends_with(key_id)),
        "PG-primary must not mirror API key to KV"
    );

    let app = build_app(auth_enabled_pg_state(pool));
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/documents")
                .header("x-api-key", api_key)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn spec027_pg_auth_list_users_reads_from_postgres() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP spec027_pg_auth_list_users_reads_from_postgres: DATABASE_URL not set");
            return;
        }
    };

    let state = auth_enabled_pg_state(pool.clone());
    state.initialize_defaults().await.expect("defaults");

    let username = format!("pg_list_{}", &Uuid::new_v4().to_string()[..8]);

    let create_app = build_app(auth_enabled_pg_state(pool.clone()));
    let create = create_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": "user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);

    let list_app = build_app(auth_enabled_pg_state(pool.clone()));
    let list = list_app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users")
                .header("x-api-key", "master-test-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(list.status(), StatusCode::OK);
    let list_body = parse_json(list).await;
    let users = list_body["users"].as_array().expect("users array");
    assert!(
        users
            .iter()
            .any(|u| u["username"].as_str() == Some(&username)),
        "list users must return PG-backed user"
    );

    let pg_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username = $1")
        .bind(&username)
        .fetch_one(&pool)
        .await
        .expect("count user");
    assert_eq!(pg_count, 1, "user must exist in PostgreSQL");
}

#[tokio::test]
async fn spec027_pg_auth_kv_mirror_env_ignored_when_pool() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP spec027_pg_auth_kv_mirror_env_ignored_when_pool: DATABASE_URL not set");
            return;
        }
    };

    let mut state = auth_enabled_pg_state(pool.clone());
    state.security.kv_identity_mirror = true;
    state.initialize_defaults().await.expect("defaults");
    let kv_storage = std::sync::Arc::clone(&state.storage.kv_storage);

    let username = format!("pg_mirror_{}", &Uuid::new_v4().to_string()[..8]);
    let app = build_app(state);

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::CONTENT_TYPE, "application/json")
                .header("x-api-key", "master-test-key")
                .body(Body::from(
                    json!({
                        "username": username,
                        "email": format!("{username}@example.com"),
                        "password": "SecurePass123!",
                        "role": "user"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::CREATED);
    let create_body = parse_json(create).await;
    let user_id = create_body["user"]["user_id"].as_str().expect("user_id");

    let kv_user_keys = kv_storage
        .keys_with_prefix("auth:user:")
        .await
        .expect("kv keys");
    assert!(
        !kv_user_keys.iter().any(|k| k.contains(user_id)),
        "EDGEQUAKE_KV_IDENTITY_MIRROR must be ignored when PG pool exists (phase 47)"
    );

    let mut health_state = auth_enabled_pg_state(pool.clone());
    health_state.security.kv_identity_mirror = true;
    let health_app = build_app(health_state);
    let health = health_app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    let health_body = parse_json(health).await;
    let caps = health_body["capabilities"]
        .as_object()
        .expect("capabilities");
    assert_eq!(caps["auth_identity_ssot"], "postgresql");
    assert_eq!(caps["kv_identity_mirror_configured"], true);
    assert_eq!(caps["kv_identity_mirror_effective"], false);
}

#[tokio::test]
async fn spec027_pg_health_oauth_capabilities_postgres() {
    let pool = match connect_and_bootstrap().await {
        Some(p) => p,
        None => {
            eprintln!("SKIP spec027_pg_health_oauth_capabilities_postgres: DATABASE_URL not set");
            return;
        }
    };

    let state = auth_enabled_pg_state(pool);
    let app = build_app(state);

    let health = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);
    let body = parse_json(health).await;
    let caps = body["capabilities"].as_object().expect("capabilities");
    assert_eq!(caps["auth_identity_ssot"], "postgresql");
    assert_eq!(caps["oauth2_oidc_builtin"], false);
    assert_eq!(caps["auth_kv_harness_active"], false);
    assert_eq!(caps["external_sso_pattern"], "oauth2-proxy");
    let mechanisms = caps["auth_mechanisms"].as_array().expect("auth_mechanisms");
    assert!(mechanisms.iter().any(|m| m == "jwt_password"));
    assert!(mechanisms.iter().any(|m| m == "api_key"));
}
