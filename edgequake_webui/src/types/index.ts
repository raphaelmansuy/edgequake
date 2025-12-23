// Graph types
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
}

// Document types
export interface Document {
  id: string;
  title?: string | null;
  content?: string;
  source_type?: "file" | "text" | "url";
  status?: "pending" | "processing" | "completed" | "failed" | "indexed";
  error_message?: string;
  file_name?: string;
  file_size?: number;
  mime_type?: string;
  chunk_count?: number;
  entity_count?: number;
  /** First 200 characters of document content (preview). */
  content_summary?: string;
  /** Total length of document content in characters. */
  content_length?: number;
  /** Track ID for batch grouping. */
  track_id?: string;
  created_at?: string;
  updated_at?: string;
  processed_at?: string;
}

/** Status counts for document filtering. */
export interface DocumentStatusCounts {
  pending: number;
  processing: number;
  completed: number;
  failed: number;
}

/** Response from list documents API. */
export interface ListDocumentsResponse {
  documents: Document[];
  total: number;
  page: number;
  page_size: number;
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
  }>;
  entities: Array<{
    id: string;
    label: string;
    relevance: number;
  }>;
  relationships: Array<{
    source: string;
    target: string;
    type: string;
    relevance: number;
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
  id: string;
  name: string;
  description?: string;
  created_at: string;
}

export interface Workspace {
  id: string;
  tenant_id: string;
  name: string;
  description?: string;
  document_count: number;
  entity_count: number;
  created_at: string;
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
  uptime_seconds: number;
  components: {
    database: "up" | "down";
    llm_provider: "up" | "down";
    storage: "up" | "down";
  };
}

// Entity types
export interface Entity {
  id: string;
  label: string;
  entity_type: string;
  description?: string;
  aliases: string[];
  source_ids: string[];
  properties: Record<string, unknown>;
  created_at: string;
  updated_at: string;
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
export interface Relationship {
  id: string;
  source_entity_id: string;
  target_entity_id: string;
  relationship_type: string;
  weight: number;
  description?: string;
  source_ids: string[];
  properties: Record<string, unknown>;
  created_at: string;
}

// Settings types
export interface GraphSettings {
  showLabels: boolean;
  showEdgeLabels: boolean;
  nodeSize: "small" | "medium" | "large";
  edgeThickness: "thin" | "medium" | "thick";
  layout: "force" | "circular" | "random";
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
}

export interface AppSettings {
  theme: "light" | "dark" | "system";
  language: "en" | "zh" | "ja" | "ko";
  graphSettings: GraphSettings;
  querySettings: QuerySettings;
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
