# Iteration 04 - ORIENT Phase

## Gap Analysis

### What Works Well
1. ✅ Confirmation dialogs with clear warnings
2. ✅ Visual indication of destructive action (red button)
3. ✅ Automatic reprocessing trigger after clear
4. ✅ Pipeline status dialog for progress tracking
5. ✅ Card variant shows current model configuration

### What Needs Improvement

| Gap | Current State | Desired State | Priority |
|-----|--------------|---------------|----------|
| No test IDs | No data-testid attributes | All buttons/actions have data-testid | P1 |
| No impact preview | Shows counts AFTER action | Shows estimated counts BEFORE action | P1 |
| No time estimate | No ETA | Show "~5 min for 100 docs" | P2 |
| Limited model info | Only in card variant | Always show current model | P3 |

## Strategic Decision Points

### Option A: Add Preview Endpoint (Backend Change)
**Pros**: Accurate counts, reusable
**Cons**: Requires Rust changes, API versioning

### Option B: Fetch Document Stats (Frontend Only)
**Pros**: No backend changes, faster to implement
**Cons**: Needs additional API call, may have stale data

### Option C: Show Counts from Previous Data (Minimal Change)
**Pros**: Fast implementation, uses existing data
**Cons**: May be stale if documents changed

## Recommended Approach

**Option B + C Hybrid**:
1. Use TanStack Query to prefetch document stats
2. Show cached counts in confirmation dialog
3. Add data-testid for E2E testing
4. Add time estimate heuristic (1 doc ≈ 2-5 seconds)

## Dependencies

- Document list query already available via `useQuery`
- Workspace stats may already be fetched
- Need to check `/documents` endpoint for count data
