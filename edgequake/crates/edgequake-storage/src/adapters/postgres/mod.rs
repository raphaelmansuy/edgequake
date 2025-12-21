//! PostgreSQL adapters using pgvector and Apache AGE.
//!
//! This module provides PostgreSQL-based storage implementations:
//! - `PgVectorStorage` - Vector storage using pgvector extension
//! - `PostgresAGEGraphStorage` - Graph storage using Apache AGE extension
//! - `PostgresKVStorage` - Key-value storage using JSONB

mod config;
mod connection;
mod graph;
mod kv;
mod vector;

pub use config::PostgresConfig;
pub use connection::PostgresPool;
pub use graph::PostgresAGEGraphStorage;
pub use kv::PostgresKVStorage;
pub use vector::PgVectorStorage;
