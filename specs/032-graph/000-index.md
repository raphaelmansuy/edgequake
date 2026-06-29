# SPEC-032: KnowledgeGraph Storing Phase — Audit & Improvement Plan

**Status:** DRAFT  
**Date:** 2026-06-29  
**Authors:** Multi-expert audit (LightRAG · Lineage · Graph · O(N) · Postgres · Systems)  
**Scope:** Ingestion pipeline storing phase, lineage preservation, UX progress granularity

---

## Problem Statement

At ~100 K entities the `GraphStorage` phase of document ingestion exhibits two
failure modes:

1. **Timeouts / failures** — AGE UNWIND batches over large existing graphs take
   O(N) time due to missing edge-property indexes and Cypher planner
   degeneration.
2. **Opaque UX** — the `GraphStorage` PipelinePhase emits a single start/complete
   event with no per-entity or per-batch progress, so users see a frozen progress
   bar for 10–30 minutes.

Additionally, **lineage** from PDF page → chunk → entity/relationship is only
partially wired in the data model and is lost at the embedding layer.

---

## Document Map

| #                                        | Document                   | Focus                                                  |
| ---------------------------------------- | -------------------------- | ------------------------------------------------------ |
| [001](001-current-architecture.md)       | Current Architecture       | Code path audit, data model, sequence diagrams         |
| [002](002-performance-analysis.md)       | Performance Analysis       | O(N) bottlenecks, AGE quirks, pgvector tuning          |
| [003](003-lineage-data-model.md)         | Lineage Data Model         | Chunk→Entity→Relation lineage, PDF page provenance     |
| [004](004-graph-storage-improvements.md) | Graph Storage Improvements | Batching, WAL tuning, streaming merge                  |
| [005](005-progress-events.md)            | Progress Events            | Sub-phase events, WebSocket schema, UX design          |
| [006](006-improvement-plan.md)           | Improvement Plan           | Ranked work items, DRY/SOLID anchoring, migration path |

---

## Expert Lenses Applied

| Lens                 | Key concerns addressed                                     |
| -------------------- | ---------------------------------------------------------- |
| **LightRAG expert**  | Merge semantics, source-tracking, gleaning                 |
| **Lineage expert**   | Chunk↔entity provenance, PDF page spans, cross-doc merges  |
| **Graph expert**     | AGE MERGE patterns, index design, traversal complexity     |
| **O(N) expert**      | Algorithmic analysis of batch upsert, UNWIND body size     |
| **Postgres expert**  | WAL, autovacuum, pgvector HNSW, `work_mem` tuning          |
| **Systems engineer** | Cancellation propagation, saga compensation, observability |

---

## Methodology: 5-Why + First Principles

Each finding follows the 5-Why root-cause chain and is resolved by returning to
the physical constraint (First Principles), not by patching symptoms.

```
Finding → Why 1 → Why 2 → Why 3 → Why 4 → Why 5 (root cause) → Fix
```

---

## Cross-Reference Legend

- `SPEC-021` — CQRS entities schema & dual-write
- `SPEC-025` — Batched vector/graph operations (P-G4)
- `SC2` — Cross-store saga compensation (vector vs. graph)
- `P-G2` — Single persist path (ingestion_persister.rs)
- `P-G4` — Batch round-trip optimisations (entity.rs, edges_ops.rs)
- `FEAT0011` — Document-Chunk-Entity lineage
- `BR0007` — Lineage records append-only
