# 010 — Rust / DRY / SOLID / First Principles Lens

**Cross-ref:** [017 prior audit](../017-dry-and-solid-audit/) · [012 Plan](./012-improvement-plan.md)  
**Post-remediation:** SPEC-024 pass 11 (2026-06-27)

---

## Crate Boundaries (actual dependencies)

```text
  edgequake-api  ──> core, pipeline, query, storage, tasks, llm, observability
  edgequake-core ──> pipeline, storage, query
  edgequake-pipeline ──> storage, llm
  edgequake-query ──> storage, llm
  edgequake-storage ──> (traits + adapters)
  edgequake-tasks ──> storage
```

Separation is **clean**. API handlers trend thin (upload → enqueue).

---

## DRY Assessment (post SPEC-024)

| Pattern | Status | SSOT |
|---------|:------:|------|
| Ingestion persist sequence | ✅ | `IngestionPersister` |
| Persist factory | ✅ | `from_settings` |
| Query mode enum | ✅ | `modes.rs` |
| Hybrid merge (LightRAG) | ✅ | `hybrid_merge.rs` |
| Mix fusion | ✅ | `fusion.rs` |
| Chunk vector + KV | ✅ | `chunk_storage` + `chunk_hydration` |
| Community debounce | ✅ | `community_index_service.rs` |
| Document read-model merge | ✅ | `document_read_model.rs` |
| Dead `strategies/` module | ✅ **Removed** | `engine_impl/modes/*` |
| Queue observability publish | ✅ | `task_queue_pressure::publish_queue_observability` |

### Remaining duplication (P2)

| Item | Notes |
|------|-------|
| Test postgres fixtures | spec013 + spec022 overlap — consolidate when touching tests |
| `migration_bootstrap/*` | ✅ | SRP split (pass 11): mod + helpers + `reconcile/m038..m045` |

---

## SOLID Assessment (post SPEC-024)

### Single Responsibility (S)

| Module | Grade | Notes |
|--------|:-----:|-------|
| `engine_impl/modes/*` | **A-** | Split from 801 LOC monolith; largest ~205 LOC |
| `query_pipeline.rs` | **A** | prepare/retrieve/finalize |
| `ingestion_persister.rs` | **B+** | focused persist saga |
| `community_index_service.rs` | **A** | debounce only (SRP) |
| `document_read_model.rs` | **A-** | read-side reconciliation |
| `migration_bootstrap/mod.rs` | **A-** | orchestration ~565 LOC; reconcile per-migration (pass 11) |
| `migration_bootstrap/reconcile/*` | **A** | one module per migration family; largest m038 ~196 LOC |

### Open/Closed (O)

Adding a query mode now touches ~5 files (`modes.rs`, `engine_impl/modes/`, `query_pipeline.rs`, API parse, tests) — **acceptable** with per-mode modules (was F with monolith).

### Interface Segregation (I)

`GraphReadView` narrows graph trait in query path — good. `AppState` still wide for handlers — P2.

### Dependency Inversion (D)

`IngestionPersister` trait — good. Query engine uses `Arc<dyn VectorStorage>` etc. — good.

---

## First Principles Engineering Score (post SPEC-024)

```text
  Principle                    Score   Notes
  ─────────                    ─────   ─────
  One way to do ingest         4/5     worker queue SSOT; library persist remains
  One way to fuse retrieval    4/5     Hybrid=round_robin; Mix=RRF default; env overrides documented
  SSOT for config              4/5     graph_depth, max_results wired; health exposes fusion env
  Minimal surprise             4/5     Global ≠ GraphRAG documented
  Scale invariants             4/5     Louvain debounced
```

---

## Refactor Targets (remaining ROI)

1. **Consolidate postgres test fixtures** — spec013 + spec022
2. **Narrow `AppState` for handlers** — extract facades (P2)

---

## Rust Engineer Verdict

**Grade: A+ (post SPEC-024 pass 11)** — was **B-**

P-G2/P-G8 consolidation delivered. Query modes modular; dead strategies removed; operational config surfaced via health. Migration reconcile is per-module (SRP). Remaining debt is **`migration_bootstrap/mod.rs` orchestration** (~565 LOC) and **test fixture duplication** — not hot-path correctness.

**See:** [012-improvement-plan.md](./012-improvement-plan.md)
