# OODA Loop 2 - Decide

## Decision

### Primary Fix
Store entity embeddings in vector storage during document ingestion.

### Implementation Plan

1. **Modify `upload_document()` in documents.rs** (lines ~360-420)
   - After storing entity in graph storage
   - If entity has embedding, upsert to vector storage with `type: entity` metadata

2. **Add `source_chunk_ids` to graph properties**
   - Ensure entity properties include source_chunk_ids for chunk retrieval

### Code Changes Required

```rust
// After graph_storage.upsert_node()
if let Some(embedding) = &entity.embedding {
    let metadata = serde_json::json!({
        "type": "entity",
        "entity_name": entity.name,
        "entity_type": entity.entity_type,
        "description": entity.description,
        "document_id": document_id,
        "source_chunk_ids": entity.source_chunk_ids,
    });
    
    let entity_id = format!("entity:{}", entity.name);
    state.vector_storage.upsert(&[(entity_id, embedding.clone(), metadata)]).await?;
}
```

### Expected Outcome

1. query_local() will find entities via vector search
2. Entity properties will include source_chunk_ids
3. Chunks will be retrieved and returned in query results

