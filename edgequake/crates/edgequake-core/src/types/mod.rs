//! Core type definitions for EdgeQuake.
//!
//! This module contains all the domain entities used throughout the system.

mod chunk;
mod document;
mod embedding;
mod entity;
mod relationship;

pub use chunk::Chunk;
pub use document::{Document, DocumentStatus};
pub use embedding::{Embedding, EmbeddingConfig};
pub use entity::GraphEntity;
pub use relationship::{GraphRelationship, RELATIONSHIP_SEP};
