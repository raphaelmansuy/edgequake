# OODA Iteration 04 - ACT

## Summary

**Issue Addressed**: High Priority Issue #4 - DocumentManager SRP Violation (partial)

**This iteration**: Extracted `useStuckDetection` hook

## Changes Made

### 1. Created useStuckDetection Hook

**File**: [use-stuck-detection.ts](../../../../edgequake_webui/src/hooks/use-stuck-detection.ts) (NEW - 131 lines)

**Features**:
- Detects documents stuck in processing state
- Configurable timeout and check interval
- Optional callback when document is stuck
- Returns list of currently stuck documents
- `checkNow()` method for manual trigger

**API**:
```typescript
function useStuckDetection(
  documents: Document[] | undefined,
  options?: {
    timeout?: number;        // ms before considered stuck (default: 30000)
    checkInterval?: number;  // ms between checks (default: 30000)
    onStuck?: (doc: Document) => void;
    enabled?: boolean;
  }
): {
  stuckDocuments: Document[];
  checkNow: () => void;
}
```

**WHY**: Reusable detection logic that can be used in any document list component, not just DocumentManager.

### 2. Updated DocumentManager to Use Hook

**File**: [document-manager.tsx](../../../../edgequake_webui/src/components/documents/document-manager.tsx#L333)

**Before** (inline, 37 lines):
```typescript
useEffect(() => {
  if (!data?.items) return;
  const checkStuckDocuments = () => { ... };
  checkStuckDocuments();
  const interval = setInterval(checkStuckDocuments, 30000);
  return () => clearInterval(interval);
}, [data?.items]);
```

**After** (4 lines):
```typescript
useStuckDetection(data?.items, {
  timeout: 30000,
  checkInterval: 30000,
});
```

## Test Results

```
TypeScript: ✅ pnpm tsc --noEmit (no errors)
```

## Metrics

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| document-manager.tsx | 1826 lines | 1793 lines | -33 lines |
| New hook | N/A | 131 lines | +131 lines |
| Reusability | Embedded | Reusable hook | ✅ |
| Testability | Hard to test | Unit testable | ✅ |

## Progress on Issue #4

| Target Component/Hook | Status |
|----------------------|--------|
| useStuckDetection | ✅ **Done** (this iteration) |
| useDocumentWebSocket | ⬜ Pending |
| DocumentUploadZone | ⬜ Pending |
| DocumentFilters | ⬜ Pending |
| DocumentBatchActions | ⬜ Pending |
| DocumentDetailPanel | ⬜ Pending |

**Remaining**: 1793 lines → target <300 lines per component

## Next Iteration

**Continue Issue #4**: Extract `useDocumentWebSocket` hook
- Combines WebSocket subscription and progress event handling
- Lines 278-328 in document-manager.tsx
