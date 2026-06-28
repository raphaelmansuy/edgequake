# 002 — Ingestion Pipeline Audit

**Cross-ref:** [001 Architecture](./001-first-principles-architecture.md) · [004 LightRAG](./004-lightrag-expert-lens.md) · [009 O(n)](./009-complexity-on-lens.md) · [012 Plan](./012-improvement-plan.md)

**Findings:** R-01, R-02, R-03, R-08, R-09, N-02, N-03, N-07, N-08, N-09, N-12

---

## 1. Flow (code-verified)

```text
  POST /documents | /upload | /upload/batch | /injection
           │
           ├── validate (size, UTF-8, extension; vision for images)
           ├── ContentHasher + workspace_hash_key
           ├── resolve_workspace_duplicate_for_reingestion()
           │      ├── NoDuplicate → proceed
           │      ├── ClearedForReingestion → delete graph/vectors/KV
           │      └── StillProcessing → reject duplicate
           ├── KV: {doc}-metadata, {doc}-content, hash→doc
           └── enqueue_task() → 202 ACCEPTED
                    │
                    v
           WorkerPool (Postgres durable + channel)
                    │
                    v
           DocumentTaskProcessor
           ├── Insert/Upload → process_text_insert()
           ├── KnowledgeInjection → process_knowledge_injection()
           └── PdfProcessing → process_pdf_processing()
                    │
                    v
           Pipeline::process_with_resilience_cancellable()
           (chunk → parallel extract → embed)
                    │
                    v
           persist_with_providers() → DefaultIngestionPersister
           (KV chunks → pgvector → AGE merge → debounced community)
```

**Grade: A-** — SPEC-024 converged HTTP on worker queue. Remaining gaps are **feature parity** and **admission DRY**, not architectural chaos.

---

## 2. What Works (brutally confirmed)

### R-01 — Unified async HTTP path ✅

All primary handlers call `AppState::enqueue_task()`:

| Handler | File | Task type |
|---------|------|-----------|
| Text | `text_upload.rs` | `Insert` |
| File | `file_upload.rs` | `Insert` |
| Batch | `batch_upload.rs` | `Insert` per file |
| Injection | `injection.rs` | `KnowledgeInjection` |

Contract: `contract_spec024_ingestion_uniformity.rs`.

### R-02 — IngestionPersister SSOT ✅

`DefaultIngestionPersister::persist()` in `ingestion_persister.rs`:

1. `build_chunk_kv_records` → KV
2. `build_chunk_vector_batch` → pgvector (`content_ref` metadata)
3. `KnowledgeGraphMerger::merge` → AGE + entity/rel vectors
4. `compensate_merge_failure` on error
5. `schedule_community_index_refresh` (debounced)

Shared by: worker (`ingestion_persist.rs`), injection (`injection_process.rs`), library (`orchestrator/ingestion.rs`).

### R-08 — Chunk text dedup ✅

`chunk_storage.rs` — vectors reference KV keys; hydration at query via `chunk_hydration.rs`.

### R-09 — Workspace cache invalidation ✅

`invalidate_query_result_cache_for_workspace` on persist; library path wired in `orchestrator/ingestion.rs`.

### R-03 — Community debounce ✅

`community_index_service.rs` — abort/restart timer per workspace; default 300s. **Not** Louvain per document anymore.

---

## 3. What Still Hurts

### N-02 — Library smarter than production (P1) ✗

`edgequake-core/src/orchestrator/ingestion.rs`:

```rust
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize { ... }
// 600 / 800 / 1200 by doc size

// GleaningExtractor wrapped when gleaning_enabled
```

**Worker path** uses workspace pipeline from `get_workspace_pipeline_strict()` — **static chunk config**, no Gleaning wrapper in `text_insert.rs`.

**Impact:** Same document ingested via SDK vs HTTP can produce **different graphs**. That is a correctness split, not a perf tweak.

### N-03 — Task payload duplicates text (P1) ✗

`TextInsertData.text` serialized into Postgres `tasks.payload` JSONB **and** stored as `{doc_id}-content` in KV.

Large PDF → 2× storage + slow task row updates + WAL pressure.

**First principle:** Task queue should carry **references** (doc_id, workspace_id), not full corpus.

### N-07 — Upload admission copy-paste (P2) ✗

`file_upload.rs`, `text_upload.rs`, `batch_upload.rs::enqueue_single_file` repeat:

- hash check
- KV metadata/content writes
- `TextInsertData` construction
- `Task::new` + enqueue

**~80 lines × 3.** Batch also lacks image/vision path present in single file upload.

### N-08 — `text_insert.rs` god module (P2) ✗

~950 LOC mixing: pipeline orchestration, PDF phases, checkpoint I/O, progress callbacks, status machine, PostgreSQL document dual-write, lineage persist.

SRP violation acknowledged in SPEC-024; not yet split.

### N-09 — Injection list still linear (P2) ⚠

`injection.rs::list_injections`:

```rust
let keys = state.storage.kv_storage.keys_with_prefix(&prefix).await?;
// then get_by_id per key
```

Prefix index (F-10 fix) beats full KV scan, but still **O(n) round trips** with no pagination cursor.

### N-12 — Admission KV before worker success (P1) ⚠

HTTP writes KV metadata **before** task completes. Worker failure leaves documents in `Failed` or stuck states; startup recovery mitigates but does not erase the pattern.

---

## 4. Worker Infrastructure (strength)

```text
  main.rs startup
  ├── recover_orphaned_tasks()
  ├── recover_orphaned_documents()
  ├── requeue_pending_tasks()
  └── WorkerPool::start(cpus×4)
        ├── tenant fairness
        ├── 7200s timeout
        ├── exponential backoff
        └── CancellationRegistry
```

`/health` → `operational.task_queue` + `pressure` (normal/elevated/critical).

This is **A+ system engineering** for a Rust RAG stack. See [008-system-engineering-lens.md](./008-system-engineering-lens.md).

---

## 5. Ingestion vs LightRAG Expectation

| Expectation | EdgeQuake | Status |
|-------------|-----------|:------:|
| One insert path | HTTP: one queue; Library: separate | ⚠ |
| Per-chunk resilient extract | Worker yes; injection no checkpoint | ⚠ |
| Consistent chunk sizing | Adaptive library only | ✗ N-02 |
| Incremental graph merge | Batch merger + debounced community | ✓ |
| Provenance on entities | `source_chunk_ids`, lineage KV | ✓ |

---

## 6. Brutal Verdict

**Before SPEC-024:** D+ (four execution models, sync uploads, spawn injection).  
**After SPEC-024:** **A-** (one worker path, SSOT persister, debounced community).

**Blockers to A+:**

1. N-02 library/API extraction parity
2. N-03 task payload slimming
3. N-07 admission SSOT helper
4. N-08 split `text_insert.rs`

**Do not regress:** Never re-add sync inline persist to upload handlers. Code is law — `contract_spec024_ingestion_uniformity.rs` will catch it.
