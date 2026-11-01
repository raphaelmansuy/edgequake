'use client';

import { MemberAccessControl } from '@/components/clearance/member-access-control';
import { GrantEditor, type GrantRowModel } from '@/components/members/grant-editor';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from '@/components/ui/sheet';
import type { MemberView, MemberViewer, OrgRole } from '@/lib/api/edgequake/members';
import { useClearanceSettings } from '@/lib/clearance/use-clearance';
import { displayName } from '@/lib/members/format';
import { linkBlock, roleChangeBlock, roleOptions } from '@/lib/members/permissions';
import { useTenantStore } from '@/stores/use-tenant-store';
import { KeyRound, ShieldCheck, WandSparkles } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { MemberActionsMenu, type MemberAction } from './member-actions-menu';
import { MemberAvatar } from './member-avatar';
import { useLockedReason } from './member-row';
import { RoleSelect } from './role-select';
import { SignInBadges } from './sign-in-badges';
import { MemberStatusBadge } from './status-badge';

function Field({ label, children, testId }: { label: string; children: React.ReactNode; testId?: string }) {
  return (
    <div className="grid grid-cols-[7.5rem_1fr] items-center gap-3 py-2.5 text-sm" data-testid={testId}>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0">{children}</dd>
    </div>
  );
}

/** The viewer's rank in this organization: the ceiling for any role they may assign. */
function viewerLevel(viewer: MemberViewer): number {
  if (viewer.instance_admin) return 4;
  switch (viewer.max_assignable_role) {
    case 'owner':
      return 4;
    case 'admin':
      return 3;
    case 'member':
      return 2;
    default:
      return 1;
  }
}

const fmt = (iso: string | null | undefined, locale: string) =>
  iso ? new Intl.DateTimeFormat(locale, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(iso)) : null;

/** Sign-in links and password reset: offered when allowed, explained when SSO owns the credential. */
function AccessSection({
  member,
  viewer,
  onAction,
}: {
  member: MemberView;
  viewer: MemberViewer;
  onAction: (a: MemberAction, m: MemberView) => void;
}) {
  const { t } = useTranslation();
  const block = linkBlock(viewer, member);
  if (block !== null && block !== 'sso') return null;
  return (
    <section className="mx-4 mt-4 space-y-2 rounded-lg border p-3" data-testid="detail-access">
      <h3 className="text-sm font-medium">{t('members.detail.accessTitle', 'Sign-in help')}</h3>
      {block === 'sso' ? (
        <p className="text-xs text-muted-foreground" data-testid="detail-access-sso">
          {t('members.detail.accessSso', 'This person signs in through single sign-on. Their identity provider manages their password and sessions.')}
        </p>
      ) : (
        <>
          <p className="text-xs text-muted-foreground">
            {t('members.detail.accessBody', 'No email needed: create a one-time link and send it to them yourself.')}
          </p>
          <div className="flex flex-wrap gap-2">
            <Button size="sm" variant="outline" onClick={() => onAction('magic-link', member)} data-testid="detail-magic-link">
              <WandSparkles className="size-4" aria-hidden /> {t('members.actions.magicLink', 'Create sign-in link')}
            </Button>
            <Button size="sm" variant="outline" onClick={() => onAction('reset-link', member)} data-testid="detail-reset-link">
              <KeyRound className="size-4" aria-hidden /> {t('members.actions.resetPassword', 'Reset password')}
            </Button>
          </div>
        </>
      )}
    </section>
  );
}

export function MemberDetailSheet({
  member,
  viewer,
  onClose,
  onRoleChange,
  grants,
  workspaces = [],
  grantsBusy,
  onChangeGrantRole,
  onRemoveGrant,
  onAddGrant,
  onAction,
}: {
  member: MemberView | null;
  viewer: MemberViewer;
  onClose: () => void;
  onRoleChange: (m: MemberView, r: OrgRole) => void;
  /** SPEC-181: grant editing, when the caller may manage this organization's grants. */
  grants?: GrantRowModel[];
  workspaces?: readonly { id: string; name: string }[];
  grantsBusy?: boolean;
  onChangeGrantRole?: (membershipId: string, role: OrgRole) => void;
  onRemoveGrant?: (membershipId: string) => void;
  onAddGrant?: (workspaceId: string | null, role: OrgRole) => void;
  onAction: (a: MemberAction, m: MemberView) => void;
}) {
  const { t, i18n } = useTranslation();
  const lockedReason = useLockedReason();
  const never = t('members.neverSignedIn', 'Never');
  const workspaceId = useTenantStore((state) => state.selectedWorkspaceId);
  const clearance = useClearanceSettings(workspaceId);
  // Access profiles apply to real members of this workspace, and only owners and admins edit them.
  const showAccessProfiles =
    Boolean(workspaceId) && clearance.data?.can_manage === true && member?.kind === 'member';

  return (
    <Sheet open={!!member} onOpenChange={(o) => !o && onClose()}>
      <SheetContent className="w-full sm:max-w-md" data-testid="member-detail-sheet">
        {member && (
          <>
            <SheetHeader className="space-y-3 border-b pb-4 pr-12">
              <div className="flex items-center gap-3">
                <MemberAvatar member={member} className="size-12" />
                <div className="min-w-0 flex-1">
                  <SheetTitle className="truncate">{displayName(member)}</SheetTitle>
                  <SheetDescription className="truncate">{member.email}</SheetDescription>
                </div>
                <MemberActionsMenu
                  member={member}
                  viewer={viewer}
                  onAction={(a, m) => {
                    if (a !== 'details') onAction(a, m);
                  }}
                />
              </div>
            </SheetHeader>

            <dl className="divide-y px-4">
              {/* SPEC-181: one person, many grants. Fall back to the single-role control when the
                  caller cannot manage grants, so a member still sees their own role. */}
              {grants && onChangeGrantRole && onRemoveGrant && onAddGrant ? (
                <div className="py-3" data-testid="detail-grants">
                  <GrantEditor
                    grants={grants}
                    workspaces={workspaces}
                    maxLevel={viewerLevel(viewer)}
                    isLastOwner={member.is_last_owner}
                    busy={grantsBusy}
                    onChangeRole={(g, role) => onChangeGrantRole(g.membership_id, role as OrgRole)}
                    onRemove={(g) => onRemoveGrant(g.membership_id)}
                    onAdd={(ws, role) => onAddGrant(ws, role as OrgRole)}
                  />
                </div>
              ) : (
                <Field label={t('members.columns.role', 'Role')} testId="detail-role">
                  <RoleSelect
                    value={member.role}
                    options={roleOptions(viewer)}
                    lockedReason={lockedReason(roleChangeBlock(viewer, member))}
                    managedBy={member.role_managed_by}
                    onChange={(r) => onRoleChange(member, r)}
                  />
                </Field>
              )}
              <Field label={t('members.columns.status', 'Status')}>
                <MemberStatusBadge status={member.status} />
              </Field>
              <Field label={t('members.columns.signIn', 'Sign-in')}>
                <SignInBadges methods={member.sign_in} />
              </Field>
              {showAccessProfiles && (
                <Field label={t('members.detail.clearance', 'Access')} testId="detail-clearance">
                  <MemberAccessControl workspaceId={workspaceId ?? ''} userId={member.id} />
                </Field>
              )}
              <Field label={t('members.detail.scope', 'Scope')}>
                {member.workspace_id
                  ? t('members.detail.workspaceOnly', 'One workspace')
                  : t('members.detail.wholeOrg', 'Whole organization')}
              </Field>
              {member.instance_role && (
                <Field label={t('members.detail.instanceRole', 'Instance role')} testId="detail-instance-role">
                  <span className="capitalize">{member.instance_role}</span>
                </Field>
              )}
              <Field label={t('members.detail.joined', 'Joined')}>{fmt(member.joined_at, i18n.language)}</Field>
              <Field label={t('members.columns.lastActive', 'Last active')}>
                {fmt(member.last_login_at, i18n.language) ?? never}
              </Field>
            </dl>

            {member.role_managed_by && (
              <div className="mx-4 mt-2 flex items-start gap-2 rounded-lg border border-teal-500/30 bg-teal-500/5 p-3 text-xs text-muted-foreground" data-testid="detail-sso-note">
                <ShieldCheck className="mt-0.5 size-4 shrink-0 text-teal-600" aria-hidden />
                <span>
                  {t(
                    'members.detail.ssoNote',
                    "This person's role is kept in sync with {{provider}} at every sign-in. Changing it here pins it so the identity provider stops overriding it.",
                    { provider: member.role_managed_by },
                  )}
                </span>
              </div>
            )}

            <AccessSection member={member} viewer={viewer} onAction={onAction} />

            <div className="mt-auto flex justify-end border-t p-4">
              <Button variant="outline" onClick={onClose}>
                {t('common.close', 'Close')}
              </Button>
            </div>
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
