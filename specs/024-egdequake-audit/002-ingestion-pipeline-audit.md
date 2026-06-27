# 002 — Ingestion Pipeline Audit

**Cross-ref:** [F-01,F-02,F-03,F-08,F-09,F-10](./README.md#cross-reference-matrix) · [004 LightRAG](./004-lightrag-expert-lens.md) · [009 O(n)](./009-complexity-on-lens.md)

---

## Entry Points (code map)

| Route | Handler | Execution | Resilience | Persist |
|-------|---------|-----------|------------|---------|
| `POST /documents` | `text_upload.rs` | Async task | `process_with_resilience_cancellable` | `ingestion_persist.rs` |
| `POST /documents/upload` | `file_upload.rs` | **Sync HTTP** | `process_with_resilience` | `persist_ingestion_result` |
| `POST /documents/upload/batch` | `batch_upload.rs` | **Sync sequential** | same, weaker dedup | same |
| `POST /documents/pdf` | `pdf_upload/upload.rs` | Async task | PDF → `text_insert` | same |
| `PUT .../injection` | `injection.rs` | **`tokio::spawn`** | **`process` (fail-fast)** | same |
| Library | `orchestrator/ingestion.rs` | Sync | `process` (fail-fast) | `DefaultIngestionPersister` |

**Finding F-01 (P0):** Same logical operation (ingest document) has **four different failure and latency profiles**.

```text
  CANONICAL (mature)                    OUTLIERS (immature)
  ─────────────────                     ───────────────────
  text_upload ──> TaskQueue             file_upload ──> blocks HTTP
       │                                batch_upload ──> no re-ingest on dup
       v                                injection ──> spawn, no resilience
  text_insert.rs                             │
       │                                     v
       ├── checkpoint save/load              process() fail-fast
       ├── cancellation gates
       ├── stage metadata / WS events
       └── strict workspace providers
```

---

## Persist SSOT (F-02) — the good news

`edgequake-pipeline/src/persistence/ingestion_persister.rs`:

```text
  build_chunk_vector_batch
         │
         v
  vector_storage.upsert          ──> pgvector UNNEST batch (1000 rows)
         │
         v
  KnowledgeGraphMerger::merge    ──> get_nodes_batch + upsert_nodes_batch
         │
    success ──> refresh_community_index
    failure ──> compensate_merge_failure
```

API wrapper `edgequake-api/src/services/ingestion_persist.rs` adds:
- `PostgresEntitySink` resolution
- `invalidate_query_result_cache` (global — F-09)

This is **genuine consolidation**. Orchestrator, worker, injection, and file upload all call the same trait.

---

## Async Worker Path (reference implementation)

File: `edgequake-api/src/processor/text_insert.rs`

```text
  Task picked up
       │
       v
  resolve workspace pipeline (strict mode)
       │
       v
  optional: load checkpoint (skip LLM if extraction done)
       │
       v
  process_with_resilience_cancellable
       │     per-chunk: timeout + 3 retries + semaphore
       v
  build_chunk_kv_records  ──> KV (duplicate of vector metadata — F-08)
       │
       v
  persist_with_providers
       │
       v
  metadata final status (completed | partial_failure | failed)
       │
       v
  optional PostgreSQL documents dual-write (best-effort)
```

**Strengths:**
- Checkpoint recovery between extraction and persist
- Tenant fairness via `TenantConcurrencyLimiter` (`edgequake-tasks/src/worker.rs`)
- PDF single-flight admission (`ingest_admission.rs`)

---

## Weaknesses (brutal)

### W1 — Sync file upload blocks and leaves orphan KV (P0)

`file_upload.rs` runs full LLM + persist on the HTTP thread. If persist fails after KV chunks written, compensation rolls back vectors/graph but **KV chunks and hash keys remain**.

### W2 — Batch upload weaker than single file (P1)

`batch_upload.rs`: duplicate content returns `(doc_id, true)` without re-ingestion; no partial_failure semantics; sequential O(files × pipeline).

### W3 — Injection fail-fast + KV scan (P0/P1)

- Uses `process`, not `process_with_resilience` (`injection.rs`)
- `list_injections` / delete: `kv_storage.keys().await` then filter — **O(all keys)** (F-10)
- Failure path: `let _ = ctx.kv_storage.upsert(...)` swallows errors

### W4 — Community refresh every ingest (P0 at scale)

`ingestion_persister.rs` calls `refresh_community_index` after every successful merge.

`community_persist.rs`:
```rust
pub async fn refresh_community_index(graph: Arc<dyn GraphStorage>) {
    // detect_communities_unchecked → Louvain → batch label persist
}
```

**First-principle violation:** Index maintenance cost must be **amortized**, not per-document.

### W5 — Chunk content stored twice (P1)

`build_chunk_vector_batch` embeds full chunk text in vector metadata JSON. Same text in KV chunk records. Large docs → bloated pgvector rows + 2× storage (F-08).

### W6 — Global cache invalidation (P1)

Every successful persist clears entire query result cache, not workspace-scoped (F-09).

### W7 — Library path ignores workspace vector registry (P1)

`EdgeQuake::insert` uses orchestrator-global `vector_storage`, not per-workspace tables. API paths use workspace registry.

---

## Merger & Embeddings

`edgequake-pipeline/src/merger/entity.rs`:
- **Good:** `get_nodes_batch` + `upsert_nodes_batch`
- **Bad:** `result.extractions.clone()` before merge (full copy)
- **Bad:** Relational sink upserts per entity in loop (not batched)

Extraction: `pipeline/extraction.rs` — O(chunks × retries × LLM latency). Dominates wall clock.

---

## Task Queue

`edgequake-tasks/src/postgres.rs` + `worker.rs`:
- JSONB payload, heartbeat, exponential backoff, circuit breaker
- Cooperative cancel via `CancellationRegistry`

Solid worker engineering. **Undermined** by bypassing queue for sync upload.

---

## Ingestion Grade

| Criterion | Grade | Note |
|-----------|:-----:|------|
| Persist correctness | B+ | Saga + compensation |
| Path uniformity | D | Four execution models |
| Scale readiness | C- | Community refresh, KV scan |
| Resilience | B (worker) / D (injection) | Bifurcated |
| Test coverage | B | spec021/022/023 E2E present |

**Bottom line:** Treat `text_insert.rs` as canonical. **Delete or redirect** sync/spawn paths to match it.
