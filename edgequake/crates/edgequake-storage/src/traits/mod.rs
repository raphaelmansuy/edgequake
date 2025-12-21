//! Storage trait definitions.

mod graph;
mod kv;
mod vector;

pub use graph::{GraphEdge, GraphNode, GraphStorage, KnowledgeGraph};
pub use kv::KVStorage;
pub use vector::{VectorSearchResult, VectorStorage};
