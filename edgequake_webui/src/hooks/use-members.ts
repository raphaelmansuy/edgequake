'use client';

/**
 * SPEC-169 — data layer of the Members screen (react-query). Rendering stays in
 * `components/members/*`; this file owns fetching, mutation and cache invalidation (SRP).
 */
import { issueAccessLink, type AccessPurpose } from '@/lib/api/edgequake/access-links';
import {
  createInvitation,
  getRoleCatalog,
  listMembers,
  removeMember,
  resendInvitation,
  revokeInvitation,
  createGrant,
  deleteGrant,
  updateGrant,
  updateMember,
  type MemberListParams,
  type OrgRole,
  type RoleCatalog,
  type UpdateMemberBody,
} from '@/lib/api/edgequake/members';
import { memberErrorKey } from '@/lib/members/errors';
import { useTenantQueryReady } from '@/hooks/use-tenant-query-ready';
import { useAuthStoreHydrated } from '@/stores/use-auth-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import { keepPreviousData, useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export const membersKey = (tenantId: string | null) => ['members', tenantId] as const;

export function useMembers(params: MemberListParams) {
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const ready = useTenantQueryReady();
  return useQuery({
    queryKey: [...membersKey(tenantId), params],
    queryFn: () => listMembers(tenantId as string, params),
    enabled: ready,
    placeholderData: keepPreviousData,
    retry: 0,
    staleTime: 10_000,
  });
}

export function useRoleCatalog() {
  const hydrated = useAuthStoreHydrated();
  return useQuery<RoleCatalog>({
    queryKey: ['roles', 'catalog'],
    queryFn: getRoleCatalog,
    enabled: hydrated,
    staleTime: 5 * 60_000,
    retry: 0,
  });
}

/** Toast with the translated reason when the API error is a known one. */
function useFailureToast() {
  const { t } = useTranslation();
  return (error: unknown, fallbackKey: string, fallback: string) => {
    const reason = memberErrorKey(error);
    const detail = reason
      ? t(`members.errors.${reason}`)
      : error instanceof Error
        ? error.message
        : '';
    toast.error(`${t(fallbackKey, fallback)}${detail ? ` — ${detail}` : ''}`);
  };
}

export function useMemberActions() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const tenantId = useTenantStore((s) => s.selectedTenantId) as string;
  const fail = useFailureToast();
  const refresh = () => qc.invalidateQueries({ queryKey: membersKey(tenantId) });

  const update = useMutation({
    mutationFn: ({ userId, body }: { userId: string; body: UpdateMemberBody }) =>
      updateMember(tenantId, userId, body),
    onSuccess: () => {
      toast.success(t('members.toasts.updated', 'Member updated'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.updateFailed', 'Could not update member'),
  });

  const remove = useMutation({
    mutationFn: (userId: string) => removeMember(tenantId, userId),
    onSuccess: () => {
      toast.success(t('members.toasts.removed', 'Member removed'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.removeFailed', 'Could not remove member'),
  });

  const invite = useMutation({
    mutationFn: (body: Parameters<typeof createInvitation>[1]) => createInvitation(tenantId, body),
    onSuccess: () => refresh(),
    onError: (e) => fail(e, 'members.toasts.inviteFailed', 'Could not send invitation'),
  });

  const resend = useMutation({
    mutationFn: (invitationId: string) => resendInvitation(tenantId, invitationId),
    onSuccess: () => refresh(),
    onError: (e) => fail(e, 'members.toasts.resendFailed', 'Could not resend invitation'),
  });

  const revoke = useMutation({
    mutationFn: (invitationId: string) => revokeInvitation(tenantId, invitationId),
    onSuccess: () => {
      toast.success(t('members.toasts.revoked', 'Invitation revoked'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.revokeFailed', 'Could not revoke invitation'),
  });

  const issueLink = useMutation({
    mutationFn: ({ userId, purpose }: { userId: string; purpose: AccessPurpose }) =>
      issueAccessLink(tenantId, userId, purpose),
    onError: (e) => fail(e, 'members.toasts.linkFailed', 'Could not create the link'),
  });

  return { update, remove, invite, resend, revoke, issueLink };
}

/**
 * SPEC-181 — grant mutations.
 *
 * One hook per operation, all invalidating the same directory key, so the table and the open
 * editor cannot disagree after a change.
 */
export function useGrantActions() {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const tenantId = useTenantStore((s) => s.selectedTenantId) as string;
  const fail = useFailureToast();
  const refresh = () => qc.invalidateQueries({ queryKey: membersKey(tenantId) });

  const create = useMutation({
    mutationFn: (body: { user_id: string; workspace_id?: string | null; role: OrgRole }) =>
      createGrant(tenantId, body),
    onSuccess: () => {
      toast.success(t('members.toasts.grantAdded', 'Grant added'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.grantFailed', 'Could not add the grant'),
  });

  const change = useMutation({
    mutationFn: ({ membershipId, role }: { membershipId: string; role: OrgRole }) =>
      updateGrant(tenantId, membershipId, role),
    onSuccess: () => {
      toast.success(t('members.toasts.grantUpdated', 'Grant updated'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.grantFailed', 'Could not update the grant'),
  });

  const remove = useMutation({
    mutationFn: (membershipId: string) => deleteGrant(tenantId, membershipId),
    onSuccess: () => {
      toast.success(t('members.toasts.grantRemoved', 'Grant removed'));
      return refresh();
    },
    onError: (e) => fail(e, 'members.toasts.grantFailed', 'Could not remove the grant'),
  });

  return { create, change, remove };
}
