# SPEC-084 — Reliability Fix (GitHub Issues Audit)

> **Product pin**: EdgeQuake v0.21.1 (SPEC-084 reliability 2026-07-24)  
> **Sources**: [#331](https://github.com/raphaelmansuy/edgequake/issues/331) · [#319](https://github.com/raphaelmansuy/edgequake/issues/319) · [#318](https://github.com/raphaelmansuy/edgequake/issues/318) · [#317](https://github.com/raphaelmansuy/edgequake/issues/317) · [#316](https://github.com/raphaelmansuy/edgequake/issues/316) · [#255](https://github.com/raphaelmansuy/edgequake/issues/255)  
> **Docs status**: Study pack + implementation complete  
> **Implementation**: [03-implementation-roadmap.md](03-implementation-roadmap.md) — Sprint 0→2 landed

## Verification status (SSOT)

See [01-issue-register.md](01-issue-register.md): **6 FIXED / 0 PARTIAL / 0 CONFIRMED / 0 RETRACTED**.

| ID | Verdict | One-line |
|----|---------|----------|
| GH-331 | FIXED | Count SQL JOINs `"Node"`; child GIN EXPLAIN + pool e2e |
| GH-319 | FIXED | Server `status` before paginate; Failed filter beyond page 100 |
| GH-318 | FIXED | Track `expected_count`; Query soft-gate + “Query anyway” |
| GH-317 | FIXED | `POST /documents/batch-delete` → one `BatchDeletion` task |
| GH-316 | FIXED | Workspace-fair claim + nested workspace ingest lanes |
| GH-255 | FIXED | COMPAT-GUARD slash allow + `llm_full_id` no double-prefix |

---

## Start here

1. Read [00-first-principles.md](00-first-principles.md) — LAW-9…LAW-14 + SOLID/DRY  
2. Skim [01-issue-register.md](01-issue-register.md) — every GH# with FIXED status  
3. Issue studies → [`issues/GH-NNN-….md`](issues/)  
4. Roadmap → [03-implementation-roadmap.md](03-implementation-roadmap.md)  
5. Tests → [04-e2e-test-matrix.md](04-e2e-test-matrix.md)

---

## Sprint snapshot

| Sprint | Goal | Status |
|--------|------|--------|
| **0** | Pool-safe source-prefix counts (#331) + Failed filter SSOT (#319) | **done** |
| **1** | Selected bulk delete (#317) + gateway model passthrough (#255) | **done** |
| **2** | Query readiness mid-ingest (#318) + workspace fairness (#316) | **done** |

---

## Deferred matrix (see register)

Playwright specs (319/317/318), GH-317 200-doc opcount, and claim-index EXPLAIN guard — root causes FIXED; harness follow-ups listed in [01-issue-register.md](01-issue-register.md).

## Out of scope (locked)

- Parent-table GIN on `_ag_label_vertex` (explicitly rejected)  
- Merging unrelated PR #229 as-is (intent absorbed into #255)  
- Raising local Ollama concurrency beyond existing caps  
- CopilotKit (not product “GIA”)
