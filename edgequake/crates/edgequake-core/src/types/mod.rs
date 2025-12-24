//! Core type definitions for EdgeQuake.
//!
//! This module contains all the domain entities used throughout the system.

mod chunk;
mod document;
mod embedding;
mod entity;
mod multitenancy;
mod query;
mod relationship;

pub use chunk::Chunk;
pub use document::{Document, DocumentStatus};
pub use embedding::{Embedding, EmbeddingConfig};
pub use entity::GraphEntity;
pub use multitenancy::{
    CreateWorkspaceRequest, Membership, MembershipRole, Tenant, TenantContext, TenantPlan,
    UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};
pub use query::{
    ContextChunk, ContextEntity, ContextRelationship, DocumentInfo, GraphStats, InsertResult,
    QueryContext, QueryMode, QueryParams, QueryResult, QueryStats,
};
pub use relationship::{GraphRelationship, RELATIONSHIP_SEP};
