//! Workspace request types (create, update) and statistics.

use edgequake_pdf::PdfParserBackend;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::workspace::Workspace;

/// Request to create a new workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// If `embedding_model` is not provided, the workspace will use server defaults:
/// - `EDGEQUAKE_DEFAULT_EMBEDDING_MODEL` or "text-embedding-3-small"
/// - Provider and dimension auto-detected from model name
///
/// If `llm_model` is not provided, the workspace will use server defaults:
/// - `EDGEQUAKE_DEFAULT_LLM_MODEL` or "gemma4:latest" (Ollama)
/// - Provider auto-detected from model name
///
/// ## Model ID Format
///
/// Models can be specified as:
/// - Simple name: "gemma3:12b" (provider auto-detected)
/// - Full ID: "ollama/gemma3:12b" (provider parsed from full ID)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateWorkspaceRequest {
    /// Human-readable name.
    pub name: String,
    /// Optional slug (generated from name if not provided).
    pub slug: Option<String>,
    /// Optional description.
    pub description: Option<String>,
    /// Optional max documents quota.
    pub max_documents: Option<usize>,

    // === LLM Configuration (SPEC-032) ===
    /// LLM model name (e.g., "gemma3:12b", "gpt-4o-mini").
    /// If None, uses server default from EDGEQUAKE_DEFAULT_LLM_MODEL.
    /// Can be a full ID like "ollama/gemma3:12b" for explicit provider.
    pub llm_model: Option<String>,

    /// LLM provider (e.g., "ollama", "openai", "lmstudio").
    /// If None, auto-detected from llm_model.
    pub llm_provider: Option<String>,

    // === Embedding Configuration (SPEC-032) ===
    /// Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
    /// If None, uses server default from EDGEQUAKE_DEFAULT_EMBEDDING_MODEL.
    /// Can be a full ID like "openai/text-embedding-3-small" for explicit provider.
    pub embedding_model: Option<String>,

    /// Embedding provider (e.g., "openai", "ollama", "lmstudio").
    /// If None, auto-detected from embedding_model.
    pub embedding_provider: Option<String>,

    /// Embedding dimension override.
    /// If None, auto-detected from embedding_model.
    pub embedding_dimension: Option<usize>,

    // === Vision LLM Configuration (SPEC-041) ===
    /// Vision LLM model for PDF-to-Markdown extraction (e.g., "gpt-4o", "gemma3:12b").
    /// If None, falls back to tenant default then server default.
    pub vision_llm_model: Option<String>,

    /// Vision LLM provider for PDF-to-Markdown extraction (e.g., "openai", "ollama").
    /// If None, auto-detected from vision_llm_model.
    pub vision_llm_provider: Option<String>,

    /// Default PDF parser backend for this workspace.
    pub pdf_parser_backend: Option<PdfParserBackend>,

    // === Entity Extraction Configuration (SPEC-085) ===
    /// Custom entity types for extraction pipeline.
    ///
    /// If None, uses [`edgequake_pipeline::prompts::default_entity_types()`].
    /// Types are normalized to UPPERCASE_UNDERSCORED and deduplicated.
    /// Maximum 20 types per workspace.
    ///
    /// ## Presets
    ///
    /// Common domain presets are available in the UI:
    /// - **General**: PERSON, ORGANIZATION, LOCATION, EVENT, CONCEPT, TECHNOLOGY, PRODUCT, DATE, DOCUMENT
    /// - **Manufacturing**: adds MACHINE, COMPONENT, DEFECT, MEASUREMENT, PROCESS, MATERIAL
    /// - **Healthcare**: adds SYMPTOM, DRUG, DIAGNOSIS, PROCEDURE, PATIENT, CONDITION
    /// - **Legal**: adds CONTRACT, CLAUSE, PARTY, REGULATION, JURISDICTION, CASE
    /// - **Research**: adds PAPER, METHOD, DATASET, HYPOTHESIS, FINDING, METRIC
    ///
    /// @implements SPEC-085: Custom entity configuration from UI
    pub entity_types: Option<Vec<String>>,
}

impl CreateWorkspaceRequest {
    /// Create a new request with just a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    // === LLM Configuration Builder Methods (SPEC-032) ===

    /// Set the LLM model.
    ///
    /// # Arguments
    ///
    /// * `model` - Model name or full ID (e.g., "gemma3:12b" or "ollama/gemma3:12b")
    pub fn with_llm_model(mut self, model: impl Into<String>) -> Self {
        self.llm_model = Some(model.into());
        self
    }

    /// Set the LLM provider.
    pub fn with_llm_provider(mut self, provider: impl Into<String>) -> Self {
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set complete LLM configuration from a full model ID.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Full model ID in `provider/model` format (e.g., "ollama/gemma3:12b")
    ///
    /// If the format is invalid, sets the entire string as the model name.
    pub fn with_llm_full_id(mut self, full_id: impl Into<String>) -> Self {
        let full_id = full_id.into();
        if let Some((provider, model)) = Workspace::parse_model_id(&full_id) {
            self.llm_provider = Some(provider);
            self.llm_model = Some(model);
        } else {
            self.llm_model = Some(full_id);
        }
        self
    }

    // === Embedding Configuration Builder Methods (SPEC-032) ===

    /// Set the embedding model.
    pub fn with_embedding_model(mut self, model: impl Into<String>) -> Self {
        self.embedding_model = Some(model.into());
        self
    }

    /// Set the embedding provider.
    pub fn with_embedding_provider(mut self, provider: impl Into<String>) -> Self {
        self.embedding_provider = Some(provider.into());
        self
    }

    /// Set the embedding dimension.
    pub fn with_embedding_dimension(mut self, dimension: usize) -> Self {
        self.embedding_dimension = Some(dimension);
        self
    }

    /// Set complete embedding configuration from a full model ID.
    ///
    /// # Arguments
    ///
    /// * `full_id` - Full model ID in `provider/model` format (e.g., "openai/text-embedding-3-small")
    ///
    /// If the format is invalid, sets the entire string as the model name.
    pub fn with_embedding_full_id(mut self, full_id: impl Into<String>) -> Self {
        let full_id = full_id.into();
        if let Some((provider, model)) = Workspace::parse_model_id(&full_id) {
            self.embedding_provider = Some(provider);
            self.embedding_model = Some(model);
        } else {
            self.embedding_model = Some(full_id);
        }
        self
    }

    /// Set complete LLM configuration with model and provider.
    ///
    /// # Arguments
    ///
    /// * `model` - LLM model name (e.g., "gemma3:12b", "gpt-4o-mini")
    /// * `provider` - Provider name (e.g., "ollama", "openai", "lmstudio")
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::CreateWorkspaceRequest;
    ///
    /// let req = CreateWorkspaceRequest::new("My Workspace")
    ///     .with_llm_config("gemma3:12b", "ollama");
    /// assert_eq!(req.llm_model, Some("gemma3:12b".to_string()));
    /// assert_eq!(req.llm_provider, Some("ollama".to_string()));
    /// ```
    pub fn with_llm_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        self.llm_model = Some(model.into());
        self.llm_provider = Some(provider.into());
        self
    }

    /// Set complete embedding configuration with model, provider, and dimension.
    ///
    /// # Arguments
    ///
    /// * `model` - Embedding model name (e.g., "text-embedding-3-small")
    /// * `provider` - Provider name (e.g., "openai", "ollama", "lmstudio")
    /// * `dimension` - Vector dimension (e.g., 1536, 768, 3072)
    ///
    /// # Example
    ///
    /// ```
    /// use edgequake_core::CreateWorkspaceRequest;
    ///
    /// let req = CreateWorkspaceRequest::new("My Workspace")
    ///     .with_embedding_config("text-embedding-3-small", "openai", 1536);
    /// assert_eq!(req.embedding_model, Some("text-embedding-3-small".to_string()));
    /// assert_eq!(req.embedding_provider, Some("openai".to_string()));
    /// assert_eq!(req.embedding_dimension, Some(1536));
    /// ```
    pub fn with_embedding_config(
        mut self,
        model: impl Into<String>,
        provider: impl Into<String>,
        dimension: usize,
    ) -> Self {
        self.embedding_model = Some(model.into());
        self.embedding_provider = Some(provider.into());
        self.embedding_dimension = Some(dimension);
        self
    }

    // === Entity Type Configuration Builder Methods (SPEC-085) ===

    /// Set custom entity types for this workspace.
    ///
    /// Types are normalized to UPPERCASE_UNDERSCORED and deduplicated.
    /// Maximum 20 types are accepted; extras are silently dropped.
    ///
    /// @implements SPEC-085: Custom entity configuration
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = Some(types);
        self
    }
}

/// Request to update a workspace.
///
/// ## Model Configuration (SPEC-032)
///
/// - LLM model/provider changes take effect immediately for new ingestions
/// - Embedding model changes require vector rebuild (use rebuild-embeddings endpoint)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateWorkspaceRequest {
    /// New name (optional).
    pub name: Option<String>,
    /// New description (optional).
    pub description: Option<String>,
    /// Whether the workspace is active.
    pub is_active: Option<bool>,
    /// Max documents quota.
    pub max_documents: Option<usize>,
    /// New LLM model for entity extraction (optional).
    /// Takes effect immediately for new document ingestions.
    pub llm_model: Option<String>,
    /// New LLM provider (optional).
    pub llm_provider: Option<String>,
    /// New embedding model (optional).
    /// WARNING: Requires vector rebuild - use rebuild-embeddings endpoint.
    pub embedding_model: Option<String>,
    /// New embedding provider (optional).
    pub embedding_provider: Option<String>,
    /// New embedding dimension (optional).
    pub embedding_dimension: Option<usize>,
    /// New Vision LLM provider for PDF extraction (optional).
    /// Set to Some("") or Some("none") to clear it.
    pub vision_llm_provider: Option<String>,
    /// New Vision LLM model for PDF extraction (optional).
    /// Set to Some("") or Some("none") to clear it.
    pub vision_llm_model: Option<String>,
    /// New PDF parser backend for PDF extraction (optional).
    /// Set to Some("") or Some("none") to clear the workspace override.
    pub pdf_parser_backend: Option<String>,
    /// Entity types for future ingestions (SPEC-085 / GitHub #216).
    pub entity_types: Option<Vec<String>>,
}

/// Statistics for a workspace.
///
/// WHY embedding_count: Mission requirement - "Ensure metric likes number of
/// Entities, Relationships, Embeddings per document" are tracked.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkspaceStats {
    /// Workspace ID.
    pub workspace_id: Uuid,
    /// Total documents.
    pub document_count: usize,
    /// Total entities (graph nodes).
    pub entity_count: usize,
    /// Total relationships (graph edges).
    pub relationship_count: usize,
    /// Total chunks (text segments).
    pub chunk_count: usize,
    /// Total embeddings (vector representations).
    pub embedding_count: usize,
    /// Storage used in bytes.
    pub storage_bytes: usize,
}
