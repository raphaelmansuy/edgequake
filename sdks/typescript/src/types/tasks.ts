/**
 * Task tracking types.
 *
 * @module types/tasks
 * @see edgequake/crates/edgequake-api/src/handlers/tasks_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Task ──────────────────────────────────────────────────────

/** Detailed error information for failed tasks. */
export interface TaskErrorDetail {
  message: string;
  step: string;
  reason: string;
  suggestion: string;
  retryable: boolean;
}

/** Full task response matching Rust TaskResponse. */
export interface TaskStatus {
  track_id: string;
  tenant_id: string;
  workspace_id: string;
  task_type: string;
  status: string;
  created_at: Timestamp;
  updated_at: Timestamp;
  started_at?: Timestamp;
  completed_at?: Timestamp;
  error_message?: string;
  error?: TaskErrorDetail;
  retry_count: number;
  max_retries: number;
  progress?: Record<string, unknown>;
  result?: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

/** Simplified task info (alias for TaskStatus). */
export interface TaskInfo extends TaskStatus {}

/** Query params for listing tasks. */
export interface ListTasksQuery {
  tenant_id?: string;
  workspace_id?: string;
  status?: string;
  task_type?: string;
  page?: number;
  page_size?: number;
  sort?: string;
  order?: string;
}

/** Paginated task list response. */
export interface TaskListResponse {
  tasks: TaskStatus[];
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

// ── Pipeline ──────────────────────────────────────────────────

/** Pipeline status matching Rust EnhancedPipelineStatusResponse. */
export interface PipelineStatus {
  is_busy: boolean;
  job_name?: string;
  job_start?: string;
  total_documents: number;
  processed_documents: number;
  current_batch: number;
  total_batches: number;
  latest_message?: string;
  history_messages: PipelineMessage[];
  cancellation_requested: boolean;
  pending_tasks: number;
  processing_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
}

export interface PipelineMessage {
  timestamp: string;
  level: string;
  message: string;
}

export interface QueueMetrics {
  queue_depth: number;
  active_workers: number;
  avg_processing_time_ms?: number;
  throughput_per_minute?: number;
}

// ── Cost Tracking ─────────────────────────────────────────────

export interface ModelPricing {
  models: Array<{
    provider: string;
    model: string;
    input_cost_per_1k: number;
    output_cost_per_1k: number;
  }>;
}

export interface CostEstimateRequest {
  content_length: number;
  operation: "extraction" | "query" | "embedding";
  model?: string;
}

export interface CostEstimate {
  estimated_cost: number;
  estimated_tokens: number;
  model: string;
  currency: string;
}
