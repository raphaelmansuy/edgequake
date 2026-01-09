//! In-memory storage implementations.
//!
//! These implementations are primarily for testing and development.
//! They provide a simple, thread-safe in-memory storage that implements
//! all storage traits.
//!
//! ## Implements
//!
//! - [`FEAT0201`]: In-memory storage adapter
//! - [`FEAT0210`]: Graph storage for entity relationships
//! - [`FEAT0211`]: Vector storage for similarity search
//! - [`FEAT0212`]: KV storage for document metadata
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

pub use graph::MemoryGraphStorage;
pub use kv::MemoryKVStorage;
pub use vector::MemoryVectorStorage;
