/**
 * Tasks resource — track async task status.
 *
 * @module resources/tasks
 * @see edgequake/crates/edgequake-api/src/handlers/tasks.rs
 */

import type {
  ListTasksQuery,
  TaskListResponse,
  TaskStatus,
} from "../types/tasks.js";
import { Resource } from "./base.js";

export class TasksResource extends Resource {
  /** Get task status by track ID. */
  async get(trackId: string): Promise<TaskStatus> {
    return this._get(`/api/v1/tasks/${trackId}`);
  }

  /** List tasks with optional filters and pagination. */
  async list(query?: ListTasksQuery): Promise<TaskListResponse> {
    const params = new URLSearchParams();
    if (query?.tenant_id) params.set("tenant_id", query.tenant_id);
    if (query?.workspace_id) params.set("workspace_id", query.workspace_id);
    if (query?.status) params.set("status", query.status);
    if (query?.task_type) params.set("task_type", query.task_type);
    if (query?.page !== undefined) params.set("page", String(query.page));
    if (query?.page_size !== undefined)
      params.set("page_size", String(query.page_size));
    if (query?.sort) params.set("sort", query.sort);
    if (query?.order) params.set("order", query.order);
    const qs = params.toString();
    return this._get(`/api/v1/tasks${qs ? `?${qs}` : ""}`);
  }

  /** Cancel a running task. */
  async cancel(trackId: string): Promise<void> {
    await this._post(`/api/v1/tasks/${trackId}/cancel`);
  }

  /** Retry a failed task. */
  async retry(trackId: string): Promise<TaskStatus> {
    return this._post(`/api/v1/tasks/${trackId}/retry`);
  }
}
