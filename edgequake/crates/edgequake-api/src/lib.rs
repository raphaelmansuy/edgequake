//! EdgeQuake API - REST API Server
//!
//! This crate provides the HTTP REST API for EdgeQuake:
//!
//! - Document ingestion endpoints
//! - Query endpoints (multiple modes)
//! - Knowledge graph exploration
//! - Health and metrics
//!
//! # API Design
//!
//! The API follows REST conventions with OpenAPI documentation.
//! All endpoints are JSON-based with proper error handling.
//!
//! # Endpoints
//!
//! - `POST /api/v1/documents` - Ingest documents
//! - `POST /api/v1/query` - Execute queries
//! - `GET /api/v1/graph` - Explore knowledge graph
//! - `GET /api/v1/health` - Health check

pub mod cache_manager;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod processor;
pub mod routes;
pub mod server;
pub mod state;
pub mod streaming;
pub mod validation;

#[cfg(feature = "postgres")]
pub mod postgres_conversation_service;

#[cfg(feature = "postgres")]
pub mod postgres_workspace_service;

// Re-export commonly used types
pub use middleware::TenantContext;

pub use error::{ApiError, ApiResult};
pub use middleware::{tenant_rate_limit, AuthConfig, AuthState, RateLimitConfig, RateLimitState};
pub use processor::DocumentTaskProcessor;
pub use routes::create_router;
pub use server::{Server, ServerConfig};
pub use state::{AppState, StorageMode};

#[cfg(feature = "postgres")]
pub use postgres_conversation_service::PostgresConversationService;

#[cfg(feature = "postgres")]
pub use postgres_workspace_service::PostgresWorkspaceService;
