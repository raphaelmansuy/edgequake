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
    ImportResult, Message, MessageContext, MessageContextEntity, MessageContextRelationship,
    MessageRole, MessageSource, PaginatedConversations, PaginatedMessages, PaginationMeta,
    UpdateConversationRequest, UpdateFolderRequest, UpdateMessageRequest,
};
pub use document::{Document, DocumentStatus};
pub use embedding::{Embedding, EmbeddingConfig};
pub use entity::GraphEntity;
pub use multitenancy::{
    CreateWorkspaceRequest,
    Membership,
    MembershipRole,
    MetricsSnapshot,
    MetricsTriggerType,
    Tenant,
    TenantContext,
    TenantPlan,
    UpdateWorkspaceRequest,
    Workspace,
    WorkspaceStats,
    // SPEC-032: Export embedding constants
    DEFAULT_EMBEDDING_DIMENSION,
    DEFAULT_EMBEDDING_MODEL,
    DEFAULT_EMBEDDING_PROVIDER,
    // SPEC-032: Export LLM constants
    DEFAULT_LLM_MODEL,
    DEFAULT_LLM_PROVIDER,
};
pub use query::{
    ContextChunk, ContextEntity, ContextRelationship, DocumentDeletionResult, DocumentInfo,
    EntityDeletionResult, GraphStats, InsertResult, QueryContext, QueryMode, QueryParams,
    QueryResult, QueryStats,
};
pub use relationship::{GraphRelationship, RELATIONSHIP_SEP};
