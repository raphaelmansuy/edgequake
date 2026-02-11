/**
 * Workspaces resource — workspace management and actions.
 *
 * @module resources/workspaces
 * @see edgequake/crates/edgequake-api/src/handlers/workspaces.rs
 */

import type {
  MetricsHistory,
  UpdateWorkspaceRequest,
  WorkspaceDetail,
  WorkspaceInfo,
  WorkspaceStats,
} from "../types/workspaces.js";
import { Resource } from "./base.js";

export class WorkspacesResource extends Resource {
  /** Get workspace details. */
  async get(workspaceId: string): Promise<WorkspaceDetail> {
    return this._get(`/api/v1/workspaces/${workspaceId}`);
  }

  /** Update workspace settings. */
  async update(
    workspaceId: string,
    request: UpdateWorkspaceRequest,
  ): Promise<WorkspaceInfo> {
    return this._put(`/api/v1/workspaces/${workspaceId}`, request);
  }

  /** Delete a workspace. */
  async delete(workspaceId: string): Promise<void> {
    await this._del(`/api/v1/workspaces/${workspaceId}`);
  }

  /** Get workspace statistics (document count, entity count, etc.). */
  async stats(workspaceId: string): Promise<WorkspaceStats> {
    return this._get(`/api/v1/workspaces/${workspaceId}/stats`);
  }

  /** Get metrics history for a workspace. */
  async metricsHistory(workspaceId: string): Promise<MetricsHistory> {
    return this._get(`/api/v1/workspaces/${workspaceId}/metrics-history`);
  }

  /** Trigger a metrics snapshot. */
  async triggerMetricsSnapshot(workspaceId: string): Promise<void> {
    await this._post(`/api/v1/workspaces/${workspaceId}/metrics-snapshot`);
  }

  /** Rebuild embeddings for a workspace. */
  async rebuildEmbeddings(workspaceId: string): Promise<void> {
    await this._post(`/api/v1/workspaces/${workspaceId}/rebuild-embeddings`);
  }

  /** Rebuild knowledge graph (after LLM model change). */
  async rebuildKnowledgeGraph(workspaceId: string): Promise<void> {
    await this._post(
      `/api/v1/workspaces/${workspaceId}/rebuild-knowledge-graph`,
    );
  }

  /** Reprocess all documents in a workspace. */
  async reprocessDocuments(workspaceId: string): Promise<void> {
    await this._post(`/api/v1/workspaces/${workspaceId}/reprocess-documents`);
  }
}
