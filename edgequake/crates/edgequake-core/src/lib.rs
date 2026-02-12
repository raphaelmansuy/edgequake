//! # EdgeQuake Core
//!
//! Core types and utilities for the EdgeQuake RAG system.
//!
//! ## Implements
//!
//! - **FEAT0801**: Core domain types (Document, Chunk, Entity, Relationship)
//! - **FEAT0802**: EdgeQuake orchestrator for RAG coordination
//! - **FEAT0803**: Conversation and workspace services
//! - **FEAT0804**: Multi-tenant isolation support
//!
//! ## Enforces
//!
//! - **BR0801**: All domain entities must be serializable
//! - **BR0802**: Services must be async-trait compatible
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

pub mod cache;
pub mod config;
pub mod conversation_service;
pub mod error;
pub mod keyword_extractor;
#[cfg(feature = "pipeline")]
pub mod orchestrator;
pub mod query;
#[cfg(feature = "pipeline")]
pub mod tenant_manager;
pub mod token_budget;
pub mod types;
pub mod utils;
pub mod workspace_service;

// Production service implementations (feature-gated)
#[cfg(feature = "postgres")]
mod conversation_service_impl;
#[cfg(feature = "postgres")]
mod workspace_service_impl;

// Re-export production services when feature is enabled
#[cfg(feature = "postgres")]
pub use conversation_service_impl::ConversationServiceImpl;
#[cfg(feature = "postgres")]
pub use workspace_service_impl::WorkspaceServiceImpl;

// Legacy aliases for backward compatibility
#[cfg(feature = "postgres")]
#[deprecated(since = "0.2.0", note = "Use ConversationServiceImpl instead")]
pub type PostgresConversationService = ConversationServiceImpl;
#[cfg(feature = "postgres")]
#[deprecated(since = "0.2.0", note = "Use WorkspaceServiceImpl instead")]
pub type PostgresWorkspaceService = WorkspaceServiceImpl;

// Re-export keyword extractor
pub use keyword_extractor::{ExtractedKeywords, KeywordExtractor};

// Re-export tenant manager
#[cfg(feature = "pipeline")]
pub use tenant_manager::{TenantConfig, TenantKBKey, TenantRAGManager, TenantService};

// Re-export workspace service
pub use workspace_service::{InMemoryWorkspaceService, WorkspaceService, WorkspaceServiceFactory};

// Re-export conversation service
pub use conversation_service::{ConversationService, InMemoryConversationService};

// Re-export token budget
pub use token_budget::{BudgetAllocation, BudgetSource, ContextSource, TokenBudget};

// Re-export commonly used types
pub use config::Config;
pub use error::{Error, Result};
#[cfg(feature = "pipeline")]
pub use orchestrator::{EdgeQuake, EdgeQuakeConfig, StorageBackend, StorageConfig};
pub use query::QueryEngine;
pub use types::{
    Chunk, ContextChunk, ContextEntity, ContextRelationship, Conversation, ConversationFilter,
    ConversationMode, ConversationSortField, CreateConversationRequest, CreateFolderRequest,
    CreateMessageRequest, CreateWorkspaceRequest, Document, DocumentInfo, DocumentStatus,
    Embedding, EmbeddingConfig, Folder, GraphEntity, GraphRelationship, GraphStats, ImportError,
    ImportResult, InsertResult, Membership, MembershipRole, Message, MessageContext, MessageRole,
    MessageSource, MetricsSnapshot, MetricsTriggerType, PaginatedConversations, PaginatedMessages,
    PaginationMeta, QueryContext, QueryMode, QueryParams, QueryResult, QueryStats, Tenant,
    TenantContext, TenantPlan, UpdateConversationRequest, UpdateFolderRequest,
    UpdateMessageRequest, UpdateWorkspaceRequest, Workspace, WorkspaceStats,
};
