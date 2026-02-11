/**
 * Tenants resource — multi-tenant management.
 *
 * @module resources/tenants
 * @see edgequake/crates/edgequake-api/src/handlers/tenants.rs
 */

import type {
  CreateTenantRequest,
  CreateWorkspaceRequest,
  TenantDetail,
  TenantInfo,
  UpdateTenantRequest,
  WorkspaceInfo,
} from "../types/workspaces.js";
import { Resource } from "./base.js";

export class TenantsResource extends Resource {
  /** Create a new tenant. */
  async create(request: CreateTenantRequest): Promise<TenantInfo> {
    return this._post("/api/v1/tenants", request);
  }

  /** List all tenants. */
  async list(): Promise<TenantInfo[]> {
    return this._get("/api/v1/tenants");
  }

  /** Get a tenant by ID. */
  async get(tenantId: string): Promise<TenantDetail> {
    return this._get(`/api/v1/tenants/${tenantId}`);
  }

  /** Update a tenant. */
  async update(
    tenantId: string,
    request: UpdateTenantRequest,
  ): Promise<TenantInfo> {
    return this._put(`/api/v1/tenants/${tenantId}`, request);
  }

  /** Delete a tenant. */
  async delete(tenantId: string): Promise<void> {
    await this._del(`/api/v1/tenants/${tenantId}`);
  }

  /** Create a workspace within a tenant. */
  async createWorkspace(
    tenantId: string,
    request: CreateWorkspaceRequest,
  ): Promise<WorkspaceInfo> {
    return this._post(`/api/v1/tenants/${tenantId}/workspaces`, request);
  }

  /** List workspaces within a tenant. */
  async listWorkspaces(tenantId: string): Promise<WorkspaceInfo[]> {
    return this._get(`/api/v1/tenants/${tenantId}/workspaces`);
  }

  /** Get workspace by slug within a tenant. */
  async getWorkspaceBySlug(
    tenantId: string,
    slug: string,
  ): Promise<WorkspaceInfo> {
    return this._get(`/api/v1/tenants/${tenantId}/workspaces/by-slug/${slug}`);
  }
}
