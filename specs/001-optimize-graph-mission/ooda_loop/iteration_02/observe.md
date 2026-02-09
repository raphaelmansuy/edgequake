# Iteration 02 - Observe

## Mission Re-read

The 500 node limit is NOT being enforced. Screenshot shows 1,708 of 1,709 nodes displayed.

## Observations

### Root Cause Discovery

**Bug Location**: `graph-settings-panel.tsx:93`

```typescript
if (storedMaxNodes) {
  const parsed = parseInt(storedMaxNodes, 10);
  if (!isNaN(parsed) && parsed >= 100 && parsed <= 10000) {
    // BUG: allows up to 10000!
    setMaxNodes(parsed);
  }
}
```

The localStorage validation allows maxNodes values up to 10,000!

### Data Flow Analysis

```
┌─────────────────────────────────────────────────────────────────────┐
│                     maxNodes Data Flow                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  localStorage('graph-max-nodes')                                     │
│         │                                                            │
│         ▼                                                            │
│  graph-settings-panel.tsx:87-95                                      │
│  Reads from localStorage, validates 100 ≤ x ≤ 10000  ← BUG HERE     │
│         │                                                            │
│         ▼                                                            │
│  use-graph-store.ts                                                  │
│  setMaxNodes(parsed) → stores in zustand state                       │
│         │                                                            │
│         ▼                                                            │
│  graph-viewer.tsx:141                                                │
│  maxNodes = useGraphStore((s) => s.maxNodes)                         │
│         │                                                            │
│         ▼                                                            │
│  use-graph-stream.ts:307                                             │
│  Passes maxNodes to API request                                      │
│         │                                                            │
│         ▼                                                            │
│  Backend: graph.rs                                                   │
│  Returns maxNodes nodes (no hard cap before 10000)                   │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Files Requiring Changes

| File                       | Line | Issue                           |
| -------------------------- | ---- | ------------------------------- |
| `graph-settings-panel.tsx` | 93   | Validation allows up to 10000   |
| `graph-settings-panel.tsx` | 142  | Auto-optimize can save 1000+    |
| `auto-optimize.ts`         | 75   | High tier allows maxNodes: 1000 |
| `truncation-banner.tsx`    | 38   | Already capped at 500 ✓         |
| `graph-viewer.tsx`         | 550  | Already capped at 500 ✓         |

### Evidence

1. Screenshot shows "1,708 of 1,709 nodes (100%)"
2. User has high-end device (macOS with 8+ cores)
3. Auto-optimize likely set maxNodes to 1000 previously
4. Value persisted in localStorage
5. On page reload, localStorage value restored, bypassing the 500 cap

## Conclusion

The 500 node cap is inconsistently applied. Need to enforce MAX_DISPLAY_NODES=500 at ALL levels:

- localStorage validation
- Auto-optimize calculation
- Settings panel slider max value
