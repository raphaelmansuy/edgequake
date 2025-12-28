//! API request handlers.

pub mod auth;
pub mod chat;
pub mod conversations;
pub mod documents;
pub mod entities;
pub mod graph;
pub mod health;
pub mod metrics;
pub mod ollama;
pub mod pipeline;
pub mod query;
pub mod relationships;
pub mod tasks;
pub mod workspaces;

pub use auth::*;
pub use chat::*;
pub use conversations::*;
pub use documents::*;
pub use entities::*;
pub use graph::*;
pub use health::*;
pub use metrics::*;
pub use ollama::*;
pub use pipeline::*;
pub use query::*;
pub use relationships::*;
pub use tasks::*;
pub use workspaces::*;
