# OODA-28 Act: Ruby SDK Lineage — COMPLETED

## Changes Made

### 1. `sdks/ruby/lib/edgequake/services.rb` — Added LineageService class
- 7 methods: `entity_lineage`, `document_lineage`, `document_full_lineage`, `export_lineage`, `chunk_detail`, `chunk_lineage`, `entity_provenance`
- `export_lineage` returns raw `String` via `get_raw()`
- `URI.encode_www_form_component` for URL-safe entity names

### 2. `sdks/ruby/lib/edgequake/client.rb` — Wired LineageService (16 → 17 services)
- Added `:lineage` to `attr_reader`
- Added `@lineage = LineageService.new(http)` in constructor

### 3. `sdks/ruby/test/unit_test.rb` — 16 lineage tests added
- `test_client_has_lineage_service`
- `test_entity_lineage` — response + path verification
- `test_entity_lineage_url_encoding` — space → +
- `test_entity_lineage_special_chars` — O'BRIEN
- `test_document_lineage` — entities + relationships
- `test_document_lineage_empty` — null extraction_stats
- `test_document_full_lineage` — /documents/{id}/lineage path
- `test_export_lineage_json` — raw string, format=json
- `test_export_lineage_csv` — format=csv
- `test_chunk_detail` — content + entities
- `test_chunk_detail_minimal` — empty content
- `test_chunk_lineage` — document_id reference
- `test_entity_provenance` — source_documents + related_entities
- `test_entity_provenance_minimal` — minimal response
- `test_lineage_error_handling` — 404 → ApiError

## Test Results
```
109 runs, 243 assertions, 0 failures, 0 errors, 0 skips
```

## Commit
`OODA-28: Ruby SDK lineage — 7 methods, 16 tests, 109 total passing`
