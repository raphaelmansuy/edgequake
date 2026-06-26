/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api } from "../client";
import { buildQueryString, withQuery } from "../query-params";

import type {
  EnhancedPipelineStatus,
  PipelineStatus,
  QueueMetrics,
  Tenant,
  TrackStatusResponse,
  Workspace,
} from "@/types";

export async function getTasksList(params?: {
  tenant_id?: string;
  workspace_id?: string;
  status?: string;
  task_type?: string;
  page?: number;
  page_size?: number;
}): Promise<import("@/types").TaskListResponse> {
  // CRITICAL: tenant_id and workspace_id drive multi-tenancy isolation.
  const query = buildQueryString({
    tenant_id: params?.tenant_id,
    workspace_id: params?.workspace_id,
    status: params?.status,
    task_type: params?.task_type,
    page: params?.page,
    page_size: params?.page_size,
  });
  return api.get<import("@/types").TaskListResponse>(withQuery("/tasks", query));
}

export async function getPipelineStatus(
  tenant_id?: string,
  workspace_id?: string,
): Promise<PipelineStatus> {
  try {
    // Derive pipeline status from the tasks list endpoint (multi-tenant scoped).
    const result = await getTasksList({
      tenant_id,
      workspace_id,
      page_size: 50,
    });

    return {
      is_busy: result.statistics.processing > 0,
      running_tasks: result.statistics.processing,
      queued_tasks: result.statistics.pending,
      completed_tasks: result.statistics.indexed,
      failed_tasks: result.statistics.failed,
      tasks: result.tasks,
      statistics: result.statistics,
    };
  } catch (error) {
    // WHY console.warn (not console.error): Next.js dev promotes
    // console.error to a full-screen overlay. A failing pipeline-status
    // fetch must not crash the dashboard. Callers see an empty status.
    console.warn("[getPipelineStatus] Error:", error);
    return {
      is_busy: false,
      running_tasks: 0,
      queued_tasks: 0,
      completed_tasks: 0,
      failed_tasks: 0,
      tasks: [],
    };
  }
}

export async function cancelPipeline(): Promise<void> {
  // Cancel all processing tasks
  const result = await getTasksList({ status: "processing" });
  for (const task of result.tasks) {
    await cancelTask(task.track_id);
  }
}

export async function getTaskStatus(
  taskId: string,
): Promise<import("@/types").TaskResponse> {
  return api.get<import("@/types").TaskResponse>(`/tasks/${taskId}`);
}

export async function cancelTask(taskId: string): Promise<void> {
  return api.post<void>(`/tasks/${taskId}/cancel`);
}

export async function retryTask(
  taskId: string,
): Promise<import("@/types").TaskResponse> {
  return api.post<import("@/types").TaskResponse>(`/tasks/${taskId}/retry`);
}

/**
 * Get track status by track ID.
 * Returns all documents uploaded with a specific track_id, along with status summary.
 */
export async function getTrackStatus(
  trackId: string,
): Promise<TrackStatusResponse> {
  return api.get<TrackStatusResponse>(`/documents/track/${trackId}`);
}

/**
 * Get real-time progress for a specific track ID.
 * Used as fallback when WebSocket is unavailable.
 */
export interface TrackProgressResponse {
  track_id: string;
  document_id: string;
  document_name: string;
  status: import("@/types/ingestion").IngestionStatus;
  progress: import("@/types/ingestion").ProgressDetail;
  started_at?: string;
  updated_at?: string;
  completed_at?: string;
}

export async function getTrackProgress(
  trackId: string,
): Promise<TrackProgressResponse> {
  return api.get<TrackProgressResponse>(`/ingestion/${trackId}/progress`);
}

/**
 * Get progress for multiple tracks at once.
 */
export async function getMultipleTrackProgress(
  trackIds: string[],
): Promise<TrackProgressResponse[]> {
  return api.post<TrackProgressResponse[]>("/ingestion/progress", {
    track_ids: trackIds,
  });
}

/**
 * Get enhanced pipeline status with history messages.
 * Falls back to basic status if enhanced endpoint not available.
 *
 * @param tenant_id - Tenant ID for isolation (optional, will use current context if not provided)
 * @param workspace_id - Workspace ID for isolation (optional, will use current context if not provided)
 *
 * CRITICAL: Always pass tenant_id and workspace_id to ensure multi-tenancy isolation
 */
export async function getEnhancedPipelineStatus(
  tenant_id?: string,
  workspace_id?: string,
): Promise<EnhancedPipelineStatus> {
  try {
    // Try enhanced endpoint first with tenant/workspace context
    const params = new URLSearchParams();
    if (tenant_id) params.append("tenant_id", tenant_id);
    if (workspace_id) params.append("workspace_id", workspace_id);
    const query = params.toString();

    return await api.get<EnhancedPipelineStatus>(
      `/pipeline/status${query ? `?${query}` : ""}`,
    );
  } catch {
    // Fall back to basic status derived from tasks with tenant/workspace filtering
    const result = await getTasksList({
      tenant_id,
      workspace_id,
      page_size: 50,
    });

    return {
      is_busy: result.statistics.processing > 0,
      job_name:
        result.statistics.processing > 0 ? "Processing documents" : undefined,
      job_start: undefined,
      total_documents: 0,
      processed_documents: 0,
      current_batch: 0,
      total_batches: 0,
      latest_message:
        result.statistics.processing > 0
          ? `Processing ${result.statistics.processing} document(s)...`
          : undefined,
      history_messages: [],
      cancellation_requested: false,
      pending_tasks: result.statistics.pending,
      processing_tasks: result.statistics.processing,
      completed_tasks: result.statistics.indexed,
      failed_tasks: result.statistics.failed,
    };
  }
}

/**
 * Request pipeline cancellation.
 */
export async function requestPipelineCancellation(): Promise<{
  status: string;
}> {
  try {
    return await api.post<{ status: string }>("/pipeline/cancel");
  } catch {
    // Fall back to cancelling individual tasks
    await cancelPipeline();
    return { status: "cancellation_requested" };
  }
}

/**
 * Get queue metrics for task queue visibility.
 *
 * @implements FEAT0570 - Queue metrics API
 * @implements OODA-21 - Queue metrics frontend integration
 * @implements OODA-04 - Multi-tenant isolation for queue metrics
 *
 * Returns worker utilization, throughput, wait times, and queue ETA.
 *
 * **CRITICAL**: This function MUST receive tenant/workspace parameters to ensure
 * queue metrics only show activity for the current workspace. Without these
 * parameters, users could see processing activity from other tenants.
 *
 * @param tenantId - Optional tenant ID for filtering. If not provided, uses header context.
 * @param workspaceId - Optional workspace ID for filtering. If not provided, uses header context.
 */
export async function getQueueMetrics(
  tenantId?: string,
  workspaceId?: string,
): Promise<QueueMetrics> {
  try {
    // Build query params for tenant/workspace isolation
    const params = new URLSearchParams();
    if (tenantId) params.append("tenant_id", tenantId);
    if (workspaceId) params.append("workspace_id", workspaceId);
    const query = params.toString();

    return await api.get<QueueMetrics>(
      `/pipeline/queue-metrics${query ? `?${query}` : ""}`,
    );
  } catch {
    // Return default metrics if endpoint not available
    return {
      pending_count: 0,
      processing_count: 0,
      active_workers: 0,
      max_workers: 1,
      worker_utilization: 0,
      avg_wait_time_seconds: 0,
      max_wait_time_seconds: 0,
      throughput_per_minute: 0,
      estimated_queue_time_seconds: 0,
      rate_limited: false,
      timestamp: new Date().toISOString(),
    };
  }
}
