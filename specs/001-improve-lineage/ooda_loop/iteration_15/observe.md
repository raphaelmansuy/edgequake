# Observation - Iteration 15

## Files Examined
- `sdks/typescript/src/resources/documents.ts` (187 lines) - Has CRUD, upload, scan, reprocess
- `sdks/typescript/src/resources/chunks.ts` (22 lines) - Has get()
- `sdks/typescript/src/types/lineage.ts` (260 lines) - Has entity/chunk/provenance types but no full lineage

## Tests Run
- `npx tsc --noEmit` — Clean
- `npx vitest run` — 247 passed, 62 skipped, 0 failures

## Gap
- No `getLineage()` or `getMetadata()` on DocumentsResource
- No `getLineage()` on ChunksResource
- No `DocumentFullLineageResponse` or `ChunkLineageResponse` types
