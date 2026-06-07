# First-Principles DRY & SOLID Audit Framework

**Spec:** 017-dry-and-solid-audit  
**Date:** 2026-05-31  
**Method:** Code is law — every finding cites source files; intent comments and ADRs are secondary evidence.

---

## 1. Why First Principles?

DRY and SOLID are not style preferences. They are **invariants for a multi-crate RAG system** where:

1. **Wrong duplication → divergent behavior** (same `QueryMode::Hybrid` producing different results in API vs orchestrator).
2. **Wrong abstraction → silent production bugs** (workspace pipeline resolved three different ways during ingestion).
3. **God objects → untestable change surface** (`AppState` with 25+ fields forces every handler to depend on everything).

First-principles analysis asks:

| Question | What we inspect |
|----------|-----------------|
| **What is the single source of truth?** | Type definitions, enums, normalization functions, config defaults |
| **Who calls whom at runtime?** | Production paths (API handlers, task processor), not dead code or benches |
| **Are substitutable implementations actually substitutable?** | Memory vs Postgres adapters, strict vs lenient pipeline resolution |
| **Does each module have one reason to change?** | LOC, concern count, mixed layers (HTTP + SQL + domain) |
| **Can we extend without editing N files?** | New query mode, new provider, new storage backend |

---

## 2. Audit Scope

### In scope (workspace crates)

| Folder | Crate | ~LOC (src) |
|--------|-------|------------|
| `002-edgequake-api/` | HTTP layer, state, processors | 51,427 |
| `003-edgequake-core/` | Orchestration, types, legacy query | 14,576 |
| `004-edgequake-pipeline/` | Chunking, extraction, merging | 14,341 |
| `005-edgequake-query/` | SOTA query engine, strategies | 10,105 |
| `006-edgequake-storage/` | Traits, memory/postgres adapters | 12,925 |
| `007-edgequake-pdf/` | PDF conversion facade | 306 |
| `008-edgequake-auth/` | JWT, RBAC, extractors | 2,960 |
| `009-edgequake-audit/` | Compliance logging | 578 |
| `010-edgequake-tasks/` | Background job queue | 5,925 |
| `011-edgequake-rate-limiter/` | Token bucket, middleware | 714 |
| `012-edgequake-webui/` | React/Next.js client | ~361 TS/TSX files |

### Out of scope

- **`edgequake-llm`** — external crate (`0.6.20` on crates.io); referenced but not audited here.
- **Legacy `lightrag/`** — being replaced.
- **Test-only duplication** — noted when it masks production parity gaps.

---

## 3. Violation Taxonomy

### DRY (Don't Repeat Yourself)

| Class | Definition | EdgeQuake example |
|-------|------------|-------------------|
| **D1 — Logic duplication** | Same algorithm in multiple places | Three workspace pipeline builders |
| **D2 — Type duplication** | Parallel structs/enums with manual mapping | `QueryMode` × 4, `StorageConfig` × 2 |
| **D3 — Config duplication** | Defaults scattered across crates | Entity types: 9 vs 5 vs inline |
| **D4 — Dead duplication** | Copy exists but production path bypasses it | `strategies/` module (~900 LOC, bench-only) |
| **D5 — Behavioral duplication** | Same name, different semantics | Hybrid merge: round-robin vs dedupe |

### SOLID

| Principle | Violation signal | EdgeQuake example |
|-----------|------------------|-------------------|
| **S — SRP** | Module >800 LOC mixing layers; struct with 10+ unrelated fields | `AppState`, `SOTAQueryEngine`, `PostgresAGEGraphStorage` |
| **O — OCP** | Adding feature requires editing match arms in N crates | New query mode → 6+ files across core + query |
| **L — LSP** | Implementations break caller assumptions | Memory graph uses trait defaults → wrong workspace counts |
| **I — ISP** | Handlers receive full state; fat traits (40+ methods) | `GraphStorage`, `State(AppState)` everywhere |
| **D — DIP** | Handlers call concrete factories, bypass abstractions | `query_execute` skips `WorkspaceProviderResolver` |

---

## 4. Priority Scale

| Priority | Meaning | Action horizon |
|----------|---------|----------------|
| **P0** | Correctness / production divergence | Fix before next release |
| **P1** | High maintenance cost or parity risk | Current sprint |
| **P2** | Moderate duplication or design debt | Next refactor cycle |
| **P3** | Polish, version alignment, cosmetic | Backlog |

---

## 5. Evidence Standard

Every violation record includes:

```
ID:       {CRATE}-DRY-001 or {CRATE}-SOLID-S-001
Priority: P0–P3
Claim:    One-sentence factual statement
Evidence: file:line or grep count
Impact:   What breaks if ignored
Fix:      Concrete remediation (module to extract, type to delete, test to add)
Verify:   Acceptance test or grep that proves fix
```

**Rejected as violations:** intentional HTTP DTO boundaries (`*_types` modules) when mapping is thin and documented — unless mapping is manual, duplicated, and error-prone.

---

## 6. Cross-Cutting Themes (Preview)

See `002-cross-crate/001-priority-matrix.md` for the full ranked backlog.

1. **Split-brain query execution** — API uses `SOTAQueryEngine`; orchestrator uses `edgequake-core::query::QueryEngine`; ~900 LOC strategies unused.
2. **Triple provider/pipeline resolution** — query-time, ingestion-time, upload-time paths diverge.
3. **Entity normalization schism** — `prompts::normalize_entity_name` ≠ `merger::normalize_entity_name` (P0 graph integrity).
4. **Memory/Postgres parity gaps** — dashboard stats wrong on memory backend.
5. **Four `QueryMode` enums** — serde and variant sets differ.

---

## 7. How to Use These Docs

1. Start with **cross-crate priority matrix** for sprint planning.
2. Drill into **per-crate folder** for evidence and file-level remediation.
3. Each remediation includes **verification steps** — run before closing tickets.
4. Re-audit after P0 fixes; update violation IDs to `FIXED` with commit SHA.
