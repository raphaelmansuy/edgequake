# Task Log: Graph Performance Fix - Duplicate API Calls

**Date:** 2026-02-09 00:47
**Mode:** beastmode
**Objective:** Implement performance improvements for graph page, starting with duplicate API call fix

## Actions

1. Analyzed `use-graph-stream.ts` hook - found SSE streaming implementation with no request deduplication
2. Analyzed `graph-viewer.tsx` - found useEffect triggering stream on dependency changes without guards
3. Added `pendingRequestRef` and `lastRequestKeyRef` to `use-graph-stream.ts` for request deduplication
4. Added `streamingInitializedRef` and `lastStreamParamsRef` to `graph-viewer.tsx` to prevent double-renders
5. Added `useRef` import to graph-viewer.tsx
6. Verified TypeScript compilation passes with no errors
7. Started services using `make dev-bg`
8. E2E tested with Playwright:
   - Page navigation: 1 API call (was 2 before)
   - Manual refresh: 1 API call per action
   - Entity selection: Works correctly
   - No console errors
9. Committed changes with detailed commit message

## Decisions

- Used ref-based deduplication instead of AbortController-only approach for better React StrictMode compatibility
- Request key includes `maxNodes`, `batchSize`, and `startNode` to allow intentional parameter changes
- Added WHY comments for clarity on why refs are needed

## Next Steps

- [ ] Install react-window for entity sidebar virtualization
- [ ] Virtualize entity sidebar (currently 200+ DOM nodes)
- [ ] Add code splitting for graph libraries
- [ ] Consider Web Worker layout for ForceAtlas2

## Lessons/Insights

- React StrictMode double-renders in development cause duplicate effect triggers
- SSE streaming hooks need explicit deduplication since they're not idempotent like REST calls
- Playwright MCP tools require Chrome to be closed before launching
- Network request verification is essential for confirming API deduplication

## Files Modified

- `edgequake_webui/src/hooks/use-graph-stream.ts`: Added pendingRequestRef, lastRequestKeyRef, deduplication in startStream
- `edgequake_webui/src/components/graph/graph-viewer.tsx`: Added streamingInitializedRef, lastStreamParamsRef, guard in useEffect

## Commit

```
31751a3a fix(graph): prevent duplicate API calls with request deduplication
```

## Performance Impact

- **Before:** 2 API calls per navigation (~1300ms total)
- **After:** 1 API call per navigation (~650ms)
- **Improvement:** ~50% reduction in initial graph load network time
