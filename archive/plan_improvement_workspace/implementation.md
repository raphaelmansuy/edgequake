# Implementation Details

This document describes the code changes made to address the workspace management issues.

## Backend Changes

### 1. Auto-Create Default Workspace on Tenant Creation

**File**: [workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

**Change**: Modified `create_tenant` handler to automatically create a "default" workspace after tenant creation.

```rust
// Auto-create a default workspace for the new tenant (R004)
let default_workspace_request = edgequake_core::CreateWorkspaceRequest {
    name: "Default Workspace".to_string(),
    slug: Some("default".to_string()),
    description: Some("Automatically created default workspace".to_string()),
    max_documents: None,
};

if let Err(e) = state
    .workspace_service
    .create_workspace(created_tenant.tenant_id, default_workspace_request)
    .await
{
    tracing::warn!(/* ... */);
} else {
    tracing::info!(/* ... */);
}
```

### 2. Get Workspace by Slug Endpoint

**File**: [workspaces.rs](../edgequake/crates/edgequake-api/src/handlers/workspaces.rs)

**New Endpoint**: `GET /api/v1/tenants/{tenant_id}/workspaces/by-slug/{slug}`

```rust
pub async fn get_workspace_by_slug(
    State(state): State<AppState>,
    Path((tenant_id, slug)): Path<(Uuid, String)>,
) -> Result<Json<WorkspaceResponse>, ApiError>
```

**Route Registration** (routes.rs):

```rust
.route(
    "/tenants/{tenant_id}/workspaces/by-slug/{slug}",
    get(handlers::get_workspace_by_slug),
)
```

---

## Frontend Changes

### 1. API Function for Workspace by Slug

**File**: [edgequake.ts](../edgequake_webui/src/lib/api/edgequake.ts)

```typescript
export async function getWorkspaceBySlug(
  tenantId: string,
  slug: string
): Promise<Workspace> {
  return api.get<Workspace>(`/tenants/${tenantId}/workspaces/by-slug/${slug}`);
}
```

### 2. Fixed TenantGuard Race Condition

**File**: [tenant-guard.tsx](../edgequake_webui/src/components/layout/tenant-guard.tsx)

**Key Changes**:

1. Added `isSettingUpContext` state to track async context setup
2. Moved mutations before handler definitions to fix TypeScript order
3. Used `mutateAsync` with proper async/await flow
4. Added optimistic updates for immediate UI feedback
5. Await query invalidation before allowing children to render

```typescript
// Handle tenant creation with proper async flow
const handleCreateTenant = useCallback(
  async () => {
    setIsSettingUpContext(true);
    try {
      const newTenant = await createTenantMutation.mutateAsync({
        name: newTenantName,
      });
      selectTenant(newTenant.id);
      await queryClient.invalidateQueries({ queryKey: ["tenants"] });
      await queryClient.invalidateQueries({
        queryKey: ["workspaces", newTenant.id],
      });
      // ...
    } catch (error) {
      setIsSettingUpContext(false);
      // ...
    }
  },
  [
    /* deps */
  ]
);
```

### 3. Slug Field in Workspace Creation Dialog

**File**: [tenant-guard.tsx](../edgequake_webui/src/components/layout/tenant-guard.tsx)

Added slug input with:

- Auto-generation from workspace name
- Real-time validation (lowercase, alphanumeric, hyphens)
- Preview of URL format

```tsx
<div className="grid gap-2">
  <Label htmlFor="workspace-slug">
    {t("workspace.slug", "URL Slug")}
    <span className="text-muted-foreground text-xs ml-2">
      {t("workspace.slugHint", "(optional, auto-generated)")}
    </span>
  </Label>
  <Input
    id="workspace-slug"
    value={newWorkspaceSlug}
    onChange={(e) =>
      setNewWorkspaceSlug(
        e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-")
      )
    }
    placeholder="my-project"
    pattern="[a-z0-9-]+"
  />
  <p className="text-xs text-muted-foreground">
    {t("workspace.slugDescription", "Used in URLs: /w/{slug}/query")}
  </p>
</div>
```

### 4. Workspace URL Synchronization Hook

**New File**: [use-workspace-url.ts](../edgequake_webui/src/hooks/use-workspace-url.ts)

Features:

- Reads workspace slug from URL query parameter (`?workspace=my-project`)
- Resolves slug to workspace ID via API
- Updates URL when workspace changes
- Handles invalid slugs gracefully

```typescript
export function useWorkspaceUrl() {
  // Initialize from URL on first load
  useEffect(
    () => {
      const workspaceSlug = searchParams.get("workspace");
      if (!workspaceSlug) return;

      const workspace = await resolveSlugToWorkspace(
        selectedTenantId,
        workspaceSlug
      );
      if (workspace) {
        selectWorkspace(workspace.id);
      }
    },
    [
      /* deps */
    ]
  );

  // Update URL when workspace changes
  useEffect(() => {
    updateUrlWithWorkspace(selectedWorkspace);
  }, [selectedWorkspace]);
}
```

### 5. Dashboard Layout Integration

**File**: [layout.tsx](<../edgequake_webui/src/app/(dashboard)/layout.tsx>)

Added `WorkspaceUrlSync` component wrapped in Suspense:

```tsx
function WorkspaceUrlSync() {
  useWorkspaceUrl();
  return null;
}

export default function DashboardLayout({ children }) {
  return (
    <div>
      <Suspense fallback={null}>
        <WorkspaceUrlSync />
      </Suspense>
      {/* ... */}
    </div>
  );
}
```

---

## Business Rules Implemented

| Rule | Implementation                                                    |
| ---- | ----------------------------------------------------------------- |
| R001 | TenantGuard auto-selects first tenant                             |
| R002 | TenantGuard auto-selects first workspace                          |
| R003 | TenantGuard shows create dialog for anonymous users               |
| R004 | Backend auto-creates "default" workspace on tenant creation       |
| R005 | Backend validates slug uniqueness within tenant                   |
| R006 | Frontend generates valid slugs (lowercase, alphanumeric, hyphens) |
| R007 | useWorkspaceUrl hook updates URL with workspace slug              |
| R008 | router.replace prevents history pollution                         |

---

## Files Modified

### Backend

1. `edgequake/crates/edgequake-api/src/handlers/workspaces.rs` - Auto-workspace + slug endpoint
2. `edgequake/crates/edgequake-api/src/routes.rs` - New route registration

### Frontend

1. `edgequake_webui/src/lib/api/edgequake.ts` - getWorkspaceBySlug function
2. `edgequake_webui/src/components/layout/tenant-guard.tsx` - Fixed race condition + slug UI
3. `edgequake_webui/src/hooks/use-workspace-url.ts` - New URL sync hook
4. `edgequake_webui/src/app/(dashboard)/layout.tsx` - Integrate URL sync
