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
use axum::http::HeaderValue;
use axum::middleware;
use axum::routing::get;
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, Any, CorsLayer},
};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::observability_middleware::observability_middleware;
use crate::openapi::ApiDoc;
use crate::routes::create_router;
use crate::state::{ApiSecurityConfig, AppState};

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

/// Build CORS layer from security config — SPEC-027 IMP-007 / GitHub #277.
pub fn build_cors_layer(security: &ApiSecurityConfig) -> CorsLayer {
    if let Some(origins) = &security.cors_origins {
        let allowed: Result<Vec<HeaderValue>, _> =
            origins.iter().map(|o| HeaderValue::from_str(o)).collect();
        match allowed {
            Ok(list) if !list.is_empty() => CorsLayer::new()
                .allow_origin(AllowOrigin::list(list))
                .allow_methods(Any)
                .allow_headers(Any),
            _ => CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        }
    } else {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
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
            app = app.layer(CompressionLayer::new());
        }

        // CORS outermost — covers API, docs, and WebSocket upgrade paths.
        if self.config.enable_cors {
            app = app.layer(build_cors_layer(&self.state.security));
        }

        app
    }

    /// Run the server.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let app = self.build_router();
        let addr: SocketAddr = format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid address");

        info!("Starting EdgeQuake API server on {}", addr);

        if self.config.enable_swagger {
            info!("Swagger UI available at http://{}/swagger-ui", addr);
        }

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
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

    #[test]
    fn test_build_router() {
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
            ..ApiSecurityConfig::default()
        };
        let _layer = build_cors_layer(&security);
    }
}
