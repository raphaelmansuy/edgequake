# OODA Loop 1 - Orient

## Root Cause Analysis

### Primary Bug: Missing source_chunk_ids Linkage

During entity extraction, entities are created but their `source_chunk_ids` field is never populated with the chunk ID they were extracted from.

**Evidence:**

- `ExtractedEntity` struct has `source_chunk_ids: Vec<String>` field
- The parser creates entities with `ExtractedEntity::new(...)` which initializes `source_chunk_ids: Vec::new()` (empty)
- `ExtractionResult` has `source_chunk_id: String` tracking which chunk was processed
- But there's NO code that calls `entity.add_source_chunk_id(extraction.source_chunk_id)`

**Impact:**

1. Local mode reads `source_chunk_ids` from entities to find related chunks
2. Since `source_chunk_ids` is always empty, no chunks are retrieved
3. Local mode returns 0 chunks → LLM has no context → "no information available"

### Code Flow Analysis

```
Pipeline::process()
  → extract_parallel()
    → LLMExtractor::extract()
      → parse_response()
        → ExtractedEntity::new(name, type, desc)  // ❌ No chunk_id set!

  → merger.merge()
    → merge_entity()
      → stores entity.source_chunk_ids  // 📝 Stores empty array
```

### LightRAG Comparison

LightRAG properly tracks source_id during extraction:

- Python: `entity["source_id"] = chunk_key`
- During query: `_find_related_text_unit_from_entities()` uses source_id to find chunks

EdgeQuake has the fields but never populates them.

### Secondary Issues

1. **Relationships also affected**: `ExtractedRelationship.source_chunk_id` is `Option<String>` and also never set
2. **Vector metadata**: Entity vectors DO get `source_chunk_ids` in metadata during merge, but graph nodes may not

## Fix Strategy

### Option A: Fix in Pipeline (Preferred)

After extraction, before merge:

```rust
for extraction in &mut extractions {
    let chunk_id = extraction.source_chunk_id.clone();
    for entity in &mut extraction.entities {
        entity.add_source_chunk_id(&chunk_id);
    }
    for rel in &mut extraction.relationships {
        rel.source_chunk_id = Some(chunk_id.clone());
    }
}
```

### Option B: Fix in Merger

During merge_entity(), extract chunk_id from extraction context.
Less clean - requires passing extra context.

### Recommendation: Option A

Fix in pipeline.rs after extraction, minimal changes.
