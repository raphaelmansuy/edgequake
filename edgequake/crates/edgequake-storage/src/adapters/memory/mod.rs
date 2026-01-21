//! In-memory storage implementations.
//!
//! These implementations are primarily for testing and development.
//! They provide a simple, thread-safe in-memory storage that implements
//! all storage traits.
//!
//! ## Implements
//!
//! @implements FEAT0201 (In-memory storage adapter)
//! @implements FEAT0210 (Graph storage for entity relationships)
//! @implements FEAT0211 (Vector storage for similarity search)
//! @implements FEAT0212 (KV storage for document metadata)
//! @implements FEAT0350 (Per-workspace vector storage)
//!
//! ## Use Cases
//!
//! - [`UC0601`]: System stores documents in memory for testing
//! - [`UC0602`]: System creates entity graph in memory
//! - [`UC0603`]: System performs vector search in memory
//!
//! ## Enforces
//!
//! - [`BR0201`]: Testing isolation via ephemeral storage
//! - [`BR0210`]: Thread-safe concurrent access via RwLock

mod graph;
mod kv;
mod vector;
mod workspace_vector;

pub use graph::MemoryGraphStorage;
pub use kv::MemoryKVStorage;
pub use vector::MemoryVectorStorage;
pub use workspace_vector::MemoryWorkspaceVectorRegistry;
