//! # EdgeQuake Core
//!
//! Core types and utilities for the EdgeQuake RAG system.
//!
//! This crate provides the fundamental domain entities and error types
//! used throughout the EdgeQuake system.
//!
//! ## Core Types
//!
//! - [`Document`] - A unit of text content to be processed
//! - [`Chunk`] - A segment of a document sized for LLM context windows
//! - [`GraphEntity`] - A named entity extracted from text
//! - [`GraphRelationship`] - A relationship between two entities
//! - [`Embedding`] - Vector representation of text
//! - [`EdgeQuake`] - High-level RAG orchestrator
//!
//! ## Example
//!
//! ```rust
//! use edgequake_core::types::{Document, DocumentStatus};
//!
//! let doc = Document::new("Hello, world!".to_string(), None);
//! assert_eq!(doc.status, DocumentStatus::Pending);
//! ```

pub mod config;
pub mod error;
pub mod orchestrator;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use error::{Error, Result};
pub use orchestrator::{
    EdgeQuake, EdgeQuakeConfig, QueryMode, QueryParams, QueryResult, QueryContext,
    InsertResult, DocumentInfo, GraphStats, StorageBackend, StorageConfig,
};
pub use types::{
    Chunk, Document, DocumentStatus, Embedding, EmbeddingConfig, GraphEntity, GraphRelationship,
};
