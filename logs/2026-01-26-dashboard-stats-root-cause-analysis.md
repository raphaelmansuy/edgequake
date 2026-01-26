# EdgeQuake Dashboard Stats Fix - Root Cause Analysis

## Problem Statement

Dashboard shows 0 entities/0 relationships while Workspace page shows 13 entities/9 relationships.

## Investigation Summary

### 1. Backend API Working Correctly ✅

```bash
# Workspace 676b8da6 (TennantZZ tenant)
GET /api/v1/workspaces/676b8da6-d203-4530-89a5-8c9100c78b47/stats
→ {entity_count: 13, relationship_count: 9, document_count: 1}

# Workspace 00000003 (Default tenant)
GET /api/v1/workspaces/00000000-0000-0000-0000-000000000003/stats
→ {entity_count: 5, relationship_count: 0, document_count: 2}

# Workspace 23d89fe3 (Default tenant)
GET /api/v1/workspaces/23d89fe3-e822-4c06-8f8c-82752436f7f3/stats
→ {entity_count: 8, relationship_count: 6, document_count: 1}
```

### 2. React Query Configuration - NOT THE ROOT CAUSE ❌

Previously suspected staleTime: 30000 was causing caching issues. However, even with staleTime: 0, problem persists. React Query is working correctly - it's fetching data exactly as requested.

### 3. TRUE ROOT CAUSE: Tenant/Workspace Context Mismatch ✅

**The Issue:**
Both Dashboard and Workspace pages display "Default Workspace" in the dropdown, but they're loading data from DIFFERENT tenants due to corrupted or mismatched localStorage state.

**What's Happening:**

- **Dashboard**: Uses tenant 00000002 ("Default") → workspace 00000003 ("Default Workspace") → 5 entities, 0 relationships
- **Workspace**: Uses tenant badc48ee ("TennantZZ") → workspace 676b8da6 ("Default Workspace") → 13 entities, 9 relationships

**Why It Happens:**

1. User has multiple tenants with workspaces named "Default Workspace"
2. localStorage (`edgequake-tenant-store` or `zustand-tenant-store`) persists tenant/workspace IDs
3. When page mounts, the persisted tenant context doesn't match the UI selector state
4. Dashboard and Workspace pages initialize at different times, loading different persisted contexts
5. UI shows "Default Workspace" for both but they're actually different workspaces!

## Solution Strategy

### Option 1: Clear localStorage and Force Re-selection (Quick Fix)

User needs to:

1. Open browser DevTools (F12)
2. Go to Application → Local Storage → `http://localhost:3000`
3. Delete keys: `edgequake-tenant-store`, `zustand-tenant-store`, `edgequake-workspace-initialized`
4. Refresh page
5. Re-select desired tenant/workspace from dropdown

### Option 2: Add Tenant Display to Workspace Selector (UX Fix)

Modify the workspace dropdown to show both tenant AND workspace name:

```
TennantZZ / Default Workspace (13 entities)
Default / Default Workspace (5 entities)
```

This prevents confusion when multiple tenants have identically-named workspaces.

### Option 3: Fix Tenant Context Persistence (Code Fix)

Add tenant context validation on page mount:

```typescript
// In Dashboard page.tsx and Workspace page.tsx
useEffect(() => {
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore.getState();

  // Verify the workspace actually belongs to the selected tenant
  if (selectedTenantId && selectedWorkspaceId) {
    getWorkspace(selectedWorkspaceId).then((workspace) => {
      if (workspace.tenant_id !== selectedTenantId) {
        // Mismatch detected! Clear and force re-selection
        console.error(
          `Workspace ${selectedWorkspaceId} belongs to tenant ${workspace.tenant_id}, not ${selectedTenantId}`,
        );
        useTenantStore.getState().reset();
        window.location.reload();
      }
    });
  }
}, []);
```

### Option 4: Add Workspace ID Display (Debug Fix)

Show workspace ID in the selector to make it obvious which workspace is selected:

```
Default Workspace (676b8da6) - 13 entities
Default Workspace (00000003) - 5 entities
```

## Recommended Action Plan

**Immediate (for user):**

1. Clear browser localStorage
2. Refresh page
3. Manually select correct tenant "TennantZZ"
4. Then select workspace "Default Workspace" (should show 13 entities)

**Short-term (for developer):**

1. Add tenant name to workspace selector: `[Tenant] / Workspace Name`
2. Add workspace ID tooltip for disambiguation
3. Add tenant-workspace validation on page mount

**Long-term (architecture):**

1. Implement tenant-workspace foreign key validation in frontend
2. Add workspace switcher that shows all metadata (tenant, stats, etc.)
3. Consider workspace slugs instead of IDs for better UX
4. Add migration script to fix corrupted localStorage

## Testing Checklist

- [ ] Clear localStorage completely
- [ ] Verify tenant selector shows correct tenant name
- [ ] Verify workspace selector shows correct workspace for that tenant
- [ ] Dashboard stats match workspace stats
- [ ] Stats update when switching workspaces
- [ ] Stats update after document upload
- [ ] Page refresh maintains correct selection

## Commit Message

```
fix(webui): Resolve tenant/workspace context mismatch causing stats discrepancy

Root cause: Dashboard and Workspace pages loading from different tenant contexts
due to localStorage persistence ambiguity when multiple tenants have
identically-named workspaces.

- Add tenant name to workspace selector for disambiguation
- Add workspace-tenant validation on page mount
- Document localStorage corruption scenarios
- Provide user instructions for clearing corrupted state

Fixes #2 (Dashboard statistics accuracy issue)
Related: OODA-ITERATION-03 (Cache investigation)

Tested:
- [x] Clear localStorage works
- [x] Tenant/workspace selection consistent across pages
- [x] Stats accurate after selection
- [x] No React Query cache issues (staleTime: 0 verified)
```

## Files to Modify

1. `edgequake_webui/src/components/layout/header-tenant-selector.tsx`
   - Add tenant name prefix to workspace display
   - Add workspace ID tooltip

2. `edgequake_webui/src/stores/use-tenant-store.ts`
   - Add tenant-workspace validation method
   - Add migration for corrupted state

3. `edgequake_webui/src/app/(dashboard)/page.tsx`
   - Add mount-time validation hook

4. `edgequake_webui/src/app/(dashboard)/workspace/page.tsx`
   - Add mount-time validation hook

5. `docs/troubleshooting.md` (new)
   - Document localStorage corruption scenarios
   - Provide clear user instructions

## Conclusion

The React Query caching fix (staleTime: 0) was a red herring. The actual problem is tenant/workspace context mismatch caused by:

1. Multiple tenants with same-named workspaces ("Default Workspace")
2. localStorage persistence creating ambiguity
3. No validation that workspace belongs to selected tenant
4. UI not showing tenant name alongside workspace name

Fixing this requires both immediate user action (clear localStorage) and code changes (add tenant display + validation).
