//! EdgeQuake API - REST API Server
//!
//! This crate provides the HTTP REST API for EdgeQuake:
//!
//! - Document ingestion endpoints
//! - Query endpoints (multiple modes)
//! - Knowledge graph exploration
//! - Health and metrics
//!
//! ## Implements
//!
//! - **FEAT0400**: RESTful API with JSON
//! - **FEAT0401**: OpenAPI/Swagger documentation
//! - **FEAT0402**: Multi-tenant workspace isolation
//! - **FEAT0008**: Authentication middleware
//! - **FEAT0403**: SSE streaming for real-time updates
//!
//! ## Enforces
//!
//! - **BR0400**: All endpoints return JSON
//! - **BR0401**: Errors follow RFC 7807 problem details
//! - **BR0402**: Workspace context required for data endpoints
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
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐
//! │   Client    │
//! └──────┬──────┘
//!        │ HTTP/JSON
//!        ▼
//! ┌─────────────┐
//! │ Middleware  │ ← Authentication, Rate Limiting, Tenant Context
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │  Handlers   │ ← Request validation, business logic
//! └──────┬──────┘
//!        │
//!        ▼
//! ┌─────────────┐
//! │  Services   │ ← Storage adapters, LLM clients, Cache
//! └─────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use edgequake_api::{Server, ServerConfig, StorageMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = ServerConfig {
//!         host: "0.0.0.0".to_string(),
//!         port: 8000,
//!         storage_mode: StorageMode::Memory,
//!         ..Default::default()
//!     };
//!
//!     let server = Server::new(config).await?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```
//!
//! # Features
//!
//! - `postgres`: Enable PostgreSQL storage backend (default)
//!
//! # Module Organization
//!
//! - **handlers**: HTTP request handlers (documents, query, graph, etc.)
//! - **middleware**: Authentication, rate limiting, tenant context
//! - **state**: Application state and storage mode configuration
//! - **streaming**: Real-time streaming response management
//! - **cache_manager**: LRU cache with TTL for hot data
//! - **processor**: Async document processing pipeline
//! - **providers**: Unified provider resolution (OODA-226)
//! - **validation**: Input validation helpers
//! - **error**: API error types and HTTP status mapping

pub mod attribution;
pub mod cache_manager;
pub mod config_resolution;
pub mod document_metadata;
pub mod document_read_model;
pub mod error;
pub mod file_validation;
pub mod handlers;
pub mod mcp;
pub mod middleware;
pub mod model_catalog;
pub mod observability_middleware;
pub mod openapi;
pub mod openapi_annotation_sync;
pub mod openapi_asyncapi;
pub mod openapi_enrichment;
pub mod openapi_examples;
pub mod openapi_path_ssot;
pub mod openapi_security;
pub mod path_validation;
pub mod pipeline_progress_callback;
#[cfg(feature = "postgres")]
pub mod postgres_entity_sink;
#[cfg(feature = "postgres")]
pub mod postgres_lineage_sink;
pub mod processor;
mod provider_catalog;
pub mod provider_types;
pub mod provider_visibility;
pub mod providers;
pub mod routes;
pub mod safety_limits;
pub mod server;
pub mod server_config_store;
pub mod services;
pub mod startup_security;
pub mod state;
pub mod storage_inspector;
pub mod streaming;
pub mod task_queue_pressure;
pub mod validation;
pub mod vision_env;
pub mod workspace_pipeline_factory;
pub mod workspace_scope;

// Re-export commonly used types
pub use middleware::TenantContext;
pub use pipeline_progress_callback::PipelineProgressCallback;

pub use error::{ApiError, ApiResult};
pub use middleware::{tenant_rate_limit, AuthConfig, AuthState, RateLimitConfig, RateLimitState};
#[cfg(feature = "postgres")]
pub use postgres_entity_sink::PostgresEntitySink;
pub use processor::DocumentTaskProcessor;
pub use routes::create_router;
pub use server::{build_cors_layer, Server, ServerConfig};
pub use state::{AppState, AuthRuntime, QueryRuntime, StorageMode, StorageRuntime, TaskRuntime};

// Re-export production services from edgequake-core when feature is enabled
#[cfg(feature = "postgres")]
pub use edgequake_core::ConversationServiceImpl;

#[cfg(feature = "postgres")]
pub use edgequake_core::WorkspaceServiceImpl;

// Legacy aliases for backward compatibility
#[cfg(feature = "postgres")]
#[allow(deprecated)]
pub use edgequake_core::PostgresConversationService;

#[cfg(feature = "postgres")]
#[allow(deprecated)]
pub use edgequake_core::PostgresWorkspaceService;
