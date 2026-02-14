# OODA-29 Orient: Go SDK Lineage Analysis

## Existing infrastructure

- Go SDK already has Lineage, Chunks, Provenance services wired
- Need to ADD methods, not create new services
- Need `getRaw` client method for export endpoint (CSV/raw data)

## Approach

1. Add 5 new types: DocumentLineageResponse, EntitySummary, RelationshipSummary, DocumentFullLineageResponse, ChunkLineageResponse
2. Add `getRaw(ctx, path, params) ([]byte, error)` to client.go
3. Add 4 methods: LineageService.ForDocument, LineageService.DocumentFullLineage, LineageService.ExportLineage, ChunkService.Lineage
4. Add 8 tests covering all new methods + error cases
