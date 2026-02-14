# OODA-29 Decide: Go SDK Lineage Enhancement

## Actions
1. Add 5 new types to types.go (DocumentLineageResponse, EntitySummary, RelationshipSummary, DocumentFullLineageResponse, ChunkLineageResponse)
2. Add `getRaw` method to client.go for raw byte downloads
3. Add 4 methods: LineageService.ForDocument, DocumentFullLineage, ExportLineage; ChunkService.Lineage
4. Add 8 tests to edgequake_test.go
