//! # EdgeQuake Storage
//!
//! Storage abstractions and adapters for the EdgeQuake RAG system.
//!
//! # Implements
//!
//! - **FEAT0201**: Vector Similarity Search
//! - **FEAT0202**: Graph Traversal  
//! - **FEAT0203**: Graph Mutation Operations
//! - **FEAT0204**: Graph Analytics
//! - **FEAT0205**: Community Detection
//! - **FEAT0010**: Document Metadata Storage
//!
//! # Enforces
//!
//! - **BR0201**: Tenant isolation (namespace-based scoping)
//! - **BR0008**: Entity names normalized before storage
//! - **BR0009**: Max 1000 nodes per query (paginated)
//!
//! This crate provides:
//! - Storage traits for key-value, vector, and graph operations
//! - In-memory implementations for testing
//! - Production adapters (PostgreSQL AGE + pgvector, SurrealDB)
//! - Community detection algorithms for graph clustering
//!
//! ## Storage Types
//!
//! | Trait | FEAT | Implementation |
//! |-------|------|----------------|
//! | [`KVStorage`] | FEAT0010 | Postgres, Memory |
//! | [`VectorStorage`] | FEAT0201 | pgvector, Memory |
//! | [`GraphStorage`] | FEAT0202-0204 | Apache AGE, Memory |
//!
//! ## Adapter Selection
//!
//! ```text
//! if DATABASE_URL set:
//!     → PostgreSQL adapters (production)
//! else:
//!     → Memory adapters (testing)
//! ```
//!
//! ## Example
//!
//! ```rust,ignore
//! use edgequake_storage::{KVStorage, MemoryKVStorage};
//!
//! let storage = MemoryKVStorage::new("documents");
//! storage.initialize().await?;
//! ```
//!
//! # See Also
//!
//! - [`crate::traits`] for storage trait definitions
//! - [`crate::adapters::memory`] for in-memory implementations
//! - [`crate::adapters::postgres`] for PostgreSQL adapters

pub mod adapters;
pub mod chunk_content;
pub mod community;
pub mod community_index_service;
pub mod community_persist;
pub mod compensation;
pub mod conversation_storage;
pub mod conversation_types;
pub mod entity_id;
pub mod entity_reconcile;
pub mod error;
pub mod kv_key_schema;
pub mod metadata_filter_sql;
pub mod pdf_storage;
pub mod traits;
pub mod vector_id;

// Re-export entity identity (RC-6 / P-G1): single normalization entry point.
pub use entity_id::{normalize_entity_name, EntityId};

// Re-export community detection
pub use crate::community_index_service::{
    community_refresh_debounce_secs, pending_community_refresh_workspaces,
    refresh_community_index_now, schedule_community_index_refresh,
};
pub use chunk_content::{
    batch_fetch_chunk_contents, content_from_kv_value, content_from_metadata_or_kv,
};
pub use community::{Community, CommunityAlgorithm, CommunityConfig, CommunityDetectionResult};
pub use community_persist::{
    backfill_communities_if_needed, community_features_enabled, detect_and_persist_communities,
    needs_community_backfill, persist_community_labels, refresh_community_index,
    spawn_community_backfill_if_needed,
};

// Re-export PDF storage types
pub use pdf_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, DocumentStatsUpdate,
    ExtractionMethod, ListPdfFilter, PdfDocument, PdfDocumentStorage, PdfList, PdfProcessingStatus,
    UpdatePdfProcessingRequest,
};

pub use conversation_storage::ConversationStorage;
pub use conversation_types::{ConversationRow, FolderRow, MessageRow};

// Re-export traits
pub use error::StorageError;
pub use traits::{
    kv_key_matches_like, GraphEdge, GraphNode, GraphReadView, GraphStorage,
    GraphStorageAnalyticsOps, GraphStorageMutateOps, GraphStorageReadOps, KVStorage,
    KnowledgeGraph, MetadataFilter, VectorSearchResult, VectorStorage, WorkspaceVectorConfig,
    WorkspaceVectorRegistry,
};

// Re-export adapters
pub use adapters::memory::{
    MemoryConversationStorage, MemoryGraphStorage, MemoryKVStorage, MemoryPdfStorage,
    MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};

// Conditionally export PostgreSQL adapters
#[cfg(feature = "postgres")]
pub use adapters::postgres::{
    PgVectorStorage, PgWorkspaceVectorRegistry, PostgresAGEGraphStorage, PostgresConfig,
    PostgresConversationStorage, PostgresKVStorage, PostgresPdfStorage, PostgresPool,
};

// Re-export KV key schema for use across all crates
pub use kv_key_schema::kv_keys;
