use crate::types::DocumentStatus;
use serde::{Deserialize, Serialize};

/// Query mode for retrieval.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueryMode {
    /// Local mode: Focus on entity-centric retrieval.
    Local,

    /// Global mode: Use high-level graph structure.
    Global,

    /// Hybrid mode: Combine local and global.
    #[default]
    Hybrid,

    /// Mix mode: Adaptive selection based on query.
    Mix,

    /// Naive mode: Simple vector search only.
    Naive,

    /// Bypass mode: Skip retrieval, direct LLM query.
    Bypass,
}

/// Query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    /// Query mode.
    pub mode: QueryMode,

    /// Whether to stream the response.
    pub stream: bool,

    /// Whether to return only context (no LLM generation).
    pub only_need_context: bool,

    /// Whether to return only the prompt.
    pub only_need_prompt: bool,

    /// Number of top entities to retrieve.
    pub top_k: usize,

    /// Maximum tokens for response.
    pub max_tokens: Option<usize>,

    /// Enable history tracking.
    pub enable_history: bool,

    /// History context to include.
    pub history_context: Option<String>,

    /// Tenant ID for multi-tenancy.
    pub tenant_id: Option<String>,

    /// Workspace ID for multi-tenancy.
    pub workspace_id: Option<String>,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            mode: QueryMode::Hybrid,
            stream: false,
            only_need_context: false,
            only_need_prompt: false,
            top_k: 60,
            max_tokens: None,
            enable_history: false,
            history_context: None,
            tenant_id: None,
            workspace_id: None,
        }
    }
}

impl QueryParams {
    /// Create new query params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set query mode.
    pub fn with_mode(mut self, mode: QueryMode) -> Self {
        self.mode = mode;
        self
    }

    /// Enable streaming.
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Set top_k.
    pub fn with_top_k(mut self, k: usize) -> Self {
        self.top_k = k;
        self
    }

    /// Return only context without LLM generation.
    pub fn context_only(mut self) -> Self {
        self.only_need_context = true;
        self
    }
}

/// Query result from EdgeQuake.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// The generated response.
    pub response: String,

    /// Query mode that was used.
    pub mode: QueryMode,

    /// Retrieved context.
    pub context: QueryContext,

    /// Statistics about the query.
    pub stats: QueryStats,
}

/// Retrieved context for a query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryContext {
    /// Retrieved entities.
    pub entities: Vec<ContextEntity>,

    /// Retrieved relationships.
    pub relationships: Vec<ContextRelationship>,

    /// Retrieved text chunks.
    pub chunks: Vec<ContextChunk>,
}

/// An entity in the query context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEntity {
    /// Entity name/ID.
    pub name: String,

    /// Entity type.
    pub entity_type: String,

    /// Entity description.
    pub description: String,

    /// Relevance score.
    pub score: f32,
}

/// A relationship in the query context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRelationship {
    /// Source entity.
    pub source: String,

    /// Target entity.
    pub target: String,

    /// Relationship type.
    pub relation_type: String,

    /// Description.
    pub description: String,

    /// Relevance score.
    pub score: f32,
}

/// A text chunk in the query context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    /// Chunk ID.
    pub chunk_id: String,

    /// Document ID.
    pub document_id: String,

    /// Chunk content.
    pub content: String,

    /// Relevance score.
    pub score: f32,

    /// Start line number in the document.
    pub start_line: Option<usize>,

    /// End line number in the document.
    pub end_line: Option<usize>,

    /// Chunk index in the document.
    pub chunk_index: Option<usize>,
}

/// Statistics from a query.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QueryStats {
    /// Time spent in retrieval (ms).
    pub retrieval_time_ms: u64,

    /// Time spent in LLM generation (ms).
    pub generation_time_ms: u64,

    /// Total time (ms).
    pub total_time_ms: u64,

    /// Number of entities retrieved.
    pub entities_retrieved: usize,

    /// Number of relationships retrieved.
    pub relationships_retrieved: usize,

    /// Number of chunks retrieved.
    pub chunks_retrieved: usize,

    /// Number of keywords extracted.
    pub keywords_extracted: usize,

    /// Tokens used in prompt.
    pub prompt_tokens: usize,

    /// Tokens in response.
    pub response_tokens: usize,
}

/// Document insertion result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertResult {
    /// Document ID.
    pub document_id: String,

    /// Whether the insertion was successful.
    pub success: bool,

    /// Number of chunks created.
    pub chunks_created: usize,

    /// Number of entities extracted.
    pub entities_extracted: usize,

    /// Number of relationships extracted.
    pub relationships_extracted: usize,

    /// Processing time in milliseconds.
    pub processing_time_ms: u64,

    /// Any error message.
    pub error: Option<String>,
}

/// Status of a document in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    /// Document ID.
    pub id: String,

    /// Original filename if available.
    pub filename: Option<String>,

    /// Document status.
    pub status: DocumentStatus,

    /// Number of chunks.
    pub chunk_count: usize,

    /// Number of entities.
    pub entity_count: usize,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: Option<String>,
}

/// Graph statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes.
    pub node_count: usize,

    /// Total number of edges.
    pub edge_count: usize,

    /// Number of entity types.
    pub entity_type_count: usize,

    /// Number of relationship types.
    pub relationship_type_count: usize,

    /// Top entity types by count.
    pub top_entity_types: Vec<(String, usize)>,

    /// Top relationship types by count.
    pub top_relationship_types: Vec<(String, usize)>,
}

/// Result of document deletion with cascade impact.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentDeletionResult {
    /// The deleted document ID.
    pub document_id: String,

    /// Number of chunks deleted.
    pub chunks_deleted: usize,

    /// Number of entities completely removed (all sources gone).
    pub entities_removed: usize,

    /// Number of entities updated (some sources removed).
    pub entities_updated: usize,

    /// Number of relationships completely removed.
    pub relationships_removed: usize,

    /// Number of relationships updated.
    pub relationships_updated: usize,
}

/// Result of entity deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityDeletionResult {
    /// The deleted entity name.
    pub entity_name: String,

    /// Whether the entity was deleted.
    pub deleted: bool,

    /// Number of relationships deleted.
    pub relationships_deleted: usize,
}
