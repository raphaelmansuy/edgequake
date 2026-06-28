# 010 — Rust / DRY / SOLID Expert Lens

**Cross-ref:** [002 Ingestion](./002-ingestion-pipeline-audit.md) · [003 Query](./003-query-retrieval-audit.md) · [001 Architecture](./001-first-principles-architecture.md)

**Findings:** R-02, R-12, N-07, N-08, N-11

---

## Post-SPEC-024 Modularity Score

| Area | Before SPEC-024 | Now | Evidence |
|------|:-----------------:|:---:|----------|
| Query modes | 1×800 LOC monolith | 6 modules ~75–205 LOC | `engine_impl/modes/*` |
| Migration bootstrap | 1×1014 LOC | mod + reconcile/* + helpers | `migration_bootstrap/` |
| Ingestion persist | 3 divergent paths | 1 trait | `IngestionPersister` |
| Hybrid merge | inline duplicate | `hybrid_merge.rs` | FEAT0104 |
| Community scheduler | in persist | `community_index_service.rs` | SRP |
| Queue pressure | ad hoc | `task_queue_pressure.rs` | SSOT |

**Grade: A-** — Major SPEC-024 win. Remaining debt is **admission layer** and **processor god module**.

---

## SOLID Assessment

### Single Responsibility (SRP)

| Module | Verdict | Note |
|--------|:-------:|------|
| `hybrid_merge.rs` | ✓ | Merge only |
| `graph_hops.rs` | ✓ | BFS only |
| `chunk_hydration.rs` | ✓ | KV fetch only |
| `ingestion_persister.rs` | ✓ | Cross-store persist |
| `text_insert.rs` | ✗ | Orchestration + status + PDF + checkpoint + dual-write |
| `file_upload.rs` | △ | Upload + vision + enqueue |

### Open/Closed (OCP)

- `TaskProcessor` trait — extend task types without worker rewrite ✓
- `IngestionPersister` trait — swap persist strategy ✓
- Query modes — add mode file + dispatch match arm ✓

### Liskov Substitution (LSP)

- `TaskType::Upload` ≡ `Insert` in processor — redundant enum variant (minor)

### Interface Segregation (ISP)

- `DocumentTaskProcessor` builder with 15+ optional deps — fat but pragmatic for test injection

### Dependency Inversion (DIP)

- Handlers → `enqueue_task` abstraction ✓
- Processor → `persist_with_providers` → trait ✓
- API query → `execute_sota_query` → engine trait bundle ✓

**Strong DIP on persist and query paths.** Handlers still reach into KV directly (pragmatic for admission).

---

## DRY Violations (remaining)

### N-07 — Upload admission triplication

```text
  file_upload.rs ────────┐
  text_upload.rs ────────┼──> ~80 lines duplicated
  batch_upload.rs ───────┘

  Should be:
  admission::accept_document(content, meta) -> EnqueueResult
```

### Workspace vector resolve (fixed ✅)

```text
  edgequake_core::workspace_vector_resolve  ← SSOT
       ↑                    ↑
  orchestrator          API middleware
```

Pass 14 consolidated this — **model for other DRY fixes**.

### Chunk KV builders (acceptable)

`ingestion_persist.rs` re-exports `edgequake_pipeline::build_chunk_kv_records` — facade pattern, OK.

---

## Rust Idioms (honest)

| Pattern | Usage | Grade |
|---------|-------|:-----:|
| `Result<T>` error handling | Consistent | A |
| `Arc<dyn Trait>` storage ports | Consistent | A |
| `tracing` not println | Consistent | A |
| `unwrap()` in prod paths | Rare; tests use more | A- |
| Async cancellation | `CancellationRegistry` + checkpoint | A |
| Contract tests | Extensive SPEC-024 suite | A |

---

## Naming / API Surface Debt

### N-11 — Dual defaults

```rust
// modes.rs — serde Default
#[default] QueryMode::Hybrid

// engine_impl/mod.rs — runtime Default
default_mode: QueryMode::Mix
```

**Fix:** Align serde default to Mix OR document `Default` as serialization-only.

---

## File Size Discipline

```text
  TARGET:  < 300 LOC per module (project convention)

  PASS:
    engine_impl/modes/*.rs     75–205
    hybrid_merge.rs            ~208
    graph_hops.rs              ~118
    migration reconcile/*.rs   ≤196

  FAIL:
    text_insert.rs             ~950  (N-08)
    migration_bootstrap/mod.rs   ~565  (acceptable orchestrator)
```

---

## Rust Expert Verdict

EdgeQuake **looks like Rust written by people who got burned by monoliths** — then stopped splitting one directory too early (`processor/`).

**Next refactors (ROI order):**

1. Extract `document_admission.rs` (N-07)
2. Split `text_insert.rs` → orchestrator / status / checkpoint (N-08)
3. Fix QueryMode default mismatch (N-11)

**Do not refactor:** Query modes again — they're in good shape.
