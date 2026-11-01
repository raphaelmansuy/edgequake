# Lens — Frontend Developer

Parent: [README](../README.md) · Spec: [10](../10-frontend-architecture.md)

## The three defects I am fixing

```tsx
// 1. Ungated — context-selector-popover.tsx
<CommandItem value="create-organization" onSelect={onCreateTenant}>…</CommandItem>

// 2. Two create sequences — create-tenant-wizard.tsx
const tenant = await createTenant({…});
setTenantContext(tenant.id);
const list = await getWorkspaces(tenant.id);
const autoWs = list.find(w => w.slug === 'default') ?? list[0];
const workspace = autoWs
  ? await updateWorkspace(tenant.id, autoWs.id, {…})   // PATCH after POST
  : await createWorkspace(tenant.id, {…});

// 3. Blank on switch — use-tenant-store.ts
set({ selectedTenantId: tenantId, selectedWorkspaceId: null, workspaces: [] });
```

One is a UX bug, two is a correctness bug, three is a one-frame flash that reads as a
slow product.

## The hook that replaces the guessing

```ts
// src/lib/access/use-access.ts
export const NO_ACCESS: Access = {
  platform: { role: 'user', admin: false },
  organization: null,
  workspaces: [],
  quota: { workspaces: { used: 0, limit: 0 }, seats: { used: 0, limit: 0 } },
  canCreate: { organization: false, workspace: false },
  canDelete: { organization: false },
  reason: { organization: 'loading', workspace: 'loading' },
};

export function useAccess(): AccessState {
  const tenantId = useTenantStore(s => s.selectedTenantId);
  const workspaceId = useTenantStore(s => s.selectedWorkspaceId);
  const q = useQuery({
    queryKey: ['me-access', tenantId, workspaceId],
    queryFn: () => fetchMeAccess(tenantId, workspaceId),
    enabled: !!tenantId,
    staleTime: 30_000,
    retry: false,
  });
  return { access: q.data ? toAccess(q.data) : NO_ACCESS, ready: !!q.data, error: !!q.error };
}
```

`toAccess` converts the wire arrays into `ReadonlySet<Capability>` once per response, so a
render pass is `set.has(x)` and never an array scan.

## Component tree, create workspace

```text
  <CreateWorkspaceDialog open onClose />
    <ScopeDialog>
      <ScopeDialogHeader title="Create a workspace" lead quota={quota.workspaces} />
      <ScopeDialogBody>
        <NameField draft onChange />                    ← reused shape, one field
        <RetrievalChoice draft onChange />              ← 2 radio cards
        <InheritRow                                        ← 3 model lines, collapsed
          label="Models" value={inherited} onEdit={…} />
        <AdvancedDrawer summary={4}>
          <ParsingRow /> <ChunkingRow /> <BudgetRow />
          <EntityTypesRow /> <LanguageRow /> <DecisionRow />
        </AdvancedDrawer>
      </ScopeDialogBody>
      <ScopeDialogFooter blockedReason canFinish onFinish />
    </ScopeDialog>
  </CreateWorkspaceDialog>
```

State: one `draft` object (`useState`), persisted by the existing
`useWizardDraftPersistence` under key `create-workspace:{tenantId}` — so the draft guarantee
([LAW-181-14](../01-first-principles.md)) comes free from code that already exists and is
already tested.

## The mutation

```ts
export function useCreateWorkspace() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateWorkspaceBody) => createWorkspace(body.tenantId, body),
    onSuccess: (res) => {
      applyCreatedScope(res);
      qc.invalidateQueries({ queryKey: ['me-access'] });
      qc.invalidateQueries({ queryKey: ['workspaces', res.tenant.id] });
    },
  });
}
```

Failure handling lives in the dialog, not the hook:

```ts
createWs.mutate(body, {
  onError: (e) => {
    const r = reasonCode(e);                       // ApiRequestError.details.reason_code
    if (r === 'workspace_quota_reached') setBlocked('quota');
    else if (r === 'workspace_slug_taken') setFieldError('slug', t('…'));
    else setBanner(t('admin.error.server', { message: messageOf(e) }));
  },
});
```

No `window.location.href`. Today both wizards end with
`window.location.href = '/documents'` inside a toast action, which throws away the router
state and remounts the whole app. The new confirmation card navigates with
`router.push`, and only when the person clicks.

## Gating, in three lines

```tsx
const { access } = useAccess();
{access.canCreate.organization && <CreateOrganizationDialog … />}
{access.canCreate.workspace && <CreateWorkspaceDialog … />}
```

When quota blocks the workspace dialog, `canCreate.workspace` is `false` and
`reason.workspace === 'workspace_quota_reached'`, so the header can render the disabled row
with the reason instead of hiding it:

```tsx
{access.reason.workspace === 'workspace_quota_reached'
  ? <QuotaDisabledRow />
  : access.canCreate.workspace && <NewWorkspaceRow />}
```

## Navigation

```ts
// lib/keyboard/destinations.ts
{ href: '/organization', icon: Building2, labelKey: 'nav.organization',
  fallback: 'Organization', chord: 'o',
  scope: 'organization', requires: 'organization.settings.manage' },
```

`destinationGroupsFor(access, caps)` filters. `SidebarNav` and the palette call it. One change
fixes both surfaces ([F-181-09](../04-findings.md)).

## Store fix

```ts
selectTenant: (tenantId) => {
  set({ selectedTenantId: tenantId, selectedWorkspaceId: null, workspacesStale: true });
  setTenantContext(tenantId);
},
```

`TenantGuard` renders with the previous list at 60% opacity while `workspacesStale` is true,
then swaps. No spinner, no blank, no CLS.

## Query keys (one namespace)

| Key | Invalidated by |
|-----|---------------|
| `['me-access', tenantId, workspaceId]` | every grant/scope mutation |
| `['tenants']` | create/update/delete organization |
| `['tenant', id]` | update organization |
| `['workspaces', tenantId]` | create/archive/delete workspace |
| `['members', tenantId]` | grant/invite changes |
| `['roles', 'catalog']` | never (5 min stale) |

Rule: a mutation invalidates the **narrowest** key that changed, plus `['me-access']` when the
change could affect capabilities. No `invalidateQueries()` with no argument (today: none, and
we will keep it that way).

## What I delete

| File | Lines | Replaced by |
|------|-------|-------------|
| `components/onboarding/create-tenant-wizard.tsx` | 275 | `admin/create-organization-dialog.tsx` |
| `components/onboarding/create-workspace-wizard.tsx` | 292 | `admin/create-workspace-dialog.tsx` |
| `lib/members/role-meta.ts::ROLE_LEVEL` | 6 | `GET /roles` `level` |
| `lib/onboarding/apply-created-workspace-context.ts` (2 of 3 fns) | ~50 | `applyCreatedScope` |

Kept: `wizard-shell.tsx`, `wizard-stepper.tsx`, `step-meta.ts`, `steps/*` (fields),
`first-run-wizard.tsx`, `reconfigure-workspace-wizard.tsx`.

## Testing notes

- The hermetic mock must be **stateful** (create really creates) so `E-success-clears-draft`
  and `E-two-grant-rows` are meaningful. Pattern: `e2e/spec169/helpers/members-mock.ts`.
- Screenshot helper: reuse `e2e/spec169/helpers/shots.ts` (it already wraps SPEC-168's
  `applyTheme` / `setViewport`).
- The four-persona sidebar snapshot is one `test.describe` with a `for` loop over personas and
  a mock option `viewer: 'platform' | 'owner' | 'wsadmin' | 'member'`.
- Assert **absence** with `expect(locator).toHaveCount(0)`, not `not.toBeVisible()` — a hidden
  element still exists in the DOM, and this pack's promise is that it does not.

## Traps

| Trap | Answer |
|------|--------|
| `useAccess` in a list row → N queries | it is one query, cached by key; rows read the parent's object |
| Radix dialog + a portaled model picker dismisses the dialog on click | the `isPortaledChoiceMenu` pointer guard already in `wizard-shell.tsx` is carried into `ScopeDialog` |
| `applyCreatedScope` before the store hydrated | it is called from a mutation after mount; guard with `_hasHydrated` |
| Deleting `create-workspace-wizard.tsx` breaks `spec180` wizard tests | update them in the same PR to target `data-testid="create-workspace-dialog"` |
| `toAccess` recomputed every render | `useMemo` on `q.data` |
