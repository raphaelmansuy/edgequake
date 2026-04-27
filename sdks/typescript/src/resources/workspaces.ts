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

  /** Create or update a knowledge injection (text). */
  async putInjection(
    workspaceId: string,
    body: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    return this._put(`/api/v1/workspaces/${workspaceId}/injection`, body);
  }

  /** Upload injection file (multipart). */
  async putInjectionFile(
    workspaceId: string,
    name: string,
    file: File | Blob,
  ): Promise<Record<string, unknown>> {
    return this.transport.upload(
      `/api/v1/workspaces/${workspaceId}/injection/file`,
      file,
      { name },
      { method: "PUT" },
    );
  }

  async listInjections(
    workspaceId: string,
  ): Promise<Record<string, unknown>> {
    return this._get(`/api/v1/workspaces/${workspaceId}/injections`);
  }

  async getInjection(
    workspaceId: string,
    injectionId: string,
  ): Promise<Record<string, unknown>> {
    return this._get(
      `/api/v1/workspaces/${workspaceId}/injections/${injectionId}`,
    );
  }

  async patchInjection(
    workspaceId: string,
    injectionId: string,
    body: Record<string, unknown>,
  ): Promise<Record<string, unknown>> {
    return this._patch(
      `/api/v1/workspaces/${workspaceId}/injections/${injectionId}`,
      body,
    );
  }

  async deleteInjection(
    workspaceId: string,
    injectionId: string,
  ): Promise<void> {
    await this._del(
      `/api/v1/workspaces/${workspaceId}/injections/${injectionId}`,
    );
  }
}
