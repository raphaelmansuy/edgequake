# Iteration 66 - OBSERVE: Category B Collision Fixes

**Date:** 2026-01-09
**Objective:** Fix FEAT0701-0705 Lineage vs API Client collisions

## Observation

### Initial State

- 45 duplicate FEAT IDs detected
- FEAT0701-0705 had collisions between Lineage features and API Client features

### Migration Analysis

| Original ID | Lineage Feature (Keep)      | API Client Feature (Migrate)  |
| ----------- | --------------------------- | ----------------------------- |
| FEAT0701    | Chunk lineage visualization | SSE streaming client          |
| FEAT0702    | Entity-to-document tracing  | Request/response interceptors |
| FEAT0703    | Multiple view modes         | Chat completions API client   |
| FEAT0704    | Chunk search and filtering  | Streaming chat responses      |
| FEAT0705    | Related entity exploration  | Query mode selection          |

### Namespace Discovery

While migrating, discovered additional collisions:

- FEAT0750 already used by thinking-display.tsx (Collapsible thinking sections)
- FEAT0751 already used by graph-controls.tsx (Graph rendering options)
- FEAT0760 already used by ingestion-progress-panel.tsx

**Solution:** Used FEAT0770-0774 range (verified empty)

### Files Modified

1. `edgequake_webui/src/lib/api/client.ts`:

   - FEAT0701 → FEAT0770 (SSE streaming client)
   - FEAT0702 → FEAT0771 (Request/response interceptors)

2. `edgequake_webui/src/lib/api/chat.ts`:
   - FEAT0703 → FEAT0772 (Chat completions API client)
   - FEAT0704 → FEAT0773 (Streaming chat responses)
   - FEAT0705 → FEAT0774 (Query mode selection)

## Metrics

| Metric        | Before | After | Delta                  |
| ------------- | ------ | ----- | ---------------------- |
| Duplicate IDs | 45     | 42    | -3                     |
| Code Features | 177    | 182   | +5 (new IDs)           |
| Undocumented  | 110    | 115   | +5 (new IDs need docs) |

## Verification

```bash
grep -r "@implements FEAT077" edgequake_webui/src/lib/api/
# Output confirms new IDs in place
```

## Remaining Collisions

Still 42 duplicates remaining - need additional iterations for:

- Category C namespace violations (FEAT0501-0506, FEAT0801-0804)
- Category A overloading (acceptable but should document)
