//! Workspace request types (create, update) and statistics.

use std::collections::HashMap;

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
    /// Maximum 50 types per workspace.
    ///
    /// Domain presets (entity + relation) live in the WebUI `kg-schema-presets`
    /// catalog (SPEC-114). General entities match server `default_entity_types()`.
    ///
    /// @implements SPEC-085: Custom entity configuration from UI
    /// @implements SPEC-114: paired with relation_types
    pub entity_types: Option<Vec<String>>,

    /// When true (default), unknown extracted types remap to OTHER/CONCEPT.
    /// When false, LLM may emit additional type labels without forced OTHER catch-all.
    pub entity_types_strict: Option<bool>,

    /// Natural-language output language for entity/relationship string values (SPEC-096).
    ///
    /// Allowlisted display names (e.g. `"Chinese"`, `"French"`). Omit to inherit
    /// `EDGEQUAKE_EXTRACTION_LANGUAGE` / English. Pass `""` or `"none"` to clear.
    pub extraction_language: Option<String>,

    /// SPEC-116: `inherit` | `adaptive` | `fixed` (Acc-fair = fixed + 1200/100).
    pub chunking_mode: Option<String>,
    /// SPEC-116: fixed chunk token size (default 1200 when mode=fixed).
    pub chunk_token_size: Option<u32>,
    /// SPEC-116: fixed chunk overlap (default 100 when mode=fixed).
    pub chunk_overlap_token_size: Option<u32>,

    /// SPEC-117: `inherit` | `custom` (omit = leave unchanged on update).
    pub extract_budget_mode: Option<String>,
    /// SPEC-117: max entities per LLM extraction response.
    pub extract_max_entities: Option<u32>,
    /// SPEC-117: max total entity+relationship rows per response.
    pub extract_max_records: Option<u32>,

    /// SPEC-160: `llm` | `decision` | `inherit` (omit = leave unchanged).
    pub extraction_mode: Option<String>,
    /// SPEC-160: `strict` | `balanced` | `recall` | `inherit`.
    pub decision_gate_preset: Option<String>,
    /// SPEC-160: decision model name, or `inherit`.
    pub decision_model: Option<String>,
    /// SPEC-160: questions per request, `1`-`16` (`0` clears).
    pub decision_pack_size: Option<i64>,
    /// SPEC-160: workspace opt-in for the decision engine.
    pub decision_enabled: Option<bool>,

    /// Custom entity-type → hex color map for graph visualization (SPEC-102).
    /// Keys normalized UPPERCASE; values `#RGB` / `#RRGGBB`. Empty map clears.
    pub entity_type_colors: Option<HashMap<String, String>>,

    /// Relation type allow-list for extraction (SPEC-114).
    /// Empty/None → free-form relation labels (backward compatible).
    pub relation_types: Option<Vec<String>>,

    /// When true (default when list non-empty), unknown relations remap.
    pub relation_types_strict: Option<bool>,

    /// Optional domain preset id for UX honesty (`general`…`finance`/`custom`).
    pub kg_schema_preset: Option<String>,

    /// Typed edge constraints (SPEC-114b). Empty/None → unconstrained endpoints.
    pub relation_edges: Option<Vec<crate::type_list::RelationEdge>>,

    /// SPEC-109: workspace metadata `default_reasoning_effort`.
    pub default_reasoning_effort: Option<String>,

    /// SPEC-109: merge into workspace metadata `llm_roles` (role → config).
    pub llm_roles: Option<serde_json::Value>,

    // === Vision extract (SPEC-015V) ===
    pub vision_extract_images: Option<bool>,
    pub vision_extract_charts: Option<bool>,
    pub vision_extract_figures: Option<bool>,
    pub vision_page_system_prompt: Option<String>,
    pub vision_image_system_prompt: Option<String>,
    pub vision_chart_system_prompt: Option<String>,
    pub vision_figure_system_prompt: Option<String>,
    /// SPEC-180: `graph` (default) or `normal`. Immutable after create.
    pub rag_profile: Option<crate::rag_profile::RagProfile>,
    /// SPEC-180: `text` (default) or `text,image`. Immutable after create.
    pub embedding_modalities: Option<crate::rag_profile::EmbeddingModalities>,
    /// SPEC-180: BM25 text-search configuration. Immutable after create.
    pub fts_regconfig: Option<crate::rag_profile::FtsRegconfig>,
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

    /// SPEC-180: choose the retrieval profile (immutable after create).
    pub fn with_rag_profile(mut self, profile: crate::rag_profile::RagProfile) -> Self {
        self.rag_profile = Some(profile);
        self
    }

    /// SPEC-180: choose what the workspace embeds (immutable after create).
    pub fn with_embedding_modalities(
        mut self,
        modalities: crate::rag_profile::EmbeddingModalities,
    ) -> Self {
        self.embedding_modalities = Some(modalities);
        self
    }

    /// SPEC-180: effective profile, modalities and regconfig after defaults, validated.
    pub fn resolved_profile(
        &self,
    ) -> Result<
        (
            crate::rag_profile::RagProfile,
            crate::rag_profile::EmbeddingModalities,
            crate::rag_profile::FtsRegconfig,
        ),
        crate::rag_profile::ProfileError,
    > {
        let profile = self.rag_profile.unwrap_or_default();
        let modalities = self.embedding_modalities.unwrap_or_default();
        crate::rag_profile::check_create_combination(profile, modalities)?;
        Ok((profile, modalities, self.fts_regconfig.unwrap_or_default()))
    }
}

impl UpdateWorkspaceRequest {
    /// SPEC-180: the first immutable profile field present, as a coded error.
    /// A value equal to the current one is rejected too, so a client cannot "confirm" it.
    pub fn check_profile_untouched(&self) -> Result<(), crate::rag_profile::ProfileError> {
        use crate::rag_profile::{codes, ProfileError};
        let hit = [
            (
                &self.rag_profile,
                codes::RAG_PROFILE_IMMUTABLE,
                "rag_profile",
            ),
            (
                &self.embedding_modalities,
                codes::EMBEDDING_MODALITIES_IMMUTABLE,
                "embedding_modalities",
            ),
            (
                &self.fts_regconfig,
                codes::FTS_REGCONFIG_IMMUTABLE,
                "fts_regconfig",
            ),
        ]
        .into_iter()
        .find(|(value, _, _)| value.is_some());
        match hit {
            Some((_, code, field)) => Err(ProfileError::new(
                code,
                format!("{field} cannot change after the workspace is created"),
            )),
            None => Ok(()),
        }
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
    /// SPEC-181: set to archive (a timestamp) or clear to restore (`Some(None)`).
    /// `None` leaves the current value untouched.
    pub archived_at: Option<Option<chrono::DateTime<chrono::Utc>>>,
    /// SPEC-181: who archived it.
    pub archived_by: Option<Uuid>,
    /// Max documents quota.
    pub max_documents: Option<usize>,
    /// New LLM model for entity extraction (optional).
    /// Takes effect immediately for new document ingestions.
    /// Set to `Some("")` or `Some("none")` to reset to server/env defaults.
    pub llm_model: Option<String>,
    /// New LLM provider (optional).
    /// Set to `Some("")` or `Some("none")` to reset to server/env defaults.
    pub llm_provider: Option<String>,
    /// New embedding model (optional).
    /// WARNING: Requires vector rebuild - use rebuild-embeddings endpoint.
    /// Set to `Some("")` or `Some("none")` to reset to server/env defaults.
    pub embedding_model: Option<String>,
    /// New embedding provider (optional).
    /// Set to `Some("")` or `Some("none")` to reset to server/env defaults.
    pub embedding_provider: Option<String>,
    /// New embedding dimension (optional).
    /// Set to `Some(0)` when clearing embedding overrides.
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

    /// Strict entity type enforcement (default true when omitted).
    pub entity_types_strict: Option<bool>,

    /// Extraction output language override (SPEC-096). Omit = leave unchanged;
    /// `""` / `"none"` clears metadata key.
    pub extraction_language: Option<String>,

    /// SPEC-116 chunking mode. Omit = leave unchanged; `inherit`/`none` clears.
    pub chunking_mode: Option<String>,
    /// SPEC-116 fixed size when mode=fixed.
    pub chunk_token_size: Option<u32>,
    /// SPEC-116 fixed overlap when mode=fixed.
    pub chunk_overlap_token_size: Option<u32>,

    /// SPEC-117: `inherit` clears keys; `custom` + ints sets budget. Omit = leave unchanged.
    pub extract_budget_mode: Option<String>,
    /// SPEC-117: max entities per LLM extraction response.
    pub extract_max_entities: Option<u32>,
    /// SPEC-117: max total entity+relationship rows per response.
    pub extract_max_records: Option<u32>,

    /// SPEC-160: `llm` | `decision` | `inherit` (omit = leave unchanged).
    pub extraction_mode: Option<String>,
    /// SPEC-160: `strict` | `balanced` | `recall` | `inherit`.
    pub decision_gate_preset: Option<String>,
    /// SPEC-160: decision model name, or `inherit`.
    pub decision_model: Option<String>,
    /// SPEC-160: questions per request, `1`-`16` (`0` clears).
    pub decision_pack_size: Option<i64>,
    /// SPEC-160: workspace opt-in for the decision engine.
    pub decision_enabled: Option<bool>,

    /// Custom entity-type → hex color map (SPEC-102). Omit = leave unchanged;
    /// empty map clears metadata key.
    pub entity_type_colors: Option<HashMap<String, String>>,

    /// Relation type allow-list (SPEC-114). Omit = leave unchanged;
    /// empty list clears (free-form).
    pub relation_types: Option<Vec<String>>,

    /// Strict relation type enforcement (SPEC-114).
    pub relation_types_strict: Option<bool>,

    /// Domain preset id (SPEC-114). `""` / `"none"` clears.
    pub kg_schema_preset: Option<String>,

    /// Typed edge constraints (SPEC-114b). Omit = leave unchanged; empty clears.
    pub relation_edges: Option<Vec<crate::type_list::RelationEdge>>,

    /// SPEC-109: set/clear workspace `default_reasoning_effort` metadata.
    /// Empty string / `"none"` / `"auto"` clears.
    pub default_reasoning_effort: Option<String>,

    /// SPEC-109: merge role effort overrides into metadata `llm_roles`.
    pub llm_roles: Option<serde_json::Value>,

    // === Vision extract (SPEC-015V) ===
    pub vision_extract_images: Option<bool>,
    pub vision_extract_charts: Option<bool>,
    pub vision_extract_figures: Option<bool>,
    pub vision_page_system_prompt: Option<String>,
    pub vision_image_system_prompt: Option<String>,
    pub vision_chart_system_prompt: Option<String>,
    pub vision_figure_system_prompt: Option<String>,
    /// SPEC-180: presence marker. Any value is rejected (`rag_profile_immutable`).
    pub rag_profile: Option<String>,
    /// SPEC-180: presence marker. Any value is rejected (`embedding_modalities_immutable`).
    pub embedding_modalities: Option<String>,
    /// SPEC-180: presence marker. Any value is rejected (`fts_regconfig_immutable`).
    pub fts_regconfig: Option<String>,
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
