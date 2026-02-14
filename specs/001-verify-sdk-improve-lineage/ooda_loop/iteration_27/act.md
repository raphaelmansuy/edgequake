# OODA-27 Act: PHP SDK Lineage — COMPLETED

## Changes Made

### 1. `sdks/php/src/Services.php` — Added LineageService class

- 7 methods: `entityLineage()`, `documentLineage()`, `documentFullLineage()`, `exportLineage()`, `chunkDetail()`, `chunkLineage()`, `entityProvenance()`
- `exportLineage()` returns `string` via `getRaw()` (supports JSON/CSV)
- `rawurlencode()` for URL-safe entity names

### 2. `sdks/php/src/Client.php` — Wired LineageService (16 → 17 services)

- Added `public readonly LineageService $lineage;`
- Added `$this->lineage = new LineageService($http);` in constructor

### 3. `sdks/php/tests/UnitTest.php` — 15 lineage tests added

- `testClientHasLineageService`
- `testEntityLineage` — response parsing + path verification
- `testEntityLineageUrlEncoding` — spaces → %20
- `testEntityLineageSpecialChars` — O'BRIEN handling
- `testDocumentLineage` — entities + relationships arrays
- `testDocumentLineageEmpty` — null extraction_stats
- `testDocumentFullLineage` — chunks + total_chunks
- `testExportLineageJson` — raw string return, format=json
- `testExportLineageCsv` — format=csv
- `testChunkDetail` — content + entities
- `testChunkDetailMinimal` — empty content
- `testChunkLineage` — document_id reference
- `testEntityProvenance` — source_documents + related_entities
- `testEntityProvenanceMinimal` — minimal response
- `testLineageErrorHandling` — 404 → ApiError
- Updated `testClientInitializesAllServices` — added Conversation, Folder, Lineage assertions

## Test Results

```
PHPUnit 11.5.53 — PHP 8.5.2
OK (106 tests, 206 assertions)
```

## Commit

`OODA-27: PHP SDK lineage — 7 methods, 15 tests, 106 total passing`
