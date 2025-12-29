# Workspace Management Improvement Specification

## Executive Summary

This document addresses five critical issues in the EdgeQuake workspace management system:

1. Workspace creation desynchronization
2. Fresh start in non-authenticated mode
3. URL not reflecting workspace context
4. UUID vs slug in URLs
5. Missing slug field in workspace creation

## Table of Contents

1. [Root Cause Analysis](#root-cause-analysis)
2. [Business Rules](#business-rules)
3. [Improvement Plan](#improvement-plan)
4. [Implementation Details](#implementation-details)
5. [Verification Plan](#verification-plan)

---

## Root Cause Analysis

### Problem 1: Workspace Creation Desynchronization

**Symptoms**:

- After creating first workspace, Query page shows errors
- LocalStorage has workspace ID but API calls fail
- Race condition between UI update and data availability

**Code Flow Analysis**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ CURRENT BROKEN FLOW                                                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  User Creates Workspace                                                     │
│         │                                                                   │
│         ▼                                                                   │
│  ┌──────────────────┐                                                       │
│  │ createWorkspace  │ ──────► API Call to Backend                          │
│  │    Mutation      │                                                       │
│  └────────┬─────────┘                                                       │
│           │                                                                 │
│           ▼                                                                 │
│  ┌──────────────────┐    ┌─────────────────────┐                           │
│  │   onSuccess:     │    │ invalidateQueries   │                           │
│  │ selectWorkspace  │ ──►│ (async, non-await)  │                           │
│  └────────┬─────────┘    └─────────┬───────────┘                           │
│           │                        │                                        │
│           ▼                        │ (runs in background)                   │
│  ┌──────────────────┐              │                                        │
│  │  Store Updated   │              │                                        │
│  │ selectedWorkspaceId             │                                        │
│  └────────┬─────────┘              │                                        │
│           │                        │                                        │
│           ▼                        ▼                                        │
│  ┌──────────────────────────────────────────────┐                          │
│  │        TenantGuard renders children          │  ◄── PROBLEM!            │
│  │   (workspacesData may not include new ws)    │      Data mismatch       │
│  └──────────────────────────────────────────────┘                          │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Root Cause Files**:

- [tenant-guard.tsx](../edgequake_webui/src/components/layout/tenant-guard.tsx#L93-L104) - onSuccess callback
- [use-tenant-store.ts](../edgequake_webui/src/stores/use-tenant-store.ts#L56-L64) - selectWorkspace function

**Issue**:

1. `invalidateQueries` is fire-and-forget, not awaited
2. `selectWorkspace()` updates localStorage immediately
3. Children render before React Query refetch completes
4. API calls use stale/missing workspace context

---

### Problem 2: Fresh Start in Non-Authenticated Mode

**Symptoms**:

- Empty localStorage leads to null tenant/workspace
- API calls fail with missing headers
- No default tenant auto-creation

**Code Flow Analysis**:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ NON-AUTHENTICATED FRESH START FLOW                                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  App Loads (localStorage empty)                                             │
│         │                                                                   │
│         ▼                                                                   │
│  ┌──────────────────┐                                                       │
│  │ TenantGuard      │                                                       │
│  │ mounts           │                                                       │
│  └────────┬─────────┘                                                       │
│           │                                                                 │
│           ├───────────────────────────────────────────┐                    │
│           │                                           │                    │
│           ▼                                           ▼                    │
│  ┌──────────────────┐                    ┌──────────────────┐              │
│  │ initializeFrom   │                    │ useQuery         │              │
│  │ Storage()        │                    │ getTenants()     │              │
│  │ (returns null)   │                    │                  │              │
│  └──────────────────┘                    └────────┬─────────┘              │
│                                                   │                        │
│                                                   ▼                        │
│                                          ┌──────────────────┐              │
│                                          │ tenants = []     │              │
│                                          │ Show "Create     │              │
│                                          │ Tenant" dialog   │              │
│                                          └──────────────────┘              │
│                                                                             │
│  ◄── CURRENT: Works, but requires manual action                            │
│  ◄── DESIRED: Auto-create default tenant for anonymous users               │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Root Cause Files**:

- [tenant-guard.tsx](../edgequake_webui/src/components/layout/tenant-guard.tsx#L158-L184) - No tenants UI
- [client.ts](../edgequake_webui/src/lib/api/client.ts#L107-L119) - getTenantContext

**Issue**:

1. No automatic default tenant creation for anonymous users
2. No automatic default workspace creation when tenant is created
3. User must manually create both, leading to friction

---

### Problem 3: URL Not Reflecting Workspace

**Current State**:

- URL: `/query`, `/documents`, `/graph`
- No workspace identifier in URL
- Users cannot share links to specific workspaces
- Browser back/forward doesn't preserve workspace context

**Root Cause Files**:

- [layout.tsx](<../edgequake_webui/src/app/(dashboard)/layout.tsx>) - No dynamic routing
- `src/app/(dashboard)/*` - Static routes

---

### Problem 4: UUID vs Slug in URL

**Current State**:

- Backend supports workspace slugs (see multitenancy.rs)
- Frontend only uses UUIDs
- UUIDs are not human-readable: `/w/550e8400-e29b-41d4-a716-446655440000`

**Desired State**:

- Human-readable URLs: `/w/my-project/query`

**Root Cause Files**:

- [multitenancy.rs](../edgequake/crates/edgequake-core/src/types/multitenancy.rs#L148-L175) - Workspace struct has slug
- [workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs#L52) - slug in API request

---

### Problem 5: Missing Slug Field in UI

**Current State**:

- Backend accepts optional `slug` in CreateWorkspaceApiRequest
- If not provided, auto-generates from name
- Frontend CreateWorkspace dialog has no slug input

**Root Cause Files**:

- [tenant-guard.tsx](../edgequake_webui/src/components/layout/tenant-guard.tsx#L262-L277) - Dialog only has name field

---

## Business Rules

| Rule | Description                                                         | Enforcement Point  |
| ---- | ------------------------------------------------------------------- | ------------------ |
| R001 | A tenant must always be selected after app initialization completes | TenantGuard        |
| R002 | A workspace must always be selected when tenant is selected         | TenantGuard        |
| R003 | In non-auth mode, auto-create default tenant if none exists         | Backend/Frontend   |
| R004 | When tenant is created, auto-create "default" workspace             | Backend            |
| R005 | Workspace slugs must be unique within tenant                        | Backend validation |
| R006 | Slugs: lowercase, alphanumeric, hyphens, 3-50 chars                 | Frontend+Backend   |
| R007 | URL must always reflect current workspace slug                      | Router             |
| R008 | Changing workspace updates URL without full page reload             | Next.js router     |

---

## Improvement Plan

### Phase 1: Backend Fixes

#### 1.1 Auto-Create Default Workspace on Tenant Creation

**File**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`

**Change**: Modify `create_tenant` handler to:

1. Create tenant
2. Auto-create "default" workspace for the tenant
3. Return tenant with workspace info

#### 1.2 Add Get Workspace by Slug Endpoint

**File**: `edgequake/crates/edgequake-api/src/handlers/workspaces.rs`

**Change**: Add new endpoint:

```
GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}
```

### Phase 2: Frontend Store Fixes

#### 2.1 Fix Race Condition in Workspace Selection

**File**: `edgequake_webui/src/stores/use-tenant-store.ts`

**Change**:

1. Add `isContextReady` computed flag
2. Only set ready when both tenant and workspace are confirmed in cache

#### 2.2 Improve TenantGuard Mutation Flow

**File**: `edgequake_webui/src/components/layout/tenant-guard.tsx`

**Change**:

1. Use `mutateAsync` and await query refetch
2. Add optimistic update pattern
3. Show loading until data is confirmed

### Phase 3: Slug Support in UI

#### 3.1 Add Slug Field to Workspace Creation

**File**: `edgequake_webui/src/components/layout/tenant-guard.tsx`

**Change**:

1. Add slug input with auto-generation from name
2. Add real-time slug validation
3. Show slug conflict errors

### Phase 4: URL Routing

#### 4.1 Create Dynamic Workspace Route

**New Files**:

- `edgequake_webui/src/app/(dashboard)/w/[workspace]/layout.tsx`
- `edgequake_webui/src/app/(dashboard)/w/[workspace]/query/page.tsx`
- (repeat for other pages)

#### 4.2 Add URL Sync Hook

**File**: `edgequake_webui/src/hooks/use-workspace-url.ts`

**Change**: Create hook that:

1. Reads workspace slug from URL on mount
2. Resolves to workspace ID via API
3. Updates URL when workspace changes

---

## Implementation Details

See [implementation.md](./implementation.md) for code changes.

---

## Verification Plan

### Manual Testing Steps

1. **Fresh Start Test**

   - Clear localStorage
   - Load app
   - Verify default tenant auto-created (or create tenant dialog shown)
   - Create tenant
   - Verify default workspace auto-created
   - Navigate to Query page
   - Verify no errors

2. **Workspace Creation Test**

   - Create new workspace with custom slug
   - Verify URL updates to include slug
   - Navigate to Query page
   - Create conversation
   - Verify no desync errors

3. **URL Sharing Test**

   - Copy URL with workspace slug
   - Open in new tab
   - Verify correct workspace loaded
   - Verify can use Query page immediately

4. **Slug Conflict Test**
   - Create workspace with slug "my-project"
   - Try to create another with same slug
   - Verify conflict error shown

### Playwright E2E Tests

See [e2e/workspace-management.spec.ts](../edgequake_webui/e2e/workspace-management.spec.ts)
