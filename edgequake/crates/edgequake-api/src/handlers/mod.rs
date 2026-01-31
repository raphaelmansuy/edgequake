//! REST API Request Handlers
//!
//! This module contains all HTTP request handlers for the EdgeQuake API.
//! Each handler module is paired with a `*_types` module containing its DTOs.
//!
//! # Module Organization
//!
//! | Module | Description | Endpoints |
//! |--------|-------------|-----------|
//! | `auth` | Authentication & authorization | `/api/v1/auth/*` |
//! | `chat` | Chat completions (OpenAI-compatible) | `/api/v1/chat/*` |
//! | `conversations` | Conversation CRUD & history | `/api/v1/conversations/*` |
//! | `documents` | Document ingestion & management | `/api/v1/documents/*` |
//! | `entities` | Knowledge graph entities | `/api/v1/graph/entities/*` |
//! | `graph` | Knowledge graph queries | `/api/v1/graph/*` |
//! | `health` | Health probes | `/health`, `/ready`, `/live` |
//! | `lineage` | Document-entity lineage | `/api/v1/lineage/*` |
//! | `ollama` | Ollama emulation API | `/api/*` |
//! | `query` | RAG query execution | `/api/v1/query/*` |
//! | `relationships` | Knowledge graph relationships | `/api/v1/graph/relationships/*` |
//! | `tasks` | Async task management | `/api/v1/tasks/*` |
//! | `websocket` | Real-time pipeline updates | `/ws/pipeline/*` |
//! | `workspaces` | Multi-tenant workspaces | `/api/v1/workspaces/*` |
//!
//! # Handler Pattern
//!
//! Each handler follows a consistent pattern:
//!
//! ```rust,ignore
//! #[utoipa::path(/* OpenAPI metadata */)]
//! pub async fn handler_name(
//!     State(state): State<AppState>,  // Application state
//!     tenant_ctx: TenantContext,       // Extracted tenant context
//!     Json(request): Json<RequestDto>, // Request body (if applicable)
//! ) -> ApiResult<Json<ResponseDto>> {
//!     // 1. Validate input
//!     // 2. Execute business logic
//!     // 3. Return response or error
//! }
//! ```
//!
//! # Error Handling
//!
//! All handlers return `ApiResult<T>` which converts errors to consistent JSON:
//!
//! ```json
//! {
//!   "code": "NOT_FOUND",
//!   "message": "Document not found: doc-123",
//!   "details": { "document_id": "doc-123" }
//! }
//! ```

pub mod auth;
pub mod auth_types;
pub mod chat;
pub mod chat_types;
pub mod conversations;
pub mod conversations_types;
pub mod costs;
pub mod costs_types;
pub mod documents;
pub mod documents_types;
pub mod entities;
pub mod entities_types;
pub mod graph;
pub mod graph_types;
pub mod health;
pub mod health_types;
pub mod lineage;
pub mod lineage_types;
pub mod metrics;
pub mod metrics_types;
pub mod models;
pub mod models_types;
pub mod ollama;
pub mod ollama_types;
pub mod pdf_upload;
pub mod pipeline;
pub mod pipeline_types;
pub mod query;
pub mod query_types;
pub mod relationships;
pub mod relationships_types;
pub mod settings;
pub mod tasks;
pub mod tasks_types;
pub mod websocket;
pub mod websocket_types;
pub mod workspaces;
pub mod workspaces_types;

// Re-export handler functions and types.
// Note: Each handler module already re-exports its *_types module contents,
// so we only need to re-export the handler modules themselves.
pub use auth::*;
pub use chat::*;
pub use conversations::*;
pub use costs::*;
pub use documents::*;
pub use entities::*;
pub use graph::*;
pub use health::*;
pub use lineage::*;
pub use metrics::*;
pub use models::*;
pub use ollama::*;
pub use pdf_upload::*;
pub use pipeline::*;
pub use query::*;
pub use relationships::*;
pub use settings::*;
pub use tasks::*;
pub use websocket::*;
pub use workspaces::*;
