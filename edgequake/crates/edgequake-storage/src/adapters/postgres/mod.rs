//! PostgreSQL adapters using pgvector and Apache AGE.
//!
//! This module provides PostgreSQL-based storage implementations:
//! - `PgVectorStorage` - Vector storage using pgvector extension
//! - `PostgresAGEGraphStorage` - Graph storage using Apache AGE extension
//! - `PostgresKVStorage` - Key-value storage using JSONB
//! - `PostgresConversationStorage` - Conversation, message, and folder storage
//! - `rls` - Row-Level Security context management for multi-tenancy

mod config;
mod connection;
mod conversation;
mod graph;
mod kv;
pub mod rls;
mod vector;

pub use config::PostgresConfig;
pub use connection::PostgresPool;
pub use conversation::{ConversationRow, FolderRow, MessageRow, PostgresConversationStorage};
pub use graph::PostgresAGEGraphStorage;
pub use kv::PostgresKVStorage;
pub use rls::{clear_tenant_context, set_tenant_context, RlsContext, RlsQueryBuilder};
pub use vector::PgVectorStorage;
