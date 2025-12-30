# Zustand & localStorage Audit Findings

## Date: 2025-01-XX

## Executive Summary

This document presents a comprehensive audit of Zustand state management and localStorage usage in the EdgeQuake WebUI. The audit identified **8 critical issues** that can cause stale state, hydration mismatches, and data corruption.

---

## 1. Store Inventory

### 1.1 Zustand Stores Overview

| Store                    | File                                                                                 | Persisted  | localStorage Key          | SSR Safe |
| ------------------------ | ------------------------------------------------------------------------------------ | ---------- | ------------------------- | -------- |
| **useTenantStore**       | [use-tenant-store.ts](../edgequake_webui/src/stores/use-tenant-store.ts)             | ✅ Yes     | `edgequake-tenant`        | ❌ No    |
| **useAuthStore**         | [use-auth-store.ts](../edgequake_webui/src/stores/use-auth-store.ts)                 | ✅ Yes     | `edgequake-auth`          | ❌ No    |
| **useSettingsStore**     | [use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts)         | ✅ Yes     | `edgequake-settings`      | ❌ No    |
| **useQueryStore**        | [use-query-store.ts](../edgequake_webui/src/stores/use-query-store.ts)               | ✅ Yes     | `edgequake-query`         | ❌ No    |
| **useQueryUIStore**      | [use-query-ui-store.ts](../edgequake_webui/src/stores/use-query-ui-store.ts)         | ✅ Partial | `edgequake-query-ui`      | ❌ No    |
| **useCostStore**         | [use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts)                 | ✅ Yes     | `edgequake-cost`          | ❌ No    |
| **useConversationStore** | [use-conversation-store.ts](../edgequake_webui/src/stores/use-conversation-store.ts) | ✅ Yes     | `edgequake-conversations` | ❌ No    |
| **useGraphStore**        | [use-graph-store.ts](../edgequake_webui/src/stores/use-graph-store.ts)               | ❌ No      | N/A                       | ✅ Yes   |
| **useIngestionStore**    | [use-ingestion-store.ts](../edgequake_webui/src/stores/use-ingestion-store.ts)       | ❌ No      | N/A                       | ✅ Yes   |
| **useBackendStore**      | [use-backend-store.ts](../edgequake_webui/src/stores/use-backend-store.ts)           | ❌ No      | N/A                       | ✅ Yes   |

### 1.2 Direct localStorage Usage

| Key                                | Location                                                                                                    | Purpose             | Zustand Duplicate?      |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------- | ----------------------- |
| `accessToken`                      | [client.ts#L67](../edgequake_webui/src/lib/api/client.ts#L67)                                               | Auth token          | ⚠️ YES - useAuthStore   |
| `refreshToken`                     | [client.ts#L68](../edgequake_webui/src/lib/api/client.ts#L68)                                               | Refresh token       | ⚠️ YES - useAuthStore   |
| `tenantId`                         | [client.ts#L124](../edgequake_webui/src/lib/api/client.ts#L124)                                             | API context         | ⚠️ YES - useTenantStore |
| `workspaceId`                      | [client.ts#L126](../edgequake_webui/src/lib/api/client.ts#L126)                                             | API context         | ⚠️ YES - useTenantStore |
| `userId`                           | [client.ts#L109](../edgequake_webui/src/lib/api/client.ts#L109)                                             | Anonymous user ID   | ❌ No                   |
| `edgequake-workspace-initialized`  | [header-tenant-selector.tsx#L149](../edgequake_webui/src/components/layout/header-tenant-selector.tsx#L149) | One-time toast flag | ❌ No                   |
| `edgequake-language`               | [i18n.ts#L33](../edgequake_webui/src/lib/i18n.ts#L33)                                                       | i18n preference     | ❌ No                   |
| `edgequake-graph-cache`            | [clear-cache-button.tsx#L73](../edgequake_webui/src/components/shared/clear-cache-button.tsx#L73)           | Graph cache         | ❌ No                   |
| `edgequake-query-history`          | [clear-cache-button.tsx#L75](../edgequake_webui/src/components/shared/clear-cache-button.tsx#L75)           | Query history       | ❌ No                   |
| `edgequake-conversations-migrated` | [use-migrate-conversations.ts#L34](../edgequake_webui/src/hooks/use-migrate-conversations.ts#L34)           | Migration flag      | ❌ No                   |

---

## 2. Critical Issues

### 2.1 ISSUE #1: Dual Storage Pattern (CRITICAL)

**Severity:** 🔴 CRITICAL  
**Impact:** State desynchronization, stale data, API calls with wrong context

**Description:**
Tenant and workspace context is stored in TWO places:

1. **Zustand Store** (`useTenantStore`):

   - Key: `edgequake-tenant`
   - Fields: `selectedTenantId`, `selectedWorkspaceId`
   - Used by: UI components

2. **Manual localStorage** ([client.ts](../edgequake_webui/src/lib/api/client.ts)):
   - Keys: `tenantId`, `workspaceId`
   - Used by: API client for HTTP headers

**Race Condition:**

```
1. User selects new workspace in UI
2. useTenantStore.selectWorkspace() updates Zustand → persists to 'edgequake-tenant'
3. setTenantContext() called → updates module variable AND 'workspaceId' key
4. If step 3 fails or runs late, API still uses old context
5. Data is fetched/saved to WRONG workspace!
```

**Affected Files:**

- [use-tenant-store.ts#L55-64](../edgequake_webui/src/stores/use-tenant-store.ts#L55-64)
- [client.ts#L116-128](../edgequake_webui/src/lib/api/client.ts#L116-128)

---

### 2.2 ISSUE #2: Auth Token Dual Storage (HIGH)

**Severity:** 🟠 HIGH  
**Impact:** Auth desynchronization, refresh token races

**Description:**
Same dual storage pattern exists for authentication:

1. **Zustand Store** (`useAuthStore`):

   - Key: `edgequake-auth`
   - Fields: `accessToken`, `refreshToken`, `expiresAt`

2. **Manual localStorage** ([client.ts](../edgequake_webui/src/lib/api/client.ts)):
   - Keys: `accessToken`, `refreshToken`

**Code Evidence:**

```typescript
// In useAuthStore.login():
setTokens(response.access_token, response.refresh_token); // Writes to localStorage
set({
  accessToken: response.access_token, // Also in Zustand state
  refreshToken: response.refresh_token,
});
```

**Affected Files:**

- [use-auth-store.ts#L30-35](../edgequake_webui/src/stores/use-auth-store.ts#L30-35)
- [client.ts#L63-82](../edgequake_webui/src/lib/api/client.ts#L63-82)

---

### 2.3 ISSUE #3: No SSR Hydration Handling (HIGH)

**Severity:** 🟠 HIGH  
**Impact:** Hydration mismatches, React errors, flash of incorrect content

**Description:**
None of the persisted Zustand stores implement SSR-safe hydration:

- No `skipHydration` option
- No `onRehydrateStorage` callback
- No hydration state tracking

**Current Workarounds (Band-aids):**

```tsx
// layout.tsx - suppresses warnings but doesn't fix issue
<html lang="en" suppressHydrationWarning>
<body suppressHydrationWarning>
```

**Best Practice (Not Implemented):**

```typescript
{
  name: 'store-key',
  onRehydrateStorage: () => {
    return (state, error) => {
      if (error) console.error('Hydration failed:', error);
      state.setHasHydrated(true);
    };
  },
}
```

**Affected Files:**

- All 7 persisted stores
- [layout.tsx](../edgequake_webui/src/app/layout.tsx#L23-24)

---

### 2.4 ISSUE #4: Duplicate Conversation Data (MEDIUM)

**Severity:** 🟡 MEDIUM  
**Impact:** Data duplication, storage bloat, potential desync

**Description:**
Conversation messages are stored in TWO Zustand stores:

1. **useQueryStore** ([use-query-store.ts#L156](../edgequake_webui/src/stores/use-query-store.ts#L156)):

   - Field: `conversationMessages`
   - Key: `edgequake-query`

2. **useConversationStore** ([use-conversation-store.ts#L240](../edgequake_webui/src/stores/use-conversation-store.ts#L240)):
   - Field: `conversations[].messages`
   - Key: `edgequake-conversations`

**Evidence:**

```typescript
// use-query-store.ts
partialize: (state) => ({
  history: state.history,
  conversationMessages: state.conversationMessages,  // Persisted!
}),

// use-conversation-store.ts
partialize: (state) => ({
  conversations: state.conversations,  // Also contains messages!
  activeConversationId: state.activeConversationId,
}),
```

---

### 2.5 ISSUE #5: No Store Versioning (MEDIUM)

**Severity:** 🟡 MEDIUM  
**Impact:** Breaking changes corrupt persisted state, no migration path

**Description:**
None of the stores use the `version` or `migrate` options from Zustand persist middleware.

**Risk Scenario:**

1. Developer renames `selectedTenantId` to `tenantId`
2. Users with old localStorage get corrupted state
3. App crashes or behaves incorrectly

**Best Practice (Not Implemented):**

```typescript
{
  name: 'edgequake-tenant',
  version: 1,
  migrate: (persistedState, version) => {
    if (version === 0) {
      // Handle migration from v0 to v1
    }
    return persistedState;
  },
}
```

---

### 2.6 ISSUE #6: Map Types Without Serialization (MEDIUM)

**Severity:** 🟡 MEDIUM  
**Impact:** Data loss on persist, Map becomes empty object

**Description:**
`useCostStore` uses `Map` types but has standard JSON storage:

```typescript
interface CostState {
  activeIngestionCosts: Map<string, number>; // Won't serialize!
  documentCosts: Map<string, DocumentCostBreakdown>; // Won't serialize!
}
```

**JSON.stringify Behavior:**

```javascript
JSON.stringify(new Map([["a", 1]])); // Returns "{}"
```

**Affected Files:**

- [use-cost-store.ts#L22-27](../edgequake_webui/src/stores/use-cost-store.ts#L22-27)

---

### 2.7 ISSUE #7: Inconsistent Storage Key Naming (LOW)

**Severity:** 🟢 LOW  
**Impact:** Maintenance difficulty, no single source of truth

**Description:**
Storage keys are scattered across files with no central definition:

- `edgequake-tenant` (Zustand)
- `tenantId` (manual)
- `edgequake-conversations` (Zustand)
- `edgequake-language` (i18n)

**No Single Source:**
Each store defines its own key inline.

---

### 2.8 ISSUE #8: API Client Mixing Concerns (LOW)

**Severity:** 🟢 LOW  
**Impact:** Code maintainability, testing difficulty

**Description:**
[client.ts](../edgequake_webui/src/lib/api/client.ts) mixes:

- HTTP client logic
- Token management
- Context management
- localStorage operations

Should be separated into:

- `http-client.ts` - Pure fetch wrapper
- `auth-service.ts` - Token management
- `context-service.ts` - Tenant/workspace context

---

## 3. Current Mitigations

### 3.1 TenantProvider

The recently added [TenantProvider](../edgequake_webui/src/providers/tenant-provider.tsx) helps but doesn't fully solve the issues:

**What it does:**

- Auto-selects first tenant/workspace
- Calls `initializeFromStorage()` on mount

**What it doesn't do:**

- Handle hydration state properly
- Prevent dual storage writes
- Validate localStorage data isn't stale

### 3.2 suppressHydrationWarning

This is a band-aid that hides warnings but doesn't fix underlying SSR issues.

---

## 4. Recommended Fixes

### Priority 1: Immediate (Breaking Issues)

| #   | Fix                                    | Effort | Impact |
| --- | -------------------------------------- | ------ | ------ |
| 1   | Create centralized storage-keys.ts     | 30m    | LOW    |
| 2   | Add hydration hooks and provider       | 2h     | HIGH   |
| 3   | Add onRehydrateStorage to tenant-store | 30m    | HIGH   |

### Priority 2: Short-term (Stale State)

| #   | Fix                                         | Effort | Impact   |
| --- | ------------------------------------------- | ------ | -------- |
| 4   | Eliminate dual storage - refactor client.ts | 3h     | CRITICAL |
| 5   | Add store versioning with migrations        | 2h     | MEDIUM   |
| 6   | Fix Map serialization in cost-store         | 1h     | MEDIUM   |

### Priority 3: Long-term (Best Practices)

| #   | Fix                                   | Effort | Impact |
| --- | ------------------------------------- | ------ | ------ |
| 7   | Consolidate conversation stores       | 4h     | MEDIUM |
| 8   | Add devtools middleware to all stores | 1h     | LOW    |
| 9   | Create state management documentation | 2h     | LOW    |

---

## 5. Store-by-Store Analysis

### 5.1 useTenantStore

**Location:** [use-tenant-store.ts](../edgequake_webui/src/stores/use-tenant-store.ts)

**Issues:**

- ⚠️ Dual storage with client.ts
- ⚠️ No hydration handling
- ⚠️ No versioning

**Persisted Fields:**

```typescript
partialize: (state) => ({
  selectedTenantId: state.selectedTenantId,
  selectedWorkspaceId: state.selectedWorkspaceId,
}),
```

**Recommendation:** Add version, migrate from legacy keys, add onRehydrateStorage

---

### 5.2 useAuthStore

**Location:** [use-auth-store.ts](../edgequake_webui/src/stores/use-auth-store.ts)

**Issues:**

- ⚠️ Dual storage with client.ts
- ⚠️ No hydration handling
- ⚠️ No versioning

**Persisted Fields:**

```typescript
partialize: (state) => ({
  isAuthenticated: state.isAuthenticated,
  user: state.user,
  expiresAt: state.expiresAt,
  // Note: tokens NOT persisted in Zustand (only in manual localStorage)
}),
```

**Recommendation:** Either persist tokens in Zustand OR use client.ts, not both

---

### 5.3 useSettingsStore

**Location:** [use-settings-store.ts](../edgequake_webui/src/stores/use-settings-store.ts)

**Issues:**

- ⚠️ No hydration handling
- ⚠️ No versioning

**Persisted Fields:**

```typescript
partialize: (state) => ({
  theme: state.theme,
  language: state.language,
  graphSettings: state.graphSettings,
  querySettings: state.querySettings,
  sidebarCollapsed: state.sidebarCollapsed,
}),
```

**Recommendation:** Add version for future graphSettings changes

---

### 5.4 useQueryStore

**Location:** [use-query-store.ts](../edgequake_webui/src/stores/use-query-store.ts)

**Issues:**

- ⚠️ Duplicates useConversationStore
- ⚠️ No hydration handling
- ⚠️ No versioning

**Persisted Fields:**

```typescript
partialize: (state) => ({
  history: state.history,
  conversationMessages: state.conversationMessages,  // DUPLICATE!
}),
```

**Recommendation:** Deprecate conversationMessages, use useConversationStore

---

### 5.5 useCostStore

**Location:** [use-cost-store.ts](../edgequake_webui/src/stores/use-cost-store.ts)

**Issues:**

- ⚠️ Map types won't serialize
- ⚠️ Uses devtools but may have hydration issues

**Persisted Fields:**

```typescript
// Has Map<string, T> types that won't serialize properly!
activeIngestionCosts: Map<string, number>;
documentCosts: Map<string, DocumentCostBreakdown>;
```

**Recommendation:** Add custom storage with Map serialization

---

### 5.6 useConversationStore

**Location:** [use-conversation-store.ts](../edgequake_webui/src/stores/use-conversation-store.ts)

**Issues:**

- ⚠️ No hydration handling
- ⚠️ No versioning

**Persisted Fields:**

```typescript
partialize: (state) => ({
  conversations: state.conversations,
  activeConversationId: state.activeConversationId,
  historyPanelOpen: state.historyPanelOpen,
}),
```

**Recommendation:** Add version, implement hydration callback

---

## 6. Testing Matrix

| Scenario                             | Current Behavior          | Expected Behavior              |
| ------------------------------------ | ------------------------- | ------------------------------ |
| Fresh load, no localStorage          | Works                     | Works                          |
| Refresh with context in localStorage | Sometimes shows "Loading" | Should auto-select             |
| Switch tenant in one tab             | Works in that tab         | Should sync across tabs        |
| Clear localStorage manually          | May cause errors          | Should gracefully reset        |
| Server restart + page refresh        | May have stale context    | Should validate context exists |
| Incognito mode                       | Works                     | Works                          |

---

## 7. Next Steps

See [02-implementation-plan.md](./02-implementation-plan.md) for detailed implementation steps.

---

## Appendix: Code References

### A. Zustand Persist Best Practices

From [Zustand Documentation](https://zustand.docs.pmnd.rs/integrations/persisting-store-data):

1. **skipHydration** - Use for SSR apps
2. **onRehydrateStorage** - Track hydration state
3. **version + migrate** - Handle breaking changes
4. **partialize** - Only persist necessary state
5. **createJSONStorage** - Custom serialization

### B. Next.js SSR with Zustand

The recommended pattern for Next.js:

```typescript
const useStore = <T, F>(
  store: (callback: (state: T) => unknown) => unknown,
  callback: (state: T) => F
) => {
  const result = store(callback) as F;
  const [data, setData] = useState<F>();

  useEffect(() => {
    setData(result);
  }, [result]);

  return data;
};
```
