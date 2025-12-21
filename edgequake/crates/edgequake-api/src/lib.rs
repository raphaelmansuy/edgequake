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

pub mod error;
pub mod handlers;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod server;
pub mod state;

pub use error::{ApiError, ApiResult};
pub use middleware::{AuthConfig, AuthState, RateLimitConfig};
pub use server::{Server, ServerConfig};
pub use state::AppState;
