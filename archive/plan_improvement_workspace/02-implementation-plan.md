# Zustand & localStorage Implementation Plan

## Date: 2025-01-XX

## Overview

This document details the implementation steps for fixing Zustand and localStorage issues identified in the audit.

---

## Phase 1: Foundation (COMPLETED ✅)

### 1.1 Create Centralized Storage Keys

**File Created:** [edgequake_webui/src/lib/storage-keys.ts](../edgequake_webui/src/lib/storage-keys.ts)

**Purpose:**

- Single source of truth for all localStorage keys
- Prevents typos and enables refactoring
- Documents all persistent state

**Key Features:**

- `ZUSTAND_STORAGE_KEYS` - Keys for Zustand persisted stores
- `LEGACY_STORAGE_KEYS` - Deprecated keys for migration
- `FLAG_STORAGE_KEYS` - One-time flags
- `STORE_VERSIONS` - Version numbers for migrations
- Utility functions for clearing storage

### 1.2 Create Hydration Utilities

**Files Created:**

- [edgequake_webui/src/hooks/use-store-hydration.ts](../edgequake_webui/src/hooks/use-store-hydration.ts)
- [edgequake_webui/src/providers/hydration-provider.tsx](../edgequake_webui/src/providers/hydration-provider.tsx)

**Purpose:**

- Safe access to persisted store data in SSR
- Prevent hydration mismatches
- Gate app rendering until critical stores hydrate

**Key Features:**

- `useStoreHydrated()` - Track single store hydration
- `useHydratedStore()` - SSR-safe selector access
- `useSyncStore()` - React 18+ optimal pattern
- `useAllStoresHydrated()` - Wait for multiple stores
- `useCrossTabSync()` - Sync across browser tabs
- `HydrationProvider` - Gates app until ready

---

## Phase 2: Store Upgrades (COMPLETED ✅)

### 2.1 useTenantStore

**File Modified:** [edgequake_webui/src/stores/use-tenant-store.ts](../edgequake_webui/src/stores/use-tenant-store.ts)

**Changes:**

- ✅ Added `_hasHydrated` state field
- ✅ Added `setHasHydrated()` action
- ✅ Use centralized `ZUSTAND_STORAGE_KEYS.TENANT_STORE`
- ✅ Added `version: 1` for migrations
- ✅ Added `migrate()` function for legacy key migration
- ✅ Added `onRehydrateStorage()` callback
- ✅ Added `useTenantStoreHydrated()` selector
- ✅ Added `useHasValidContext()` selector

**Migration Logic:**

```typescript
migrate: (persistedState, version) => {
  if (version === 0) {
    // Migrate from legacy localStorage keys
    const legacyTenantId = localStorage.getItem("tenantId");
    const legacyWorkspaceId = localStorage.getItem("workspaceId");
    // Apply to state...
  }
  return state;
};
```

### 2.2 useAuthStore

**File Modified:** [edgequake_webui/src/stores/use-auth-store.ts](../edgequake_webui/src/stores/use-auth-store.ts)

**Changes:**

- ✅ Added `_hasHydrated` state field
- ✅ Added `setHasHydrated()` action
- ✅ Use centralized `ZUSTAND_STORAGE_KEYS.AUTH_STORE`
- ✅ Added `version: 1` for migrations
- ✅ Added `migrate()` function
- ✅ Added `onRehydrateStorage()` callback
- ✅ Added `useAuthStoreHydrated()` selector

### 2.3 useSettingsStore

**File Modified:** [edgequake_webui/src/stores/use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts)

**Changes:**

- ✅ Added `_hasHydrated` state field
- ✅ Added `setHasHydrated()` action
- ✅ Use centralized `ZUSTAND_STORAGE_KEYS.SETTINGS_STORE`
- ✅ Added `version: 1` for migrations
- ✅ Added `merge()` function for deep nested objects
- ✅ Added `onRehydrateStorage()` callback
- ✅ Added `useSettingsStoreHydrated()` selector

### 2.4 useCostStore

**File Modified:** [edgequake_webui/src/stores/use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts)

**Changes:**

- ✅ Use centralized `ZUSTAND_STORAGE_KEYS.COST_STORE`
- ✅ Added `version: 1` for migrations
- Note: Map types are NOT persisted (intentionally - they're transient state)

---

## Phase 3: Provider Integration (COMPLETED ✅)

### 3.1 AppProviders Update

**File Modified:** [edgequake_webui/src/providers/index.tsx](../edgequake_webui/src/providers/index.tsx)

**Changes:**

- ✅ Added `HydrationProvider` import
- ✅ Added `HydrationProvider` to provider hierarchy
- ✅ Documented provider order and rationale

**Provider Order:**

```
QueryProvider → ThemeProvider → HydrationProvider → I18nProvider → TenantProvider → ...
```

**Rationale:**

1. `QueryProvider` - Must be first (React Query context)
2. `ThemeProvider` - Before hydration to prevent theme flash
3. `HydrationProvider` - Gates until Zustand hydrates
4. Remaining providers can safely access hydrated stores

---

## Phase 4: Future Work (TODO)

### 4.1 Eliminate Dual Storage in client.ts

**Status:** DOCUMENTED - Not blocking, stores sync on hydration

The dual storage pattern (Zustand + manual localStorage) still exists but is mitigated by:

- `onRehydrateStorage` syncs store → API client after hydration
- Migration function imports legacy keys into Zustand store

**Full Solution (Deferred):**

1. Refactor `client.ts` to subscribe to Zustand stores
2. Remove manual `setTenantContext()` calls from stores
3. Clean up legacy keys after migration period

### 4.2 Consolidate Conversation Stores

**Status:** DOCUMENTED - Low priority

`useQueryStore.conversationMessages` duplicates `useConversationStore.conversations`.

**Solution (Deferred):**

1. Mark `conversationMessages` as deprecated
2. Migrate usage to `useConversationStore`
3. Remove from persistence after migration

### 4.3 Update Remaining Stores

**Stores to Update:**

- [ ] `useQueryStore` - Add version, hydration
- [ ] `useQueryUIStore` - Add version, hydration
- [ ] `useConversationStore` - Add version, hydration

---

## Testing Checklist

### Manual Testing

- [ ] Fresh browser (no localStorage) - App loads correctly
- [ ] With existing localStorage - App loads with saved state
- [ ] Clear localStorage manually - App resets gracefully
- [ ] Multiple tabs - Changes sync (if cross-tab enabled)
- [ ] Incognito mode - App works without persistence
- [ ] Theme switching - No flash on reload

### E2E Testing

- [ ] Dashboard loads with tenant/workspace context
- [ ] Query page works after refresh
- [ ] Documents page shows correct workspace
- [ ] Graph page loads knowledge graph
- [ ] Settings persist after refresh

---

## Rollback Plan

If issues occur:

1. **Revert HydrationProvider:**

   - Remove from `providers/index.tsx`
   - App will work but may have hydration warnings

2. **Revert Store Changes:**

   - Store changes are backward compatible
   - Old localStorage format still works
   - Migration only adds data, doesn't delete

3. **Clear User Storage:**
   - Users can clear via Settings > Clear Cache
   - Or manually clear localStorage

---

## Metrics for Success

1. **No React Hydration Warnings** - Check browser console
2. **No "Select Context" Flash** - App loads with saved selection
3. **Cross-Tab Sync Works** - Changes reflect in other tabs
4. **Store Migrations Run Once** - Check version in localStorage
5. **E2E Tests Pass** - All existing tests still work
