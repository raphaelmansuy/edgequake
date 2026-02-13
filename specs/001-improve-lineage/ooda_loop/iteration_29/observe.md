# Observation - Iteration 29

## Files Examined
- lineage.rs (1–1274) — full handler file
- lineage_types.rs (1–100) — DTO definitions

## WHY Comment Audit
- ✅ `LINEAGE_CACHE_TTL` — has WHY
- ✅ `LINEAGE_CACHE_MAX_ENTRIES` — has WHY
- ✅ `cached_kv_get` — has WHY
- ✅ `invalidate_lineage_cache` — has WHY
- ✅ `lineage_to_csv` — has WHY
- ✅ CSV field escaping — has WHY
- ✅ OODA-27 entity provenance doc-name resolution — has WHY
- ❌ `get_chunk_detail` — no WHY for doc ID extraction from chunk ID format
- ❌ `get_entity_lineage` — no WHY for entity name normalization
- ❌ `get_document_lineage` — no WHY for chunk prefix filtering
- ❌ `get_chunk_lineage` — no WHY for content_preview truncation
- ❌ `get_document_full_lineage` — no WHY for combining lineage + metadata

## Error Message Audit (Q4)
- ✅ `"Chunk '{}' not found"` — includes chunk ID
- ✅ `"Entity '{}' not found"` — includes entity ID, but doesn't suggest normalization
- ✅ `"Document '{}' not found"` — includes doc ID
- ✅ `"Lineage for document '{}' not found. Document may not have been processed yet."` — excellent
- ❌ `"Lineage for document '{}' not found."` (in export handler) — no guidance about processing

## Clippy
- 0 warnings on `cargo clippy -p edgequake-api --lib`

## Tests
- 459 passed, 0 failed
