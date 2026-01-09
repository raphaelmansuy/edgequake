//! PostgreSQL adapters using pgvector and Apache AGE.
//!
//! This module provides PostgreSQL-based storage implementations:
//! - `PgVectorStorage` - Vector storage using pgvector extension
//! - `PostgresAGEGraphStorage` - Graph storage using Apache AGE extension
//! - `PostgresKVStorage` - Key-value storage using JSONB
//! - `PostgresConversationStorage` - Conversation, message, and folder storage
//! - `rls` - Row-Level Security context management for multi-tenancy
//!
//! ## Implements
//!
//! - [`FEAT0202`]: PostgreSQL with pgvector adapter
//! - [`FEAT0203`]: Apache AGE graph storage
//! - [`FEAT0240`]: JSONB key-value storage
//! - [`FEAT0250`]: Conversation persistence
//! - [`FEAT0260`]: Row-Level Security for multi-tenancy
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores documents in PostgreSQL
//! - [`UC0602`]: System stores entities in Apache AGE graph
//! - [`UC0603`]: System performs vector similarity search with pgvector
//! - [`UC0801`]: System manages conversation history
//!
//! ## Enforces
//!
//! - [`BR0202`]: ACID transactions for data integrity
//! - [`BR0240`]: Tenant isolation via RLS policies

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
