/**
 * Admin API — tenant quotas and server defaults.
 *
 * @module resources/admin
 */

import { Resource } from "./base.js";

export interface UpdateTenantQuotaResponse {
  tenant_id: string;
  max_workspaces: number;
  previous_max_workspaces: number;
  current_workspace_count: number;
}

export interface ServerDefaultsResponse {
  default_max_workspaces: number;
  note?: string;
}

export class AdminResource extends Resource {
  async patchTenantQuota(
    tenantId: string,
    maxWorkspaces: number,
  ): Promise<UpdateTenantQuotaResponse> {
    return this._patch(`/api/v1/admin/tenants/${tenantId}/quota`, {
      max_workspaces: maxWorkspaces,
    });
  }

  async getServerDefaults(): Promise<ServerDefaultsResponse> {
    return this._get("/api/v1/admin/config/defaults");
  }

  async patchServerDefaults(
    defaultMaxWorkspaces: number,
  ): Promise<ServerDefaultsResponse> {
    return this._patch("/api/v1/admin/config/defaults", {
      default_max_workspaces: defaultMaxWorkspaces,
    });
  }
}

export class EffectiveConfigResource extends Resource {
  async get(): Promise<Record<string, unknown>> {
    return this._get("/api/v1/config/effective");
  }
}
