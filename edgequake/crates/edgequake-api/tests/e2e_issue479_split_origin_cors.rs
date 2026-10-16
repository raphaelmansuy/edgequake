//! GitHub #479 — split-origin WebUI sends `credentials: include`.
//!
//! Browsers reject `Access-Control-Allow-Origin: *` on those responses.
//! Open/dev CORS must echo the request origin and allow credentials.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use edgequake_api::{AppState, Server, ServerConfig};

const UI_ORIGIN: &str = "http://lan-host:3000";

fn dev_split_origin_app() -> axum::Router {
    let mut state = AppState::test_state();
    state.security.cors_origins = None;
    state.security.cors_fail_closed = false;
    Server::new(
        ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            enable_cors: true,
            enable_compression: false,
            enable_swagger: false,
        },
        state,
    )
    .build_router()
}

fn header<'a>(response: &'a axum::response::Response, name: &str) -> Option<&'a str> {
    response
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
}

#[tokio::test]
async fn issue_479_dev_preflight_echoes_origin_and_allows_credentials() {
    let app = dev_split_origin_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/health")
                .header("Origin", UI_ORIGIN)
                .header("Access-Control-Request-Method", "GET")
                .header(
                    "Access-Control-Request-Headers",
                    "content-type,x-tenant-id,x-workspace-id,x-user-id,traceparent",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        header(&response, "access-control-allow-origin"),
        Some(UI_ORIGIN),
        "credentialed CORS must echo the UI origin, not *"
    );
    assert_ne!(header(&response, "access-control-allow-origin"), Some("*"));
    assert_eq!(
        header(&response, "access-control-allow-credentials"),
        Some("true")
    );
    let allowed = header(&response, "access-control-allow-headers").unwrap_or("");
    assert!(
        allowed.to_ascii_lowercase().contains("x-tenant-id"),
        "preflight must allow the WebUI tenant header, got {allowed}"
    );
}

#[tokio::test]
async fn issue_479_dev_get_echoes_origin_and_allows_credentials() {
    let app = dev_split_origin_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .header("Origin", UI_ORIGIN)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        header(&response, "access-control-allow-origin"),
        Some(UI_ORIGIN)
    );
    assert_eq!(
        header(&response, "access-control-allow-credentials"),
        Some("true")
    );
}

/// Real TCP preflight, the same bytes a browser sends. `tower` oneshot does
/// not go through a socket.
#[tokio::test]
async fn issue_479_tcp_curl_preflight_echoes_origin_and_credentials() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = dev_split_origin_app();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // `std::process::Command` blocks a runtime worker. On a small pool that
    // stalls the server task, so curl never gets a response.
    let mut output = String::new();
    for _ in 0..20 {
        let result = tokio::process::Command::new("curl")
            .args([
                "-sS",
                "--max-time",
                "2",
                "--connect-timeout",
                "1",
                "-D",
                "-",
                "-o",
                "/dev/null",
                "-X",
                "OPTIONS",
                &format!("http://{addr}/health"),
                "-H",
                &format!("Origin: {UI_ORIGIN}"),
                "-H",
                "Access-Control-Request-Method: GET",
                "-H",
                "Access-Control-Request-Headers: content-type,x-tenant-id,x-workspace-id",
            ])
            .output()
            .await
            .expect("curl");
        output = String::from_utf8_lossy(&result.stdout).to_string();
        if result.status.success() && !output.is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    println!("CURL_PREFLIGHT\n{output}");
    let lower = output.to_ascii_lowercase();
    assert!(
        lower.contains("access-control-allow-origin: http://lan-host:3000"),
        "curl preflight must echo the UI origin:\n{output}"
    );
    assert!(
        lower.contains("access-control-allow-credentials: true"),
        "curl preflight must allow credentials:\n{output}"
    );
    assert!(
        !lower.contains("access-control-allow-origin: *"),
        "wildcard origin is illegal on a credentialed request:\n{output}"
    );
}

#[test]
fn issue_477_server_pools_probe_age_after_from_existing() {
    let src = include_str!("../src/state/postgres.rs");
    let ingest = src
        .find("PostgresPool::from_existing(pool.clone(), pg_config.clone())")
        .expect("ingest from_existing");
    let after_ingest = &src[ingest..];
    assert!(
        after_ingest.contains("ingest_pool.probe_graph_extension_available()"),
        "ingest pool must probe AGE (GitHub #477)"
    );
    assert!(
        src.contains("query_pg.probe_graph_extension_available()"),
        "query pool must probe AGE (GitHub #477)"
    );
}
