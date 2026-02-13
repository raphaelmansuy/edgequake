# Decision - Iteration 11

## Approach: Additive API surface, no breaking changes

1. Keep `getDocumentLineage` pointing to old `/lineage/documents/:id` endpoint with `DocumentLineageResponse` return type
2. Add new `getDocumentFullLineage` → `/documents/:id/lineage` → `DocumentFullLineageResponse`
3. Add new `getDocumentMetadata` → `/documents/:id/metadata`
4. Update `getChunkLineage` return type to `ChunkLineageApiResponse` (no UI consumers yet)
5. Add `useDocumentFullLineage` and `useDocumentMetadata` React Query hooks
6. Keep `useDocumentLineage` unchanged for existing lineage-explorer.tsx

## Rationale
Additive-only approach prevents regression in `lineage-explorer.tsx` (534 lines, 23+ type dependencies on old shape).
New components can adopt new hooks incrementally.
