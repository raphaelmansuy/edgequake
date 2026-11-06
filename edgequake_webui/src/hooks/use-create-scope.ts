/**
 * SPEC-181 — one mutation per creation intent.
 *
 * The pre-181 wizard owned the whole sequence inline: two API calls (create the tenant, then
 * PATCH the workspace the tenant endpoint had already made), a context switch, two cache
 * invalidations and a toast. That is how `POST /tenants` came to produce an organization nobody
 * owned: the sequence existed only inside a React component.
 *
 * Here the sequence is data, so a test can assert it and a second surface can reuse it.
 */

import {
  createTenant,
  createWorkspace,
  getWorkspaces,
  type CreateTenantRequest,
} from '@/lib/api/edgequake';
import type { CreateWorkspaceRequest } from '@/types';
import { setTenantContext } from '@/lib/api/client-context';
import { meAccessKey } from '@/lib/access/use-access';
import { useTenantStore } from '@/stores/use-tenant-store';
import type { Tenant, Workspace } from '@/types';
import { useMutation, useQueryClient } from '@tanstack/react-query';

/** What a create produced, in the shape the confirmation card renders. */
export interface CreatedScope {
  tenant: Tenant;
  workspace: Workspace | null;
}

/**
 * Select the new scope everywhere: store, request headers and URL.
 *
 * Replaces `applyCreatedTenantContext` and `applyCreatedWorkspaceContext`, which differed only
 * in which id they passed (LAW-181-15, one entry point per intent).
 */
export function applyCreatedScope(tenant: Tenant, workspace: Workspace | null): void {
  const store = useTenantStore.getState();
  if (!store.tenants.some((t) => t.id === tenant.id)) {
    store.setTenants([...store.tenants, tenant]);
  }
  store.selectTenant(tenant.id);
  if (workspace) {
    store.setWorkspaces([workspace]);
    store.selectWorkspace(workspace.id);
    setTenantContext(tenant.id, workspace.id);
  } else {
    setTenantContext(tenant.id);
  }
}

export function useCreateOrganization() {
  return useMutation({
    mutationFn: async (body: CreateTenantRequest): Promise<CreatedScope> => {
      const tenant = await createTenant(body);
      // The server created the default workspace inside the same transaction. Read the
      // *workspace list of the new tenant*, not `me/access[0]`, which can be a sibling from a
      // cache that has not settled yet (produced "/w/legal" on a brand-new org in round-09).
      const created = await getWorkspaces(tenant.id).catch(() => []);
      const first = created[0];
      const workspace = first ?? null;
      return { tenant, workspace };
    },
    // Deliberately no cache invalidation here. `TenantGuard` renders the create dialog *inside*
    // its "no organizations yet" branch, so invalidating `['tenants']` while the dialog is open
    // makes the guard see a non-empty list, drop the branch, and unmount the confirmation before
    // it is read (found in rounds 05-07). The dialog settles the cache when it closes.
  });
}

export function useCreateWorkspace() {
  const tenantId = useTenantStore((s) => s.selectedTenantId);

  return useMutation({
    mutationFn: async (
      body: CreateWorkspaceRequest & { tenantId?: string },
    ): Promise<CreatedScope> => {
      const target = body.tenantId ?? tenantId;
      if (!target) throw new Error('no_tenant');
      const workspace = await createWorkspace(target, body);
      const tenant =
        useTenantStore.getState().tenants.find((t) => t.id === target) ??
        ({ id: target, name: '', slug: '' } as Tenant);
      return { tenant, workspace };
    },
    // Same reasoning as `useCreateOrganization`: settle on close, not on response.
  });
}

/**
 * Settle every cache a creation touched. Called when the dialog closes, never on response:
 * see the note in `useCreateOrganization`.
 */
export function settleAfterCreate(
  qc: ReturnType<typeof useQueryClient>,
  tenantId: string,
  workspaceId: string | null,
): void {
  void qc.invalidateQueries({ queryKey: ['tenants'] });
  void qc.invalidateQueries({ queryKey: ['workspaces', tenantId] });
  void qc.invalidateQueries({ queryKey: meAccessKey(tenantId, workspaceId) });
}

/** Turn a server reason code into the field or banner a dialog should highlight. */
export type CreateFailureKind =
  | 'quota'
  | 'slug'
  | 'capability'
  | 'inactive'
  | 'models'
  | 'server';

/**
 * The reason codes the create routes can return, per `specs/178-…/08-api-contract.md` and
 * `model_policy_enforcement`. The server sends these in the body's `code` field (`ApiError::Coded`
 * serialises `{ kind, code, message, status }`), so reading `code` is the contract; scraping the
 * prose is a fallback for transports that lose the body.
 */
const MODEL_REASON_CODES = new Set([
  'model_not_eligible',
  'capability_mismatch',
  'model_pair_incomplete',
  'embedding_dimension_mismatch',
  'embedding_dimension_rejected',
  'embedding_not_multimodal',
  'embedding_model_not_multimodal',
  'embedding_auth',
  'embedding_request_failed',
  'usage_locked',
]);

export function classifyCreateFailure(error: unknown): {
  kind: CreateFailureKind;
  message: string;
} {
  const message = error instanceof Error ? error.message : String(error);
  const code =
    error && typeof error === 'object' && 'code' in error
      ? String((error as { code?: unknown }).code ?? '')
      : '';

  // Structured first: an explicit code cannot be misread, unlike prose.
  if (code) {
    if (code === 'workspace_quota_reached') return { kind: 'quota', message };
    if (code === 'tenant_slug_taken' || code === 'workspace_slug_taken') {
      return { kind: 'slug', message };
    }
    if (code === 'capability_required') return { kind: 'capability', message };
    if (code === 'scope_inactive') return { kind: 'inactive', message };
    if (MODEL_REASON_CODES.has(code)) return { kind: 'models', message };
  }

  // Fallback: the code folded into the message, or a human sentence about the same causes.
  if (/workspace_quota_reached/i.test(message)) return { kind: 'quota', message };
  if (/tenant_slug_taken|workspace_slug_taken/i.test(message)) return { kind: 'slug', message };
  if (/capability_required|Admin role required|owner\/admin membership/i.test(message)) {
    return { kind: 'capability', message };
  }
  if (/scope_inactive|archived/i.test(message)) return { kind: 'inactive', message };
  if (
    /models?_not_eligible|not_multimodal|embedding_dimension_mismatch|embedding_dimension_rejected|model_pair_incomplete|usage_locked|embedding_auth|embedding_request_failed|not offered to this/i.test(
      message,
    )
  ) {
    return { kind: 'models', message: readableCreateMessage(message) };
  }
  return { kind: 'server', message: readableCreateMessage(message) };
}

/** Drop the transport prefix so the dialog can print the server's sentence. */
function readableCreateMessage(message: string): string {
  return message.replace(
    /^(Bad request|Conflict|Validation error|Configuration error|Internal error):\s*/i,
    '',
  );
}
