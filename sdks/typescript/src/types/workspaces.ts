/**
 * Workspace and tenant types.
 *
 * @module types/workspaces
 * @see edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Tenants ───────────────────────────────────────────────────

export interface CreateTenantRequest {
  name: string;
  metadata?: Record<string, unknown>;
}

export interface TenantInfo {
  tenant_id: string;
  name: string;
  created_at: Timestamp;
}

export interface TenantDetail extends TenantInfo {
  workspace_count: number;
  metadata?: Record<string, unknown>;
}

export interface TenantResponse extends TenantInfo {}

export interface UpdateTenantRequest {
  name?: string;
  metadata?: Record<string, unknown>;
}

// ── Workspaces ────────────────────────────────────────────────

export interface CreateWorkspaceRequest {
  name: string;
  slug?: string;
  description?: string;
  metadata?: Record<string, unknown>;
}

export interface WorkspaceInfo {
  workspace_id: string;
  tenant_id: string;
  name: string;
  slug: string;
  created_at: Timestamp;
}

export interface WorkspaceDetail extends WorkspaceInfo {
  description?: string;
  metadata?: Record<string, unknown>;
  document_count?: number;
  entity_count?: number;
}

export interface WorkspaceResponse extends WorkspaceInfo {}

export interface UpdateWorkspaceRequest {
  name?: string;
  slug?: string;
  description?: string;
  metadata?: Record<string, unknown>;
}

export interface WorkspaceStats {
  workspace_id: string;
  document_count: number;
  entity_count: number;
  relationship_count: number;
  chunk_count: number;
  storage_bytes?: number;
}

export interface MetricsHistoryQuery {
  from?: string;
  to?: string;
  interval?: string;
}

export interface MetricsHistory {
  workspace_id: string;
  data_points: Array<{
    timestamp: Timestamp;
    document_count: number;
    entity_count: number;
    relationship_count: number;
  }>;
}
