# Analysis - Iteration 08

## Gap Identified

Mission deliverable #3 requires: `GET /api/v1/chunks/:id/lineage — Chunk lineage with parent refs`

The existing `/chunks/:id` detail endpoint returns entities/relationships but NOT:
- Parent document metadata
- Document type (pdf/markdown)
- Content preview (returns full content which is heavy)
- Entity summary (names only, lightweight)

## Solution

Create `ChunkLineageResponse` DTO and `get_chunk_lineage` endpoint that returns:

```
GET /api/v1/chunks/:id/lineage
  └─ KV get: chunk_id        → chunk data (position, tokens, content)
  └─ KV get: {doc_id}-metadata → parent document context
  └─ Graph: entities by source_id → entity names
  └─ Graph: edges by source_id   → relationship count
  └─ Returns: { chunk_id, document_id, document_name, document_type,
                index, start_line, end_line, token_count,
                content_preview, entity_count, relationship_count,
                entity_names, document_metadata }
```

This is lighter than `get_chunk_detail` (preview vs full content, counts vs full lists) and adds document context.

## Risk

Low — adds new endpoint without modifying existing ones.
