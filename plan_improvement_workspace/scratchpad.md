# Workspace Improvement Scratchpad

## Analysis Notes

### Current State Analysis

#### Tenant Store (`use-tenant-store.ts`)

- Uses Zustand with persist middleware
- Persists `selectedTenantId` and `selectedWorkspaceId` to localStorage
- Initial state has `selectedTenantId: null` and `selectedWorkspaceId: null`
- `initializeFromStorage()` reads from `getTenantContext()` in client.ts
- **Issue**: Race condition between localStorage restore and API fetch

#### TenantGuard (`tenant-guard.tsx`)

- Acts as a guard ensuring tenant/workspace are selected
- Shows "Create Tenant" when no tenants exist
- Shows "Create Workspace" when tenant selected but no workspaces
- Shows loading spinner when `!selectedTenantId || !selectedWorkspaceId`
- **Issue**: After workspace creation, query invalidation may not complete before children render

#### Client API (`client.ts`)

- `getTenantContext()` reads from localStorage
- `setTenantContext()` writes to localStorage
- Headers include `X-Tenant-ID` and `X-Workspace-ID`
- **Issue**: If localStorage is cleared/empty, headers will be null

#### Conversation Store (`use-conversation-store.ts`)

- Stores conversations per workspace (has tenantId/workspaceId fields)
- `createConversation()` takes optional tenant/workspace IDs
- Uses localStorage persistence
- **Issue**: If workspace changes, old conversations remain in localStorage

### Root Cause Analysis

#### Problem 1: First Workspace Desync

**Flow**:

1. User lands on app → no tenant/workspace in localStorage
2. TenantGuard fetches tenants → 0 found
3. User creates tenant → selectTenant() called
4. TenantGuard fetches workspaces → 0 found
5. User creates workspace → selectWorkspace() called
6. Query invalidation for workspaces happens
7. BUT: Children may already be rendering before invalidation completes

**Root Cause**:

- Race condition between mutation success callback and query cache update
- `selectWorkspace()` is called before workspacesData is refetched
- The store has the ID, but the `workspacesQuery.data` may not include it yet

**Fix**:

- Invalidate and await refetch before rendering children
- Use mutation's returned workspace to update store immediately

#### Problem 2: Default Tenant Not Selected

**Flow in non-auth mode**:

1. App loads → localStorage empty
2. `initializeFromStorage()` returns null tenant
3. `getTenants()` called
4. If tenants exist, auto-select first one
5. If no tenants exist, show "Create Tenant" dialog

**Issue**:

- What if API returns error or empty but localStorage had stale value?
- Need to handle: None tenant = impossible state

**Fix**:

- Ensure if no tenants exist AND user is not authenticated, create a "Default" tenant automatically
- Or: Block app until at least one tenant exists

#### Problem 3: URL Not Reflecting Workspace

**Current state**:

- URL is just `/query`, `/documents`, etc.
- No workspace identifier in URL

**Fix**:

- Add workspace slug to URL: `/w/{workspace-slug}/query`
- Or add as query param: `/query?workspace=my-workspace`

#### Problem 4: UUID vs Slug in URL

**Current state**:

- Backend already supports `slug` field on Workspace
- Frontend uses UUIDs everywhere

**Fix**:

- Add `get_workspace_by_slug` endpoint if not exists
- Use slug in URL, resolve to UUID for API calls

#### Problem 5: Slug Field Missing from UI

**Current state**:

- CreateWorkspaceApiRequest has optional `slug` field
- Backend auto-generates if not provided
- Frontend doesn't expose slug in creation form

**Fix**:

- Add slug input to workspace creation dialog
- Add validation for slug format (lowercase, alphanumeric, hyphens)
- Show conflict error if slug exists in tenant

## Business Rules

- R001: It is impossible to have no tenant selected after initial load completes
- R002: It is impossible to have no workspace selected after tenant is selected
- R003: In non-authenticated mode, a default tenant must always exist
- R004: Each tenant must have at least one workspace (auto-create "default")
- R005: Workspace slugs must be unique within a tenant
- R006: Slugs must be URL-safe: lowercase, alphanumeric, hyphens only
- R007: URL must reflect the current workspace context

## Implementation Plan

### Step 1: Backend - Add Default Workspace Auto-Creation

File: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`

- Modify `create_tenant` to auto-create a "default" workspace

### Step 2: Frontend - Fix Tenant Store Race Condition

File: `edgequake_webui/src/stores/use-tenant-store.ts`

- Add `isReady` flag that only becomes true when BOTH tenant and workspace are confirmed

### Step 3: Frontend - Improve TenantGuard

File: `edgequake_webui/src/components/layout/tenant-guard.tsx`

- Wait for workspace query to settle after creation
- Use optimistic update pattern

### Step 4: Frontend - Add Slug to Workspace Creation

File: `edgequake_webui/src/components/layout/tenant-guard.tsx`

- Add slug input field
- Add slug validation
- Generate slug from name if not provided

### Step 5: Frontend - URL Routing with Workspace

- Create new dynamic route: `/w/[workspace]/[...path]`
- Read workspace from URL on page load
- Update URL when workspace changes
