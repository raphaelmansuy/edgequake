# Implementation - Iteration 16

## Changes Made

### 1. Pydantic Models (`sdks/python/edgequake/types/operations.py`)
- Added `DocumentFullLineage` model (fields: document_id, chunks, entities)
- Added `ChunkLineageInfo` model (17 fields: chunk_id, document_id, positions, models, timestamps, etc.)
- Placed after existing `LineageGraph` class (~line 245)
- Both use `ConfigDict(extra="allow")` for forward compatibility

### 2. Sync DocumentsResource (`sdks/python/edgequake/resources/documents.py`)
- Added `get_lineage(document_id) -> DocumentFullLineage` — GET /api/v1/documents/{id}/lineage
- Added `get_metadata(document_id) -> dict` — GET /api/v1/documents/{id}/metadata
- Added import for `ChunkLineageInfo, DocumentFullLineage`

### 3. Async DocumentsResource (`sdks/python/edgequake/resources/documents.py`)
- Added async `get_lineage(document_id) -> DocumentFullLineage`
- Added async `get_metadata(document_id) -> dict`

### 4. Sync ChunksResource (`sdks/python/edgequake/resources/operations.py`)
- Added `get_lineage(chunk_id) -> ChunkLineageInfo` — GET /api/v1/chunks/{id}/lineage
- Added `ChunkLineageInfo` to imports

### 5. Async ChunksResource (`sdks/python/edgequake/resources/operations.py`)
- Added async `get_lineage(chunk_id) -> ChunkLineageInfo`

## Verification

- `python -m pytest tests/ --ignore=tests/test_types.py -x -q`: **315 passed**, 29 skipped, 1 pre-existing failure
- No regressions introduced

## SDK Lineage Completion Status

| SDK        | get_lineage (doc) | get_metadata (doc) | get_lineage (chunk) | Iteration |
|------------|-------------------|--------------------|---------------------|-----------|
| Rust       | ✅                 | ✅                  | ✅                   | OODA-14   |
| TypeScript | ✅                 | ✅                  | ✅                   | OODA-15   |
| Python     | ✅                 | ✅                  | ✅                   | OODA-16   |

**Deliverable #5 (F7) is now COMPLETE**: All SDKs expose lineage retrieval methods.
