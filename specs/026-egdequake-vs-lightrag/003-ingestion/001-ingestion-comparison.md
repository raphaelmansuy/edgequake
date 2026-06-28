# 001 — Ingestion Pipeline Comparison

**Cross-ref:** [002 Algorithms](../002-algorithms/001-algorithm-comparison.md) · [005 Features](../005-features/001-feature-matrix.md) · [006 Robustness](../006-robustness/001-robustness-comparison.md)

**Findings:** C-03, C-05, C-11, C-12

---

## 1. Pipeline Topology

### LightRAG

```text
  insert / pipeline workers
  ─────────────────────────

  Source file / text
       │
       ▼
  ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
  │ PARSE       │────>│ ANALYZE      │────>│ EXTRACT     │
  │ (native/    │     │ (multimodal  │     │ (entities)  │
  │  mineru/    │     │  VLM)        │     │             │
  │  docling)   │     └──────────────┘     └──────┬──────┘
  └─────────────┘                                  │
       │                                           ▼
       │                                    merge_nodes_and_edges
       │                                           │
       └───────────────────────────────────────────┘
                         │
                         ▼
              entities_vdb + rels_vdb + chunks_vdb
              + graph + text_chunks KV + doc_status
```

**Source:** `lightrag/pipeline.py` (`_PipelineMixin`), queue sizes in `constants.py` (`DEFAULT_QUEUE_SIZE_PARSE/ANALYZE/INSERT`).

Worker pipeline features:
- Cancellation (`PipelineCancelledException`)
- Content hash dedup
- Doc status machine (PENDING → PARSING → ANALYZING → PROCESSING → PROCESSED/FAILED)
- Multimodal image analysis with VLM
- Resume from checkpoint on failure

### EdgeQuake

```text
  HTTP upload / injection
  ─────────────────────

  POST handler
       │
       ▼
  document_admission.rs (SSOT)
       ├── hash dedup + reingest policy
       ├── KV: metadata + content + hash→doc
       └── enqueue_task (slim payload: text="")
       │
       ▼
  WorkerPool (Postgres durable)
       │
       ▼
  text_insert/ module (SRP split)
       ├── prepare → extract → persist → finalize
       └── resolve_text_insert_content() from KV
       │
       ▼
  build_ingestion_pipeline(doc_size, gleaning)
       │
       ▼
  DefaultIngestionPersister
       ├── chunk KV (content_ref SSOT)
       ├── pgvector batch
       ├── AGE merge + compensation saga
       └── schedule_community_index_refresh (300s debounce)
```

**Source:** `document_admission.rs`, `processor/text_insert/`, `ingestion_persister.rs`, `community_index_service.rs`.

---

## 2. Format Support (C-12 — brutal gap)

| Format | LightRAG | EdgeQuake |
|--------|:--------:|:---------:|
| Plain text | ✓ | ✓ |
| Markdown (native IR) | ✓ rich | △ basic |
| DOCX (native parser) | ✓ golden tests | ✗ |
| PDF (native) | △ via parsers | ✓ pdfium embedded |
| PDF (MinerU) | ✓ sidecar | ✗ |
| PDF (Docling) | ✓ sidecar | ✗ |
| Images / VLM | ✓ multimodal | ✗ |
| HTML | △ | ✗ |
| Batch upload | ✓ | ✓ |
| Knowledge injection API | ✗ | ✓ extension |

LightRAG parser registry: `lightrag/parser/registry.py`, tests under `tests/parser/` (100+ files).

EdgeQuake: PDF via `edgequake-pdf2md` (vision LLM), text/markdown via API.

**Verdict:** LightRAG wins ingestion **breadth** decisively. EdgeQuake wins **PDF-in-binary** simplicity (no MinerU sidecar).

---

## 3. Chunking Comparison (C-03)

| Capability | LightRAG | EdgeQuake |
|------------|:--------:|:---------:|
| Fixed token | ✓ | ✓ adaptive |
| Recursive character | ✓ | ✗ |
| Semantic vector | ✓ async | ✗ |
| Paragraph semantic | ✓ | ✗ |
| Overlap configurable | ✓ per strategy | ✓ adaptive |
| Heading-aware splits | ✓ breadcrumb | △ |
| Token enforcement pre-embed | ✓ | ✓ |

EdgeQuake adaptive sizes (`600/800/1200`) are a **reasonable default** but not equivalent to semantic chunking on long structured docs (legal, technical manuals).

---

## 4. Persist Path

### LightRAG

Persistence scattered across:
- `operate.py::merge_nodes_and_edges` — graph + VDB upserts
- Storage impl per backend (`kg/postgres_impl.py`, `networkx_impl.py`, etc.)
- No cross-store saga — compensation depends on backend

### EdgeQuake

```text
  DefaultIngestionPersister (explicit ordering)
  ────────────────────────────────────────────

  1. KV chunk records (SSOT for text)
  2. pgvector chunk embeddings (content_ref metadata)
  3. AGE graph merge (idempotent, source-tracked)
  4. on merge fail → compensate: delete doc vectors
  5. debounced Louvain community index

  WHY vectors-before-graph: only remaining fallible step
  is merge; vectors are doc-scoped and deletable.
```

**C-05:** EdgeQuake has **explicit saga compensation**. LightRAG relies on storage impl correctness without unified cross-store protocol.

---

## 5. Concurrency & Backpressure

| Mechanism | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Parallel chunk extract | ✓ semaphores | ✓ tokio parallel |
| Parse/analyze/insert queues | ✓ sized queues | △ single worker pool |
| Task cancellation | ✓ | ✓ CancellationToken |
| Queue pressure metrics | △ | ✓ `/health` + metrics |
| Orphan task recovery | △ doc_status | ✓ startup recovery |
| Slim task payload | N/A (in-memory text) | ✓ KV ref only |

---

## 6. Community Index (C-11)

```text
  LightRAG                         EdgeQuake
  ────────                         ─────────

  No community index at ingest     Louvain @ ingest (debounced 300s)
  No community at query            community_id used in Global expand

  Pure LightRAG path               Intentional deviation
```

Debatable: helps Global mode co-membership; costs CPU on large graphs after idle period.

---

## 7. Ingestion Grades

| Dimension | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Format breadth | **A** | **C+** |
| Parser quality | **A** | **B-** (PDF only) |
| Algorithm core | **A** | **A** |
| Durability | **B** | **A+** |
| Ops visibility | **B** | **A+** |
| Code modularity | **C** (5995 LOC operate) | **A-** (split modules) |
| Multimodal | **A** | **F** |

**Net:** LightRAG for **research ingestion lab**. EdgeQuake for **production text/PDF pipeline**.
