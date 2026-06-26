# 01 — Ingestion Pipeline

> **Spec**: 021-storage-study  
> **File**: 03-pipelines/01-ingestion-pipeline.md  
> **Date**: 2026-06-25  
> **Source**: `edgequake-core/src/orchestrator/ingestion.rs`,  
> `edgequake-pipeline/src/pipeline/processing.rs`,  
> `edgequake-pipeline/src/merger/`,  
> `edgequake-api/src/processor/`

---

## Two Ingestion Entry Points

There are **two distinct ingestion code paths** that both ultimately store data:

```
1. API path (via DocumentTaskProcessor)
   HTTP POST /api/v1/documents
       |-- Validate file
       |-- Write to `documents` table (status=pending)
       |-- Write to `pdf_documents` if PDF
       |-- Enqueue task in `edgequake_tasks`
       |-- Return 202 Accepted
       |-- [Background] DocumentTaskProcessor picks up task
       |-- [Background] Run text extraction (PDF → markdown)
       |-- [Background] Call EdgeQuake::insert() OR workspace pipeline
       |-- Update `documents` status → indexed | failed

2. Direct API path (EdgeQuake::insert())
   Called by:
   - DocumentTaskProcessor (from task queue)
   - Direct Rust API users
   - Integration tests
```

---

## Stage-by-Stage Ingestion Flow

### Stage 0 — API Layer (documents table)

```
DocumentTaskProcessor::process_task()
    |
    +--> Read task from `edgequake_tasks` WHERE status='pending'
    +--> Update `documents`.status = 'processing'
    +--> Extract text from PDF (edgequake-pdf crate)
    +--> Call EdgeQuake::insert(content, doc_id)
    +--> Update `documents`.status = 'indexed' | 'failed'
    +--> Update `documents`.chunk_count, entity_count, relationship_count
```

**Storage writes at Stage 0**:
- `documents` (status update, counter update)
- `edgequake_tasks` (status: running → completed)

---

### Stage 1 — Pipeline: Chunking

```
Pipeline::process_with_resilience_cancellable()
    |
    +--> Chunker::chunk_async(content, doc_id)
    |    |-- Split text into TextChunk[]
    |    |-- Each chunk: {id, text, index, start_line, end_line, token_count}
    |    |-- Strategy: token-based (default 1200 tokens, 8% overlap)
    |
    +--> Output: Vec<TextChunk>
```

**Storage writes at Stage 1**: None.

---

### Stage 2 — Pipeline: Entity Extraction

```
Pipeline::resilient_extract_parallel()
    |
    +--> For each TextChunk (parallel, configurable concurrency):
    |    |-- LLMExtractor::extract(chunk_text)
    |    |-- OR GleaningExtractor::extract() (multi-pass)
    |    |-- Parse SOTA tuple format: ("entity","PERSON","description")
    |    |-- Normalize entity name: UPPERCASE_UNDERSCORE
    |
    +--> Output: Vec<ExtractionResult> {entities, relationships, chunk_id}
```

**Storage writes at Stage 2**: None (pure compute).

---

### Stage 3 — Pipeline: Embedding Generation

```
Pipeline::generate_all_embeddings()
    |
    +--> For each TextChunk: embed chunk.content
    +--> For each ExtractedEntity: embed "{name} {description}"
    +--> For each ExtractedRelationship: embed "{source} {relation_type} {target}"
    |
    +--> Output: embeddings attached to chunks and extractions in memory
```

**Storage writes at Stage 3**: None (pure compute).

---

### Stage 4 — Orchestrator: Vector Store Write (FIRST DURABLE WRITE)

```
EdgeQuake::insert() [orchestrator/ingestion.rs]
    |
    +--> Collect all chunk embeddings into batch:
    |    [{id: "{doc_id}-chunk-{n}", embedding: [f32;dim], metadata: {...}}]
    |
    +--> vector_storage.upsert(batch)  // single UNNEST transaction (QW2)
    |    |-- INSERT INTO eq_*_vectors ON CONFLICT DO UPDATE
    |    |-- metadata.type = "chunk"
    |    |-- metadata.document_id = doc_id
    |    |-- metadata.tenant_id / workspace_id (for isolation)
    |
    +--> Collect entity embeddings into batch:
    |    [{id: entity_name, embedding: [f32;dim], metadata: {type:"entity",...}}]
    |
    +--> vector_storage.upsert(entity_batch)
    |
    +--> Collect relationship embeddings:
    |    [{id: "SRC::TGT", embedding, metadata: {type:"relationship",...}}]
    |
    +--> vector_storage.upsert(rel_batch)
```

**Storage writes at Stage 4**: `eq_*_vectors` (chunk + entity + relationship embeddings)

> **SAGA POINT**: After this stage, vectors are durable. If Stage 5 fails, the
> SAGA compensation deletes all vectors for this `doc_id`.

---

### Stage 5 — Orchestrator: KV Store Write

```
EdgeQuake::insert() continued
    |
    +--> Store document metadata in KV:
    |    kv_storage.upsert([("{doc_id}-metadata", DocumentMetadata)])
    |
    +--> Store chunk text in KV:
    |    kv_storage.upsert([
    |      ("{doc_id}-chunk-0", ChunkContent),
    |      ("{doc_id}-chunk-1", ChunkContent),
    |      ...
    |    ])
```

**Storage writes at Stage 5**: `eq_*_kv` (document metadata + chunk text)

---

### Stage 6 — Orchestrator: Graph Merge (LAST DURABLE WRITE)

```
KnowledgeGraphMerger::merge(extractions, graph_storage)
    |
    +--> For each extracted entity (dedup by normalized name):
    |    |-- If existing node: merge descriptions via LLM summarizer (if enabled)
    |    |-- graph_storage.upsert_nodes_batch(nodes)  // UNWIND MERGE, 500/chunk
    |
    +--> For each extracted relationship (dedup by source+target):
    |    |-- If existing edge: merge/update properties
    |    |-- graph_storage.upsert_edges_batch(edges)  // UNWIND MERGE, 500/chunk
    |
    +--> On SUCCESS: return MergeStats
    +--> On FAILURE: [SAGA compensation]
         |-- vector_storage.delete(doc_id chunk vectors)
         |-- kv_storage.delete(doc_id metadata + chunks)
         |-- Return Error
```

**Storage writes at Stage 6**: Apache AGE graph (Node + EDGE records)

---

## Write Order Summary

```
Stage 4: vector_storage.upsert()   -> eq_*_vectors
Stage 5: kv_storage.upsert()       -> eq_*_kv
Stage 6: graph_storage.upsert_*()  -> AGE graph (Node, EDGE)

SAGA: if stage 6 fails:
  vector_storage.delete(doc_id vectors)
  kv_storage.delete(doc_id keys)
```

---

## What is NOT written by the ingestion pipeline

| Table                 | Why not written                                                   |
| --------------------- | ----------------------------------------------------------------- |
| `chunks` table        | Pipeline stores chunk text in KV, not relational table            |
| `entities` table      | Pipeline stores entities in AGE graph, not relational table       |
| `relationships` table | Pipeline stores relationships in AGE graph, not relational table  |
| `chunks.embedding`    | Pipeline stores embeddings in vector store, not relational column |
| `entities.embedding`  | Same as above                                                     |

These tables/columns were the **original design** (migration 002) but were
superseded when the KV+Vector+AGE architecture was adopted without removing
the legacy schema.
