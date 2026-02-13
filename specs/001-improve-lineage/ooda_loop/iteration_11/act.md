# Action - Iteration 11

## Changes Made

### edgequake_webui/src/lib/api/edgequake.ts
- Kept `getDocumentLineage` → `/lineage/documents/:id` → `DocumentLineageResponse` (unchanged)
- Added `getDocumentFullLineage` → `/documents/:id/lineage` → `DocumentFullLineageResponse`
- Added `getDocumentMetadata` → `/documents/:id/metadata` → `Record<string, unknown>`
- Updated `getChunkLineage` return type → `ChunkLineageApiResponse`
- Updated exports to include new functions

### edgequake_webui/src/hooks/use-lineage.ts
- Added `useDocumentFullLineage(documentId)` hook
- Added `useDocumentMetadata(documentId)` hook
- Kept `useDocumentLineage(documentId)` unchanged
- Added query keys: `documentFullLineage`, `documentMetadata`

## Verification
- `npx tsc --noEmit` — CLEAN (0 errors)
- lineage-explorer.tsx — unaffected (still uses old `useDocumentLineage` hook)

## Lesson Learned
**Never change return types of existing API functions consumed by large components.**
Use additive pattern: new function → new name → new hook. Components migrate incrementally.
