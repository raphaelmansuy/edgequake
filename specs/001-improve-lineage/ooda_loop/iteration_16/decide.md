# Decision - Iteration 16

## Changes to Make

1. Add `DocumentFullLineage` and `ChunkLineageInfo` Pydantic models to `sdks/python/edgequake/types/operations.py`
2. Add `get_lineage()` and `get_metadata()` to sync `DocumentsResource` in `documents.py`
3. Add `get_lineage()` and `get_metadata()` to async `AsyncDocumentsResource` in `documents.py`
4. Add `get_lineage()` to sync `ChunksResource` in `operations.py`
5. Add `get_lineage()` to async `AsyncChunksResource` in `operations.py`
6. Add `ChunkLineageInfo` import to `operations.py`

## Priority

1. High impact — completes SDK deliverable #5 (F7: All SDKs expose lineage retrieval methods)
2. Low effort — follows established patterns in the Python SDK

## Expected Outcome

Python users can call:
```python
# Sync
lineage = client.documents.get_lineage(doc_id)
metadata = client.documents.get_metadata(doc_id)
chunk_lineage = client.chunks.get_lineage(chunk_id)

# Async
lineage = await client.documents.get_lineage(doc_id)
metadata = await client.documents.get_metadata(doc_id)
chunk_lineage = await client.chunks.get_lineage(chunk_id)
```
