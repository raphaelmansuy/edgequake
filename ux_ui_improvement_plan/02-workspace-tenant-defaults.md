# Workspace/Tenant Default Selection

**Priority:** HIGH  
**Estimated Effort:** 2-3 days  
**Complexity:** Medium

## Problem Statement

Currently, users must manually select a workspace and tenant every time they use the application. This creates friction in the user experience, especially for:

1. **First-time users:** No default workspace, see empty state
2. **Returning users:** Must re-select their context on each visit
3. **Single-tenant users:** Unnecessary selection step for 80% of users

## Current Behavior Analysis

### Existing Implementation
```typescript
// From use-tenant-store.ts
useEffect(() => {
  if (tenantsData) {
    setTenants(tenantsData);
    // ❌ PROBLEM: Only selects if no selection exists
    if (!selectedTenantId && tenantsData.length > 0) {
      selectTenant(tenantsData[0].id);
    }
  }
}, [tenantsData, setTenants, selectedTenantId, selectTenant]);
```

### Issues Identified
1. **No automatic workspace selection:** Tenant is selected but not workspace
2. **Modal blocking:** Create workspace dialog appears on empty state (screenshot shows this)
3. **No persistence priority:** Last-used context not remembered across sessions
4. **No onboarding flow:** First-time users get confused

## Solution Design

### 1. Intelligent Default Selection Algorithm

```typescript
/**
 * Smart Context Selection Priority:
 * 1. Last used context (from localStorage)
 * 2. Default workspace/tenant (user preference)
 * 3. First available workspace
 * 4. Create workspace prompt (only if none exist)
 */

interface ContextSelectionStrategy {
  // Check localStorage for last context
  getLastUsedContext(): { tenantId: string; workspaceId: string } | null;
  
  // Get user's preferred default
  getDefaultContext(): { tenantId: string; workspaceId: string } | null;
  
  // Auto-select first available
  selectFirstAvailable(): void;
  
  // Show onboarding for new users
  showOnboarding(): boolean;
}
```

### 2. Enhanced Store Implementation

```typescript
// Enhanced use-tenant-store.ts
export const useTenantStore = create<TenantStore>()(
  persist(
    (set, get) => ({
      ...initialState,
      
      // New: Auto-initialization flag
      isInitialized: false,
      
      // New: Onboarding state
      needsOnboarding: false,
      
      // Enhanced: Smart initialization
      initializeContext: async () => {
        const { tenants, workspaces } = get();
        
        // 1. Try last used context
        const lastUsed = getLastUsedContext();
        if (lastUsed && validateContext(lastUsed)) {
          set({
            selectedTenantId: lastUsed.tenantId,
            selectedWorkspaceId: lastUsed.workspaceId,
            isInitialized: true,
          });
          return;
        }
        
        // 2. Try default context from API
        const defaultContext = await fetchDefaultContext();
        if (defaultContext) {
          set({
            selectedTenantId: defaultContext.tenantId,
            selectedWorkspaceId: defaultContext.workspaceId,
            isInitialized: true,
          });
          return;
        }
        
        // 3. Auto-select first available
        if (tenants.length > 0) {
          const tenant = tenants[0];
          const workspace = workspaces[0];
          
          if (workspace) {
            set({
              selectedTenantId: tenant.id,
              selectedWorkspaceId: workspace.id,
              isInitialized: true,
            });
            return;
          }
          
          // Has tenant but no workspace -> create default
          const newWorkspace = await createWorkspace(tenant.id, {
            name: 'Default Workspace',
            description: 'Automatically created workspace'
          });
          
          set({
            selectedTenantId: tenant.id,
            selectedWorkspaceId: newWorkspace.id,
            isInitialized: true,
          });
          return;
        }
        
        // 4. No tenants/workspaces -> show onboarding
        set({ 
          needsOnboarding: true,
          isInitialized: true 
        });
      },
      
      // Save last used context
      saveContextPreference: (tenantId: string, workspaceId: string) => {
        localStorage.setItem('edgequake:last-context', JSON.stringify({
          tenantId,
          workspaceId,
          timestamp: Date.now()
        }));
      }
    }),
    {
      name: "edgequake-tenant",
      partialize: (state) => ({
        selectedTenantId: state.selectedTenantId,
        selectedWorkspaceId: state.selectedWorkspaceId,
        // New: Store last selection timestamp
        lastSelectionTime: Date.now()
      }),
    }
  )
);
```

### 3. Onboarding Experience

Create a welcoming first-time experience instead of empty states:

```tsx
// components/onboarding/workspace-onboarding.tsx
export function WorkspaceOnboarding() {
  return (
    <div className="flex items-center justify-center min-h-[60vh]">
      <Card className="w-full max-w-md p-8 text-center">
        <div className="mb-6">
          <div className="mx-auto w-16 h-16 rounded-full bg-primary/10 flex items-center justify-center mb-4">
            <FolderKanban className="w-8 h-8 text-primary" />
          </div>
          <h2 className="text-2xl font-bold mb-2">Welcome to EdgeQuake!</h2>
          <p className="text-muted-foreground">
            Let's get you started by creating your first workspace
          </p>
        </div>
        
        <div className="space-y-4">
          <QuickCreateWorkspaceForm onSuccess={handleWorkspaceCreated} />
          
          <div className="relative">
            <div className="absolute inset-0 flex items-center">
              <Separator />
            </div>
            <div className="relative flex justify-center text-xs uppercase">
              <span className="bg-background px-2 text-muted-foreground">
                Or
              </span>
            </div>
          </div>
          
          <Button variant="outline" onClick={handleSkipOnboarding}>
            I'll set this up later
          </Button>
        </div>
      </Card>
    </div>
  );
}
```

### 4. Header Selector Improvements

```tsx
// Enhanced header-tenant-selector.tsx
export function HeaderTenantSelector({ className }: HeaderTenantSelectorProps) {
  const { isInitialized, needsOnboarding } = useTenantStore();
  
  // Show skeleton during initialization
  if (!isInitialized) {
    return <Skeleton className="h-8 w-32" />;
  }
  
  // Show onboarding prompt if needed
  if (needsOnboarding) {
    return (
      <Button 
        variant="default" 
        size="sm"
        onClick={() => setShowOnboarding(true)}
        className="animate-pulse-soft"
      >
        <Plus className="h-4 w-4 mr-2" />
        Create Workspace
      </Button>
    );
  }
  
  // Rest of existing implementation...
}
```

## Implementation Checklist

### Phase 1: Core Functionality
- [ ] Add `isInitialized` and `needsOnboarding` to store
- [ ] Implement `initializeContext()` method
- [ ] Add last-used context persistence
- [ ] Create context validation helpers
- [ ] Update header selector loading states

### Phase 2: Onboarding
- [ ] Create `WorkspaceOnboarding` component
- [ ] Create `QuickCreateWorkspaceForm` component
- [ ] Add onboarding routing logic
- [ ] Implement "skip for now" functionality
- [ ] Add welcome animations

### Phase 3: User Preferences
- [ ] Add "Set as default" option in workspace selector
- [ ] Create user preferences API endpoint
- [ ] Implement default context API
- [ ] Add preference management UI

### Phase 4: Testing
- [ ] Unit tests for store logic
- [ ] E2E tests for default selection
- [ ] E2E tests for onboarding flow
- [ ] Test localStorage edge cases
- [ ] Test multi-tenant scenarios

## Edge Cases & Handling

| Scenario | Behavior |
|----------|----------|
| localStorage cleared | Fall back to first available workspace |
| Last-used workspace deleted | Select first available in tenant |
| Last-used tenant deleted | Select first available tenant |
| No tenants exist | Show onboarding |
| No workspaces in tenant | Auto-create default workspace |
| Multiple browser tabs | Sync context across tabs (BroadcastChannel) |
| API failure | Show cached context with error banner |

## Success Criteria

✅ **User Experience**
- First-time users see onboarding, not empty state
- Returning users automatically enter their last workspace
- Single-tenant users never see tenant selector
- Context selection completes in < 500ms

✅ **Technical**
- 100% E2E test coverage for selection flows
- Zero race conditions in initialization
- Proper TypeScript types for all new code
- Backward compatible with existing localStorage data

✅ **Metrics**
- Time to first interaction: < 2s (down from 10s)
- Onboarding completion rate: > 80%
- Context selection errors: < 1%

## Migration Strategy

1. **Backward Compatibility:** Support old localStorage format
2. **Gradual Rollout:** Feature flag for new behavior
3. **User Communication:** In-app notification about new feature
4. **Fallback:** Revert to old behavior if initialization fails

## Files to Modify

1. `src/stores/use-tenant-store.ts` - Core store logic
2. `src/components/layout/header-tenant-selector.tsx` - UI updates
3. `src/components/onboarding/workspace-onboarding.tsx` - New file
4. `src/components/onboarding/quick-create-form.tsx` - New file
5. `src/lib/api/context.ts` - New API helpers
6. `src/hooks/use-context-initialization.ts` - New hook
7. `e2e/workspace-selection.spec.ts` - New tests

---

**Next:** [Document Detail Page Redesign](./03-document-detail-redesign.md)
