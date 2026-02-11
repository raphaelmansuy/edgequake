/**
 * Common types shared across all resource namespaces.
 *
 * @module types/common
 */

// ── Pagination ────────────────────────────────────────────────

/** A single page of results. */
export interface Page<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}

/** Standard list query parameters (offset-based). */
export interface ListQuery {
  limit?: number;
  offset?: number;
}

/** Page-based query parameters. */
export interface PageQuery {
  page?: number;
  per_page?: number;
}

// ── Async Task ────────────────────────────────────────────────

/** Status of an asynchronous task. */
export type TaskStatusValue =
  | "pending"
  | "processing"
  | "completed"
  | "failed"
  | "cancelled";

/** Response when an async task is created. */
export interface TaskResponse {
  track_id: string;
  status: TaskStatusValue;
  message?: string;
}

// ── Timestamp ─────────────────────────────────────────────────

/** ISO 8601 timestamp string. */
export type Timestamp = string;

// ── Generic API Response Wrappers ─────────────────────────────

/** Wrapper for delete operations that return a count. */
export interface DeleteResponse {
  deleted: number;
  message?: string;
}

/** Wrapper for bulk operations. */
export interface BulkOperationResponse {
  success: number;
  failed: number;
  errors?: Array<{ id: string; error: string }>;
}
