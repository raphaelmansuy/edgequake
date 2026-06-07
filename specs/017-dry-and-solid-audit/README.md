# Spec 017 — DRY & SOLID Audit

First-principles analysis of EdgeQuake workspace crates. **Code is law** — findings cite source files, not intent docs.

**Date:** 2026-05-31  
**Scope:** 10 Rust workspace crates + web UI  
**Total src LOC audited:** ~113,857 (Rust) + ~361 TS/TSX files

---

## Document Index

| Folder | Crate | Audit | Priority focus |
|--------|-------|-------|----------------|
| [001-methodology](./001-methodology/001-first-principles-framework.md) | — | Framework & taxonomy | How to read this audit |
| [002-cross-crate](./002-cross-crate/001-priority-matrix.md) | All | [Priority matrix](./002-cross-crate/001-priority-matrix.md), [Debt map](./002-cross-crate/002-architecture-debt-map.md) | Sprint planning |
| [003-edgequake-api](./003-edgequake-api/001-audit.md) | API | Full audit | P0 pipeline resolution, AppState |
| [004-edgequake-core](./004-edgequake-core/001-audit.md) | Core | Full audit | P0 split-brain query |
| [005-edgequake-pipeline](./005-edgequake-pipeline/001-audit.md) | Pipeline | Full audit | P0 normalizer, chunker |
| [006-edgequake-query](./006-edgequake-query/001-audit.md) | Query | Full audit | P0 SOTA dedup |
| [007-edgequake-storage](./007-edgequake-storage/001-audit.md) | Storage | Full audit | P0 memory parity |
| [008-edgequake-pdf](./008-edgequake-pdf/001-audit.md) | PDF | Full audit | Merge candidate |
| [009-edgequake-auth](./009-edgequake-auth/001-audit.md) | Auth | Full audit | Clean ✅ |
| [010-edgequake-audit](./010-edgequake-audit/001-audit.md) | Audit | Full audit | Merge candidate |
| [011-edgequake-tasks](./011-edgequake-tasks/001-audit.md) | Tasks | Full audit | Keep ✅ |
| [012-edgequake-rate-limiter](./012-edgequake-rate-limiter/001-audit.md) | Rate limiter | Full audit | Keep ✅ |
| [013-edgequake-webui](./013-edgequake-webui/001-audit.md) | Web UI | Full audit | P1 API monolith |

---

## Top 6 P0 Findings (Fix First)

1. **Dual entity normalizers** — `edgequake-pipeline`: merger vs prompts (`PIPE-DRY-001`)
2. **Split-brain query** — core orchestrator vs API SOTA engine (`CORE-DRY-001`, `QUERY-DRY-001`)
3. **Triple pipeline resolution** — API upload vs processor vs state (`API-DRY-001`)
4. **Chunker ignores strategy** — configured chunking never runs (`PIPE-DRY-002`)
5. **Memory graph parity** — wrong workspace stats on memory backend (`STORE-SOLID-L-001`)
6. **`StorageConfig` name collision** — two structs in core (`CORE-DRY-004`)

---

## Estimated Remediation Impact

| Category | Addressable duplicate LOC | Crates |
|----------|---------------------------|--------|
| Query stacks | ~2,500+ | core, query, api |
| SOTA entry pipeline | ~1,900 | query |
| Pipeline/API ops | ~550 | api, pipeline |
| Storage filters/tests | ~200 | storage |
| **Total** | **~5,500+** | — |

---

## Re-Audit Trigger

Re-run this audit when:
- P0 items closed (update IDs to `FIXED` + commit SHA)
- New crate added to workspace
- Query engine migration completes

---

## Related Specs

- ADR-0002: Modular crate architecture (`edgequake/docs/adr/0002-modular-crate-architecture.md`)
- OODA-226: WorkspaceProviderResolver migration
- SPEC-140: Embedding override consolidation
