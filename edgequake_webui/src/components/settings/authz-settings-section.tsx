'use client';

/**
 * Document security settings: members table, roles, attributes, policies, break-glass.
 * Shown when EDGEQUAKE_DOC_ABAC is on (/health capabilities.doc_abac).
 */

import { AuthzDisclosure } from '@/components/settings/authz-disclosure';
import { AuthzStack } from '@/components/settings/authz-stack';
import { PrincipalSelect, principalDisplayName } from '@/components/security/principal-select';
import { DocumentPickerPopover } from '@/components/query/document-picker-popover';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { useDocAbacEnabled } from '@/hooks/use-doc-abac';
import { useUsers } from '@/hooks/use-users';
import {
  createAttributeDefinition,
  createAuthzMember,
  createAuthzRole,
  createBreakGlass,
  createPolicy,
  deleteAuthzMember,
  listAttributeDefinitions,
  listAuthzMembers,
  listAuthzRoles,
  listBreakGlass,
  listPolicies,
  listPrincipalAttributes,
  publishPolicyVersion,
  revokeBreakGlass,
  upsertPrincipalAttribute,
  type RoleBindingDto,
  type WorkspaceRoleDto,
} from '@/lib/api/edgequake/authz';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Shield } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

const CUSTOM_PERMISSIONS = [
  'document:list_meta',
  'document:read',
  'document:create',
  'document:set_labels',
  'graph:read',
  'query:execute',
  'policy:manage',
] as const;

const POLICY_TEMPLATES: { id: string; name: string; cedar: string }[] = [
  {
    id: 'workspace',
    name: 'Workspace read',
    cedar: 'permit (principal, action == Action::"document_read", resource);',
  },
  {
    id: 'owner',
    name: 'Owner only',
    cedar:
      'permit (principal, action == Action::"document_read", resource) when { resource.owner == principal };',
  },
];

function formatBgExpiry(iso: string): string {
  const exp = Date.parse(iso);
  if (Number.isNaN(exp)) return iso;
  return new Date(exp).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

function useWorkspaceId() {
  return useTenantStore((s) => s.selectedWorkspaceId);
}

const PERMISSION_LABELS: Record<string, string> = {
  'document:list_meta': 'List documents',
  'document:read': 'Read documents',
  'document:create': 'Upload documents',
  'document:set_labels': 'Set security labels',
  'document:admin': 'Administer documents',
  'graph:read': 'Read graph',
  'query:execute': 'Run queries',
  'policy:manage': 'Manage policies',
};

function humanPermission(slug: string): string {
  return PERMISSION_LABELS[slug] ?? slug.replace(/[_:]/g, ' ');
}

function permissionSummary(permissions: string[]): string {
  if (permissions.length === 0) return '—';
  const labels = permissions.map(humanPermission);
  if (labels.length <= 3) return labels.join(', ');
  return `${labels.slice(0, 2).join(', ')} +${labels.length - 2}`;
}

type AttrRow = { principal_id: string; name: string; value: unknown };

const ATTR_CHIP_CAP = 3;

/** Build capped attr chip labels for a principal (clearance first). */
function principalAttrChips(
  principalId: string,
  attrs: AttrRow[],
): { chips: string[]; overflow: number } {
  const mine = attrs.filter((a) => a.principal_id === principalId);
  const sorted = [...mine].sort((a, b) => {
    if (a.name === 'clearance') return -1;
    if (b.name === 'clearance') return 1;
    return a.name.localeCompare(b.name);
  });
  const labels = sorted.map((a) => {
    const v = typeof a.value === 'string' ? a.value : JSON.stringify(a.value);
    return a.name === 'clearance' ? v : `${a.name}:${v}`;
  });
  if (labels.length === 0) return { chips: [], overflow: 0 };
  if (labels.length <= ATTR_CHIP_CAP) return { chips: labels, overflow: 0 };
  return {
    chips: labels.slice(0, ATTR_CHIP_CAP),
    overflow: labels.length - ATTR_CHIP_CAP,
  };
}

function PrincipalAttrChips({
  principalId,
  attrs,
}: {
  principalId: string;
  attrs: AttrRow[];
}) {
  const { chips, overflow } = principalAttrChips(principalId, attrs);
  if (chips.length === 0) {
    return <span className="text-xs text-muted-foreground">—</span>;
  }
  return (
    <span
      className="inline-flex flex-wrap items-center gap-1 min-w-0"
      data-testid="spec146-member-attrs"
    >
      {chips.map((c) => (
        <Badge
          key={c}
          variant="outline"
          className="text-[10px] font-normal max-w-[10rem] truncate"
        >
          {c}
        </Badge>
      ))}
      {overflow > 0 ? (
        <span className="text-[10px] text-muted-foreground">+{overflow}</span>
      ) : null}
    </span>
  );
}

export function AuthzSettingsSection() {
  const { docAbacEnabled, isLoading } = useDocAbacEnabled();
  const workspaceId = useWorkspaceId();
  const { t } = useTranslation();

  if (isLoading) {
    return (
      <Card data-testid="spec146-authz-settings">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            {t('security.authz.title', 'Document security')}
          </CardTitle>
          <CardDescription>
            {t('security.authz.loading', 'Loading security settings…')}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <Skeleton className="h-16 w-full" />
        </CardContent>
      </Card>
    );
  }
  if (!docAbacEnabled) return null;
  if (!workspaceId) {
    return (
      <Card data-testid="spec146-authz-settings">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            {t('security.authz.title', 'Document security')}
          </CardTitle>
          <CardDescription>
            {t(
              'security.authz.noWorkspace',
              'Select a workspace to manage document security.',
            )}
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return (
    <div className="space-y-page" data-testid="spec146-authz-settings">
      <AuthzMembersCard workspaceId={workspaceId} />
      <AuthzRolesCard workspaceId={workspaceId} />
      <AuthzAttrsCard workspaceId={workspaceId} />
      <AuthzPoliciesCard workspaceId={workspaceId} />
      <AuthzBreakGlassCard workspaceId={workspaceId} />
    </div>
  );
}

function AuthzMembersCard({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { users, isLoading: usersLoading } = useUsers();
  const [principalId, setPrincipalId] = useState('');
  const [roleId, setRoleId] = useState('');

  const { data, isLoading } = useQuery({
    queryKey: ['authz', 'members', workspaceId],
    queryFn: () => listAuthzMembers(workspaceId),
  });

  const rolesQuery = useQuery({
    queryKey: ['authz', 'roles', workspaceId],
    queryFn: () => listAuthzRoles(workspaceId),
  });

  const createMut = useMutation({
    mutationFn: () =>
      createAuthzMember(workspaceId, {
        principal_kind: 'user',
        principal_id: principalId.trim(),
        role_id: roleId.trim(),
      }),
    onSuccess: () => {
      toast.success(t('security.authz.invite', 'Add member'));
      qc.invalidateQueries({ queryKey: ['authz', 'members', workspaceId] });
      setPrincipalId('');
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const deleteMut = useMutation({
    mutationFn: (b: RoleBindingDto) =>
      deleteAuthzMember(workspaceId, b.principal_kind, b.principal_id, b.role_id),
    onSuccess: () => {
      toast.success(t('security.authz.remove', 'Remove'));
      qc.invalidateQueries({ queryKey: ['authz', 'members', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const hydrating = usersLoading || isLoading || rolesQuery.isLoading;

  const attrsQuery = useQuery({
    queryKey: ['authz', 'principal-attrs', workspaceId],
    queryFn: () => listPrincipalAttributes(workspaceId),
  });

  const principalAttrs = attrsQuery.data?.attributes ?? [];

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('security.authz.members', 'Members')}</CardTitle>
        <CardDescription className="min-w-0 break-words text-pretty">
          {t(
            'security.authz.membersDescription',
            'Workspace capability roles (viewer, editor, admin).',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {hydrating ? (
          <Skeleton className="h-24 w-full" data-testid="spec146-members-skeleton" />
        ) : (
          <div data-testid="spec146-members-list">
            {(data?.bindings ?? []).length === 0 ? (
              <p className="text-sm text-muted-foreground">
                {t('security.authz.noMembers', 'No members yet.')}
              </p>
            ) : (
              <AuthzStack
                cardsTestId="spec146-members-cards"
                tableTestId="spec146-members-table"
                cards={
                  <div className="flex flex-col gap-2 min-w-0 w-full">
                    {(data?.bindings ?? []).map((b: RoleBindingDto) => {
                      const u = users.find((x) => x.user_id === b.principal_id);
                      return (
                        <div
                          key={`${b.principal_id}-${b.role_id}`}
                          className="rounded-lg border bg-background p-3 space-y-2 min-w-0 w-full"
                          data-testid="spec146-member-card"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="min-w-0">
                              <p className="font-medium truncate">
                                {principalDisplayName(u, b.principal_id)}
                              </p>
                              <p className="text-xs text-muted-foreground">
                                {t('security.authz.membershipRole', 'App role')}:{' '}
                                {u?.role ?? '—'}
                              </p>
                            </div>
                            <Button
                              size="sm"
                              variant="ghost"
                              className="h-7 text-xs shrink-0"
                              onClick={() => deleteMut.mutate(b)}
                            >
                              {t('security.authz.remove', 'Remove')}
                            </Button>
                          </div>
                          <div className="flex flex-wrap items-center gap-2 min-w-0">
                            <Badge variant="secondary" className="text-xs font-normal">
                              {b.role_name ?? '—'}
                            </Badge>
                            <PrincipalAttrChips
                              principalId={b.principal_id}
                              attrs={principalAttrs}
                            />
                          </div>
                        </div>
                      );
                    })}
                  </div>
                }
                table={
                  <Table>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t('security.authz.user', 'User')}</TableHead>
                        <TableHead>
                          {t('security.authz.membershipRole', 'App role')}
                        </TableHead>
                        <TableHead>
                          {t('security.authz.workspaceRole', 'Workspace role')}
                        </TableHead>
                        <TableHead>
                          {t('security.authz.attrsSummary', 'Attributes')}
                        </TableHead>
                        <TableHead className="w-[72px]">
                          <span className="sr-only">
                            {t('security.authz.remove', 'Remove')}
                          </span>
                        </TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {(data?.bindings ?? []).map((b: RoleBindingDto) => {
                        const u = users.find((x) => x.user_id === b.principal_id);
                        return (
                          <TableRow key={`${b.principal_id}-${b.role_id}`}>
                            <TableCell className="font-medium">
                              {principalDisplayName(u, b.principal_id)}
                            </TableCell>
                            <TableCell className="text-xs text-muted-foreground">
                              {u?.role ?? '—'}
                            </TableCell>
                            <TableCell>
                              <Badge variant="secondary" className="text-xs font-normal">
                                {b.role_name ?? '—'}
                              </Badge>
                            </TableCell>
                            <TableCell className="text-xs">
                              <PrincipalAttrChips
                                principalId={b.principal_id}
                                attrs={principalAttrs}
                              />
                            </TableCell>
                            <TableCell>
                              <Button
                                size="sm"
                                variant="ghost"
                                className="h-7 text-xs"
                                onClick={() => deleteMut.mutate(b)}
                              >
                                {t('security.authz.remove', 'Remove')}
                              </Button>
                            </TableCell>
                          </TableRow>
                        );
                      })}
                    </TableBody>
                  </Table>
                }
              />
            )}
          </div>
        )}
        <div className="grid gap-2 sm:grid-cols-3">
          <div>
            <Label className="text-xs">{t('security.authz.user', 'User')}</Label>
            <PrincipalSelect
              users={users}
              value={principalId}
              onValueChange={setPrincipalId}
              isLoading={usersLoading}
              testId="spec146-member-user"
            />
          </div>
          <div>
            <Label className="text-xs">{t('security.authz.workspaceRole', 'Workspace role')}</Label>
            <Select value={roleId || undefined} onValueChange={setRoleId}>
              <SelectTrigger data-testid="spec146-member-role" disabled={rolesQuery.isLoading}>
                <SelectValue placeholder={t('security.authz.selectRole', 'Select role')} />
              </SelectTrigger>
              <SelectContent>
                {(rolesQuery.data?.roles ?? []).map((r: WorkspaceRoleDto) => (
                  <SelectItem key={r.role_id} value={r.role_id}>
                    {r.name}
                    {r.is_builtin
                      ? ` (${t('security.authz.builtin', 'builtin')})`
                      : ''}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="flex flex-col gap-1 sm:col-span-1">
            <div className="flex items-end">
              <Button
                size="sm"
                className="w-full sm:w-auto"
                disabled={!principalId || !roleId || createMut.isPending}
                onClick={() => createMut.mutate()}
                data-testid="spec146-member-invite"
              >
                {t('security.authz.invite', 'Add member')}
              </Button>
            </div>
            {!principalId || !roleId ? (
              <p
                className="text-[11px] text-muted-foreground"
                data-testid="spec146-member-invite-hint"
              >
                {t(
                  'security.authz.inviteHint',
                  'Select a user and role to enable Add member.',
                )}
              </p>
            ) : null}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function AuthzRolesCard({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [name, setName] = useState('');
  const [perms, setPerms] = useState<string[]>(['document:list_meta', 'document:read']);
  const [createOpen, setCreateOpen] = useState(false);

  const { data } = useQuery({
    queryKey: ['authz', 'roles', workspaceId],
    queryFn: () => listAuthzRoles(workspaceId),
  });

  const createMut = useMutation({
    mutationFn: () =>
      createAuthzRole(workspaceId, {
        name: name.trim(),
        permissions: perms,
      }),
    onSuccess: () => {
      toast.success(t('security.authz.createRole', 'Create role'));
      qc.invalidateQueries({ queryKey: ['authz', 'roles', workspaceId] });
      setName('');
      setCreateOpen(false);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const togglePerm = (p: string, checked: boolean) => {
    setPerms((prev) => (checked ? [...prev, p] : prev.filter((x) => x !== p)));
  };

  return (
    <Card>
      <CardHeader className="min-w-0">
        <CardTitle>{t('security.authz.roles', 'Roles')}</CardTitle>
        <CardDescription className="min-w-0 break-words text-pretty">
          {t(
            'security.authz.rolesDescription',
            'Builtin roles are locked. Custom roles use a permission checklist.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 min-w-0">
        <AuthzStack
          cardsTestId="spec146-roles-cards"
          tableTestId="spec146-roles-summary"
          cards={
            <div className="flex flex-col gap-2 min-w-0 w-full">
              {(data?.roles ?? []).map((r: WorkspaceRoleDto) => (
                <div
                  key={r.role_id}
                  className="rounded-lg border bg-background p-3 space-y-1 min-w-0 w-full"
                >
                  <div className="flex items-center gap-2 flex-wrap min-w-0">
                    <span className="font-medium">{r.name}</span>
                    {r.is_builtin ? (
                      <Badge variant="outline" className="text-[10px]">
                        {t('security.authz.builtin', 'builtin')}
                      </Badge>
                    ) : null}
                  </div>
                  <p className="text-xs text-muted-foreground break-words text-pretty">
                    {permissionSummary(r.permissions)}
                  </p>
                </div>
              ))}
            </div>
          }
          table={
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>{t('security.authz.roleName', 'Role name')}</TableHead>
                  <TableHead>{t('security.authz.permissions', 'Permissions')}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {(data?.roles ?? []).map((r: WorkspaceRoleDto) => (
                  <TableRow key={r.role_id}>
                    <TableCell className="font-medium">
                      {r.name}{' '}
                      {r.is_builtin ? (
                        <Badge variant="outline" className="text-[10px] ml-1">
                          {t('security.authz.builtin', 'builtin')}
                        </Badge>
                      ) : null}
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {permissionSummary(r.permissions)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          }
        />
        <AuthzDisclosure
          open={createOpen}
          onOpenChange={setCreateOpen}
          testId="spec146-roles-create-toggle"
          panelTestId="spec146-roles-create-form"
          showLabel={t('security.authz.createRole', 'Create custom role')}
          hint={t(
            'security.authz.createRoleHint',
            'Need a custom capability set? Open the create form.',
          )}
        >
          <Label className="text-xs">{t('security.authz.roleName', 'Role name')}</Label>
          <Input value={name} onChange={(e) => setName(e.target.value)} />
          <div className="grid gap-1 sm:grid-cols-2">
            {CUSTOM_PERMISSIONS.map((p) => (
              <label key={p} className="flex items-center gap-2 text-xs">
                <Checkbox
                  checked={perms.includes(p)}
                  onCheckedChange={(c) => togglePerm(p, c === true)}
                />
                {humanPermission(p)}
              </label>
            ))}
          </div>
          <Button
            size="sm"
            disabled={!name.trim() || createMut.isPending}
            onClick={() => createMut.mutate()}
            data-testid="spec146-roles-create-submit"
          >
            {t('security.authz.createRole', 'Create role')}
          </Button>
        </AuthzDisclosure>
      </CardContent>
    </Card>
  );
}

function AuthzAttrsCard({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const { users, isLoading: usersLoading } = useUsers();
  const [attrName, setAttrName] = useState('');
  const [principalId, setPrincipalId] = useState('');
  const [value, setValue] = useState('');
  const [advanced, setAdvanced] = useState(false);

  const defs = useQuery({
    queryKey: ['authz', 'attr-defs', workspaceId],
    queryFn: () => listAttributeDefinitions(workspaceId),
  });
  const attrs = useQuery({
    queryKey: ['authz', 'principal-attrs', workspaceId],
    queryFn: () => listPrincipalAttributes(workspaceId),
  });

  const createDef = useMutation({
    mutationFn: () =>
      createAttributeDefinition(workspaceId, {
        scope: 'subject',
        name: attrName.trim(),
        value_type: 'string',
      }),
    onSuccess: () => {
      toast.success(t('security.authz.addDefinition', 'Add definition'));
      qc.invalidateQueries({ queryKey: ['authz', 'attr-defs', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const upsert = useMutation({
    mutationFn: () => {
      let parsed: unknown = value;
      if (advanced) {
        try {
          parsed = JSON.parse(value);
        } catch {
          parsed = value;
        }
      }
      return upsertPrincipalAttribute(workspaceId, {
        principal_kind: 'user',
        principal_id: principalId.trim(),
        name: attrName.trim(),
        value: parsed,
      });
    },
    onSuccess: () => {
      toast.success(t('security.authz.setAttribute', 'Set attribute'));
      qc.invalidateQueries({ queryKey: ['authz', 'principal-attrs', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const defNames = (defs.data?.attributes ?? []).map((d) => d.name);

  return (
    <Card data-testid="spec146-attrs-card">
      <CardHeader>
        <CardTitle>{t('security.authz.attributes', 'Attributes')}</CardTitle>
        <CardDescription>
          {t(
            'security.authz.attributesDescription',
            'Catalog definitions and principal values.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {(attrs.data?.attributes ?? []).length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="spec146-attrs-empty">
            {t(
              'security.authz.noAttributes',
              'No principal attributes yet. Add a definition, then set a value.',
            )}
          </p>
        ) : (
          <AuthzStack
            cardsTestId="spec146-attrs-cards"
            tableTestId="spec146-attrs-table"
            cards={
              <div className="flex flex-col gap-2">
                {(attrs.data?.attributes ?? []).slice(0, 20).map((a) => (
                  <div
                    key={`${a.principal_id}-${a.name}`}
                    className="rounded-lg border bg-background p-3 space-y-1"
                  >
                    <p className="text-sm font-medium">{a.name}</p>
                    <p className="text-xs text-muted-foreground">
                      {principalDisplayName(
                        users.find((u) => u.user_id === a.principal_id),
                        a.principal_id,
                      )}
                    </p>
                    <p className="text-xs truncate">
                      {typeof a.value === 'string' ? a.value : JSON.stringify(a.value)}
                    </p>
                  </div>
                ))}
              </div>
            }
            table={
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t('security.authz.attrName', 'Attribute name')}</TableHead>
                    <TableHead>{t('security.authz.principal', 'Principal')}</TableHead>
                    <TableHead>{t('security.authz.attrValue', 'Value')}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(attrs.data?.attributes ?? []).slice(0, 20).map((a) => (
                    <TableRow key={`${a.principal_id}-${a.name}`}>
                      <TableCell className="text-xs">{a.name}</TableCell>
                      <TableCell className="text-xs">
                        {principalDisplayName(
                          users.find((u) => u.user_id === a.principal_id),
                          a.principal_id,
                        )}
                      </TableCell>
                      <TableCell className="text-xs truncate max-w-[12rem]">
                        {typeof a.value === 'string' ? a.value : JSON.stringify(a.value)}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            }
          />
        )}
        <div className="grid gap-2 sm:grid-cols-2">
          <div>
            <Label className="text-xs">{t('security.authz.attrName', 'Attribute name')}</Label>
            {defNames.length > 0 ? (
              <Select value={attrName || undefined} onValueChange={setAttrName}>
                <SelectTrigger data-testid="spec146-attr-name">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {defNames.map((n) => (
                    <SelectItem key={n} value={n}>
                      {n}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <Input
                value={attrName}
                onChange={(e) => setAttrName(e.target.value)}
                data-testid="spec146-attr-name"
              />
            )}
          </div>
          <div className="flex items-end">
            <Button size="sm" variant="outline" onClick={() => createDef.mutate()}>
              {t('security.authz.addDefinition', 'Add definition')}
            </Button>
          </div>
          <div>
            <Label className="text-xs">{t('security.authz.user', 'User')}</Label>
            <PrincipalSelect
              users={users}
              value={principalId}
              onValueChange={setPrincipalId}
              isLoading={usersLoading}
              testId="spec146-attr-principal"
            />
          </div>
          <div>
            <Label className="text-xs">{t('security.authz.attrValue', 'Value')}</Label>
            <Input value={value} onChange={(e) => setValue(e.target.value)} />
          </div>
        </div>
        <AuthzDisclosure
          open={advanced}
          onOpenChange={setAdvanced}
          testId="spec146-attr-json-toggle"
          showLabel={t('security.authz.advancedJson', 'Advanced (JSON value)')}
          hint={t(
            'security.authz.advancedJsonHint',
            'Parse the value field as JSON when setting attributes.',
          )}
        >
          <p className="text-xs text-muted-foreground">
            {t(
              'security.authz.advancedJsonHint',
              'Parse the value field as JSON when setting attributes.',
            )}
          </p>
        </AuthzDisclosure>
        <Button
          size="sm"
          disabled={!principalId.trim() || !attrName.trim() || upsert.isPending}
          onClick={() => upsert.mutate()}
        >
          {t('security.authz.setAttribute', 'Set attribute')}
        </Button>
      </CardContent>
    </Card>
  );
}

function AuthzPoliciesCard({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [name, setName] = useState('');
  const [policyId, setPolicyId] = useState('');
  const [cedar, setCedar] = useState(POLICY_TEMPLATES[0].cedar);
  const [advanced, setAdvanced] = useState(false);

  const { data } = useQuery({
    queryKey: ['authz', 'policies', workspaceId],
    queryFn: () => listPolicies(workspaceId),
  });

  const createMut = useMutation({
    mutationFn: () => createPolicy(workspaceId, { name: name.trim() }),
    onSuccess: (res) => {
      toast.success(t('security.authz.publish', 'Publish'));
      setPolicyId(res.policy.policy_id);
      qc.invalidateQueries({ queryKey: ['authz', 'policies', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const publishMut = useMutation({
    mutationFn: () =>
      publishPolicyVersion(workspaceId, policyId, { cedar_text: cedar }),
    onSuccess: () => {
      toast.success(t('security.authz.publish', 'Publish'));
      qc.invalidateQueries({ queryKey: ['authz', 'policies', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const validate = () => {
    if (!cedar.trim()) {
      toast.error(
        t('security.authz.validateEmpty', 'Cedar text cannot be empty.'),
      );
      return;
    }
    // Server validates on publish (4xx → toast). Client only checks non-empty.
    toast.message(
      t(
        'security.authz.validateHint',
        'Looks non-empty. Full Cedar validation runs when you publish.',
      ),
    );
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('security.authz.policies', 'Policies')}</CardTitle>
        <CardDescription>
          {t(
            'security.authz.policiesDescription',
            'Start from a template, then publish.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex flex-wrap gap-2">
          {POLICY_TEMPLATES.map((tmpl) => (
            <Button
              key={tmpl.id}
              type="button"
              size="sm"
              variant="outline"
              onClick={() => {
                setName(tmpl.name);
                setCedar(tmpl.cedar);
              }}
            >
              {tmpl.name}
            </Button>
          ))}
        </div>
        {(data?.policies ?? []).length > 0 ? (
          <div className="space-y-1">
            <Label className="text-xs">
              {t('security.authz.policySelect', 'Policy')}
            </Label>
            <Select value={policyId || undefined} onValueChange={setPolicyId}>
              <SelectTrigger data-testid="spec146-policy-select">
                <SelectValue
                  placeholder={t('security.authz.selectPolicy', 'Select policy')}
                />
              </SelectTrigger>
              <SelectContent>
                {(data?.policies ?? []).map((p) => (
                  <SelectItem key={p.policy_id} value={p.policy_id}>
                    {p.name} v{p.active_version}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        ) : null}
        <div className="flex flex-col gap-2 sm:flex-row">
          <Input
            placeholder={t('security.authz.policyName', 'Policy name')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            data-testid="spec146-policy-name"
          />
          <Button
            size="sm"
            disabled={!name.trim() || createMut.isPending}
            onClick={() => createMut.mutate()}
          >
            {t('common.create', 'Create')}
          </Button>
        </div>
        <AuthzDisclosure
          open={advanced}
          onOpenChange={setAdvanced}
          testId="spec146-policy-advanced-toggle"
          panelTestId="spec146-policy-cedar"
          showLabel={t('security.authz.advancedCedar', 'Advanced Cedar')}
          hint={t(
            'security.authz.advancedCedarHint',
            'Edit Cedar text, validate, then publish.',
          )}
        >
          <Label className="text-xs">{t('security.authz.cedar', 'Cedar policy')}</Label>
          <Textarea
            className="font-mono text-xs min-h-[100px]"
            value={cedar}
            onChange={(e) => setCedar(e.target.value)}
          />
          <div className="flex gap-2">
            <Button size="sm" variant="outline" onClick={validate}>
              {t('security.authz.validate', 'Validate')}
            </Button>
            <Button
              size="sm"
              disabled={!policyId || !cedar.trim()}
              onClick={() => publishMut.mutate()}
            >
              {t('security.authz.publish', 'Publish')}
            </Button>
          </div>
        </AuthzDisclosure>
        {!advanced ? (
          <Button
            size="sm"
            disabled={!policyId || !cedar.trim()}
            onClick={() => publishMut.mutate()}
          >
            {t('security.authz.publish', 'Publish')}
          </Button>
        ) : null}
      </CardContent>
    </Card>
  );
}

function AuthzBreakGlassCard({ workspaceId }: { workspaceId: string }) {
  const { t } = useTranslation();
  const qc = useQueryClient();
  const [reason, setReason] = useState('');
  const [scopeDocIds, setScopeDocIds] = useState<string[]>([]);
  const [unscopedOpen, setUnscopedOpen] = useState(false);

  const { data } = useQuery({
    queryKey: ['authz', 'break-glass', workspaceId],
    queryFn: () => listBreakGlass(workspaceId),
  });

  const createMut = useMutation({
    mutationFn: (unscoped: boolean) =>
      createBreakGlass(workspaceId, {
        reason: reason.trim(),
        ttl_minutes: 15,
        scope_doc_ids: unscoped ? undefined : scopeDocIds,
      }),
    onSuccess: () => {
      toast.success(t('security.breakGlass.create', 'Create 15m session'));
      qc.invalidateQueries({ queryKey: ['authz', 'break-glass', workspaceId] });
      setReason('');
      setScopeDocIds([]);
      setUnscopedOpen(false);
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const revokeMut = useMutation({
    mutationFn: (id: string) => revokeBreakGlass(workspaceId, id),
    onSuccess: () => {
      toast.success(t('security.breakGlass.revoke', 'Revoke'));
      qc.invalidateQueries({ queryKey: ['authz', 'break-glass', workspaceId] });
    },
    onError: (e: Error) => toast.error(e.message),
  });

  const submit = () => {
    if (!reason.trim()) {
      toast.error(t('security.breakGlass.reason', 'Reason'));
      return;
    }
    if (scopeDocIds.length === 0) {
      setUnscopedOpen(true);
      return;
    }
    createMut.mutate(false);
  };

  return (
    <Card data-testid="spec146-break-glass-card">
      <CardHeader>
        <CardTitle>{t('security.breakGlass.title', 'Break-glass')}</CardTitle>
        <CardDescription>
          {t(
            'security.breakGlass.cardDescription',
            'Temporary elevated access. All actions are audited.',
          )}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {(data?.sessions ?? []).length === 0 ? (
          <p className="text-sm text-muted-foreground" data-testid="spec146-bg-empty">
            {t(
              'security.breakGlass.noSessions',
              'No break-glass sessions. Create a 15m session below when needed.',
            )}
          </p>
        ) : (
          <ul className="text-sm space-y-1" data-testid="spec146-bg-sessions">
            {(data?.sessions ?? []).map((s) => (
              <li key={s.session_id} className="flex items-center justify-between gap-2">
                <span className="truncate min-w-0">
                  {s.reason} · {formatBgExpiry(s.expires_at)}
                  {s.revoked_at ? ` (${t('security.breakGlass.revoke', 'Revoke')})` : ''}
                </span>
                {!s.revoked_at ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    className="h-7 text-xs shrink-0"
                    onClick={() => revokeMut.mutate(s.session_id)}
                  >
                    {t('security.breakGlass.revoke', 'Revoke')}
                  </Button>
                ) : null}
              </li>
            ))}
          </ul>
        )}
        <div>
          <Label className="text-xs">{t('security.breakGlass.reason', 'Reason')}</Label>
          <Input
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            data-testid="spec146-bg-reason"
          />
        </div>
        <div>
          <Label className="text-xs">
            {t('security.breakGlass.documentScope', 'Document scope')}
          </Label>
          <DocumentPickerPopover
            selectedIds={scopeDocIds}
            onSelectionChange={setScopeDocIds}
          />
        </div>
        <Button size="sm" disabled={createMut.isPending} onClick={submit}>
          {t('security.breakGlass.create', 'Create 15m session')}
        </Button>
        <AlertDialog open={unscopedOpen} onOpenChange={setUnscopedOpen}>
          <AlertDialogContent>
            <AlertDialogHeader>
              <AlertDialogTitle>
                {t('security.breakGlass.unscopedTitle', 'Unscoped break-glass')}
              </AlertDialogTitle>
              <AlertDialogDescription>
                {t(
                  'security.breakGlass.unscopedDescription',
                  'This session applies to every document in the workspace. All actions are audited.',
                )}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel>{t('common.cancel', 'Cancel')}</AlertDialogCancel>
              <AlertDialogAction
                data-testid="spec146-bg-unscoped-confirm"
                onClick={() => createMut.mutate(true)}
              >
                {t('security.breakGlass.unscopedConfirm', 'Create unscoped session')}
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardContent>
    </Card>
  );
}
