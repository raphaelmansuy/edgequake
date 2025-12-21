//! # EdgeQuake Storage
//!
//! Storage abstractions and adapters for the EdgeQuake RAG system.
//!
//! This crate provides:
//! - Storage traits for key-value, vector, and graph operations
//! - In-memory implementations for testing
//! - Production adapters (PostgreSQL AGE + pgvector, SurrealDB)
//! - Community detection algorithms for graph clustering
//!
//! ## Storage Types
//!
//! - [`KVStorage`] - Key-value storage for documents, chunks, and cache
//! - [`VectorStorage`] - Vector similarity search for embeddings
//! - [`GraphStorage`] - Knowledge graph storage for entities and relationships
//!
//! ## Example
//!
//! ```rust,ignore
//! use edgequake_storage::{KVStorage, MemoryKVStorage};
//!
//! let storage = MemoryKVStorage::new("documents");
//! storage.initialize().await?;
//! ```

pub mod adapters;
pub mod community;
pub mod error;
pub mod traits;

// Re-export community detection
pub use community::{
    Community, CommunityAlgorithm, CommunityConfig, CommunityDetectionResult,
    detect_communities,
};

// Re-export traits
pub use error::StorageError;
pub use traits::{
    GraphEdge, GraphNode, GraphStorage, KVStorage, KnowledgeGraph, VectorSearchResult,
    VectorStorage,
};

// Re-export adapters
pub use adapters::memory::{MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage};

// Conditionally export PostgreSQL adapters
#[cfg(feature = "postgres")]
pub use adapters::postgres::{
    PostgresAGEGraphStorage, PostgresConfig, PostgresKVStorage, PostgresPool, PgVectorStorage,
};
