//! Request DTOs for workspace management API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ── Tenant Requests ───────────────────────────────────────────────────────

/// Request to create a new tenant.
///
/// ## Model Configuration (SPEC-032)
///
/// When creating a tenant, you can specify default LLM and embedding models
/// that will be inherited by all new workspaces within this tenant.
///
/// **LLM Examples (for knowledge graph generation, summarization):**
/// - OpenAI: `"gpt-4o-mini"`, `"gpt-4o"`
/// - Ollama: `"gemma3:12b"`, `"llama3.2"`
/// - LM Studio: `"gemma-3n-e4b-it-mlxmodel"`
///
/// **Embedding Examples:**
/// - OpenAI: `"text-embedding-3-small"` (1536 dims), `"text-embedding-3-large"` (3072 dims)
/// - Ollama: `"embeddinggemma:latest"` (768 dims), `"nomic-embed-text"` (768 dims)
/// - LM Studio: `"nomic-ai/nomic-embed-text-v1.5"` (768 dims)
///
/// **Model ID Format:**
/// Models can be specified as `model_name` or `provider/model_name`:
/// - `"gemma3:12b"` - auto-detects provider as "ollama"
/// - `"ollama/gemma3:12b"` - explicit provider
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateTenantRequest {
    /// Tenant name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Plan type (free, basic, pro, enterprise).
    pub plan: Option<String>,

    // === Default LLM Configuration (SPEC-032) ===
    /// Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_LLM_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_model: Option<String>,

    /// Default LLM provider for new workspaces ("openai", "ollama", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_LLM_PROVIDER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_llm_provider: Option<String>,

    // === Default Embedding Configuration (SPEC-032) ===
    /// Default embedding model for new workspaces (e.g., "text-embedding-3-small").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_model: Option<String>,

    /// Default embedding provider for new workspaces ("openai", "ollama", "lmstudio").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_EMBEDDING_PROVIDER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_provider: Option<String>,

    /// Default embedding dimension for new workspaces (e.g., 1536 for OpenAI, 768 for Ollama).
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name or uses EDGEQUAKE_DEFAULT_EMBEDDING_DIMENSION.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_embedding_dimension: Option<usize>,

    // === Default Vision LLM Configuration (SPEC-041) ===
    /// Default Vision LLM model for PDF-to-Markdown extraction (e.g., "gpt-4o", "gemma3:12b").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, server default is used (or upload-time override).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_vision_llm_model: Option<String>,

    /// Default Vision LLM provider for PDF-to-Markdown extraction (e.g., "openai", "ollama").
    /// Workspaces inherit this if not explicitly configured.
    /// If not provided, auto-detected from model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_vision_llm_provider: Option<String>,
}

/// Request to update a tenant.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateTenantRequest {
    /// New tenant name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New plan.
    pub plan: Option<String>,
    /// Whether the tenant is active.
    pub is_active: Option<bool>,
}

// ── Workspace Requests ────────────────────────────────────────────────────

/// Request to create a new workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// When creating a workspace, you can specify both LLM and embedding models.
/// If not provided, server defaults are used (configurable via env vars or models.toml).
///
/// **LLM Examples (for knowledge graph generation, summarization):**
/// - OpenAI: `"gpt-4o-mini"`, `"gpt-4o"`
/// - Ollama: `"gemma3:12b"`, `"llama3.2"`
/// - LM Studio: `"gemma-3n-e4b-it-mlxmodel"`
///
/// **Embedding Examples:**
/// - OpenAI: `"text-embedding-3-small"` (1536 dims), `"text-embedding-3-large"` (3072 dims)
/// - Ollama: `"embeddinggemma:latest"` (768 dims), `"nomic-embed-text"` (768 dims)
/// - LM Studio: `"nomic-ai/nomic-embed-text-v1.5"` (768 dims)
///
/// **Model ID Format:**
/// Models can be specified as `model_name` or `provider/model_name`:
/// - `"gemma3:12b"` - auto-detects provider as "ollama"
/// - `"ollama/gemma3:12b"` - explicit provider
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkspaceApiRequest {
    /// Workspace name.
    pub name: String,
    /// URL-friendly slug (auto-generated if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model for knowledge graph generation, summarization, entity extraction.
    /// Format: "model_name" or "provider/model_name" (e.g., "gemma3:12b", "ollama/gemma3:12b").
    /// If not provided, uses server default from models.toml or EDGEQUAKE_DEFAULT_LLM_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// LLM provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from llm_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// If not provided, uses server default from EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Embedding provider ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from embedding_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Embedding vector dimension override.
    /// If not provided, auto-detected from embedding_model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,

    // === Vision LLM Configuration (SPEC-041) ===
    /// Vision LLM model for PDF-to-Markdown extraction (e.g., "gpt-4o", "gemma3:12b").
    /// If not provided, inherits from tenant default_vision_llm_model, then server default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_model: Option<String>,

    /// Vision LLM provider for PDF-to-Markdown extraction ("openai", "ollama", "lmstudio").
    /// If not provided, auto-detected from vision_llm_model or inherited from tenant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_provider: Option<String>,

    /// Default PDF parser backend for this workspace ("vision" or "edgeparse").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_parser_backend: Option<String>,

    // === Entity Type Configuration (SPEC-085) ===
    /// Custom entity types to extract from documents (e.g., ["MACHINE", "COMPONENT", "DEFECT"]).
    /// Names are normalized to UPPERCASE_UNDERSCORED format.
    /// If not provided, uses the workspace default (server default_entity_types).
    /// Maximum 20 types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// When true (default), limit extraction to listed types; unknown → OTHER.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types_strict: Option<bool>,
}

/// Request to update a workspace.
///
/// ## Model Configuration Updates (SPEC-032)
///
/// Changing LLM provider/model is safe and takes effect immediately for new ingestions.
/// Changing embedding provider/model requires rebuilding vectors (use rebuild-embeddings endpoint).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkspaceApiRequest {
    /// New workspace name.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Maximum number of documents.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// Update LLM model (takes effect on next ingestion).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Update LLM provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Update embedding model.
    /// WARNING: Requires vector rebuild - use rebuild-embeddings endpoint after updating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,

    /// Update embedding provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_provider: Option<String>,

    /// Update embedding dimension.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_dimension: Option<usize>,

    // === Vision LLM Configuration (SPEC-040) ===
    /// Vision LLM model for PDF page image extraction (e.g., "gpt-4o", "gemma3:latest").
    /// When set, this workspace will use this model for PDF → Markdown vision extraction.
    /// Pass empty string or "none" to clear the workspace override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_model: Option<String>,

    /// Vision LLM provider for PDF extraction ("openai", "ollama", "lmstudio").
    /// Pass empty string or "none" to clear the workspace override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_llm_provider: Option<String>,

    /// Default PDF parser backend for this workspace ("vision" or "edgeparse").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_parser_backend: Option<String>,

    /// Entity types for future ingestions (does not rewrite existing graph nodes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types: Option<Vec<String>>,

    /// Strict entity type limit (default true). Set false to allow free-form types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_types_strict: Option<bool>,
}
