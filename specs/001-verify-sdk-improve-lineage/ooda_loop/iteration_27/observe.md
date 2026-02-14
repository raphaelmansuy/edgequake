# OODA-27 Observe: PHP SDK Lineage

## Current State

- PHP SDK: 16 services, 89 unit tests, 0 lineage support
- Services.php: 220 lines, all return `array` (no typed models)
- Client.php: Wires 16 services via `public readonly` properties
- MockHttpHelper: `$calls` array tracks `[method, path, body]`, `willReturn()` preset
- HttpHelper: `get() → array`, `post() → array`, `delete() → array`, `getRaw() → string`

## Lineage Gap

- No `LineageService` class in Services.php
- No lineage tests in UnitTest.php (865 lines)
- Client.php only wires 16 services (no lineage)

## API Surface (from mission file)

7 lineage endpoints to map:

1. `GET /api/v1/lineage/entities/{name}` → Entity lineage
2. `GET /api/v1/lineage/documents/{id}` → Document lineage
3. `GET /api/v1/documents/{id}/lineage` → Full document lineage
4. `GET /api/v1/documents/{id}/lineage/export?format=` → Export (JSON/CSV)
5. `GET /api/v1/chunks/{id}` → Chunk detail
6. `GET /api/v1/chunks/{id}/lineage` → Chunk lineage
7. `GET /api/v1/entities/{id}/provenance` → Entity provenance
