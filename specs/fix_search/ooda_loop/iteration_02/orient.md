# OODA Loop 2 - Orient

## Analysis of Root Cause

### Architecture Understanding

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           EdgeQuake Pipeline                             │
├──────────────────────────────────────────────────────────────────────────┤
│  1. Chunker          → Creates text chunks from documents                │
│  2. Extractor        → LLM extracts entities + relationships             │
│  3. EmbeddingProvider→ Generates embeddings for chunks, entities, rels   │
│  4. Pipeline         → Links entities to source chunk IDs                │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                      API Document Handler                                │
├──────────────────────────────────────────────────────────────────────────┤
│  upload_document()                                                       │
│  ├─ Store chunks in KV storage         ✅ Working                        │
│  ├─ Store chunk embeddings in vector   ✅ Working                        │
│  ├─ Store entities in graph            ✅ Working (but missing fields)   │
│  ├─ Store entity embeddings in vector  ❌ MISSING!                       │
│  └─ Store relationships in graph       ✅ Working (but missing fields)   │
└──────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌──────────────────────────────────────────────────────────────────────────┐
│                        SOTAQueryEngine                                   │
├──────────────────────────────────────────────────────────────────────────┤
│  query_local()                                                           │
│  ├─ Vector search for entities         → Returns 0 (no entity vectors)  │
│  ├─ Fallback to popular entities       → No source_chunk_ids            │
│  └─ Chunk retrieval                    → Empty chunks list              │
└──────────────────────────────────────────────────────────────────────────┘
```

### Why This Was Missed

1. **Multiple code paths**: `upload_document()` vs `upload_file()` have separate entity storage code
2. **Testing gap**: Tests may have used mocks that don't validate vector storage content
3. **Silent failure**: Entity embeddings discarded silently, no error logged

### LightRAG Comparison

Looking at LightRAG's implementation pattern, entities MUST be stored in both:
- Graph storage (for relationship traversal)
- Vector storage (for semantic similarity search)

EdgeQuake was missing the vector storage step.

