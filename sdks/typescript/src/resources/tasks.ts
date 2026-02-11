/**
 * Tasks resource — track async task status.
 *
 * @module resources/tasks
 * @see edgequake/crates/edgequake-api/src/handlers/tasks.rs
 */

import type { TaskInfo } from "../types/tasks.js";
import { Resource } from "./base.js";

export class TasksResource extends Resource {
  /** Get task status by track ID. */
  async get(trackId: string): Promise<TaskInfo> {
    return this._get(`/api/v1/tasks/${trackId}`);
  }

  /** List all tasks. */
  async list(): Promise<TaskInfo[]> {
    return this._get("/api/v1/tasks");
  }

  /** Cancel a running task. */
  async cancel(trackId: string): Promise<void> {
    await this._post(`/api/v1/tasks/${trackId}/cancel`);
  }

  /** Retry a failed task. */
  async retry(trackId: string): Promise<TaskInfo> {
    return this._post(`/api/v1/tasks/${trackId}/retry`);
  }
}
