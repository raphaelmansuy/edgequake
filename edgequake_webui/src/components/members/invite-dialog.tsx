'use client';

import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { InvitationIssued, MemberViewer, OrgRole } from '@/lib/api/edgequake/members';
import { inviteUrl } from '@/lib/api/edgequake/members';
import { roleOptions } from '@/lib/members/permissions';
import { ROLE_ICON } from '@/lib/members/role-meta';
import { cn } from '@/lib/utils';
import { Check, Loader2, MailCheck } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ShareLinkPanel } from './share-link-panel';
import { useRoleDescription, useRoleLabel } from './use-role-label';

const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/** Step 2: the single-use invitation link, shown once. */
export function InviteLinkPanel({ issued, onDone }: { issued: InvitationIssued; onDone: () => void }) {
  const { t } = useTranslation();
  return (
    <ShareLinkPanel
      testIdPrefix="invite"
      icon={MailCheck}
      url={inviteUrl(issued.token)}
      onDone={onDone}
      inputLabel={t('members.invite.link', 'Invite link')}
      message={t('members.invite.created', 'Invitation created for {{email}}. Share this link with them — it works once and expires in {{days}} days.', {
        email: issued.invitation.email,
        days: issued.expires_in_days,
      })}
      hint={t('members.invite.shownOnce', 'For security this link is shown only once. If you lose it, resend the invitation to get a new one.')}
    />
  );
}

export function InviteDialog({
  open,
  viewer,
  busy,
  issued,
  workspaces = [],
  onSubmit,
  onClose,
}: {
  open: boolean;
  viewer: MemberViewer;
  busy: boolean;
  issued: InvitationIssued | null;
  /** SPEC-181: who may be invited to a single workspace rather than the whole organization. */
  workspaces?: readonly { id: string; name: string }[];
  onSubmit: (email: string, role: OrgRole, workspaceId?: string | null) => void;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const label = useRoleLabel();
  const describe = useRoleDescription();
  const options = roleOptions(viewer).filter((r) => r !== 'owner' || viewer.max_assignable_role === 'owner');
  const [email, setEmail] = useState('');
  const [role, setRole] = useState<OrgRole>('member');
  const [touched, setTouched] = useState(false);
  // `null` = the whole organization. A workspace choice limits the grant to that workspace.
  const [workspaceId, setWorkspaceId] = useState<string | null>(null);
  const valid = EMAIL.test(email.trim());

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    setTouched(true);
    if (valid) onSubmit(email.trim(), role, workspaceId);
  };

  const close = () => {
    setEmail('');
    setRole('member');
    setWorkspaceId(null);
    setTouched(false);
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-lg" data-testid="invite-dialog">
        <DialogHeader>
          <DialogTitle>{t('members.invite.title', 'Invite someone')}</DialogTitle>
          <DialogDescription>
            {t('members.invite.description', 'They join this organization with the role you choose. You can change it any time.')}
          </DialogDescription>
        </DialogHeader>

        {issued ? (
          <InviteLinkPanel issued={issued} onDone={close} />
        ) : (
          <form onSubmit={submit} className="space-y-4" noValidate>
            <div className="space-y-1.5">
              <Label htmlFor="invite-email">{t('members.invite.email', 'Email address')}</Label>
              <Input
                id="invite-email"
                data-testid="invite-email"
                type="email"
                autoFocus
                autoComplete="off"
                placeholder="name@company.com"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                onBlur={() => setTouched(true)}
                aria-invalid={touched && !valid}
                aria-describedby={touched && !valid ? 'invite-email-error' : undefined}
              />
              {touched && !valid && (
                <p id="invite-email-error" role="alert" className="text-xs text-destructive">
                  {t('members.errors.invalidEmail', 'Enter a valid email address.')}
                </p>
              )}
            </div>

            <fieldset className="space-y-1.5" data-testid="invite-scope">
              <legend className="text-sm font-medium">{t('members.invite.scope', 'Add to')}</legend>
              <div role="radiogroup" className="grid gap-2">
                <button
                  type="button"
                  role="radio"
                  aria-checked={workspaceId === null}
                  onClick={() => setWorkspaceId(null)}
                  data-testid="invite-scope-org"
                  className={
                    workspaceId === null
                      ? 'rounded-md border border-primary bg-primary/5 p-2 text-left text-[12.5px] ring-1 ring-primary'
                      : 'rounded-md border p-2 text-left text-[12.5px] hover:bg-muted/40'
                  }
                >
                  {t('members.invite.scopeOrg', 'The whole organization')}
                </button>
                {workspaces.map((w) => (
                  <button
                    key={w.id}
                    type="button"
                    role="radio"
                    aria-checked={workspaceId === w.id}
                    onClick={() => setWorkspaceId(w.id)}
                    data-testid={`invite-scope-ws-${w.id}`}
                    className={
                      workspaceId === w.id
                        ? 'rounded-md border border-primary bg-primary/5 p-2 text-left text-[12.5px] ring-1 ring-primary'
                        : 'rounded-md border p-2 text-left text-[12.5px] hover:bg-muted/40'
                    }
                  >
                    {t('members.invite.scopeWs', 'Only {{name}}', { name: w.name })}
                  </button>
                ))}
              </div>
            </fieldset>

            <fieldset className="space-y-1.5">
              <legend className="text-sm font-medium">{t('members.invite.role', 'Role')}</legend>
              <div role="radiogroup" className="grid gap-2">
                {options.map((r) => {
                  const Icon = ROLE_ICON[r];
                  const selected = r === role;
                  return (
                    <button
                      key={r}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      data-testid={`invite-role-${r}`}
                      onClick={() => setRole(r)}
                      className={cn(
                        'flex items-start gap-3 rounded-lg border p-3 text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
                        selected ? 'border-primary bg-primary/5' : 'hover:bg-muted/50',
                      )}
                    >
                      <Icon className="mt-0.5 size-4 text-muted-foreground" aria-hidden />
                      <span className="min-w-0 flex-1">
                        <span className="block text-sm font-medium">{label(r)}</span>
                        <span className="block text-xs text-muted-foreground">{describe(r)}</span>
                      </span>
                      <span
                        className={cn(
                          'mt-0.5 flex size-4 items-center justify-center rounded-full border',
                          selected ? 'border-primary bg-primary text-primary-foreground' : 'border-muted-foreground/40',
                        )}
                        aria-hidden
                      >
                        {selected && <Check className="size-3" />}
                      </span>
                    </button>
                  );
                })}
              </div>
            </fieldset>

            <DialogFooter>
              <Button type="button" variant="ghost" onClick={close} disabled={busy}>
                {t('common.cancel', 'Cancel')}
              </Button>
              <Button type="submit" disabled={busy} data-testid="invite-submit">
                {busy && <Loader2 className="size-4 animate-spin" aria-hidden />}
                {t('members.invite.submit', 'Create invitation')}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}
