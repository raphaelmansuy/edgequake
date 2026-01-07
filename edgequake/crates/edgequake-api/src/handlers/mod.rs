//! API request handlers.

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
pub mod ollama;
pub mod ollama_types;
pub mod pipeline;
pub mod pipeline_types;
pub mod query;
pub mod query_types;
pub mod relationships;
pub mod relationships_types;
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
pub use ollama::*;
pub use pipeline::*;
pub use query::*;
pub use relationships::*;
pub use tasks::*;
pub use websocket::*;
pub use workspaces::*;
