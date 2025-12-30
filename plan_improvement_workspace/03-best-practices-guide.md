# Zustand & localStorage Best Practices Guide

## EdgeQuake WebUI State Management

This guide documents best practices for state management in EdgeQuake WebUI.

---

## 1. Store Architecture

### 1.1 Store Types

| Type | Persistence | SSR Safe | Use Case |
|------|-------------|----------|----------|
| **Persisted Store** | localStorage | Requires handling | User preferences, saved state |
| **Session Store** | None | Yes | Current session state |
| **Derived Store** | From other stores | Depends | Computed state |

### 1.2 When to Use What

```
User Preferences → Persisted Store (settings, theme)
Auth State → Persisted Store (tokens, user info)
Context Selection → Persisted Store (tenant, workspace)
UI State → Session Store (panels, modals)
Server Data → React Query (documents, graph)
Transient State → Session Store (loading, errors)
```

---

## 2. Creating a New Persisted Store

### 2.1 Template

```typescript
"use client";

import {
  STORE_VERSIONS,
  ZUSTAND_STORAGE_KEYS,
} from "@/lib/storage-keys";
import { create } from "zustand";
import { persist } from "zustand/middleware";

// ============================================================================
// Types
// ============================================================================

interface MyState {
  // State fields
  someValue: string;
  
  // Hydration tracking
  _hasHydrated: boolean;
}

interface MyActions {
  // Actions
  setSomeValue: (value: string) => void;
  reset: () => void;
  setHasHydrated: (hydrated: boolean) => void;
}

type MyStore = MyState & MyActions;

// ============================================================================
// Initial State
// ============================================================================

const initialState: MyState = {
  someValue: "",
  _hasHydrated: false,
};

// ============================================================================
// Store Definition
// ============================================================================

export const useMyStore = create<MyStore>()(
  persist(
    (set) => ({
      ...initialState,

      setSomeValue: (value) => set({ someValue: value }),
      
      reset: () => set(initialState),
      
      setHasHydrated: (hydrated) => set({ _hasHydrated: hydrated }),
    }),
    {
      // Use centralized key
      name: ZUSTAND_STORAGE_KEYS.MY_STORE, // Add to storage-keys.ts
      
      // Use centralized version
      version: STORE_VERSIONS[ZUSTAND_STORAGE_KEYS.MY_STORE],
      
      // Only persist necessary fields (never _hasHydrated)
      partialize: (state) => ({
        someValue: state.someValue,
      }),
      
      // Handle version migrations
      migrate: (persistedState, version) => {
        const state = persistedState as Partial<MyState>;
        
        if (version === 0) {
          // Handle migration from v0 to v1
        }
        
        return state as MyState;
      },
      
      // Track hydration for SSR
      onRehydrateStorage: () => {
        return (state, error) => {
          if (error) {
            console.error("[MyStore] Hydration failed:", error);
          }
          state?.setHasHydrated(true);
        };
      },
    }
  )
);

// ============================================================================
// Selectors
// ============================================================================

export const useMyStoreHydrated = () => {
  return useMyStore((state) => state._hasHydrated);
};

export default useMyStore;
```

### 2.2 Checklist for New Stores

- [ ] Add key to `ZUSTAND_STORAGE_KEYS` in `storage-keys.ts`
- [ ] Add version to `STORE_VERSIONS` in `storage-keys.ts`
- [ ] Include `_hasHydrated` field for SSR safety
- [ ] Use `partialize` to only persist necessary fields
- [ ] Implement `migrate` for future schema changes
- [ ] Add `onRehydrateStorage` callback
- [ ] Create hydration selector
- [ ] Add to `HydrationProvider` if critical

---

## 3. SSR and Hydration

### 3.1 The Problem

Next.js renders on the server where `localStorage` doesn't exist:

```
Server Render: state = { selectedTenantId: null }
Client Hydrate: localStorage = { selectedTenantId: "abc123" }
→ React Error: "Text content does not match"
```

### 3.2 The Solution

1. **Track Hydration State:**
```typescript
interface State {
  _hasHydrated: boolean;
}

onRehydrateStorage: () => (state) => {
  state?.setHasHydrated(true);
}
```

2. **Gate Rendering:**
```tsx
// Option A: Use HydrationProvider (preferred)
<HydrationProvider>
  <MyComponent />
</HydrationProvider>

// Option B: Check in component
const hydrated = useMyStoreHydrated();
if (!hydrated) return <Skeleton />;
```

3. **SSR-Safe Selectors:**
```tsx
// Returns undefined until hydrated
const value = useHydratedStore(useMyStore, (s) => s.value);

// Returns fallback on server
const value = useSyncStore(useMyStore, (s) => s.value, "fallback");
```

### 3.3 Never Do This

```tsx
// ❌ WRONG - Causes hydration mismatch
function MyComponent() {
  const value = useMyStore((s) => s.value);
  return <div>{value}</div>; // Different on server vs client!
}
```

---

## 4. Storage Key Management

### 4.1 Always Use Centralized Keys

```typescript
// ✅ CORRECT
import { ZUSTAND_STORAGE_KEYS } from "@/lib/storage-keys";

persist({
  name: ZUSTAND_STORAGE_KEYS.TENANT_STORE,
})

// ❌ WRONG - Magic string
persist({
  name: "edgequake-tenant",
})
```

### 4.2 Document New Keys

When adding a new storage key:

1. Add to appropriate category in `storage-keys.ts`
2. Add JSDoc comment explaining purpose
3. Add to `getLogoutClearKeys()` if user-specific
4. Add to `getCacheClearKeys()` if cache data

---

## 5. Version Migrations

### 5.1 When to Bump Version

- Renaming a persisted field
- Changing field type
- Removing a field
- Restructuring nested objects

### 5.2 Migration Pattern

```typescript
{
  version: 2, // Bump from 1 to 2
  migrate: (persistedState, version) => {
    const state = persistedState as any;
    
    if (version === 0) {
      // v0 → v1: Legacy migration
      state.newField = state.oldField;
      delete state.oldField;
    }
    
    if (version === 1) {
      // v1 → v2: New migration
      state.renamedField = state.originalField;
      delete state.originalField;
    }
    
    return state;
  },
}
```

### 5.3 Testing Migrations

```typescript
// In test file
it("migrates from v0 to v1", () => {
  // Set up v0 data
  localStorage.setItem("store-key", JSON.stringify({
    state: { oldField: "value" },
    version: 0,
  }));
  
  // Create store (triggers migration)
  const { getState } = useMyStore;
  
  // Verify migration
  expect(getState().newField).toBe("value");
  expect(getState().oldField).toBeUndefined();
});
```

---

## 6. Avoiding Common Pitfalls

### 6.1 Dual Storage

**Problem:** Same data in both Zustand and manual localStorage

```typescript
// ❌ BAD - Dual storage
const selectTenant = (id) => {
  set({ selectedTenantId: id });  // Zustand
  localStorage.setItem("tenantId", id);  // Manual
};
```

**Solution:** Single source of truth

```typescript
// ✅ GOOD - Single source
const selectTenant = (id) => {
  set({ selectedTenantId: id });
  // Zustand persist handles localStorage
};
```

### 6.2 Map/Set Serialization

**Problem:** Maps and Sets don't serialize to JSON

```typescript
// ❌ BAD - Won't persist correctly
state: {
  myMap: new Map(),  // Becomes {}
  mySet: new Set(),  // Becomes {}
}
```

**Solution A:** Convert to Array before persist

```typescript
partialize: (state) => ({
  myMapEntries: Array.from(state.myMap.entries()),
})
```

**Solution B:** Don't persist (use for transient data only)

```typescript
partialize: (state) => ({
  // Don't include myMap - it's transient
  otherField: state.otherField,
})
```

### 6.3 Circular Dependencies

**Problem:** Store A imports Store B, Store B imports Store A

**Solution:** Use `getState()` instead of hooks for cross-store access

```typescript
// ❌ BAD - Circular import
import { useTenantStore } from "./use-tenant-store";
const tenantId = useTenantStore.getState().selectedTenantId;

// ✅ GOOD - Dynamic import or event
import { useTenantStore } from "./use-tenant-store";
const getTenantId = () => useTenantStore.getState().selectedTenantId;
```

### 6.4 Over-Persistence

**Problem:** Persisting too much data causes storage bloat

**Solution:** Only persist what's needed

```typescript
// ❌ BAD - Persisting everything
partialize: (state) => state,

// ✅ GOOD - Selective persistence
partialize: (state) => ({
  selectedId: state.selectedId,
  // Don't persist: list, isLoading, error, etc.
})
```

---

## 7. Cross-Tab Synchronization

### 7.1 Enable Cross-Tab Sync

```typescript
import { useCrossTabSync } from "@/hooks/use-store-hydration";

function App() {
  // Rehydrates when localStorage changes in another tab
  useCrossTabSync(useMyStore, ZUSTAND_STORAGE_KEYS.MY_STORE);
  
  return <Children />;
}
```

### 7.2 Manual Approach

```typescript
useEffect(() => {
  const handler = (e: StorageEvent) => {
    if (e.key === "my-store-key") {
      useMyStore.persist.rehydrate();
    }
  };
  
  window.addEventListener("storage", handler);
  return () => window.removeEventListener("storage", handler);
}, []);
```

---

## 8. Testing Strategies

### 8.1 Unit Testing Stores

```typescript
import { act, renderHook } from "@testing-library/react";
import { useMyStore } from "./use-my-store";

beforeEach(() => {
  // Clear store between tests
  useMyStore.getState().reset();
  localStorage.clear();
});

it("updates state", () => {
  const { result } = renderHook(() => useMyStore());
  
  act(() => {
    result.current.setSomeValue("test");
  });
  
  expect(result.current.someValue).toBe("test");
});

it("persists to localStorage", () => {
  const { result } = renderHook(() => useMyStore());
  
  act(() => {
    result.current.setSomeValue("test");
  });
  
  const stored = JSON.parse(localStorage.getItem("store-key")!);
  expect(stored.state.someValue).toBe("test");
});
```

### 8.2 E2E Testing

```typescript
// Clear storage before test
test.beforeEach(async ({ page }) => {
  await page.evaluate(() => localStorage.clear());
});

// Test persistence
test("persists selection across reload", async ({ page }) => {
  // Select something
  await page.click('[data-testid="option-1"]');
  
  // Reload
  await page.reload();
  
  // Verify still selected
  await expect(page.locator('[data-testid="selected"]')).toHaveText("Option 1");
});
```

---

## 9. Quick Reference

### Store Configuration Options

| Option | Purpose | Required |
|--------|---------|----------|
| `name` | localStorage key | Yes |
| `version` | Schema version for migrations | Yes |
| `partialize` | Select fields to persist | Yes |
| `migrate` | Handle schema changes | Yes |
| `onRehydrateStorage` | Track hydration | Yes |
| `merge` | Custom merge for nested objects | If needed |
| `storage` | Custom storage engine | No |
| `skipHydration` | Manual hydration control | No |

### Hydration Hooks

| Hook | Purpose |
|------|---------|
| `useStoreHydrated(store)` | Boolean: is store hydrated? |
| `useHydratedStore(store, selector)` | SSR-safe selector (undefined until hydrated) |
| `useSyncStore(store, selector, fallback)` | React 18 optimal pattern |
| `useAllStoresHydrated([stores])` | Wait for multiple stores |
| `useCrossTabSync(store, key)` | Sync across tabs |
| `useRehydrateStore(store)` | Manual rehydration |

### Storage Key Categories

| Category | Purpose |
|----------|---------|
| `ZUSTAND_STORAGE_KEYS` | Zustand persist keys |
| `LEGACY_STORAGE_KEYS` | Deprecated, for migration |
| `FLAG_STORAGE_KEYS` | One-time flags |
| `CACHE_STORAGE_KEYS` | Clearable cache |

---

## 10. Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2025-01-XX | 1.0 | Initial guide |
