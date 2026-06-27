//! Upload request/response DTOs.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::defaults::{
    default_enable_gleaning, default_max_gleaning, default_use_llm_summarization,
};

/// Document upload request.
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UploadDocumentRequest {
    /// Document content.
    pub content: String,

    /// Optional document title.
    #[serde(default)]
    pub title: Option<String>,

    /// Optional document metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,

    /// Whether to process asynchronously (default: false for backwards compatibility)
    #[serde(default)]
    pub async_processing: bool,

    /// Optional track ID for batch grouping. If not provided, one will be generated.
    #[serde(default)]
    pub track_id: Option<String>,

    /// Enable gleaning (multiple extraction passes) for higher quality entity extraction.
    #[serde(default = "default_enable_gleaning")]
    pub enable_gleaning: bool,

    /// Maximum number of gleaning passes (1-3 recommended).
    #[serde(default = "default_max_gleaning")]
    pub max_gleaning: usize,

    /// Enable LLM-powered description summarization during merge.
    #[serde(default = "default_use_llm_summarization")]
    pub use_llm_summarization: bool,

    /// Chunk strategy: `fixed`, `recursive`, or `markdown` (SPEC-026 Phase 2).
    #[serde(default)]
    pub chunk_strategy: Option<String>,

    /// Optional chunk size/overlap/separator overrides.
    #[serde(default)]
    pub chunk_options: Option<serde_json::Value>,
}

/// Document upload response.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadDocumentResponse {
    /// Generated document ID.
    pub document_id: String,

    /// Processing status.
    pub status: String,

    /// Task track ID (only set when async_processing is true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Track ID for batch grouping.
    pub track_id: String,

    /// ID of existing document if this is a duplicate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duplicate_of: Option<String>,

    /// Number of chunks created (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_count: Option<usize>,

    /// Number of entities extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_count: Option<usize>,

    /// Number of relationships extracted (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship_count: Option<usize>,

    /// Cost information (only set for sync processing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<DocumentCostInfo>,
}

/// Cost information for a processed document.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct DocumentCostInfo {
    /// Total cost in USD.
    pub total_cost_usd: f64,

    /// Formatted cost string (e.g., "$0.0045").
    pub formatted_cost: String,

    /// Total input tokens used.
    pub input_tokens: usize,

    /// Total output tokens used.
    pub output_tokens: usize,

    /// Total tokens (input + output).
    pub total_tokens: usize,

    /// LLM model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,

    /// Embedding model used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
}

impl DocumentCostInfo {
    /// Format cost to 6 decimal places.
    pub fn format_cost(cost: f64) -> String {
        format!("${:.6}", cost)
    }

    /// Create a new DocumentCostInfo from raw values.
    /// Used when constructing from pipeline result stats.
    pub fn new(
        cost_usd: f64,
        input_tokens: usize,
        output_tokens: usize,
        total_tokens: usize,
        llm_model: Option<String>,
        embedding_model: Option<String>,
    ) -> Self {
        Self {
            total_cost_usd: cost_usd,
            formatted_cost: Self::format_cost(cost_usd),
            input_tokens,
            output_tokens,
            total_tokens,
            llm_model,
            embedding_model,
        }
    }
}
