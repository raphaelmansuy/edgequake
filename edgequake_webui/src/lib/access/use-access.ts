/**
 * SPEC-181 — the one place the WebUI learns what it may do.
 *
 * Everything a surface offers is derived from `GET /me/access`:
 * capabilities (not roles), quota, and the two create answers. A component asks
 * `access.organization.capabilities.has('organization.workspaces.create')`; nothing in the
 * client compares a role level, so the button and the handler that would refuse it cannot
 * drift ([LAW-181-1](../../../../specs/181-admin/01-first-principles.md),
 * [LAW-181-3](../../../../specs/181-admin/01-first-principles.md)).
 *
 * Fails closed: before the server answers, every capability set is empty and both create
 * answers are false, so nothing gated is rendered and then removed.
 */

import { fetchMeAccess, type MeAccessResponse, type AccessBudget } from '@/lib/api/edgequake/access';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';

export type Capability = string;

export interface OrganizationAccess {
  id: string;
  name: string;
  slug: string;
  /** Effective organization role; null when the caller holds only workspace grants. */
  role: string | null;
  level: number;
  isActive: boolean;
  isArchived: boolean;
  capabilities: ReadonlySet<Capability>;
}

export interface WorkspaceAccess {
  id: string;
  name: string;
  slug: string;
  role: string;
  level: number;
  /** The role comes from the organization grant, so the grant editor must not edit it here. */
  inherited: boolean;
  capabilities: ReadonlySet<Capability>;
}

export interface PlatformAccess {
  role: string;
  admin: boolean;
  capabilities: ReadonlySet<Capability>;
}

export interface Access {
  platform: PlatformAccess;
  organization: OrganizationAccess | null;
  workspaces: readonly WorkspaceAccess[];
  quota: { workspaces: AccessBudget; seats: AccessBudget };
  canCreate: { organization: boolean; workspace: boolean };
  canDelete: { organization: boolean };
  /** Why a create is unavailable: `capability_required:…`, `workspace_quota_reached`, … */
  reason: { organization: string | null; workspace: string | null };
}

/**
 * The empty grant. Used before the request resolves and after it fails, so the product is
 * never permissive by accident.
 */
export const NO_ACCESS: Access = {
  platform: { role: 'readonly', admin: false, capabilities: new Set() },
  organization: null,
  workspaces: [],
  quota: {
    workspaces: { used: 0, limit: 0 },
    seats: { used: 0, limit: 0 },
  },
  canCreate: { organization: false, workspace: false },
  canDelete: { organization: false },
  reason: { organization: 'loading', workspace: 'loading' },
};

/**
 * Convert the wire arrays into sets once per response, so a render pass is a hash lookup.
 *
 * WHY the `platform` guard: `GET /me/access` is fetched with `retry: false` and the result is
 * mapped unconditionally, so any 200 whose body is not the expected shape used to throw
 * `Cannot read properties of undefined (reading 'role')` inside render and blank the whole
 * route behind the error boundary. That is not hypothetical — the e2e hermetic guard answers an
 * unregistered `/api/v1/**` call with `{ error: … }`, which took `/documents` down and with it
 * every chromium spec that merely opened that page. An unreadable access payload means "no
 * access", which is what `NO_ACCESS` already says, and it is a state the UI renders.
 */
export function toAccess(payload: MeAccessResponse): Access {
  const platform = payload?.platform;
  if (!platform) return NO_ACCESS;
  return {
    platform: {
      role: platform.role,
      admin: platform.admin,
      capabilities: new Set(platform.capabilities ?? []),
    },
    organization: payload.organization
      ? {
          id: payload.organization.id,
          name: payload.organization.name,
          slug: payload.organization.slug,
          role: payload.organization.role ?? null,
          level: payload.organization.level,
          isActive: payload.organization.is_active,
          isArchived: payload.organization.is_archived,
          capabilities: new Set(payload.organization.capabilities ?? []),
        }
      : null,
    workspaces: (payload.workspaces ?? []).map((w) => ({
      id: w.id,
      name: w.name,
      slug: w.slug,
      role: w.role,
      level: w.level,
      inherited: w.inherited,
      capabilities: new Set(w.capabilities ?? []),
    })),
    quota: {
      workspaces: payload.quota?.workspaces ?? { used: 0, limit: 0 },
      seats: payload.quota?.seats ?? { used: 0, limit: 0 },
    },
    canCreate: {
      organization: payload.can_create?.organization === true,
      workspace: payload.can_create?.workspace === true,
    },
    canDelete: { organization: payload.can_delete?.organization === true },
    reason: {
      organization: payload.reason?.organization ?? null,
      workspace: payload.reason?.workspace ?? null,
    },
  };
}

export interface AccessState {
  access: Access;
  /** False until the server has answered. Gated content must not render before this. */
  ready: boolean;
  error: boolean;
  refetch: () => void;
}

/**
 * The query key every mutation invalidates. One key, so a grant change or a created scope is
 * reflected everywhere without five coordinated invalidations.
 */
export const meAccessKey = (tenantId: string | null, workspaceId: string | null) =>
  ['me-access', tenantId, workspaceId] as const;

export function useAccess(): AccessState {
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const workspaceId = useTenantStore((s) => s.selectedWorkspaceId);

  const query = useQuery({
    queryKey: meAccessKey(tenantId, workspaceId),
    queryFn: () => fetchMeAccess(tenantId, workspaceId),
    enabled: !!tenantId,
    staleTime: 30_000,
    retry: false,
  });

  const access = useMemo(
    () => (query.data ? toAccess(query.data) : NO_ACCESS),
    [query.data],
  );

  return {
    access,
    ready: !!query.data,
    error: !!query.error,
    refetch: () => void query.refetch(),
  };
}

// ── The two questions the create buttons actually ask ────────────────────────
// Kept here, not in the components, so the mapping lives in one place.

export const canCreateOrganization = (a: Access) => a.canCreate.organization;
export const canCreateWorkspace = (a: Access) => a.canCreate.workspace;

/** Capabilities of one workspace, or the empty set when the caller cannot read it. */
export const workspaceCapabilities = (a: Access, id: string): ReadonlySet<Capability> =>
  a.workspaces.find((w) => w.id === id)?.capabilities ?? EMPTY_CAPS;

const EMPTY_CAPS: ReadonlySet<Capability> = new Set();

/** True when the organization axis grants the capability (platform admin included). */
export function organizationCan(a: Access, capability: Capability): boolean {
  if (a.platform.admin) return capability.startsWith('organization.');
  return a.organization?.capabilities.has(capability) ?? false;
}

/** True when the platform axis grants the capability. */
export function platformCan(a: Access, capability: Capability): boolean {
  return a.platform.capabilities.has(capability);
}

/**
 * The quota reason is the only refusal a caller can fix themselves, so it is shown as a
 * disabled action with the fix; everything else is absent (LAW-181-3).
 */
export function isQuotaBlocked(a: Access): boolean {
  return a.reason.workspace === 'workspace_quota_reached';
}
