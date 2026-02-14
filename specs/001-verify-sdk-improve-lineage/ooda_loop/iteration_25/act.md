# OODA-25 Orient + Decide + Act: C# Lineage Tests

## Added Tests (15 new)

- 7 endpoint tests (EntityLineage, DocumentLineage, DocumentFullLineage, ExportLineage, ChunkDetail, ChunkLineage, EntityProvenance) — each verifies all fields + URL correctness
- 1 URL-encoding edge case test (special characters in entity name)
- 5 edge case tests (empty source docs, empty graph, null optional fields, minimal fields, no related entities)
- 1 multi-source document test (3 sources, 2 description versions)
- 1 client accessor test (Lineage property is LineageService)

## Test Evidence

```
dotnet test --filter "FullyQualifiedName~LineageTest"
Passed! - Failed: 0, Passed: 54, Total: 54

dotnet test --filter "FullyQualifiedName~UnitTest|FullyQualifiedName~LineageTest"
Passed! - Failed: 0, Passed: 133, Total: 133
```

## Commit

SHA: (pending)
