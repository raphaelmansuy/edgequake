'use client';

import { Button } from '@/components/ui/button';
import { useGrantActions, useMemberActions, useMembers } from '@/hooks/use-members';
import type { AccessLinkIssued } from '@/lib/api/edgequake/access-links';
import {
  listGrants,
  type InvitationIssued,
  type MemberView,
  type OrgRole,
} from '@/lib/api/edgequake/members';
import { getWorkspaces } from '@/lib/api/edgequake';
import { useTenantStore } from '@/stores/use-tenant-store';
import { isForbidden } from '@/lib/members/errors';
import { viewerCanManage } from '@/lib/members/permissions';
import { UserPlus } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { AccessLinkDialog, type PendingLink } from './access-link-dialog';
import { ConfirmMemberDialog, type PendingConfirm } from './confirm-member-dialog';
import { InviteDialog } from './invite-dialog';
import type { MemberAction } from './member-actions-menu';
import { MemberDetailSheet } from './member-detail-sheet';
import { MembersTable } from './members-table';
import {
  MembersAlone,
  MembersEmpty,
  MembersError,
  MembersNoMatch,
  MembersPager,
  MembersRestricted,
  MembersSkeleton,
} from './members-states';
import { MembersSummary } from './members-summary';
import { MembersToolbar } from './members-toolbar';
import { PAGE_SIZE, useMemberFilters } from './use-member-filters';

/** The directory: summary, filters, table, and every dialog it can open. */
export function MembersTab() {
  const { t } = useTranslation();
  const filters = useMemberFilters();
  const query = useMembers(filters.params);
  const actions = useMemberActions();
  // SPEC-181: the workspace grant axis is writable, so the detail sheet edits grants.
  const grantActions = useGrantActions();

  const [inviteOpen, setInviteOpen] = useState(false);
  const [issued, setIssued] = useState<InvitationIssued | null>(null);
  const [pending, setPending] = useState<PendingConfirm | null>(null);
  const [linkFor, setLinkFor] = useState<PendingLink | null>(null);
  const [link, setLink] = useState<AccessLinkIssued | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const data = query.data;
  const selected =
    data?.members?.find((m) => m.id === selectedId && m.kind === 'member') ?? null;

  // The directory's `grants` summary has no membership id, so real grant rows are fetched for
  // the open person before the editor can mutate anything.
  const tenantId = useTenantStore((s) => s.selectedTenantId);
  const selectedUserId = selected?.user_id ?? null;
  const grantRows = useQuery({
    queryKey: ['grants', tenantId, selectedUserId],
    queryFn: () => listGrants(tenantId as string, { user_id: selectedUserId as string }),
    enabled: !!tenantId && !!selectedUserId,
    staleTime: 10_000,
  });
  const workspaceOptions = useQuery({
    queryKey: ['workspaces', tenantId],
    queryFn: () => getWorkspaces(tenantId as string),
    enabled: !!tenantId,
    staleTime: 60_000,
  });

  if (query.isError && isForbidden(query.error)) return <MembersRestricted />;
  if (query.isError && !data) return <MembersError onRetry={() => query.refetch()} />;
  if (!data) return <MembersSkeleton />;

  const { viewer } = data;
  const canInvite = viewerCanManage(viewer);

  const changeRole = (m: MemberView, role: OrgRole) =>
    actions.update.mutate({ userId: m.user_id ?? m.id, body: { role } });

  const onAction = (action: MemberAction, m: MemberView) => {
    if (action === 'details') return setSelectedId(m.id);
    if (action === 'magic-link' || action === 'reset-link') {
      setLink(null);
      return setLinkFor({ purpose: action === 'magic-link' ? 'magic_login' : 'password_reset', member: m });
    }
    if (action === 'resend') {
      return actions.resend.mutate(m.id, {
        onSuccess: (res) => {
          setIssued(res);
          setInviteOpen(true);
          toast.success(t('members.toasts.resent', 'New invitation link created'));
        },
      });
    }
    setPending({ action: action as PendingConfirm['action'], member: m });
  };

  const confirm = ({ action, member }: PendingConfirm) => {
    const done = { onSuccess: () => setPending(null) };
    const userId = member.user_id ?? member.id;
    if (action === 'remove') actions.remove.mutate(userId, done);
    else if (action === 'revoke') actions.revoke.mutate(member.id, done);
    else actions.update.mutate({ userId, body: { is_active: action === 'restore' } }, done);
  };

  const createLink = ({ purpose, member }: PendingLink) =>
    actions.issueLink.mutate({ userId: member.user_id ?? member.id, purpose }, { onSuccess: setLink });
  const closeLink = () => {
    setLinkFor(null);
    setLink(null);
  };

  const busy = actions.remove.isPending || actions.revoke.isPending || actions.update.isPending;
  // "Alone" = exactly one member, and it is the viewer. An empty list is not alone.
  const onlyMe =
    !filters.filtered &&
    data.summary.total <= 1 &&
    data.members.length === 1 &&
    data.members[0].is_you === true;

  return (
    <div className="space-y-4" data-testid="members-tab">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <MembersSummary data={data} />
        {canInvite && (
          <Button
            data-testid="members-invite"
            onClick={() => {
              setIssued(null);
              setInviteOpen(true);
            }}
          >
            <UserPlus className="size-4" aria-hidden />
            {t('members.invite.cta', 'Invite people')}
          </Button>
        )}
      </div>

      <MembersToolbar filters={filters} summary={data.summary} />

      {data.members.length === 0 ? (
        // An empty page is either a filter miss or an org with no members.
        // Never show both: "No one matches" only when a filter is actually on.
        filters.filtered ? (
          <MembersNoMatch onClear={filters.clear} />
        ) : (
          <MembersEmpty canInvite={canInvite} onInvite={() => setInviteOpen(true)} />
        )
      ) : (
        <div className={query.isFetching ? 'opacity-70 transition-opacity' : 'transition-opacity'}>
          <MembersTable
            members={data.members}
            viewer={viewer}
            sort={filters.sort}
            dir={filters.dir}
            onSort={filters.toggleSort}
            onRoleChange={changeRole}
            onAction={onAction}
          />
          <MembersPager page={filters.page} pageSize={PAGE_SIZE} total={data.total} onPage={filters.setPage} />
        </div>
      )}

      {onlyMe && <MembersAlone canInvite={canInvite} onInvite={() => setInviteOpen(true)} />}

      <InviteDialog
        open={inviteOpen}
        viewer={viewer}
        busy={actions.invite.isPending}
        issued={issued}
        onClose={() => {
          setInviteOpen(false);
          setIssued(null);
        }}
        workspaces={(Array.isArray(workspaceOptions.data) ? workspaceOptions.data : []).map((w) => ({ id: w.id, name: w.name }))}
        onSubmit={(email, role, workspaceId) =>
          actions.invite.mutate(
            { email, role, workspace_id: workspaceId ?? undefined },
            { onSuccess: setIssued },
          )
        }
      />

      <ConfirmMemberDialog
        pending={pending}
        busy={busy}
        tenantName={data.tenant_name}
        onConfirm={confirm}
        onClose={() => setPending(null)}
      />

      <AccessLinkDialog
        pending={linkFor}
        issued={link}
        busy={actions.issueLink.isPending}
        onCreate={createLink}
        onClose={closeLink}
      />

      <MemberDetailSheet
        member={selected}
        viewer={viewer}
        onClose={() => setSelectedId(null)}
        onRoleChange={changeRole}
        onAction={onAction}
        // `Array.isArray` rather than an optional-chained map: a 200 that carries an object
        // instead of a list (a proxy error body, or the e2e hermetic guard's JSON) made the
        // whole members page throw and lose the directory. A missing grant list is not a
        // reason to lose the page; the editor simply opens with no rows.
        grants={(Array.isArray(grantRows.data) ? grantRows.data : []).map((g) => ({
          membership_id: g.membership_id,
          user_id: g.user_id,
          scope: g.scope,
          workspace_id: g.workspace_id ?? null,
          workspace_name: g.workspace_name ?? null,
          role: g.role,
          level: g.level,
          sso_managed: g.sso_managed,
        }))}
        workspaces={(Array.isArray(workspaceOptions.data) ? workspaceOptions.data : []).map((w) => ({ id: w.id, name: w.name }))}
        grantsBusy={
          grantActions.create.isPending ||
          grantActions.change.isPending ||
          grantActions.remove.isPending
        }
        onChangeGrantRole={(membershipId, role) =>
          grantActions.change.mutate({ membershipId, role })
        }
        onRemoveGrant={(membershipId) => grantActions.remove.mutate(membershipId)}
        onAddGrant={(workspaceId, role) =>
          grantActions.create.mutate({
            user_id: selected?.user_id ?? selected?.id ?? "",
            workspace_id: workspaceId,
            role,
          })
        }
      />
    </div>
  );
}
