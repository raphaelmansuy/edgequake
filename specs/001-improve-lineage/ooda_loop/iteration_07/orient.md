# Analysis - Iteration 07

## Gaps Identified

1. **ChunkDetailResponse incomplete**: OODA-05 persisted `start_line`, `end_line` to KV but the API response type didn't expose them
2. **Field name mismatch**: KV stores `index` but handler read `chunk_index` → always returned 0
3. **No single-call lineage endpoint**: Must call `/lineage/documents/:id` + `/documents/:id` + chunk queries separately
4. **No metadata endpoint**: Document metadata requires parsing `/documents/:id` response

## Possible Solutions

### Solution A: Add fields to existing endpoints
- Pros: No new routes, backward compatible
- Cons: Doesn't satisfy F5 (single API call for complete lineage)
- Risk: Low

### Solution B: Create new dedicated endpoints (chosen)
- `GET /documents/:id/lineage` — returns persisted DocumentLineage + metadata
- `GET /documents/:id/metadata` — returns all KV metadata in one call
- Pros: Satisfies F5, clean REST design, O(1) KV lookups
- Cons: 2 new routes to maintain
- Risk: Low

## Recommendation

Solution B — create new endpoints AND fix existing ChunkDetailResponse. This addresses F5 (single-call lineage) and T2 (no N+1 queries) directly.

## Architecture

```
GET /api/v1/documents/:id/lineage
  └─ KV get: {id}-lineage  → DocumentLineage (from OODA-06)
  └─ KV get: {id}-metadata → document metadata
  └─ Returns: { document_id, metadata, lineage }

GET /api/v1/documents/:id/metadata
  └─ KV get: {id}-metadata → all document metadata
  └─ Returns: serde_json::Value (full metadata blob)
```
