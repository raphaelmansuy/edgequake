# OODA Iteration 04 - ORIENT

## Analysis of Findings

### Problem Statement

DocumentManager is 1826 lines with 15+ state variables. This violates SRP:
- Hard to test individual features
- High cognitive load for developers
- Difficult to reuse functionality elsewhere

### Extraction Strategy

Given the complexity, I'll use an **incremental approach** - one extraction per iteration.

**This iteration**: Extract `useStuckDetection` hook

**Why start here**:
1. **Completely isolated** - no dependencies on other document-manager logic
2. **Clear interface** - takes documents, returns nothing (side effect only)
3. **Test motivation** - creates example for future hook extractions
4. **Low risk** - if extraction fails, easy to revert

### Hook Design

**Current implementation** (embedded in document-manager.tsx):
- Checks every 30 seconds for documents without updates
- Logs warning to console
- No return value (pure side effect)

**Enhanced design** (as reusable hook):

```typescript
interface UseStuckDetectionOptions {
  /** Timeout in milliseconds before considering a document stuck (default: 30000) */
  timeout?: number;
  /** Interval to check for stuck documents (default: 30000) */
  checkInterval?: number;
  /** Callback when a document is detected as stuck */
  onStuck?: (document: Document) => void;
  /** Enable/disable detection (default: true) */
  enabled?: boolean;
}

interface UseStuckDetectionResult {
  /** Currently detected stuck documents */
  stuckDocuments: Document[];
  /** Manually trigger a check */
  checkNow: () => void;
}

function useStuckDetection(
  documents: Document[] | undefined,
  options?: UseStuckDetectionOptions
): UseStuckDetectionResult;
```

**Enhancements over current**:
1. Return stuck documents (for UI display if needed)
2. Configurable timeout/interval
3. Optional callback instead of just console.warn
4. Enable/disable toggle

### Decision

**Extract useStuckDetection with enhanced interface**

**Rationale**:
1. Follows React hook conventions
2. Can be unit tested in isolation
3. Reusable across different document list components
4. Sets pattern for future hook extractions

### Files to Create/Modify

| File | Action |
|------|--------|
| `hooks/use-stuck-detection.ts` | **Create** - new hook |
| `document-manager.tsx` | **Modify** - import and use hook |
