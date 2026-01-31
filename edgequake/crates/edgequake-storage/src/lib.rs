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
pub mod community;
pub mod error;
pub mod pdf_storage;
pub mod traits;

// Re-export community detection
pub use community::{
    detect_communities, Community, CommunityAlgorithm, CommunityConfig, CommunityDetectionResult,
};

// Re-export PDF storage types
pub use pdf_storage::{
    calculate_pdf_checksum, validate_pdf_data, CreatePdfRequest, ExtractionMethod, ListPdfFilter,
    PdfDocument, PdfDocumentStorage, PdfList, PdfProcessingStatus, UpdatePdfProcessingRequest,
};

// Re-export traits
pub use error::StorageError;
pub use traits::{
    GraphEdge, GraphNode, GraphStorage, KVStorage, KnowledgeGraph, VectorSearchResult,
    VectorStorage, WorkspaceVectorConfig, WorkspaceVectorRegistry,
};

// Re-export adapters
pub use adapters::memory::{
    MemoryGraphStorage, MemoryKVStorage, MemoryVectorStorage, MemoryWorkspaceVectorRegistry,
};

// Conditionally export PostgreSQL adapters
#[cfg(feature = "postgres")]
pub use adapters::postgres::{
    ConversationRow, FolderRow, MessageRow, PgVectorStorage, PgWorkspaceVectorRegistry,
    PostgresAGEGraphStorage, PostgresConfig, PostgresConversationStorage, PostgresKVStorage,
    PostgresPdfStorage, PostgresPool,
};
