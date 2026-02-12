/**
 * @module types
 * @description Core TypeScript type definitions for EdgeQuake WebUI.
 * Provides shared interfaces for API responses, graph data, and UI state.
 *
 * ## Key Type Categories
 *
 * - **Graph types**: GraphNode, GraphEdge, KnowledgeGraph
 * - **Document types**: Document, UploadDocumentRequest/Response
 * - **Query types**: QueryRequest, QueryResponse, QueryContext
 * - **Auth types**: AuthState, LoginRequest/Response
 * - **Tenant types**: Tenant, Workspace
 *
 * @implements FEAT0001 - Document model for ingestion
 * @implements FEAT0007 - Query request/response types
 * @implements FEAT0601 - Graph node/edge types for visualization
 * @implements FEAT0870 - Auth state and token types
 *
 * @see {@link docs/features.md} for feature specifications
 */

// ============================================================================
// Re-export Types from Modules
// ============================================================================

// Ingestion types for real-time progress tracking
export * from "./ingestion";

// Cost tracking types for LLM cost monitoring
export * from "./cost";

// Lineage types for document provenance tracking
export * from "./lineage";

// ============================================================================
// Graph types
// ============================================================================

export interface GraphNode {
  id: string;
  label: string;
  node_type: string;
  description?: string;
  degree?: number;
  properties?: Record<string, unknown>;
  created_at?: string;
  updated_at?: string;
}

export interface GraphEdge {
  id: string;
  source: string;
  target: string;
  relationship_type: string;
  weight: number;
  description?: string;
  source_ids: string[];
  properties?: Record<string, unknown>;
  created_at: string;
}

export interface KnowledgeGraph {
  nodes: GraphNode[];
  edges: GraphEdge[];
  metadata: {
    node_count: number;
    edge_count: number;
    entity_types: string[];
    relationship_types: string[];
  };
  /** Whether the graph was truncated due to max_nodes limit */
  is_truncated?: boolean;
  /** Total node count in storage (before truncation) */
  total_nodes?: number;
  /** Total edge count in storage (before truncation) */
  total_edges?: number;
}

// Document types
export interface Document {
  id: string;
  title?: string | null;
  content?: string;
  source_type?: "file" | "text" | "url" | "pdf" | "markdown";
  status?:
    | "pending"
    | "processing"
    | "completed"
    | "partial_failure"
    | "failed"
    | "indexed"
    | "cancelled";
  error_message?: string;
  file_name?: string;
  file_size?: number;
  mime_type?: string;
  chunk_count?: number;
  entity_count?: number;
  /** Number of relationships extracted. */
  relationship_count?: number;
  /** First 200 characters of document content (preview). */
  content_summary?: string;
  /** Total length of document content in characters. */
  content_length?: number;
  /** Content hash for deduplication (SHA-256). */
  content_hash?: string;
  /** Track ID for batch grouping. */
  track_id?: string;
  /** Tenant ID for multi-tenancy. */
  tenant_id?: string;
  /** Workspace ID for multi-tenancy. */
  workspace_id?: string;
  created_at?: string;
  updated_at?: string;
  processed_at?: string;
  /** Extraction lineage information. */
  lineage?: DocumentLineage;
  /** Total processing cost in USD. */
  cost_usd?: number;
  /** Input tokens used for processing. */
  input_tokens?: number;
  /** Output tokens used for processing. */
  output_tokens?: number;
  /** Total tokens (input + output). */
  total_tokens?: number;
  /** LLM model used for processing. */
  llm_model?: string;
  /** Embedding model used for processing. */
  embedding_model?: string;

  // ========================================================================
  // SPEC-002: Unified Ingestion Pipeline Fields
  // ========================================================================

  /**
   * Current ingestion stage (aligned with UnifiedStage enum).
   * Stages: uploading, converting, preprocessing, chunking, extracting,
   * gleaning, merging, summarizing, embedding, storing, completed, failed.
   * @implements SPEC-002
   */
  current_stage?: string;

  /**
   * Progress within current stage (0.0 to 1.0).
   * @implements SPEC-002
   */
  stage_progress?: number;

  /**
   * Human-readable message for current stage.
   * @implements SPEC-002
   */
  stage_message?: string;

  /**
   * Linked PDF document ID (only set if source_type is "pdf").
   * Used to fetch PDF content for viewing.
   * @implements SPEC-002
   */
  pdf_id?: string;
}

/** Extraction lineage information for a document. */
export interface DocumentLineage {
  /** LLM model used for entity extraction. */
  llm_model?: string;
  /** Embedding model used for vector embeddings. */
  embedding_model?: string;
  /** Embedding dimensions. */
  embedding_dimensions?: number;
  /** List of keywords extracted. */
  keywords?: string[];
  /** Entity types extracted. */
  entity_types?: string[];
  /** Relationship types extracted. */
  relationship_types?: string[];
  /** Chunking strategy used. */
  chunking_strategy?: string;
  /** Average chunk size in characters. */
  avg_chunk_size?: number;
  /** Processing duration in milliseconds. */
  processing_duration_ms?: number;
  /** Entity extraction duration in milliseconds. */
  entity_extraction_ms?: number;
  /** Relationship extraction duration in milliseconds. */
  relationship_extraction_ms?: number;
  /** Graph indexing duration in milliseconds. */
  graph_indexing_ms?: number;
  /** Vector embedding duration in milliseconds. */
  vector_embedding_ms?: number;
}

/** Status counts for document filtering. */
export interface DocumentStatusCounts {
  pending: number;
  processing: number;
  completed: number;
  partial_failure: number;
  failed: number;
  cancelled: number;
}

/** Response from list documents API. */
export interface ListDocumentsResponse {
  documents: Document[];
  total: number;
  page: number;
  page_size: number;
  /** Total number of pages. */
  total_pages?: number;
  /** Whether there are more pages after this one. */
  has_more?: boolean;
  status_counts: DocumentStatusCounts;
}

/** Track status response for batch grouping (Phase 2). */
export interface TrackStatusResponse {
  /** Track ID for this batch. */
  track_id: string;
  /** When the first document was uploaded. */
  created_at?: string;
  /** Documents in this batch. */
  documents: Document[];
  /** Total number of documents. */
  total_count: number;
  /** Status summary for the batch. */
  status_summary: DocumentStatusCounts;
  /** Whether processing is complete (all docs completed or failed). */
  is_complete: boolean;
  /** Latest processing message. */
  latest_message?: string;
}

/** Pipeline message from the server (Phase 3). */
export interface PipelineMessage {
  timestamp: string;
  level: "info" | "warn" | "error";
  message: string;
}

/** Enhanced pipeline status response (Phase 3). */
export interface EnhancedPipelineStatus {
  /** Whether the pipeline is currently processing. */
  is_busy: boolean;
  /** Current job name. */
  job_name?: string;
  /** When the current job started. */
  job_start?: string;
  /** Total documents to process. */
  total_documents: number;
  /** Documents processed so far. */
  processed_documents: number;
  /** Current batch number. */
  current_batch: number;
  /** Total number of batches. */
  total_batches: number;
  /** Latest status message. */
  latest_message?: string;
  /** History of pipeline messages. */
  history_messages: PipelineMessage[];
  /** Whether cancellation has been requested. */
  cancellation_requested: boolean;
  /** Number of pending tasks. */
  pending_tasks: number;
  /** Number of processing tasks. */
  processing_tasks: number;
  /** Number of completed tasks. */
  completed_tasks: number;
  /** Number of failed tasks. */
  failed_tasks: number;
}

/**
 * Queue metrics for Objective B: Workspace-Level Task Queue Visibility.
 *
 * @implements FEAT0570 - Queue metrics API
 * @implements OODA-21 - Queue metrics frontend integration
 */
export interface QueueMetrics {
  /** Number of pending tasks in the queue. */
  pending_count: number;
  /** Number of tasks currently being processed. */
  processing_count: number;
  /** Number of workers currently active. */
  active_workers: number;
  /** Maximum configured workers. */
  max_workers: number;
  /** Worker utilization percentage (0-100). */
  worker_utilization: number;
  /** Average wait time in seconds for recently started tasks. */
  avg_wait_time_seconds: number;
  /** Maximum wait time in seconds among pending tasks. */
  max_wait_time_seconds: number;
  /** Current throughput in documents per minute. */
  throughput_per_minute: number;
  /** Estimated time to clear the queue in seconds. */
  estimated_queue_time_seconds: number;
  /** Whether the system is currently rate limited. */
  rate_limited: boolean;
  /** When these metrics were captured (ISO 8601). */
  timestamp: string;
}

export interface DocumentChunk {
  id: string;
  document_id: string;
  content: string;
  chunk_index: number;
  tokens: number;
  embedding_id?: string;
}

export interface UploadDocumentRequest {
  content: string;
  title?: string;
  source_type?: "text" | "file" | "url";
  metadata?: Record<string, unknown>;
  async_processing?: boolean;
  /** Optional track ID for batch grouping. If not provided, one will be generated. */
  track_id?: string;
}

export interface UploadDocumentResponse {
  document_id: string;
  status: string;
  task_id?: string;
  /** Track ID for batch grouping. */
  track_id: string;
  /** ID of existing document if this is a duplicate. */
  duplicate_of?: string;
  chunk_count?: number;
  entity_count?: number;
  relationship_count?: number;
}

// PDF Upload types
export interface PdfUploadOptions {
  /** Enable vision LLM processing (default: true) */
  enable_vision?: boolean;
  /** Vision provider to use (default: "openai") */
  vision_provider?: string;
  /** Vision model override (optional) */
  vision_model?: string;
  /** Document title (optional) */
  title?: string;
  /** Custom metadata (optional) */
  metadata?: Record<string, unknown>;
  /** Batch tracking ID (optional) - OODA-19 */
  track_id?: string;
}

export interface PdfMetadata {
  filename: string;
  file_size_bytes: number;
  page_count?: number;
  sha256_checksum: string;
}

export interface PdfUploadResponse {
  pdf_id: string;
  document_id?: string;
  status: string;
  task_id: string;
  track_id?: string;
  message: string;
  estimated_time_seconds: number;
  metadata: PdfMetadata;
  duplicate_of?: string;
}

// Query types
export type QueryMode = "local" | "global" | "hybrid" | "naive";

export interface QueryRequest {
  query: string;
  mode: QueryMode;
  top_k?: number;
  max_tokens?: number;
  temperature?: number;
  stream?: boolean;
  only_context?: boolean;
}

export interface QueryContext {
  chunks: Array<{
    content: string;
    document_id: string;
    score: number;
    file_path?: string;
    start_line?: number;
    end_line?: number;
    chunk_index?: number;
  }>;
  entities: Array<{
    id: string;
    label: string;
    relevance: number;
    /** Source document ID for citation link */
    source_document_id?: string;
    /** Original file path for citation display */
    source_file_path?: string;
    /** Source chunk IDs for provenance */
    source_chunk_ids?: string[];
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
    relevance: number;
    /** Source document ID for citation link */
    source_document_id?: string;
    /** Original file path for citation display */
    source_file_path?: string;
  }>;
}

export interface QueryResponse {
  answer: string;
  context: QueryContext;
  mode: QueryMode;
  tokens_used: number;
  duration_ms: number;
}

export interface QueryStreamChunk {
  type: "token" | "context" | "done" | "error";
  content?: string;
  context?: QueryContext;
  error?: string;
  tokens_used?: number;
  duration_ms?: number;
  /** LLM provider used for this query (lineage tracking). @implements SPEC-032 */
  llm_provider?: string;
  /** LLM model used for this query (lineage tracking). @implements SPEC-032 */
  llm_model?: string;
}

// Auth types
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
  user: {
    id: string;
    username: string;
    email?: string;
    roles: string[];
  };
}

export interface AuthState {
  isAuthenticated: boolean;
  user: LoginResponse["user"] | null;
  accessToken: string | null;
  refreshToken: string | null;
  expiresAt: number | null;
}

// Tenant types
export interface Tenant {
  /** Tenant unique identifier (UUID). */
  id: string;
  /** Tenant display name. */
  name: string;
  /** URL-friendly slug. */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Subscription plan (free, basic, pro, enterprise). */
  plan?: string;
  /** Whether the tenant is active. */
  is_active?: boolean;
  /** Maximum workspaces allowed for this tenant. */
  max_workspaces?: number;

  // === Default LLM Configuration (SPEC-032) ===

  /**
   * Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
   * @implements SPEC-032: Tenant-level LLM configuration defaults
   */
  default_llm_model?: string;
  /**
   * Default LLM provider for new workspaces (e.g., "ollama", "openai", "lmstudio").
   * @implements SPEC-032: Tenant-level LLM configuration defaults
   */
  default_llm_provider?: string;
  /**
   * Fully qualified default LLM model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  default_llm_full_id?: string;

  // === Default Embedding Configuration (SPEC-032) ===

  /**
   * Default embedding model for new workspaces (e.g., "text-embedding-3-small").
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_model?: string;
  /**
   * Default embedding provider for new workspaces (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_provider?: string;
  /**
   * Default embedding dimension for new workspaces (e.g., 1536 for OpenAI, 768 for Ollama).
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_dimension?: number;
  /**
   * Fully qualified default embedding model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  default_embedding_full_id?: string;

  /** Creation timestamp. */
  created_at: string;
  /** Last update timestamp. */
  updated_at?: string;
}

export interface Workspace {
  /** Workspace unique identifier (UUID). */
  id: string;
  /** Parent tenant ID. */
  tenant_id: string;
  /** Workspace display name. */
  name: string;
  /** URL-friendly slug. */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Whether the workspace is active. */
  is_active?: boolean;
  /** Maximum documents allowed. */
  max_documents?: number;
  /** Number of documents (from stats, may not be returned inline). */
  document_count?: number;
  /** Number of entities (from stats, may not be returned inline). */
  entity_count?: number;
  /**
   * LLM model name for knowledge graph generation and summarization.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_model?: string;
  /**
   * LLM provider ID (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_provider?: string;
  /**
   * Fully qualified LLM model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  llm_full_id?: string;
  /**
   * Embedding model name (e.g., "text-embedding-3-small").
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_model?: string;
  /**
   * Embedding provider ID (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_provider?: string;
  /**
   * Embedding dimension (e.g., 1536 for OpenAI, 768 for Ollama).
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_dimension?: number;
  /**
   * Fully qualified embedding model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  embedding_full_id?: string;
  /** Creation timestamp. */
  created_at: string;
  /** Last update timestamp. */
  updated_at?: string;
}

/**
 * Request to create a new workspace.
 * @implements SPEC-032: Workspace LLM and embedding configuration on creation
 */
export interface CreateWorkspaceRequest {
  /** Workspace display name. */
  name: string;
  /** URL-friendly slug (optional, auto-generated from name). */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Maximum documents allowed (optional). */
  max_documents?: number;
  /**
   * LLM model name for knowledge graph generation and summarization.
   * If not provided, uses server default (e.g., "gemma3:12b").
   * Can be a full ID like "ollama/gemma3:12b" for explicit provider.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_model?: string;
  /**
   * LLM provider ID (e.g., "openai", "ollama", "lmstudio").
   * If not provided, auto-detected from llm_model.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_provider?: string;
  /**
   * Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
   * If not provided, uses server default.
   * Can be a full ID like "openai/text-embedding-3-small" for explicit provider.
   */
  embedding_model?: string;
  /**
   * Embedding provider ID (e.g., "openai", "ollama", "lmstudio").
   * If not provided, auto-detected from embedding_model.
   */
  embedding_provider?: string;
  /**
   * Embedding dimension override.
   * If not provided, auto-detected from embedding_model.
   */
  embedding_dimension?: number;
}

/**
 * Workspace statistics response.
 * @implements SPEC-032: Workspace stats for detail page
 */
export interface WorkspaceStats {
  /** Total number of documents in workspace */
  document_count: number;
  /** Total number of entities extracted */
  entity_count: number;
  /** Total number of relationships */
  relationship_count: number;
  /** Number of distinct entity types (e.g., PERSON, ORGANIZATION, …) */
  entity_type_count?: number;
  /** Total number of text chunks */
  chunk_count: number;
  /** Total number of vectors stored */
  vector_count?: number;
  /** Total characters processed */
  total_characters?: number;
  /** Total tokens used */
  total_tokens?: number;
}

// Task/Pipeline types

/** Detailed error information for task failures. */
export interface TaskError {
  /** Human-readable error message. */
  message: string;
  /** Which processing step failed (chunking, embedding, extraction, indexing). */
  step: string;
  /** Technical reason for the failure. */
  reason: string;
  /** Suggested action to resolve the issue. */
  suggestion: string;
  /** Whether the task can be retried. */
  retryable: boolean;
}

export interface TaskResponse {
  track_id: string;
  tenant_id: string;
  workspace_id: string;
  task_type: string;
  status: "pending" | "processing" | "indexed" | "failed" | "cancelled";
  created_at: string;
  updated_at: string;
  started_at?: string;
  completed_at?: string;
  /** Simple error message (backward compatible). */
  error_message?: string;
  /** Detailed error information with step, reason, suggestion. */
  error?: TaskError;
  retry_count: number;
  max_retries: number;
  progress?: Record<string, unknown>;
  result?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

export interface TaskListResponse {
  tasks: TaskResponse[];
  pagination: {
    total: number;
    page: number;
    page_size: number;
    total_pages: number;
  };
  statistics: {
    pending: number;
    processing: number;
    indexed: number;
    failed: number;
    cancelled: number;
  };
}

// Derived pipeline status (for UI compatibility)
export interface PipelineStatus {
  is_busy: boolean;
  running_tasks: number;
  queued_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  tasks: TaskResponse[];
  statistics?: TaskListResponse["statistics"];
}

// Health types
export interface HealthResponse {
  status: "healthy" | "degraded" | "unhealthy";
  version: string;
  /** Build metadata (git hash, timestamp, build number) */
  build_info?: {
    git_hash: string;
    git_branch: string;
    build_timestamp: string;
    build_number: string;
  };
  uptime_seconds?: number;
  workspace_id?: string;
  components: {
    database?: "up" | "down";
    llm_provider: "up" | "down" | boolean;
    storage: "up" | "down" | boolean;
    kv_storage?: boolean;
    vector_storage?: boolean;
    graph_storage?: boolean;
  };
  /** LLM provider name (e.g., "openai", "mock", "ollama") */
  llm_provider_name?: string;
  /** Current active provider configuration (LLM and embedding) */
  providers?: {
    llm: {
      name: string;
      model: string;
    };
    embedding: {
      name: string;
      model: string;
      dimension: number;
    };
  };
  /** Database schema health (PostgreSQL only) */
  schema?: {
    latest_version?: number;
    migrations_applied: number;
    last_applied_at?: string;
  };
  /** Whether PDF storage is enabled */
  pdf_storage_enabled?: boolean;
}

// Entity types
/** Entity returned from the API - matches backend EntityResponse. */
export interface Entity {
  /** Unique entity ID. */
  id: string;
  /** Entity name (used as label for display). */
  entity_name: string;
  /** Entity type (e.g., PERSON, ORGANIZATION). */
  entity_type: string;
  /** Human-readable description. */
  description?: string;
  /** Source document ID. */
  source_id?: string;
  /** Node degree (number of connections). */
  degree?: number;
  /** Additional metadata. */
  metadata?: Record<string, unknown>;
  /** ISO timestamp of creation. */
  created_at?: string;
  /** ISO timestamp of last update. */
  updated_at?: string;

  // Legacy fields for backward compatibility
  /** @deprecated Use entity_name instead. */
  label?: string;
  /** @deprecated Use metadata instead. */
  properties?: Record<string, unknown>;
  /** @deprecated Use source_id instead. */
  source_ids?: string[];
  /** @deprecated Aliases are now in metadata. */
  aliases?: string[];
}

export interface MergeEntitiesRequest {
  source_ids: string[];
  target_label: string;
  target_type?: string;
}

export interface MergeEntitiesResponse {
  merged_entity: Entity;
  merged_count: number;
}

// Relationship types
/** Relationship returned from the API - matches backend RelationshipResponse. */
export interface Relationship {
  /** Unique relationship ID. */
  id: string;
  /** Source entity ID. */
  src_id: string;
  /** Target entity ID. */
  tgt_id: string;
  /** Relationship type/label. */
  relation_type: string;
  /** Keywords describing the relationship. */
  keywords?: string;
  /** Weight/strength of the relationship. */
  weight?: number;
  /** Human-readable description. */
  description?: string;
  /** Source document ID. */
  source_id?: string;
  /** ISO timestamp of creation. */
  created_at?: string;

  // Legacy fields for backward compatibility
  /** @deprecated Use src_id instead. */
  source_entity_id?: string;
  /** @deprecated Use tgt_id instead. */
  target_entity_id?: string;
  /** @deprecated Use relation_type instead. */
  relationship_type?: string;
  /** @deprecated Use source_id instead. */
  source_ids?: string[];
  /** @deprecated Use metadata in Entity instead. */
  properties?: Record<string, unknown>;
}

// Settings types
export interface GraphSettings {
  showLabels: boolean;
  showEdgeLabels: boolean;
  nodeSize: "small" | "medium" | "large";
  edgeThickness: "thin" | "medium" | "thick";
  layout:
    | "force"
    | "circular"
    | "random"
    | "circlepack"
    | "noverlaps"
    | "force-directed"
    | "hierarchical";
  colorBy: "type" | "community" | "degree";
  enableNodeDrag?: boolean;
  highlightNeighbors?: boolean;
  hideUnselectedEdges?: boolean;
}

export interface QuerySettings {
  mode: QueryMode;
  topK: number;
  maxTokens: number;
  temperature: number;
  stream: boolean;
  /** Enable reranking for improved retrieval precision */
  enableRerank: boolean;
  /** Top K results to keep after reranking */
  rerankTopK: number;
  /**
   * LLM provider ID to use for queries (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Provider selection in query interface
   */
  provider?: string;
  /**
   * Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b").
   * Combined with provider to form full model ID: "ollama/gemma3:12b"
   * @implements SPEC-032: Full model selection in query interface
   */
  model?: string;
}

export interface IngestionSettings {
  /** Enable gleaning (multiple extraction passes) for higher quality entity extraction */
  enableGleaning: boolean;
  /** Maximum number of gleaning passes (1-3 recommended) */
  maxGleaning: number;
  /** Enable LLM-powered description summarization during merge */
  useLLMSummarization: boolean;
}

export interface AppSettings {
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "ja" | "ko";
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
  ingestionSettings: IngestionSettings;
}

// API error types
export interface ApiError {
  message: string;
  code?: string;
  details?: Record<string, unknown>;
  status: number;
}

// Pagination types
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  /** Total number of pages. */
  total_pages?: number;
  has_more: boolean;
}

export interface PaginationParams {
  page?: number;
  page_size?: number;
  sort_by?: string;
  sort_order?: "asc" | "desc";
}

// Query history
export interface QueryHistoryItem {
  id: string;
  query: string;
  mode: QueryMode;
  response?: string;
  timestamp: string;
  isFavorite: boolean;
}

// ============================================================================
// Conversation Types (Server-synced)
// ============================================================================

export type ConversationMode = "local" | "global" | "hybrid" | "naive" | "mix";

export interface ServerConversation {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  title: string;
  mode: ConversationMode;
  is_pinned: boolean;
  is_archived: boolean;
  folder_id?: string | null;
  share_id?: string | null;
  message_count: number;
  last_message_preview?: string | null;
  meta: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ConversationWithMessages extends ServerConversation {
  messages: ServerMessage[];
}

export interface ServerMessage {
  id: string;
  conversation_id: string;
  parent_id?: string | null;
  role: "user" | "assistant" | "system";
  content: string;
  mode?: ConversationMode | null;
  tokens_used?: number | null;
  duration_ms?: number | null;
  thinking_time_ms?: number | null;
  context?: ServerMessageContext | null;
  is_error: boolean;
  created_at: string;
  updated_at: string;
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llm_provider?: string | null;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llm_model?: string | null;
}

export interface ServerMessageContext {
  sources?: MessageSource[];
  entities?: ServerContextEntity[];
  relationships?: ServerContextRelationship[];
  thinking?: string;
}

export interface MessageSource {
  id: string;
  title?: string;
  content: string;
  score: number;
  /** Source type: chunk, entity, or relationship */
  source_type?: string;
  /** Document ID for citation link */
  document_id?: string;
  /** Original file path for citation display */
  file_path?: string;
}

/** Entity returned in context with source tracking */
export interface ServerContextEntity {
  name: string;
  entity_type: string;
  description?: string;
  score: number;
  /** Source document ID for citation link */
  source_document_id?: string;
  /** Original file path for citation display */
  source_file_path?: string;
  /** Source chunk IDs for provenance */
  source_chunk_ids?: string[];
}

/** Relationship returned in context with source tracking */
export interface ServerContextRelationship {
  source: string;
  target: string;
  relation_type: string;
  description?: string;
  score: number;
  /** Source document ID for citation link */
  source_document_id?: string;
  /** Original file path for citation display */
  source_file_path?: string;
}

export interface ConversationFolder {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  name: string;
  parent_id?: string | null;
  position: number;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Conversation Request/Response Types
// ============================================================================

export interface CreateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  folder_id?: string | null;
}

export interface UpdateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  is_pinned?: boolean;
  is_archived?: boolean;
  folder_id?: string | null;
}

export interface CreateMessageRequest {
  content: string;
  role: "user";
  parent_id?: string | null;
  stream?: boolean;
}

export interface UpdateMessageRequest {
  content?: string;
  tokens_used?: number;
  duration_ms?: number;
  thinking_time_ms?: number;
  context?: ServerMessageContext;
  is_error?: boolean;
}

// ============================================================================
// Conversation Pagination Types
// ============================================================================

export interface CursorPaginationParams {
  cursor?: string;
  limit?: number;
}

export interface ConversationFilterParams {
  mode?: ConversationMode[];
  archived?: boolean;
  pinned?: boolean;
  folder_id?: string;
  search?: string;
  date_from?: string;
  date_to?: string;
  sort?: "updated_at" | "created_at" | "title";
  order?: "asc" | "desc";
}

export interface PaginatedConversations {
  items: ServerConversation[];
  pagination: CursorPaginationMeta;
}

export interface PaginatedMessages {
  items: ServerMessage[];
  pagination: CursorPaginationMeta;
}

export interface CursorPaginationMeta {
  next_cursor?: string | null;
  prev_cursor?: string | null;
  total: number;
  has_more: boolean;
}

export interface ShareConversationResponse {
  share_id: string;
  share_url: string;
}

// ============================================================================
// Conversation Import Types (localStorage migration)
// ============================================================================

export interface ImportConversationsRequest {
  conversations: LocalStorageConversation[];
}

export interface LocalStorageConversation {
  id: string;
  title: string;
  messages: {
    id: string;
    role: "user" | "assistant";
    content: string;
    mode?: ConversationMode;
    tokensUsed?: number;
    durationMs?: number;
    thinkingTimeMs?: number;
    context?: ServerMessageContext;
    isError?: boolean;
    timestamp?: number;
  }[];
  createdAt: number;
  updatedAt: number;
}

export interface ImportConversationsResponse {
  imported: number;
  failed: number;
  errors?: { id: string; error: string }[];
}

// ============================================================================
// Folder Request Types
// ============================================================================

export interface CreateFolderRequest {
  name: string;
  parent_id?: string | null;
}

export interface UpdateFolderRequest {
  name?: string;
  parent_id?: string | null;
  position?: number;
}
