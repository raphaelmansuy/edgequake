# OODA-28 Observe: Ruby SDK Lineage

## Current State

- Ruby SDK: 16 services, ~90 unit tests, 0 lineage support
- services.rb: 16 classes inside `module EdgeQuake`, each with `def initialize(http) = @http = http`
- client.rb: 16 `attr_reader` properties wired in constructor
- MockHttpHelper: `@calls` array tracks `{method:, path:, body:}`, `will_return(json, status)`, `last_call`
- HttpHelper: `get() → Hash`, `post() → Hash`, `delete() → Hash`, `get_raw() → String`
- Tests: Minitest with `assert_equal`, `assert_kind_of`, `assert_raises`, `assert_includes`

## Lineage Gap

- No `LineageService` class in services.rb
- No lineage tests in unit_test.rb (753 lines)
- Client.rb only wires 16 services (no lineage)

## API Surface (7 endpoints)

1. `GET /api/v1/lineage/entities/{name}` → Entity lineage
2. `GET /api/v1/lineage/documents/{id}` → Document lineage
3. `GET /api/v1/documents/{id}/lineage` → Full document lineage
4. `GET /api/v1/documents/{id}/lineage/export?format=` → Export (JSON/CSV)
5. `GET /api/v1/chunks/{id}` → Chunk detail
6. `GET /api/v1/chunks/{id}/lineage` → Chunk lineage
7. `GET /api/v1/entities/{id}/provenance` → Entity provenance
