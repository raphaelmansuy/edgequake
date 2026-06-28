# 007 — Rust / DRY / SOLID / First Principles Lens

**Cross-ref:** [003 Ingestion](../003-ingestion/001-ingestion-comparison.md) · [001 First Principles](../001-first-principles/001-first-principles.md)

---

## Code Organization

### LightRAG (Python)

```text
  lightrag/
  ├── lightrag.py        (4470 LOC) — god class + mixins
  ├── operate.py         (5995 LOC) — extract/query/merge monolith
  ├── pipeline.py        (4480 LOC) — ingestion mixin
  ├── utils.py           (5033 LOC)
  └── kg/                (13 storage impls)

  Pattern: Mixin inheritance + monolithic functions
  SRP:     Violated in operate.py (acceptable for research velocity)
  DRY:     Utils extracted; pipeline/operate overlap exists
```

### EdgeQuake (Rust)

```text
  edgequake/crates/
  ├── edgequake-core/      orchestrator SSOT
  ├── edgequake-pipeline/  chunk/extract/merge/persist
  ├── edgequake-query/     engine_impl/modes/* (modular)
  ├── edgequake-storage/   trait-based adapters
  ├── edgequake-api/       handlers + processor/text_insert/*
  └── edgequake-tasks/     durable queue types

  Pattern: Crate boundaries + trait objects
  SRP:     Strong post SPEC-025 (text_insert split, document_admission)
  DRY:     build_ingestion_pipeline, IngestionPersister, query_pipeline
```

---

## SSOT Compliance (EdgeQuake)

| Concern | SSOT Module | Status |
|---------|-------------|:------:|
| Ingestion persist | `IngestionPersister` | ✅ |
| Pipeline build | `build_ingestion_pipeline` | ✅ |
| Upload admission | `document_admission.rs` | ✅ |
| Query pipeline | `query_pipeline.rs` | ✅ |
| Workspace pipeline | `workspace_pipeline_factory.rs` | ✅ |
| Text insert worker | `processor/text_insert/` | ✅ |
| Injection list | `injection_list.rs` | ✅ |
| Hybrid merge | `hybrid_merge.rs` | ✅ |

---

## SOLID Assessment

| Principle | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| **S** Single Responsibility | **D** | **A-** |
| **O** Open/Closed | **B** (storage plugins) | **B+** (traits) |
| **L** Liskov Substitution | **B** | **A-** |
| **I** Interface Segregation | **C** (fat base classes) | **A-** (graph_read_ops split) |
| **D** Dependency Inversion | **B+** (storage factory) | **A** (Arc<dyn Trait>) |

LightRAG's storage plugin architecture is its **best SOLID story** — 13 backends via factory.

EdgeQuake's crate split is its **best SOLID story** — compile-time boundaries.

---

## DRY Wins & Violations

### EdgeQuake Wins (post SPEC-025)

- Upload handlers → `document_admission.rs` (was 80 LOC × 3)
- Library + worker → `build_ingestion_pipeline`
- Graph hops → batch API (was duplicated BFS patterns)

### EdgeQuake Remaining Debt

| Violation | Location | Severity |
|-----------|----------|:--------:|
| `query_bootstrap::build_ingestion_pipeline` wrapper | api/state | P3 |
| Lenient vs Strict pipeline fallback | upload vs worker | P2 |
| Saga KV admission before worker | document_admission | P1 |

### LightRAG Debt

| Violation | Location | Severity |
|-----------|----------|:--------:|
| operate.py + pipeline.py overlap | merge vs extract | P2 |
| Sync/async wrapper duplication | lightrag.py | P3 |
| Storage env var checks scattered | utils + kg | P3 |

---

## First Principles Engineering Score

```text
  Principle                          LightRAG    EdgeQuake
  ─────────                          ────────    ─────────
  One way to do ingest               △           ✓
  One way to persist                 △           ✓
  One way to query                   ✓           ✓
  Types enforce invariants           △ Python    ✓ Rust
  Tests as specification             ✓           ✓
  Code size per module < 500 LOC     ✗           ✓ (mostly)
```

---

## Rust Engineer Verdict

| Dimension | LightRAG | EdgeQuake |
|-----------|:--------:|:---------:|
| Modularity | **C+** | **A-** |
| Type safety | **C** | **A** |
| Maintainability | **C** | **A-** |
| Plugin extensibility | **A** | **B** |
| Onboarding clarity | **B-** | **A-** |

**EdgeQuake is the better engineered codebase** for long-term maintenance.

**LightRAG is the better plugin platform** for storage/parser experimentation.

**Recommendation:** EdgeQuake should adopt LightRAG's **parser registry pattern** (trait + factory) without adopting its monolith structure.
