# 03 — Data Flow Diagrams

> **Spec**: 021-storage-study  
> **File**: 03-pipelines/03-data-flow-diagrams.md  
> **Date**: 2026-06-25

---

## Diagram 1: Full System Architecture

```
+------------------------------- EdgeQuake System --------------------------------+
|                                                                                 |
|  +-----------+    +-------------+    +------------------+    +--------------+  |
|  |  Browser  |    | Next.js     |    | Axum REST API    |    | Background   |  |
|  |  WebUI    +--->+ Frontend    +--->+ edgequake-api    +--->+ Task Queue   |  |
|  |           |    | (port 3000) |    | (port 8080)      |    | (Tokio)      |  |
|  +-----------+    +-------------+    +------------------+    +------+-------+  |
|                                              |                       |          |
|                                    +---------+---------+    +-------+-------+  |
|                                    |  AppState         |    | DocumentTask  |  |
|                                    |  StorageRuntime   |    | Processor     |  |
|                                    |  QueryRuntime     |    +-------+-------+  |
|                                    |  TaskRuntime      |            |          |
|                                    +---------+---------+    +-------v-------+  |
|                                              |              | EdgeQuake     |  |
|                              +---------------+              | Orchestrator  |  |
|                              |               |              +-------+-------+  |
|                              v               v                      |          |
|                   +----------+--+    +-------+-------+              |          |
|                   | SOTA Query  |    | Pipeline      |<-------------+          |
|                   | Engine      |    | (chunk+embed  |                         |
|                   |             |    |  +extract)    |                         |
|                   +------+------+    +-------+-------+                         |
|                          |                   |                                 |
|         +----------------+--+        +-------+--------+                        |
|         |                   |        |                |                        |
|         v                   v        v                v                        |
|  +------+------+  +---------+--+  +--+----------+  +-+----------+             |
|  | Vector      |  | KV Store   |  | Graph Store |  | Doc Store  |             |
|  | Storage     |  | (JSONB)    |  | (AGE/Cypher)|  | (SQL)      |             |
|  |             |  |            |  |             |  |            |             |
|  | eq_*_vectors|  | eq_*_kv    |  | AGE graph   |  | documents  |             |
|  | ws_*_vectors|  |            |  | Node / EDGE |  | tasks      |             |
|  +-------------+  +------------+  +-------------+  | pdf_docs   |             |
|                                                     | tenants    |             |
|                                                     | workspaces |             |
|                                                     +------------+             |
|                                                                                 |
+---------------------------------------------------------------------------------+
                              PostgreSQL 15+
```

---

## Diagram 2: Ingestion Data Flow

```
HTTP POST /documents (with PDF)
         |
         v
+--------+--------+
| DocumentTask    |  1. Write documents (status=pending)
| Processor       |  2. Write pdf_documents (pdf_data=BYTEA)
|                 |  3. Enqueue edgequake_tasks (type=ingest)
+--------+--------+
         |  [async]
         v
+--------+--------+
| Pick up task    |  READ edgequake_tasks WHERE status='pending'
| from queue      |  UPDATE status='running'
+--------+--------+
         |
         v
+--------+--------+
| PDF Extraction  |  edgequake-pdf: PDF -> Markdown text
| (edgequake-pdf) |  UPDATE pdf_documents.markdown_content
+--------+--------+
         |
         v
+--------+---------+  Stage 1: CHUNKING
| Pipeline         |  TextChunk[] in memory
|                  |  (no storage write)
+--------+---------+
         |
         v
+--------+---------+  Stage 2: EXTRACTION
| LLM Extractor    |  ExtractedEntity[], ExtractedRelationship[] in memory
| (parallel)       |  (no storage write)
+--------+---------+
         |
         v
+--------+---------+  Stage 3: EMBEDDING
| Embedding        |  embed all chunks + entities + relationships
| Provider         |  (no storage write yet)
+--------+---------+
         |
         v
+--------+---------+  Stage 4: VECTOR WRITE (first durable write)
| VectorStorage    |  UPSERT chunk vectors  -> eq_*_vectors
| .upsert()        |  UPSERT entity vectors -> eq_*_vectors
|  [SAGA POINT]    |  UPSERT rel vectors    -> eq_*_vectors
+--------+---------+
         |
         v
+--------+---------+  Stage 5: KV WRITE
| KVStorage        |  UPSERT doc metadata  -> eq_*_kv (key: {id}-metadata)
| .upsert()        |  UPSERT chunk text    -> eq_*_kv (key: {id}-chunk-{n})
+--------+---------+
         |
         v
+--------+---------+  Stage 6: GRAPH MERGE (last durable write)
| GraphMerger      |  MERGE Node[] into AGE graph
| .merge()         |  MERGE EDGE[] into AGE graph
+--------+---------+
         |
     +---+---+
     |       |
   OK        FAIL
     |       |
     |       v
     |  [SAGA compensation]
     |  DELETE chunk vectors from eq_*_vectors
     |  DELETE doc keys from eq_*_kv
     |  Return Error
     |
     v
+--------+---------+
| UPDATE           |  documents.status = 'indexed'
| documents table  |  documents.chunk_count = N
|                  |  documents.entity_count = M
+------------------+
```

---

## Diagram 3: Query Data Flow (Hybrid Mode)

```
HTTP POST /query {query, mode=hybrid, workspace_id}
         |
         v
+--------+---------+
| Auth + Workspace |  Verify workspace access
| Context          |  Resolve workspace vector table
+--------+---------+
         |
         v
+--------+---------+  Step 1: KEYWORD EXTRACTION
| CachedKeyword    |  Check eq_*_kv for {hash}-kwcache
| Extractor        |  MISS: LLM call -> ExtractedKeywords
|                  |  WRITE cache -> eq_*_kv
+--------+---------+
         |
         v
+--------+---------+  Step 2: EMBEDDING
| Embedding        |  embed(query) -> query_vec
| Provider         |  embed(hl_keywords) -> hl_vec
|                  |  embed(ll_keywords) -> ll_vec
+--------+---------+
         |
         +---------------------+
         |                     |
         v                     v
+--------+--------+   +--------+--------+  Step 3: VECTOR RETRIEVAL
| Local: entity   |   | Global: rel     |
| vector search   |   | vector search   |
| (ll_vec, top60) |   | (hl_vec, top60) |
|                 |   |                 |
| eq_ws_*_vectors |   | eq_ws_*_vectors |
+--------+--------+   +--------+--------+
         |                     |
         v                     v
+--------+--------+   +--------+--------+  Step 4: GRAPH READS
| AGE graph:      |   | AGE graph:      |
| get_node()      |   | get_edge()      |
| get_node_edges()|   | get_node()      |
+--------+--------+   +--------+--------+
         |                     |
         +---------------------+
         |
         v
+--------+---------+  Step 5: CHUNK TEXT RETRIEVAL
| KVStorage        |  get_by_ids(source_ids from graph nodes/edges)
| .get_by_ids()    |  -> chunk text content
+--------+---------+
         |
         v
+--------+---------+  Step 6: RERANKING
| BM25Reranker     |  Score and rank chunks by BM25
|                  |  Keep top-K above min_score
+--------+---------+
         |
         v
+--------+---------+  Step 7: TOKEN BUDGET
| TruncationConfig |  Truncate to fit 30K token window
|                  |  Priority: entities > relationships > chunks
+--------+---------+
         |
         v
+--------+---------+  Step 8: LLM GENERATION
| LLM Provider     |  Build prompt with context
|                  |  [Optional: read conversations table]
|                  |  Generate response text
|                  |  [Optional: write messages table]
+------------------+
         |
         v
    HTTP Response (JSON or SSE stream)
```

---

## Diagram 4: Storage System Relationships

```
                      SOURCE OF TRUTH MAP
                      ===================

  AGE Graph                    KV Store               Vector Store
  (Node, EDGE)                 (eq_*_kv)              (eq_*_vectors)
  +-------------------+        +-------------------+  +-------------------+
  | Node:             |        | {id}-metadata:    |  | {id}-chunk-{n}:   |
  |   node_id         |<----+  |   id              |  |   embedding[1536] |
  |   entity_type     |     |  |   title           |  |   metadata.type=  |
  |   description     |     |  |   chunk_count     |  |     "chunk"       |
  |   source_ids[]    |--+  |  |   entity_count    |  |   metadata.doc_id |
  |   tenant_id       |  |  |  +-------------------+  +-------------------+
  |   workspace_id    |  |  |                          +-------------------+
  +-------------------+  |  |  {id}-chunk-{n}:         | {entity_name}:    |
                          |  |    content            |  |   embedding[1536] |
  +-------------------+   |  |    chunk_index        |  |   metadata.type=  |
  | EDGE:             |   |  |    start_line         |  |     "entity"      |
  |   source_id ------+---+  |    token_count        |  |   metadata.entity_|
  |   target_id ------+---+  +-------------------+  |  |     name          |
  |   relation_type   |  |                          |  +-------------------+
  |   description     |  |  {hash}-kwcache:         |  +-------------------+
  |   source_ids[]  --+  |    keywords[]             |  | {src}::{tgt}:     |
  |   weight          |  |    expires_at             |  |   embedding[1536] |
  +-------------------+  |  +-------------------+   |  |   metadata.type=  |
                          |                          |  |     "relationship"|
  DOCUMENTS TABLE         |  NOTE: chunk source_ids  |  +-------------------+
  +-------------------+   |  in graph nodes point    |
  | documents:        |   |  to KV keys above -------+
  |   id              |<--+
  |   status          |      Note: entities table and
  |   chunk_count (*)  |     relationships table exist
  |   entity_count (*) |     in SQL but are NOT written
  +-------------------+     by the active pipeline.
  (*) = denormalized counter                          (* = orphaned schema)
```
