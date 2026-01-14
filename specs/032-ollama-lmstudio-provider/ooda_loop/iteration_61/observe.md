# OODA 61: Observe

## Deeplink Page + TenantGuard Integration Issue

### Problem
When navigating to a deeplink like `/w/default-workspace/query`, the TenantGuard component sometimes shows "Create a Workspace" even when workspaces exist in the database.

### Root Cause Analysis

1. **localStorage is cleared** in E2E beforeEach hook
2. **No stored tenant/workspace selection** - both start as null
3. **TenantGuard runs first** and:
   - Fetches tenants → auto-selects first
   - Fetches workspaces for that tenant
   - Checks `workspacesData.length === 0` before deeplink page can set workspace
4. **Deeplink page runs** and:
   - Also fetches tenants → auto-selects first
   - Fetches workspace by slug
   - Calls `selectWorkspace(workspace.id)`
5. **Race condition**: TenantGuard's check happens before selectWorkspace completes

### Current TenantGuard Logic (Line 396)
```tsx
// Tenant selected but no workspaces exist - prompt to create one
if (selectedTenantId && workspacesData && workspacesData.length === 0) {
  return <CreateWorkspaceUI />;
}
```

This check is correct for normal pages but incorrect for deeplink pages that load a specific workspace.

### Observation: Breadcrumb Shows Correct Route
Even when "Create Workspace" UI appears, the breadcrumb correctly shows:
```
EdgeQuake > w > default-workspace > Query
```

This proves the route resolved correctly but TenantGuard blocked the render.

### Current Workaround
E2E tests accept "Create Workspace" UI as valid when breadcrumb shows correct workspace slug.

### Desired Behavior
Deeplink pages should render their content without TenantGuard blocking, OR TenantGuard should recognize when a workspace is being loaded via deeplink.
