//! Core type definitions for EdgeQuake.
//!
//! This module contains all the domain entities used throughout the system.

mod chunk;
mod conversation;
mod document;
mod embedding;
mod entity;
mod multitenancy;
mod query;
mod relationship;

pub use chunk::Chunk;
pub use conversation::{
    Conversation, ConversationFilter, ConversationMode, ConversationSortField,
    CreateConversationRequest, CreateFolderRequest, CreateMessageRequest, Folder, ImportError,
    ImportResult, Message, MessageContext, MessageRole, MessageSource, PaginatedConversations,
    PaginatedMessages, PaginationMeta, UpdateConversationRequest, UpdateFolderRequest,
    UpdateMessageRequest,
};
pub use document::{Document, DocumentStatus};
pub use embedding::{Embedding, EmbeddingConfig};
pub use entity::GraphEntity;
pub use multitenancy::{
    CreateWorkspaceRequest, Membership, MembershipRole, Tenant, TenantContext, TenantPlan,
    UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};
pub use query::{
    ContextChunk, ContextEntity, ContextRelationship, DocumentDeletionResult, DocumentInfo,
    EntityDeletionResult, GraphStats, InsertResult, QueryContext, QueryMode, QueryParams,
    QueryResult, QueryStats,
};
pub use relationship::{GraphRelationship, RELATIONSHIP_SEP};
