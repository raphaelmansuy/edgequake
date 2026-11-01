/**
 * SPEC-181 — the `GET /me/access` client.
 *
 * One function reads the endpoint; `useAccess` is its only caller. Mirrors
 * `handlers/me_access.rs`, which is the authority on what each field means.
 */

import { api } from '../client';

/** One plan budget. `limit === 0` means unlimited. */
export interface AccessBudget {
  used: number;
  limit: number;
}

export interface MeAccessOrganization {
  id: string;
  name: string;
  slug: string;
  role: string | null;
  level: number;
  is_active: boolean;
  is_archived: boolean;
  capabilities: string[];
}

export interface MeAccessWorkspace {
  id: string;
  name: string;
  slug: string;
  role: string;
  level: number;
  /** The role is inherited from the organization grant; there is no row on this workspace. */
  inherited: boolean;
  capabilities: string[];
}

export interface MeAccessResponse {
  user_id: string;
  platform: { role: string; admin: boolean; capabilities: string[] };
  organization: MeAccessOrganization | null;
  workspaces: MeAccessWorkspace[];
  quota: { workspaces: AccessBudget; seats: AccessBudget };
  can_create: { organization: boolean; workspace: boolean };
  can_delete: { organization: boolean };
  reason: { organization?: string | null; workspace?: string | null };
}

/**
 * Read the caller's grants for one scope.
 *
 * `tenant_id` selects the organization; `workspace_id` lets the response include a workspace
 * that is not in the header selection (a deeplink).
 */
export const fetchMeAccess = (tenantId: string | null, workspaceId: string | null) => {
  const qs = new URLSearchParams();
  if (tenantId) qs.set('tenant_id', tenantId);
  if (workspaceId) qs.set('workspace_id', workspaceId);
  const query = qs.toString();
  return api.get<MeAccessResponse>(`/me/access${query ? `?${query}` : ''}`);
};
