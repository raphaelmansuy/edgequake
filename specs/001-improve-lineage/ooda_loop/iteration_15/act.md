# Action - Iteration 15

## Changes Made

### sdks/typescript/src/types/lineage.ts
- Added `DocumentFullLineageResponse` interface (3 fields)
- Added `ChunkLineageResponse` interface (16 fields)
- Both appended after legacy aliases section

### sdks/typescript/src/resources/documents.ts
- Added import for `DocumentFullLineageResponse`
- Added `getLineage(documentId)` → GET /documents/:id/lineage
- Added `getMetadata(documentId)` → GET /documents/:id/metadata

### sdks/typescript/src/resources/chunks.ts
- Added import for `ChunkLineageResponse`
- Added `getLineage(chunkId)` → GET /chunks/:id/lineage

## Verification
- `npx tsc --noEmit` — CLEAN
- `npx vitest run` — 247 passed, 0 failures
