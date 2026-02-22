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
use axum::middleware;
use serde::{Deserialize, Serialize};
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::info;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::middleware::{request_id, request_logging};
use crate::openapi::ApiDoc;
use crate::routes::create_router;
use crate::state::AppState;

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

        // Add middleware
        app = app
            .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100 MB limit for file uploads
            .layer(middleware::from_fn(request_logging))
            .layer(middleware::from_fn(request_id))
            .layer(TraceLayer::new_for_http());

        // CORS
        if self.config.enable_cors {
            let cors = CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any);
            app = app.layer(cors);
        }

        // Compression
        if self.config.enable_compression {
            app = app.layer(CompressionLayer::new());
        }

        // Swagger UI
        if self.config.enable_swagger {
            app = app.merge(
                SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
            );
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
}
