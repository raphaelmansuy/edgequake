/** Task queue, pipeline, and health check types. */

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
