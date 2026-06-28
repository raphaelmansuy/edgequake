# 001 — Robustness & Reliability Comparison

**Cross-ref:** [003 Ingestion](../003-ingestion/001-ingestion-comparison.md) · [005 Features](../005-features/001-feature-matrix.md) · [008 System Engineering](../008-expert-lenses/005-system-engineering.md)

**Findings:** C-05, C-08

---

## 1. Failure Model

```text
  LightRAG failure surfaces              EdgeQuake failure surfaces
  ─────────────────────────              ──────────────────────────

  Doc status → FAILED                    Task status → Failed
  Pipeline cancel exception              CancellationToken abort
  Storage impl exception                 ApiError + structured logs
  Partial VDB upsert (backend-dep)       Saga compensation (vectors)
  Retry via re-insert                    Startup orphan recovery
  LLM cache for idempotency              Content hash dedup
```

---

## 2. Cross-Store Consistency (C-05)

### LightRAG

No unified saga. `merge_nodes_and_edges` upserts graph then VDBs sequentially. Failure mid-merge leaves state **backend-dependent**:

- NetworkX: in-memory, process-local
- Postgres impl: has retry logic (`test_postgres_retry_integration.py`) but no doc-scoped compensation

### EdgeQuake

```text
  SC2 Saga (orchestrator/ingestion.rs, ingestion_persister.rs)
  ────────────────────────────────────────────────────────────

  Step 1: Write chunk vectors (atomic UNNEST per doc)
  Step 2: Graph merge (idempotent, source-tracked)
  Step 3: On merge fail → delete doc vectors + quarantine log

  Admission caveat (N-12): KV written at HTTP accept,
  worker failure leaves Failed docs — mitigated by recovery jobs.
```

**Winner: EdgeQuake** — explicit compensation protocol documented in code.

---

## 3. Concurrency Safety

| Concern | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| Graph keyed locks | ✓ `get_storage_keyed_lock` | ✓ AGE transactions |
| Parallel insert limit | ✓ `DEFAULT_MAX_PARALLEL_INSERT` | ✓ worker pool config |
| Workspace isolation | ✓ namespace tests | ✓ UUID scoping + tests |
| Sync wrapper guard | ✓ `test_sync_wrapper_guard` | N/A (async-native) |
| Race on duplicate doc | ✓ content hash | ✓ hash + reingest policy |

LightRAG has extensive workspace isolation tests (`tests/workspace/` — 5 modules). EdgeQuake has tenant/workspace middleware + contract tests.

---

## 4. Cancellation & Recovery

### LightRAG

- `PipelineCancelledException` propagated through parse/analyze/extract
- Doc status tracks in-flight states (`_INFLIGHT_DOC_STATUSES`)
- Tests: `test_pipeline_cancellation.py`, `test_pipeline_internal_abort.py`

### EdgeQuake

- `CancellationToken` in `text_insert/cancel.rs`
- `recover_orphaned_tasks()` + `recover_orphaned_documents()` at startup
- Task queue pressure metrics (`task_queue_pressure.rs`)
- Cancellable pipeline: `process_with_resilience_cancellable()`

**Winner: EdgeQuake** on durable recovery; **LightRAG** on pipeline-stage granularity.

---

## 5. Input Sanitization

| Vector | LightRAG | EdgeQuake |
|--------|:--------:|:---------:|
| Control char strip | ✓ `strip_control_characters` | ✓ UTF-8 validation |
| Entity name truncation | ✓ char+byte limits | △ |
| Content hash normalize | ✓ | ✓ ContentHasher |
| Max document size | △ config | ✓ explicit limit |
| PDF bomb / zip | △ parser-dep | △ pdfium limits |

LightRAG has more battle-tested edge cases (5995 LOC `operate.py`, 224 test modules).

---

## 6. Observability

```text
  LightRAG                          EdgeQuake
  ────────                          ─────────

  logger + performance_timing_log   tracing spans
  doc_status metadata               /health JSON (components)
  △ metrics                         edgequake-observability metrics
  △ OTEL                            △ OTEL (SPEC-018 deferred)
```

EdgeQuake `/health` returns component booleans (kv, vector, graph, llm) — operator-friendly.

---

## 7. Test Robustness (C-08)

| Metric | LightRAG | EdgeQuake |
|--------|:--------:|:---------:|
| Test modules | ~224 | ~192 |
| Parser golden tests | ✓ extensive | △ PDF only |
| Pipeline tests | ✓ 15+ | ✓ contract + e2e |
| Storage impl tests | ✓ per backend | ✓ postgres + memory |
| Workspace isolation | ✓ dedicated suite | ✓ middleware tests |
| Query contract tests | △ | ✓ spec024/025 |

LightRAG wins **breadth** (especially parser/kg). EdgeQuake wins **targeted contracts** for algorithm parity.

---

## 8. Production Readiness Matrix

| Scenario | LightRAG (default JSON+NX) | LightRAG (Postgres) | EdgeQuake |
|----------|:--------------------------:|:-------------------:|:---------:|
| Process crash mid-ingest | ✗ state lost | △ | ✓ task durable |
| Multi-tenant isolation | △ | ✓ | ✓ |
| Horizontal scale | ✗ file locks | △ | △ (single writer) |
| Backup/restore | ✗ | ✓ | ✓ PG native |
| LLM provider failover | △ | △ | ✓ workspace providers |

**Brutal verdict:**

- LightRAG default install = **dev/research robustness**
- LightRAG + Postgres = **B+ production**
- EdgeQuake Postgres-only = **A production substrate**, **B scale-out** (worker pool not sharded)
