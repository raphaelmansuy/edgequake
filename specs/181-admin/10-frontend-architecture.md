# 10 — Frontend architecture

Parent: [README](README.md) · Prev: [09-ux-ui-spec](09-ux-ui-spec.md) ·
Next: [11-design-system](11-design-system.md)

One access hook, one mutation per intent, one dialog per intent. Every other surface reads
those. This file is the code-level contract; [09](09-ux-ui-spec.md) is the drawing.

---

## Layer diagram

```text
  ┌──────────────────────────────────────────────────────────────────────────────┐
  │  PRESENTATION                                                                │
  │  HeaderTenantSelector   OrganizationPage   WorkspacesSection   MembersPage    │
  │  ContextSelectorPopover TenantGuard        OrganizationsSection GrantEditor   │
  └───────────────────────────────┬──────────────────────────────────────────────┘
                                  │ reads access, calls intents
  ┌───────────────────────────────▼──────────────────────────────────────────────┐
  │  INTENT                                                                      │
  │  <CreateOrganizationDialog/>  <CreateWorkspaceDialog/>  <GrantEditor/>        │
  │  <InviteDialog/>              <DeleteScopeDialog/>                            │
  └───────────────────────────────┬──────────────────────────────────────────────┘
                                  │ useCreateOrganization / useCreateWorkspace …
  ┌───────────────────────────────▼──────────────────────────────────────────────┐
  │  DATA                                                                        │
  │  hooks/use-access.ts        hooks/use-create-scope.ts                        │
  │  lib/api/edgequake/access.ts  …/workspaces.ts  …/members.ts  …/grants.ts     │
  │  TanStack Query cache: ['me-access', tenantId, workspaceId]                  │
  └───────────────────────────────┬──────────────────────────────────────────────┘
                                  │
  ┌───────────────────────────────▼──────────────────────────────────────────────┐
  │  SERVER   GET /me/access · POST /tenants · POST /tenants/{t}/workspaces · …   │
  └──────────────────────────────────────────────────────────────────────────────┘
```

Dependencies point down. A presentation component never builds a request body; a data hook
never renders.

---

## 1 — The access hook (the single source of truth in the UI)

`src/lib/access/use-access.ts`

```ts
export interface Access {
  platform: { role: InstanceRole; admin: boolean };
  organization: {
    id: string; name: string; slug: string;
    role: OrgRole | null; level: number;
    capabilities: ReadonlySet<Capability>;
  } | null;
  workspaces: readonly {
    id: string; name: string; slug: string;
    role: OrgRole; level: number;
    capabilities: ReadonlySet<Capability>;
  }[];
  quota: { workspaces: Budget; seats: Budget };
  canCreate: { organization: boolean; workspace: boolean };
  canDelete: { organization: boolean };
  reason: { organization: string | null; workspace: string | null };
}

export function useAccess(): { access: Access; ready: boolean; error: boolean }
```

Rules, all enforced by lint and test:

1. **Fail closed.** Before the server answers, `access` is `NO_ACCESS`: empty capability sets,
   `canCreate` false. Gated content is not rendered, so nothing flashes and then disappears
   ([LAW-181-3](01-first-principles.md)).
2. **One query key.** `['me-access', tenantId, workspaceId]`. Every mutation that changes a
   grant, a quota or a scope invalidates that prefix — not five separate keys.
3. **No role arithmetic in the client.** A component asks `access.organization.capabilities
   .has('organization.workspaces.create')`. Nobody writes `level >= 3`.
4. **No local rank table.** `lib/members/role-meta.ts` keeps labels and icons; `ROLE_LEVEL`
   is deleted and read from `GET /roles`'s `level` field.

`use-access.ts` also exports the two questions the buttons actually ask, so the mapping lives
in one place:

```ts
export const canCreateOrganization = (a: Access) => a.canCreate.organization;
export const canCreateWorkspace = (a: Access) => a.canCreate.workspace;
export const workspaceCapabilities = (a: Access, id: string) =>
  a.workspaces.find(w => w.id === id)?.capabilities ?? EMPTY;
```

---

## 2 — The create mutation

`src/hooks/use-create-scope.ts` owns the entire sequence, which today is spread across
`create-tenant-wizard.tsx` (two API calls, `setTenantContext`, two `invalidateQueries`, a
toast, a `window.location.href`).

```ts
export function useCreateOrganization() {
  return useMutation({
    mutationFn: (body: CreateOrganizationBody) => createOrganization(body),
    onSuccess: async (res) => {
      applyCreatedScope(res);                       // store + client-context + URL
      await Promise.all([
        qc.invalidateQueries({ queryKey: ['tenants'] }),
        qc.invalidateQueries({ queryKey: ['me-access'] }),
        qc.invalidateQueries({ queryKey: ['workspaces', res.tenant.id] }),
      ]);
    },
  });
}
```

`applyCreatedScope` replaces both `applyCreatedTenantContext` and
`applyCreatedWorkspaceContext` (they differ only in which id they pass):

```ts
export function applyCreatedScope(res: CreatedScope): void {
  const s = useTenantStore.getState();
  s.selectTenant(res.tenant.id);          // does not blank workspaces (see below)
  s.setWorkspaces(res.workspaces);
  s.selectWorkspace(res.workspace.id);
  setTenantContext(res.tenant.id, res.workspace.id);
  syncUrl(res.workspace, res.tenant);
}
```

### Store change required

`selectTenant` currently does:

```ts
set({ selectedTenantId: tenantId, selectedWorkspaceId: null, workspaces: [] });
```

Emptying `workspaces` synchronously means `TenantGuard` renders a spinner for one frame after
every create and every switch. The change: keep the old list until the new one arrives, and
mark it stale.

```ts
set({ selectedTenantId: tenantId, selectedWorkspaceId: null, workspacesStale: true });
```

`TenantGuard` reads `workspacesStale` to dim the switcher instead of unmounting the page
([EC-181-33](12-edge-cases.md)).

---

## 3 — The dialog shell

Both dialogs share `components/admin/scope-dialog.tsx`:

```text
  <ScopeDialog>
    <ScopeDialogHeader  title lead quota? />
    <ScopeDialogBody>            // the 3 decisions, scrolls
      …
      <AdvancedDrawer summary={n} />   // optional, last
    </ScopeDialogBody>
    <ScopeDialogFooter
      blockedReason={string | null}
      cancelLabel finishLabel busy
      onCancel onFinish />
  </ScopeDialog>
```

It reuses the accessibility work already proven in `wizard-shell.tsx` — focus on the first
field, `aria-live` step/quota region, `Esc` with a dirty-confirm, portaled-menu pointer guard —
**without** the stepper, the step counter, the back button or the `steps[]` machinery. That is
the DRY split: `wizard-shell` remains for `first-run-wizard` (which legitimately has ordered
steps on an empty install), while `scope-dialog` is the 3-decision case.

`blockedReason` is the one channel for "why can't I finish": missing name, quota full,
server refusal. It renders in the footer where the eye already is, matching today's
`wizard-blocked-reason` placement.

---

## 4 — Wizard engine changes

`lib/onboarding/wizard-state.ts`:

```ts
} else if (kind === 'create-tenant' || kind === 'create-workspace') {
  // The short path. Fields live in the Advanced drawer, not in steps.
  steps = ['basics', 'review'];
}
```

`kind: 'first-run'` and `'reconfigure-workspace'` keep their long step lists: the first is the
one time the operator *is* choosing everything, and the second is a change review.

New blocker codes:

| Code | Meaning | Rendered as |
|------|---------|-------------|
| `quota_reached` | `quota.workspaces.used >= limit` | the quota strip copy |
| `capability_missing` | the caller somehow reached the dialog without the right | "You don't have permission to create workspaces here." |
| `slug_taken` | server 409 | inline under the name field, with [Open it] |
| `scope_inactive` | organization archived | "This organization is archived. Restore it first." |

---

## 5 — Navigation from access

`lib/keyboard/destinations.ts` gains `requires` and a scope:

```ts
export interface Destination {
  href: string; icon: LucideIcon; labelKey: string; fallback: string;
  chord: string;
  scope: 'workspace' | 'organization' | 'deployment';
  requires?: Capability;        // absent when the capability is missing
  needsGraph?: boolean;         // SPEC-180, unchanged
}
```

```ts
export function destinationGroupsFor(access: Access, caps?: GraphView) {
  return DESTINATION_GROUPS
    .map(g => ({ ...g, items: g.items.filter(d =>
        isOffered(d, caps) &&
        (!d.requires || hasCapability(access, d.scope, d.requires)) }))
    .filter(g => g.items.length > 0);
}
```

`SidebarNav` already calls `destinationGroupsFor(useWorkspaceCapabilities())`; it becomes
`destinationGroupsFor(access, useWorkspaceCapabilities())`. The command palette reads the same
function, so the two cannot disagree ([F-181-09](04-findings.md)).

---

## 6 — Route guards

Three thin wrappers over `useAccess`, each a redirect or a state — never a spinner that hangs:

| Guard | Rule | On failure |
|-------|------|-----------|
| `<RequireOrganization>` | `access.organization !== null` | [W-02](09-ux-ui-spec.md#w-02-empty-state--no-organization) empty state |
| `<RequireOrganizationManager>` | `organization.capabilities.has('organization.settings.manage')` | redirect `/` |
| `<RequirePlatform>` | `access.platform.admin` | redirect `/` |

They compose inside the existing `AuthGuard` → `TenantGuard` chain in
`app/(dashboard)/layout.tsx`, so there is still exactly one guard stack.

---

## 7 — Server components?

No. The whole WebUI is client-rendered with TanStack Query and the shell already ships as
`'use client'`. Introducing RSC for these two dialogs would add a second data path for one
screen. The rules above are the contract; the rendering model stays as it is.

---

## 8 — File-level rules, enforceable by lint

| Rule | Why | How |
|------|-----|-----|
| No `level >=` or `role === 'owner'` comparisons in `components/**` | Prevents a second permission model | `no-restricted-syntax` on `>=`/`===` near `role`/`level` in components |
| No `fetch(`/`api.` outside `lib/api/**` and `hooks/**` | SRP | existing project convention, extended to the new files |
| One export named `createOrganization` / `createWorkspace` | One entry point per intent | grep gate in [14](14-e2e-test-matrix.md#structural-gates) |
| `useAccess` is imported, never re-implemented | one source | grep gate: `me/access` appears in exactly one file |
| Every new user-facing string is in `locales/{en,fr,zh}.json` | i18n parity | existing lint |

---

## 9 — Performance budget

| Interaction | Budget | How |
|-------------|--------|-----|
| Switcher open | < 50 ms | tenants + workspaces already cached; access cached 30 s |
| Create organization round trip | < 400 ms p95 | one transaction, one response, no follow-up GET |
| Create workspace round trip | < 400 ms p95 | same |
| `/me/access` | < 25 ms p95 | three indexed statements (see [07 §4](07-data-and-db-contract.md#4--query-patterns-the-new-surfaces-use)) |
| Grant change reflected in the UI | < 150 ms | optimistic update + invalidate `['me-access']` |
| CLS on the switcher and both dialogs | 0 | fixed heights, skeletons that match final size, no font swap |

`/me/access` is fetched once per `(tenant, workspace)` per 30 s and is invalidated by every
mutating hook, so the effective rate is bounded by user actions, not by polling.

---

## 10 — Migration of existing components

| Existing | Becomes | Notes |
|----------|---------|-------|
| `create-tenant-wizard.tsx` (275 lines) | deleted | logic → `useCreateOrganization`, fields → `create-organization-dialog.tsx` |
| `create-workspace-wizard.tsx` (292 lines) | deleted | same |
| `steps/model-defaults-step.tsx` | reused inside the drawer | unchanged component, new host |
| `steps/workspace-basics-step.tsx` | reused | name/slug fields |
| `steps/workspace-retrieval-step.tsx` | reused | the retrieval radios |
| `steps/{chunking,extract-budget,extraction,document-parsing}-step.tsx` | reused in the drawer | |
| `steps/admin-credentials-step.tsx` | kept | first-run only |
| `steps/review-step.tsx` | kept for first-run | the short dialogs use [W-05](09-ux-ui-spec.md#w-05-confirmation-after-create) instead of a review step |
| `apply-created-workspace-context.ts` | `apply-created-scope.ts` | one function |
| `header-tenant-selector.tsx` create rows | gated | reads `useAccess` |
| `admin-quota-section.tsx` | absorbed by `organizations-section.tsx` | the per-tenant list replaces the card |

Net lines: **−1,050** in `components/onboarding`, **+900** in `components/admin` +
`lib/access`, so the change is roughly size-neutral while removing two wizards and three
duplicated sequences.
