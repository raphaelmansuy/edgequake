# Plan: Deep Links for Tenant / Workspace State

**Addresses:** F-WS-02  
**Files to change:** `src/stores/use-tenant-store.ts`, `src/hooks/use-workspace-url.ts`, `src/app/(dashboard)/layout.tsx`

---

## Current State

The workspace is persisted in `localStorage` via Zustand. URL only gets `?workspace=<slug>` param on the dashboard page. Tenant is never in the URL.

## Target State

All routes support `?tenant=<tenantSlug>&workspace=<workspaceSlug>` query params.  
These params take precedence over localStorage on first load.  
Sharing a URL preserves the full context.

---

## Implementation

### 1. Extend `use-workspace-url.ts` to include tenant

```ts
// Current: only writes ?workspace=
// Add: also write ?tenant=

const tenantSlug = selectedTenant?.name
  .toLowerCase()
  .replace(/[^a-z0-9]+/g, '-');

params.set('tenant', tenantSlug);
params.set('workspace', workspace.slug);
```

### 2. Read tenant from URL on mount

```ts
// In header-tenant-selector.tsx useEffect (after tenantsData loads):
const tenantFromUrl = searchParams.get('tenant');
if (tenantFromUrl && tenantsData) {
  const match = tenantsData.find(t =>
    t.name.toLowerCase().replace(/[^a-z0-9]+/g, '-') === tenantFromUrl
  );
  if (match && match.id !== selectedTenantId) {
    selectTenant(match.id);
  }
}
```

### 3. Add /t/[tenantSlug]/w/[workspaceSlug] route alias (optional)

For clean deeplinks, the `/w/[slug]` route already exists. Extend to `/t/[tenantSlug]/w/[workspaceSlug]`.

---

## URL Examples

```
# Dashboard, specific workspace
http://localhost:3000/?tenant=default&workspace=research

# Documents in production workspace  
http://localhost:3000/documents?tenant=production&workspace=main

# Graph in staging
http://localhost:3000/graph?tenant=staging&workspace=default
```

---

## Acceptance Criteria

- [ ] Navigating to `/?tenant=default&workspace=default` loads the correct context
- [ ] Switching workspace updates the URL params
- [ ] Switching tenant updates the URL params
- [ ] On fresh page load, URL params take precedence over localStorage
- [ ] Invalid tenant/workspace slugs fall back to first available
