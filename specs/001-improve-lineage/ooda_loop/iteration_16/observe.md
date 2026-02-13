# Observation - Iteration 16

## Focus: Python SDK Lineage Methods

## Files Examined

- `sdks/python/edgequake/types/operations.py` — Contains Pydantic models for all operation types (LineageGraph, ChunkDetail, ProvenanceRecord, etc.)
- `sdks/python/edgequake/resources/documents.py` — Sync `DocumentsResource` and async `AsyncDocumentsResource` classes
- `sdks/python/edgequake/resources/operations.py` — Sync `ChunksResource`, `ProvenanceResource`, async `AsyncChunksResource`, etc.
- `sdks/python/edgequake/resources/__init__.py` — Re-exports all resource classes

## Current State Before Changes

- Python SDK had NO lineage-specific methods on Documents or Chunks resources
- No `DocumentFullLineage` or `ChunkLineageInfo` Pydantic models existed
- `ChunksResource` only had a `get(chunk_id)` method
- `DocumentsResource` had standard CRUD + upload/delete/retry but no lineage retrieval
- Async counterparts mirrored sync classes with no lineage methods

## Tests Run

- `python -m pytest tests/ --ignore=tests/test_types.py -x -q`
- Results: **315 passed**, 29 skipped, 1 pre-existing failure (`test_complete` in chat resource — unrelated)
- Pre-existing issue: `test_types.py` fails to import `ChatChoice` from `edgequake.types.chat` — not introduced by our changes

## SDK Pattern Observed

- Sync resources extend `SyncResource` base class with `_get()`, `_post()`, etc.
- Async resources extend `AsyncResource` base class with async versions
- Types use Pydantic `BaseModel` with `model_config = ConfigDict(extra="allow")`
- Methods return typed Pydantic models via `response_type=` parameter
