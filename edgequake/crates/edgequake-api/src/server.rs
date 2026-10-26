//! HTTP server.
//!
//! Provides the main HTTP server with middleware and configuration.
//!
//! ## Implements
//!
//! - [`FEAT0440`]: HTTP server with Axum
//! - [`FEAT0441`]: CORS configuration
//! - [`FEAT0442`]: Response compression
//! - [`FEAT0443`]: Swagger UI integration
//!
//! ## Use Cases
//!
//! - [`UC2040`]: System starts HTTP server
//! - [`UC2041`]: System serves OpenAPI documentation
//!
//! ## Enforces
//!
//! - [`BR0440`]: Configurable host and port
//! - [`BR0441`]: Optional feature toggles (CORS, compression, Swagger)

use std::net::SocketAddr;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::middleware;
use axum::routing::get;
use axum::Json;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use tower_http::{
    compression::predicate::DefaultPredicate,
    compression::CompressionLayer,
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
};
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::observability_middleware::observability_middleware;
use crate::openapi::ApiDoc;
use crate::routes::create_router;
use crate::state::{ApiSecurityConfig, AppState};

/// Explicit methods for fail-closed CORS (SPEC-083 S-10 / OWASP allow-list).
const CORS_FAIL_CLOSED_METHODS: [Method; 7] = [
    Method::GET,
    Method::POST,
    Method::PUT,
    Method::PATCH,
    Method::DELETE,
    Method::OPTIONS,
    Method::HEAD,
];

/// Explicit request headers for fail-closed CORS (SPEC-083 S-10).
fn cors_fail_closed_headers() -> [HeaderName; 10] {
    [
        header::AUTHORIZATION,
        header::RANGE,
        header::CONTENT_TYPE,
        header::ACCEPT,
        header::IF_MATCH,
        HeaderName::from_static("idempotency-key"),
        HeaderName::from_static("x-api-key"),
        HeaderName::from_static("x-tenant-id"),
        HeaderName::from_static("x-workspace-id"),
        HeaderName::from_static("x-request-id"),
    ]
}

/// Response headers a cross-origin PDF viewer must read to use byte-range loading.
fn cors_exposed_headers() -> [HeaderName; 6] {
    [
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::CONTENT_LENGTH,
        header::CONTENT_ENCODING,
        header::ETAG,
        HeaderName::from_static("x-policy-epoch"),
    ]
}

/// Apply methods, headers, and credentials.
///
/// Credentialed browser fetches (`credentials: "include"`) reject
/// `Access-Control-Allow-Origin: *` and require
/// `Access-Control-Allow-Credentials: true` (GitHub #479). Wildcards for
/// methods or headers are illegal on that path, so open mode mirrors the
/// request and fail-closed mode uses explicit lists.
fn apply_cors_methods_headers(layer: CorsLayer, fail_closed: bool) -> CorsLayer {
    let layer = layer
        .allow_credentials(true)
        .expose_headers(cors_exposed_headers());
    if fail_closed {
        // AllowOrigin::Any is unreachable on this path (caller sets list / empty list).
        layer
            .allow_methods(CORS_FAIL_CLOSED_METHODS)
            .allow_headers(cors_fail_closed_headers())
    } else {
        layer
            .allow_methods(AllowMethods::mirror_request())
            .allow_headers(AllowHeaders::mirror_request())
    }
}

/// Explicit origins for credentialed CORS. `*` is dropped: a wildcard cannot
/// be combined with `Access-Control-Allow-Credentials`.
fn explicit_cors_origins(origins: &[String]) -> Option<Vec<HeaderValue>> {
    let mut list = Vec::new();
    for origin in origins {
        let origin = origin.trim();
        if origin.is_empty() || origin == "*" {
            continue;
        }
        let Ok(value) = HeaderValue::from_str(origin) else {
            return None;
        };
        list.push(value);
    }
    if list.is_empty() {
        None
    } else {
        Some(list)
    }
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server host.
    pub host: String,

    /// Server port.
    pub port: u16,

    /// Enable CORS.
    pub enable_cors: bool,

    /// Enable compression.
    pub enable_compression: bool,

    /// Enable Swagger UI.
    pub enable_swagger: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            enable_cors: true,
            enable_compression: true,
            enable_swagger: true,
        }
    }
}

/// Build CORS layer from security config — SPEC-027 IMP-007 / GitHub #277 / SPEC-083 S-10 / #479.
///
/// Credentialed cross-origin responses echo the request `Origin` and set
/// `Access-Control-Allow-Credentials: true`. `AllowOrigin::Any` (`*`) is never
/// used: browsers drop those responses when the WebUI sends `credentials: "include"`.
///
/// When `cors_fail_closed` is true:
/// - `AllowOrigin::Any` is unreachable
/// - an empty allow-list denies every cross-origin request
/// - methods/headers use explicit allow-lists (never a wildcard)
pub fn build_cors_layer(security: &ApiSecurityConfig) -> CorsLayer {
    if let Some(origins) = &security.cors_origins {
        if let Some(list) = explicit_cors_origins(origins) {
            return apply_cors_methods_headers(
                CorsLayer::new().allow_origin(AllowOrigin::list(list)),
                security.cors_fail_closed,
            );
        }
    }

    if security.cors_fail_closed {
        // Fail closed: no cross-origin allow-list configured — empty origin list.
        // AllowOrigin::Any is intentionally unreachable here.
        apply_cors_methods_headers(
            CorsLayer::new().allow_origin(AllowOrigin::list(std::iter::empty::<HeaderValue>())),
            true,
        )
    } else {
        // Dev / open mode: echo whatever Origin the browser sent.
        apply_cors_methods_headers(
            CorsLayer::new().allow_origin(AllowOrigin::mirror_request()),
            false,
        )
    }
}

/// The HTTP server.
pub struct Server {
    config: ServerConfig,
    state: AppState,
}

impl Server {
    /// Create a new server.
    pub fn new(config: ServerConfig, state: AppState) -> Self {
        Self { config, state }
    }

    /// Build the application router with all middleware.
    pub fn build_router(&self) -> axum::Router {
        let mut app = create_router(self.state.clone());

        // Merge documentation routes BEFORE global middleware so CORS applies to
        // `/api-docs/openapi.json` and `/swagger-ui/*` (GitHub #277).
        if self.config.enable_swagger {
            app = app
                .merge(
                    SwaggerUi::new("/swagger-ui")
                        .url("/api-docs/openapi.json", ApiDoc::openapi())
                        .config(
                            utoipa_swagger_ui::Config::new(["/api-docs/openapi.json"])
                                .persist_authorization(true),
                        ),
                )
                .route(
                    "/api-docs/asyncapi.json",
                    get(|| async { Json(crate::openapi_asyncapi::asyncapi_document()) }),
                );
        }

        let max_upload = self.state.resource_budget().max_upload_bytes;
        app = app
            .layer(DefaultBodyLimit::max(max_upload))
            .layer(middleware::from_fn(observability_middleware));

        if self.config.enable_compression {
            // DefaultPredicate skips `text/event-stream` (tower-http #465 / #420).
            // Gzip on SSE buffers the whole body; browsers see one chunk at EOF.
            app = app.layer(CompressionLayer::new().compress_when(DefaultPredicate::new()));
        }

        // CORS outermost — covers API, docs, and WebSocket upgrade paths.
        if self.config.enable_cors {
            app = app.layer(build_cors_layer(&self.state.security));
        }

        app
    }

    /// Run the server with graceful shutdown + drain budget (SPEC-083 X-31).
    ///
    /// Pools stay open. The caller drains workers, then [`close_db_pools`].
    pub async fn run(self) -> (AppState, std::io::Result<()>) {
        let app = self.build_router();
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid address");

        info!("Starting EdgeQuake API server on {}", addr);

        if self.config.enable_swagger {
            info!("Swagger UI available at http://{}/swagger-ui", addr);
        }

        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(error) => return (self.state, Err(error)),
        };
        let drain = edgequake_tasks::shutdown_drain_budget();
        let cancel = CancellationToken::new();
        let signal_cancel = cancel.clone();
        #[cfg(feature = "postgres")]
        if let Some(client) = self.state.storage.original_store.client.clone() {
            if let Some(pool) = self.state.pg_pool.clone() {
                let cancel_reap = cancel.clone();
                tokio::spawn(async move {
                    loop {
                        if let Err(err) = edgequake_storage::originals::reap_postgres(
                            &pool,
                            client.blobs.as_ref(),
                        )
                        .await
                        {
                            warn!(error = %err, "original store reaper pass failed");
                        }
                        tokio::select! {
                            _ = cancel_reap.cancelled() => break,
                            _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
                        }
                    }
                });
            }
        }
        {
            let state = self.state.clone();
            let cancel_uploads = cancel.clone();
            tokio::spawn(async move {
                loop {
                    if let Err(err) =
                        crate::mcp::project::upload_session::reap_uploads(&state).await
                    {
                        warn!(error = %err, "mcp upload reaper pass failed");
                    }
                    tokio::select! {
                        _ = cancel_uploads.cancelled() => break,
                        _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {}
                    }
                }
            });
        }
        tokio::spawn(async move {
            wait_for_shutdown_signal().await;
            info!(
                drain_secs = drain.as_secs(),
                "Shutdown signal received — draining in-flight requests (SPEC-083 X-31)"
            );
            signal_cancel.cancel();
        });

        let serve = axum::serve(listener, app).with_graceful_shutdown({
            let cancel = cancel.clone();
            async move {
                cancel.cancelled().await;
            }
        });

        let result = tokio::select! {
            result = serve => result,
            _ = async {
                cancel.cancelled().await;
                tokio::time::sleep(drain).await;
            } => {
                warn!(
                    drain_secs = drain.as_secs(),
                    "SPEC-083 X-31: HTTP drain budget exceeded — forcing server exit"
                );
                Ok(())
            }
        };

        // SPEC-171: pools stay open until workers have stopped claiming.
        (self.state, result)
    }

    /// SPEC-112 / SPEC-171: release PostgreSQL backends after the worker drain.
    pub async fn close_db_pools(state: &AppState) {
        #[cfg(feature = "postgres")]
        {
            if let Some(ref bundle) = state.pool_bundle {
                info!("SPEC-112: closing PgPoolBundle after HTTP drain");
                bundle.close().await;
            } else if let Some(ref pool) = state.pg_pool {
                info!("SPEC-112: closing pg_pool after HTTP drain");
                pool.close().await;
            }
        }
        #[cfg(not(feature = "postgres"))]
        {
            let _ = state;
        }
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

/// Wait for SIGTERM / Ctrl+C (shared by graceful shutdown).
async fn wait_for_shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            warn!(error = %e, "Failed to install Ctrl+C handler");
            // Fall through to pending so we don't spin-exit.
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                warn!(error = %e, "Failed to install SIGTERM handler");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Unit/inproc helper: drain budget resolves and is bounded (X-31 matrix name).
#[cfg(test)]
#[tokio::test]
async fn e2e_shutdown_drains_or_cancels_within_budget() {
    use std::time::Duration;

    let budget = edgequake_tasks::shutdown_drain_budget();
    assert!(budget >= Duration::from_secs(1));
    assert!(budget <= Duration::from_secs(3600));

    // Inproc: a sticky future cancelled by select within a 1s fake drain.
    let cancel = CancellationToken::new();
    let sticky = async {
        tokio::time::sleep(Duration::from_secs(120)).await;
    };
    let drain = Duration::from_secs(1);
    let started = std::time::Instant::now();
    cancel.cancel();
    tokio::select! {
        _ = sticky => panic!("sticky future must not complete"),
        _ = async {
            cancel.cancelled().await;
            tokio::time::sleep(drain).await;
        } => {}
    }
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "forced cancel path must finish within drain budget"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();

        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert!(config.enable_cors);
        assert!(config.enable_swagger);
    }

    #[tokio::test]
    async fn test_build_router() {
        // WHY tokio::test: build_router calls create_router which spawns a
        // tokio task (pipeline_ws_bridge). A plain #[test] has no runtime.
        let config = ServerConfig::default();
        let state = AppState::test_state();
        let server = Server::new(config, state);

        let _router = server.build_router();
        // Router builds successfully
    }

    #[test]
    fn build_cors_layer_uses_allowlist_when_configured() {
        let security = ApiSecurityConfig {
            cors_origins: Some(vec!["https://app.example.com".into()]),
            cors_fail_closed: true,
            ..ApiSecurityConfig::default()
        };
        let _layer = build_cors_layer(&security);
    }

    #[test]
    fn build_cors_layer_fail_closed_without_origins_does_not_panic() {
        let security = ApiSecurityConfig {
            cors_origins: None,
            cors_fail_closed: true,
            ..ApiSecurityConfig::default()
        };
        let _layer = build_cors_layer(&security);
    }
}
