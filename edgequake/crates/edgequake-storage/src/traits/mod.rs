//! Storage trait definitions.
//!
//! # Implements
//!
//! This module defines the core storage abstractions:
//!
//! - [`KVStorage`] (FEAT0010): Document and metadata storage
//! - [`VectorStorage`] (FEAT0201): Embedding similarity search
//! - [`GraphStorage`] (FEAT0202-0204): Entity/relationship graph
//! - [`WorkspaceVectorRegistry`] (FEAT0350): Per-workspace vector isolation
//!
//! # Enforces
//!
//! - **BR0201**: All traits support namespace-based tenant isolation
//! - **BR0008**: GraphStorage normalizes entity names on write
//! - **BR0350**: Each workspace has isolated vector storage
//!
//! # WHY: Trait-Based Abstraction
//!
//! Using traits instead of concrete types enables:
//! - **Testing**: Mock implementations for unit tests
//! - **Flexibility**: Multiple backend support (Postgres, Memory, SurrealDB)
//! - **Modularity**: Storage can be swapped without changing business logic

mod embedder;
mod graph;
mod graph_analytics_ops;
mod graph_isp;
mod graph_mutate_ops;
mod graph_read_ops;
mod graph_read_view;
mod graph_scan_ops;
mod graph_traversal;
mod kv;
mod vector;
mod workspace_vector;

pub mod domain;

pub use domain::{
    Chunk, ChunkCursor, ChunkId, ChunkRepository, ChunkText, DocumentId, DocumentRepository,
    EmbeddingCapabilities, EmbeddingIndex, EmbeddingRow, FleetEmbeddingIndex, FleetEmbeddingKey,
    FleetEmbeddingRow, InsertReport, MigrationJobSnapshot, MigrationLedger, MirrorLegacyReport,
    ModelId, Page, ScoredChunk, ScoredFleet, TenantId, UnitOfWork, UpsertReport, VectorQuery,
    WorkspaceId,
};
pub use embedder::TextEmbedder;
pub use graph::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};
pub use graph_analytics_ops::GraphStorageAnalyticsOps;
pub use graph_isp::{GraphStorageAnalyticsCap, GraphStorageMutator, GraphStorageReader};
pub use graph_mutate_ops::{GraphPropertyWriteMode, GraphStorageMutateOps};
pub use graph_read_ops::GraphStorageReadOps;
pub use graph_read_view::GraphReadView;
pub use graph_scan_ops::{
    collect_source_references, edge_matches_list_filter, edge_matches_relationship_id,
    edge_matches_scope_dims, edge_matches_tenant_workspace, edge_relationship_id,
    is_topology_entity_ref, node_matches_list_filter, node_matches_tenant_workspace,
    scope_dim_matches_legacy_null, sources_match_prefixes, EdgeListFilter, GraphScanOps,
    NodeListFilter, PagedGraphResult,
};
pub use graph_traversal::{
    traverse_graph_bounded, BoundedGraphTraversal, GraphExpansionAuthorizer, GraphTraversalBudget,
    GraphTruncationReason,
};
pub use kv::{kv_key_matches_like, KVStorage};
pub use vector::{
    vector_upsert_chunk_size, MetadataFilter, VectorSearchResult, VectorStorage,
    DEFAULT_VECTOR_UPSERT_CHUNK,
};
pub use workspace_vector::{WorkspaceVectorConfig, WorkspaceVectorRegistry};
