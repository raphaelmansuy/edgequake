# SPEC-032-001: Current Architecture Audit

**Parent:** [SPEC-032](000-index.md)  
**Cross-refs:** `P-G2` `P-G4` `SC2` `FEAT0011` `SPEC-021`

---

## 1. Ingestion Pipeline — End-to-End Sequence

```
Client (HTTP/REST)
  │  POST /api/documents  (multipart PDF or text)
  │
  ▼
edgequake-api  ──── DocumentTaskProcessor ────────────────────────────┐
                    processor/text_insert/                            │
                                                                      │
  ┌──── PHASE 1: PREPARE ─────────────────────────────────────────┐  │
  │  prepare.rs                                                    │  │
  │  • PDF→Markdown   (edgequake-pdf2md, pdfium embedded)         │  │
  │  • adaptive_chunker  (ChunkStrategy + ChunkerConfig)          │  │
  │  • build_ingestion_pipeline()                                  │  │
  │  PipelinePhase::PdfConversion → PipelinePhase::Chunking       │  │
  └────────────────────────────────────────────────────────────────┘  │
                                                                      │
  ┌──── PHASE 2: EXTRACT ─────────────────────────────────────────┐  │
  │  Pipeline::process_with_resilience_cancellable()               │  │
  │  • Parallel chunk entity extraction  (LLMExtractor/Gleaning)  │  │
  │  • EmbedProgressCallback → generate_all_embeddings()          │  │
  │  • build_lineage()                                             │  │
  │  PipelinePhase::Extraction                                     │  │
  └────────────────────────────────────────────────────────────────┘  │
                                                                      │
  ┌──── PHASE 3: PERSIST (P-G2) ──────────────────────────────────┐  │
  │  persist.rs  →  ingestion_persister.rs                         │  │
  │                                                                │  │
  │  Step A: KV chunk text upsert  (kv_storage.upsert)            │  │
  │  Step B: Chunk vector upsert   (vector_storage.upsert QW2)    │  │
  │  Step C: Entity vector batch   (merger.collect_entity_vector) │  │
  │  Step D: Entity graph batch    (merger.merge_entities_batch)  │  │
  │            └─ get_nodes_batch  (1 round-trip)                 │  │
  │            └─ upsert_nodes_batch  UNWIND chunks of 500        │  │
  │  Step E: Relationship vector batch                            │  │
  │  Step F: Relationship graph batch  UNWIND chunks of 500       │  │
  │  PipelinePhase::GraphStorage  (single start/complete event)   │  │
  └────────────────────────────────────────────────────────────────┘  │
                                                                      │
  finalize.rs ──► update document status → "indexed"  ◄─────────────┘
```

**Code paths (verbatim file references):**

| Step                 | File                                                             | Key function                            |
| -------------------- | ---------------------------------------------------------------- | --------------------------------------- |
| Pipeline build       | `edgequake-pipeline/src/ingestion_pipeline.rs`                   | `build_ingestion_pipeline()`            |
| Chunk extraction     | `edgequake-pipeline/src/pipeline/processing.rs`                  | `process_with_resilience_cancellable()` |
| Embedding            | `edgequake-pipeline/src/pipeline/helpers/embeddings.rs`          | `generate_all_embeddings()`             |
| Persist entry        | `edgequake-pipeline/src/persistence/ingestion_persister.rs`      | `persist_processing_result_impl()`      |
| KV upsert            | `edgequake-storage/src/adapters/postgres/kv.rs`                  | `upsert()`                              |
| Vector upsert        | `edgequake-storage/src/adapters/postgres/vector/storage_impl.rs` | `upsert()` (QW2)                        |
| Entity graph         | `edgequake-storage/src/adapters/postgres/graph/nodes_ops.rs`     | `pg_upsert_nodes_batch()`               |
| Edge graph           | `edgequake-storage/src/adapters/postgres/graph/edges_ops.rs`     | `pg_upsert_edges_batch()`               |
| Merger orchestration | `edgequake-pipeline/src/merger/mod.rs`                           | `merge()`                               |
| Entity merge logic   | `edgequake-pipeline/src/merger/entity.rs`                        | `merge_entities_batch()`                |

---

## 2. Data Model — Current State

### 2.1 Relational Tables (PostgreSQL public schema)

```
┌─────────────────────────────────────────────────────────────────────┐
│ pdf_documents                                                        │
│  id UUID PK │ checksum │ tenant_id │ workspace_id │ status │ ...    │
└──────────────────────────┬──────────────────────────────────────────┘
                           │ 1:N
┌──────────────────────────▼──────────────────────────────────────────┐
│ chunks                                                               │
│  id UUID PK │ document_id FK │ content │ chunk_index                │
│  tenant_id  │ workspace_id   │ tokens  │ source_file_path           │
│  metadata JSONB             (lineage: start_line, end_line via meta)│
└──────────────────────────┬──────────────────────────────────────────┘
                           │ N:M  (via source_chunk_ids TEXT[])
┌──────────────────────────▼──────────────────────────────────────────┐
│ entities  (CQRS read model — migration 039)                          │
│  id UUID PK │ name │ entity_type │ description                      │
│  tenant_id  │ workspace_id      │ source_chunk_ids TEXT[]           │
│  keywords TEXT[] │ tsv tsvector GENERATED                           │
│  sync_status VARCHAR(20)                                            │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│ eq_*_vectors   (pgvector table per workspace)                        │
│  id TEXT PK │ embedding VECTOR(N) │ metadata JSONB                  │
│  tenant_id TEXT │ workspace_id TEXT                                 │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Graph (Apache AGE)

```
 AGE Graph: "edgequake_graph"
 ┌──────────────────────────────────────────────────────────────┐
 │  :Node label                                                  │
 │   node_id TEXT (UPPERCASE_UNDERSCORED)                       │
 │   entity_type TEXT                                           │
 │   description TEXT                                           │
 │   source_id TEXT  ("chunk1|chunk2|…")   ← lineage pipe-sep  │
 │   tenant_id TEXT │ workspace_id TEXT                         │
 │   keywords TEXT[] │ ...                                      │
 └──────────────────────┬───────────────────────────────────────┘
                        │  :EDGE
 ┌──────────────────────▼───────────────────────────────────────┐
 │   source_id TEXT │ target_id TEXT                            │
 │   weight FLOAT   │ relation_type TEXT                        │
 │   description TEXT │ keywords TEXT[]                        │
 │   source_chunk_ids TEXT  (pipe-sep, same as node)            │
 └──────────────────────────────────────────────────────────────┘
```

### 2.3 KV Store (chunk text)

```
Key pattern:   "{document_id}-chunk-{index}"
Value:         raw chunk text
Purpose:       citation retrieval without re-chunking
```

---

## 3. Lineage: Current Wiring (Partial)

### What IS tracked

```
PDF document
  └─► chunks[]  (via document_id FK in relational chunks table)
         └─► entities (via source_chunk_ids TEXT[] — migration 039)
         └─► AGE Node.source_id  (pipe-sep string — merger entity.rs)
         └─► ChunkLineage struct (in-memory during pipeline, not persisted)
```

### What is NOT tracked / broken

| Gap                                   | Location                                                 | Impact                        |
| ------------------------------------- | -------------------------------------------------------- | ----------------------------- |
| PDF page number → chunk               | `chunk_storage.rs` / chunker                             | Cannot cite "page 5"          |
| Chunk→embedding vector linkage        | vector metadata partial                                  | Cannot trace vector→chunk→doc |
| Cross-doc entity lineage completeness | `source_id` only stores current doc's chunks             | Older source docs overwritten |
| Relationship source chunks            | `source_chunk_ids` is pipe-sep string in AGE, not TEXT[] | Joins impossible              |
| `ChunkLineage` not persisted          | in-memory only during `build_lineage()`                  | Lineage lost after ingestion  |

---

## 4. Progress Events: Current State

```
GraphStorage phase lifecycle in the UX:
┌───────────────────────────────────────────────────────────────┐
│  start_pdf_phase(GraphStorage, total = entities + rels)       │
│    │                                                           │
│    │   ...nothing emitted for minutes on large graphs...      │
│    │                                                           │
│  complete_pdf_phase(GraphStorage)                             │
└───────────────────────────────────────────────────────────────┘

Code reference: persist.rs:125 and finalize.rs:156
```

**Root cause:** `persist_processing_result_impl()` in `ingestion_persister.rs`
calls `merger.merge()` which is a single await — no progress callbacks are
accepted or emitted inside the merger loop.

---

## 5. Identified Findings (Summary)

| ID   | Finding                                                                                                                      | Severity |
| ---- | ---------------------------------------------------------------------------------------------------------------------------- | -------- |
| F-01 | AGE UNWIND Cypher literal body grows O(N·P) per batch                                                                        | High     |
| F-02 | `get_nodes_batch` issues one round-trip per ExtractionResult, not globally batched                                           | High     |
| F-03 | GraphStorage phase emits 0 intermediate progress events                                                                      | High     |
| F-04 | ChunkLineage is in-memory only; never persisted                                                                              | Medium   |
| F-05 | `source_id` in AGE nodes is pipe-sep string (no GIN index possible on AGE side)                                              | Medium   |
| F-06 | PDF page→chunk mapping not stored                                                                                            | Medium   |
| F-07 | LLM summarizer called synchronously per entity (blocks merge loop)                                                           | Medium   |
| F-08 | `upsert_nodes_batch` CHUNK=500 is a fixed magic constant (not tunable)                                                       | Low      |
| F-09 | Entity vector batch collected then upserted once globally, but relationship vectors done per ExtractionResult (inconsistent) | Low      |
| F-10 | No WAL/autovacuum tuning guidance for 100K+ entity graphs                                                                    | Low      |

Detailed analysis of each finding is in [SPEC-032-002](002-performance-analysis.md).
