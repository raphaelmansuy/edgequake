# 002 — Ingestion Pipeline Audit

**Cross-ref:** [F-01,F-02,F-03,F-08,F-09,F-10](./README.md#cross-reference-matrix) · [004 LightRAG](./004-lightrag-expert-lens.md) · [009 O(n)](./009-complexity-on-lens.md)  
**Post-remediation:** SPEC-024 pass 12 (2026-06-27)

---

## Entry Points (code map — post pass 12)

| Route | Handler | Execution | Resilience | Persist |
|-------|---------|-----------|------------|---------|
| `POST /documents` | `text_upload.rs` | **Async task (202)** | `text_insert` + resilience | `IngestionPersister` |
| `POST /documents/upload` | `file_upload.rs` | **Async task (202)** | same | same |
| `POST /documents/upload/batch` | `batch_upload.rs` | **Async per file (202)** | same + re-ingest SSOT | same |
| `POST /documents/pdf` | `pdf_upload/upload.rs` | Async task | PDF → `text_insert` | same |
| `PUT .../injection` | `injection.rs` | **Queued task** | worker resilience | same |
| Library | `orchestrator/ingestion.rs` | Sync | `process` (fail-fast) | `DefaultIngestionPersister` |

**Finding F-01:** **Mitigated** — all API upload paths enqueue `TaskRuntime`; duplicate handling unified via `resolve_workspace_duplicate_for_reingestion`.

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
- `invalidate_query_result_cache_for_workspace` (F-09 — pass 13 library parity)

**Pass 13:** Chunk KV writes moved into `IngestionPersister` — worker, injection, and library paths no longer diverge on storage layout.

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

### W7 — Library path ignores workspace vector registry (P1) ✅ Fixed pass 14

~~`EdgeQuake::insert` uses orchestrator-global `vector_storage`, not per-workspace tables.~~

**Fixed:** `EdgeQuake::with_workspace_vector_support` + `resolve_ingestion_vector_storage()` delegate to `workspace_vector_resolve::resolve_workspace_vector_storage` (same SSOT as API `storage_helpers` / worker `workspace_resolver`). E2E: `spec024_orchestrator_workspace_vector_registry.rs`.

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

## Ingestion Grade (post SPEC-024 pass 14)

| Criterion | Grade | Note |
|-----------|:-----:|------|
| Persist correctness | **A-** | Saga + compensation; KV in persister |
| Path uniformity | **A** | API + library share vector registry SSOT |
| Scale readiness | **A** | Louvain debounced; scheduled workspaces in `/health` |
| Resilience | **A-** | Worker canonical; injection queued |
| Storage efficiency | **A** | Chunk text in KV; vector `content_ref`; workspace tables |
| Test coverage | **A** | spec024 E2E: workspace vector registry, batch re-ingest, hybrid |

**Bottom line:** `text_insert.rs` remains canonical for HTTP. Library `EdgeQuake::insert` is the remaining non-queue path but now shares **IngestionPersister**, **workspace vector registry**, and **workspace-scoped cache bust** with the worker (W7 closed pass 14).
