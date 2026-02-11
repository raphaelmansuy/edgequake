/**
 * Task tracking types.
 *
 * @module types/tasks
 * @see edgequake/crates/edgequake-api/src/handlers/tasks_types.rs
 */

import type { Timestamp, TaskStatusValue } from "./common.js";

// ── Task ──────────────────────────────────────────────────────

export interface TaskStatus {
  track_id: string;
  status: TaskStatusValue;
  progress?: number;
  message?: string;
  result?: Record<string, unknown>;
  error?: string;
  created_at: Timestamp;
  updated_at?: Timestamp;
}

export interface TaskInfo {
  track_id: string;
  status: TaskStatusValue;
  task_type?: string;
  created_at: Timestamp;
  updated_at?: Timestamp;
}

export interface ListTasksQuery {
  status?: TaskStatusValue;
  limit?: number;
  offset?: number;
}

// ── Pipeline ──────────────────────────────────────────────────

export interface PipelineStatus {
  status: string;
  active_tasks: number;
  queued_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
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
