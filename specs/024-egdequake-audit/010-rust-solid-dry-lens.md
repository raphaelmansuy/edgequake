# 010 — Rust / DRY / SOLID / First Principles Lens

**Cross-ref:** [017 prior audit](../017-dry-and-solid-audit/) · [001 Architecture](./001-first-principles-architecture.md) · [012 Plan](./012-improvement-plan.md)

---

## Crate Boundaries (actual dependencies)

```text
  edgequake-api  ──> core, pipeline, query, storage, tasks, llm
  edgequake-core ──> pipeline, storage, query (legacy paths)
  edgequake-pipeline ──> storage, llm
  edgequake-query ──> storage, llm
  edgequake-storage ──> (traits + adapters)
  edgequake-tasks ──> storage
```

Separation is **mostly clean**. Leakage: API handlers contain too much orchestration logic (file_upload 258 lines removed in recent commit — improving).

---

## DRY Assessment

| Pattern | Status | Evidence |
|---------|:------:|----------|
| Ingestion persist sequence | ✓ Fixed | `IngestionPersister` trait P-G2 |
| Persist factory | ✓ | `from_settings` SSOT |
| Query mode enum | ✓ | Single `modes.rs` |
| Workspace pipeline resolution | △ | Improved; strict mode still optional |
| Hybrid merge logic | ✗ | Different from Mix fusion |
| Chunk vector build | ✓ | Shared in persister |
| Test postgres fixtures | △ | spec013 + spec022 duplication |

### D4 — Dead duplication (still present)

`edgequake-query/src/strategies/` — appears bench/unused on production hot path (`engine_impl` is SSOT). Maintenance tax without runtime value.

**Action:** Delete or gate behind `#[cfg(test)]` / bench feature.

---

## SOLID Assessment

### Single Responsibility (S)

| Module | LOC | Concerns | Grade |
|--------|-----|----------|:-----:|
| `vector_queries.rs` | ~801 | all query modes + fallbacks | F |
| `migration_bootstrap.rs` | ~1015 | migrations + extensions + community backfill | D |
| `text_insert.rs` | large | worker + checkpoint + persist + dual-write | C |
| `ingestion_persister.rs` | ~437 | focused persist saga | B+ |
| `query_pipeline.rs` | moderate | prepare/retrieve/finalize only | A |

### Open/Closed (O)

Adding new query mode requires:
1. `modes.rs` enum
2. `vector_queries.rs` new function
3. `query_pipeline.rs` dispatch arm
4. API `query_types.rs` parse
5. Tests across contracts

**~5+ files** — acceptable for Rust match-based dispatch, but `vector_queries.rs` monolith violates OCP.

**Fix:** `trait QueryModeStrategy` per mode module.

### Liskov Substitution (L)

Memory vs Postgres adapters: tests use memory; production uses Postgres. Contract tests (`contract_*`) mitigate LSP breaks.

**Known gap:** Memory graph trait defaults caused workspace count bugs (017 audit) — verify fixed in spec021 tests.

### Interface Segregation (I)

`GraphStorage` trait — fat (40+ methods). Callers use `GraphReadView` in query path — **good narrowing**.

`AppState` — handlers receive full state. ISP violation persists in API layer.

### Dependency Inversion (D)

`IngestionPersister` trait — **good DIP** for persist.

Handlers still call concrete factories in places (`create_workspace_pipeline`) — improved but not pure.

---

## Rust Idioms (code quality)

| Practice | Observed |
|----------|----------|
| `Result<T>` error handling | ✓ pervasive |
| `Arc<dyn Trait>` for storage | ✓ |
| `async_trait` on ports | ✓ |
| `tracing` not println | ✓ |
| `unwrap()` in production | rare; mostly tests |
| Clippy/fmt | project requires |

**Concern:** `let _ =` error swallowing in injection failure paths — not idiomatic for production Rust.

---

## First Principles Engineering Score

```text
  Principle                    Score   Key violation
  ─────────                    ─────   ───────────────
  One way to do ingest         2/5     four paths
  One way to fuse retrieval    3/5     Hybrid vs Mix
  SSOT for config              3/5     dead graph_depth, max_results
  Minimal surprise             3/5     Global ≠ GraphRAG (documented)
  Scale invariants             2/5     Louvain per ingest
```

---

## Refactor Targets (ordered by ROI)

1. **Split `vector_queries.rs`** → `modes/naive.rs`, `local.rs`, `global.rs`, `hybrid.rs`, `mix.rs`
2. **Extract `CommunityIndexService`** — debounced Louvain, called from persister hook
3. **Unify ingest handlers** → thin HTTP → enqueue task (DRY with text_insert)
4. **Remove dead config fields** or wire them
5. **Shrink `migration_bootstrap.rs`** — one module per migration concern

---

## Rust Engineer Verdict

**Grade: B-**

Recent P-G2 work shows **disciplined refactoring intent**. Remaining debt is **concentrated monoliths** and **path duplication** — classic post-feature consolidation phase.

The codebase is **maintainable by a strong team**; it is **not yet maintainable by default** for new contributors touching query modes or ingestion entry points.

See [012-improvement-plan.md](./012-improvement-plan.md) Phase 2 (structure).
