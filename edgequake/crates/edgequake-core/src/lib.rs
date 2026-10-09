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
pub mod chunking_metadata;
pub mod config;
pub mod conversation_service;
pub mod decision_metadata;
pub mod entity_type_colors;
pub mod env;
pub mod error;
pub mod extract_budget_metadata;
pub mod graph_mapping;
pub mod keyword_extractor;
pub mod llm_roles;
pub mod model_resolution;
#[cfg(feature = "pipeline")]
pub mod orchestrator;
pub mod resource;
pub mod server_config_overrides;
pub mod sota_bridge;
#[cfg(feature = "pipeline")]
pub mod tenant_manager;
pub mod token_budget;
pub mod type_list;
pub mod types;
pub mod utils;
pub mod workspace_model_update;
pub mod workspace_service;
pub mod workspace_vector_resolve;

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
pub use llm_roles::{
    env_extract_role_llm, env_keyword_role_llm, metadata_connection_id, parse_llm_roles_map,
    resolve_extract_role_llm, resolve_role_llm, resolve_role_reasoning_effort,
    role_capability_hint, role_config_from_workspace, role_uses_structured_effort_floor,
    workspace_default_reasoning_effort, LlmRole, ResolvedReasoningEffort, ResolvedRoleLlm,
    RoleLlmConfig,
};
pub use model_resolution::{
    compiled_vision_model_for, env_embedding_provider_model, env_llm_provider_model,
    env_vision_model, env_vision_provider, resolve_embedding_choice, resolve_llm_choice,
    resolve_vision_llm_choice, ModelResolutionSource, ResolvedEmbedding, ResolvedProviderModel,
};
pub use server_config_overrides::{
    current_defaults, install_server_config, merge_config_field, ConfigPriorityMode,
    ServerLlmDefaults,
};

// Re-export tenant manager
#[cfg(feature = "pipeline")]
pub use tenant_manager::{TenantConfig, TenantKBKey, TenantRAGManager, TenantService};

// Re-export workspace service
pub use workspace_service::{
    InMemoryWorkspaceService, UpdateTenantQuotaResult, WorkspaceService, WorkspaceServiceFactory,
};
pub use workspace_vector_resolve::{
    default_workspace_uuid, is_dimension_mismatch_error, resolve_workspace_uuid,
    resolve_workspace_vector_storage, WorkspaceVectorResolveInput, WorkspaceVectorResolvePolicy,
};

// Re-export conversation service
pub use conversation_service::{ConversationService, InMemoryConversationService};

// Re-export token budget
pub use token_budget::{BudgetAllocation, BudgetSource, ContextSource, TokenBudget};

// SPEC-006: Resource safety SSOT
pub use resource::{
    max_batch_upload_files, AdmissionDecision, GraphMaterializationSemaphore, GraphOperation,
    PdfVisionSemaphore, ResourceBudgetConfig, ResourceGuard, MAX_BATCH_UPLOAD_FILES,
    MAX_BATCH_UPLOAD_FILES_ENV, MAX_GRAPH_DEPTH, MAX_GRAPH_NODES, MAX_ORCHESTRATOR_CONTEXT_TOKENS,
    MAX_UPLOAD_BYTES,
};

// Re-export commonly used types
pub use config::Config;
pub use error::{Error, Result};
#[cfg(feature = "pipeline")]
pub use orchestrator::{
    EdgeQuake, EdgeQuakeConfig, EdgeQuakeConfigOverrides, StorageBackend, StorageConfig,
};
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
