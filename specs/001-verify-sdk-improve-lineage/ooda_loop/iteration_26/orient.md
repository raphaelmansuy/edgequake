# OODA-26 Orient: Swift SDK Lineage Analysis

## Gap Analysis

1. Zero lineage types → need 19 structs matching Rust lineage_types.rs
2. Zero lineage methods → need 7 service methods (entityLineage, documentLineage, documentFullLineage, exportLineage, chunkDetail, chunkLineage, entityProvenance)
3. Missing service convenience methods → tests reference .complete(), .query(request:), .uploadText(request:), .get(id:), .providerHealth(name:), .status()
4. Pre-existing test bug → JSONDecoder missing snake_case strategy

## Risk Assessment

- LOW: Adding new files/methods — no breaking changes to existing API
- MEDIUM: Adding convenience overloads to existing services — must not conflict with existing signatures
- LOW: Model expansion — all fields optional (Codable handles missing keys gracefully)

## Approach

Create LineageModels.swift (19 structs) + LineageService.swift (7 methods), wire into EdgeQuakeClient, add convenience methods to existing services, fix pre-existing test bug, add ~19 LineageService tests.
