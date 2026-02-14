# OODA-29 Act: Go SDK Lineage Enhancement — COMPLETED

## Changes Made

### 1. `sdks/go/types.go` — Added 5 lineage types
- `DocumentLineageResponse` — document_id, entities, relationships, extraction_stats
- `EntitySummary` — entity_name, entity_type, mentions, confidence
- `RelationshipSummary` — source_entity, target_entity, keywords, weight
- `DocumentFullLineageResponse` — document_id, chunks, total_chunks, metadata
- `ChunkLineageResponse` — chunk_id, document_id, entities, relationships, metadata

### 2. `sdks/go/client.go` — Added `getRaw` method
- `getRaw(ctx, path, params) ([]byte, error)` — returns raw bytes for export endpoint
- Includes retry logic matching `do()` pattern

### 3. `sdks/go/services.go` — Added 4 methods
- `LineageService.ForDocument(ctx, documentID)` → `*DocumentLineageResponse`
- `LineageService.DocumentFullLineage(ctx, documentID)` → `*DocumentFullLineageResponse`
- `LineageService.ExportLineage(ctx, documentID, format)` → `[]byte`
- `ChunkService.Lineage(ctx, id)` → `*ChunkLineageResponse`

### 4. `sdks/go/edgequake_test.go` — Added 8 tests
- `TestLineage_ForDocument` — entities + relationships parsing
- `TestLineage_ForDocumentEmpty` — empty response
- `TestLineage_DocumentFullLineage` — chunks + total_chunks
- `TestLineage_ExportLineageJSON` — raw JSON bytes
- `TestLineage_ExportLineageCSV` — CSV string with ALICE
- `TestChunks_Lineage` — chunk_id + document_id + entities
- `TestLineage_ForEntityError` — 404 → APIError

## Test Results
```
ok  github.com/edgequake/edgequake-go  6.877s
216 tests passing (--- PASS count)
```

## Commit
`OODA-29: Go SDK lineage — 5 types, 4 methods, 8 tests, 216 total passing`
