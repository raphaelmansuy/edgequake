# SPEC-006: Ensure Performance & Resource Safety — Index

**Spec ID:** `006-ensure-perf`  
**Status:** Implemented (P0–P8)  
**Created:** 2026-06-06  
**Trigger:** Production exit code **137** (OOM kill) on large workspaces  
**Method:** First principles · Code is law · DRY · SOLID · Zero-regression contracts

---

## Purpose

Define a **binding specification** for how EdgeQuake must never exhaust host resources
(RAM, CPU, DB connections, LLM quota) while remaining correct and fast. Every rule
cites production code paths. Implementation must be DRY (one budget authority) and
SOLID (bounded traits, composable guards).

---

## Document Map

| # | Document | Role | Primary audience |
|---|----------|------|------------------|
| [001](001_problem_statement.md) | Problem Statement | WHY + scope + success criteria | PM, operators, engineers |
| [002](002_first_principles_model.md) | First-Principles Model | Resource math invariants | Architects |
| [003](003_codebase_audit.md) | Codebase Audit | Code-is-law inventory of limits & violations | Implementers |
| [004](004_resource_budget_catalog.md) | Resource Budget Catalog | Single registry of all caps (target SSOT) | All crates |
| [005](005_violation_registry.md) | Violation Registry | Edge cases, failure modes, remediations | QA, SRE |
| [006](006_architecture_remediation.md) | Architecture Remediation | DRY/SOLID design for fixes | Implementers |
| [007](007_adr.md) | ADR-006 | Accepted architecture: `ResourceGuard` + pushed-down queries | Tech leads |
| [008](008_regression_contract.md) | Regression Contract | Tests, CI gates, proof commands | CI, reviewers |
| [009](009_operator_runbook.md) | Operator Runbook | Docker, env vars, alerts, incident playbooks | SRE, DevOps |
| [e2e/](e2e/000-e2e-index.md) | E2E Proof Suite | Runnable proofs + `make resource-proof` | CI, reviewers |
| [010-brutal](010-brutal-assessment.md) | Brutal Assessment | Honest post-P9 grade (A) + remaining risks | Tech leads, SRE |
| [011](011_migration_rollout.md) | Migration 038 Rollout | Prod-safe apply / concurrent / rollback | SRE, DevOps |
| [012](012_production_delivery.md) | Production Delivery | GO/NO-GO checklist + deploy sequence | SRE, release |

---

## Cross-Reference Matrix

```
                         ┌─────────────────────────────────────────┐
                         │         002 First Principles          │
                         │  PeakRAM = f(objects, copies, conc.)  │
                         └──────────────────┬──────────────────────┘
                                            │
         ┌──────────────────────────────────┼──────────────────────────────────┐
         │                                  │                                  │
         v                                  v                                  v
┌─────────────────┐              ┌─────────────────────┐              ┌─────────────────────┐
│ 003 Code Audit  │─────────────>│ 004 Budget Catalog  │─────────────>│ 006 Architecture    │
│ (code is law)   │   cites      │ (SSOT targets)      │   designs    │ (DRY/SOLID fixes)   │
└────────┬────────┘              └──────────┬──────────┘              └──────────┬──────────┘
         │                                  │                                  │
         v                                  v                                  v
┌─────────────────┐              ┌─────────────────────┐              ┌─────────────────────┐
│ 005 Violations  │─────────────>│ 007 ADR             │─────────────>│ 008 Regression      │
│ (edge cases)    │   informs    │ (decision)          │   gates      │ (no regression)     │
└────────┬────────┘              └──────────┬──────────┘              └──────────┬──────────┘
         │                                  │                                  │
         └──────────────────────────────────┴──────────────────────────────────┘
                                            │
                                            v
                                 ┌─────────────────────┐
                                 │ 009 Operator Runbook  │
                                 └─────────────────────┘
```

---

## Requirement ID Namespace

| Prefix | Meaning | Example |
|--------|---------|---------|
| **NFR-006** | Non-functional requirement (resource) | NFR-006-001: No unbounded graph load in API hot path |
| **BR-006** | Business rule (enforced in code) | BR-006-010: `max_nodes` clamped server-side |
| **TR-006** | Technical requirement | TR-006-003: `list_nodes` pushed to SQL with LIMIT |
| **OR-006** | Operational requirement | OR-006-001: Container `mem_limit` documented |
| **UC-006** | Use case | UC-006-002: Delete document on 200k-node graph without OOM |

All IDs must appear in code (`// SPEC-006: NFR-006-001`) and in [008_regression_contract.md](008_regression_contract.md).

---

## Related Specs & Issues

| Ref | Relationship |
|-----|--------------|
| [SPEC-017 dry-and-solid-audit](../../specs/017-dry-and-solid-audit/001-methodology/001-first-principles-framework.md) | Methodology parent |
| [SPEC-028 document size](../) (code: `AppConfig::max_document_size`) | Upload budget |
| [SPEC-018 observability](../../specs/018-observability/) | Metrics for resource guards |
| Production incident | Exit 137, `ghcr.io/raphaelmansuy/edgequake` container |

---

## Implementation Phases (ordered)

| Phase | Deliverable | Status |
|-------|-------------|--------|
| **P0** | `ResourceBudget` crate module + `ResourceGuard` | ✅ |
| **P0** | Replace `get_all_*` on API/delete/lineage paths | ✅ |
| **P0** | Remove graph traversal full-graph fallback | ✅ |
| **P1** | Align body/upload limits; Docker mem_limit docs | ✅ |
| **P1** | `GraphMaterializationSemaphore` on `AppState` | ✅ |
| **P1** | `document_graph_cascade` service (DRY delete/lineage) | ✅ |
| **P2** | SQL prefix push-down + relationship bounded lookup | ✅ |
| **P3** | Community detection admission guard + GIN indexes | ✅ |
| **P4** | Migration 038 prod-safe package | ✅ |
| **P5** | Size-aware bootstrap + `/ready` gate | ✅ |
| **P6** | Community seal + readiness battle tests + CI | ✅ |
| **P7** | `ResourceGuard` on `AppState` (DRY budget authority) | ✅ |
| **P8** | `graph_materialization` service + full endpoint coverage + upload SSOT | ✅ |
| **P9** | Orchestrator deletion bounded + runbook env sync + prod delivery proof | ✅ |

---

## Definition of Done (spec-level)

- [x] Every NFR-006-* has a test in `edgequake-api/tests/resource_safety*.rs` (core paths + P8 materialization)
- [x] `rg 'get_all_nodes\(\)' edgequake/crates/edgequake-api/src` returns zero matches (allowlist empty)
- [x] `make resource-proof` passes — CI job `resource-proof` in `.github/workflows/ci.yml`
- [x] `ResourceBudgetConfig` injected via `AppState::resource_guard` / `resource_budget()` (`spec006_no_adhoc_resource_budget.sh`)
- [x] Server upload limit uses `resource_budget().max_upload_bytes` (honors `EDGEQUAKE_MAX_UPLOAD_BYTES`)
- [x] Graph materialization semaphore on all materialization endpoints (proof 018)
- [x] Operator runbook env vars match `PipelineConfig::from_env()` and `main.rs` worker config (`spec006_runbook_env_sync.sh`)
- [x] Unbounded `Vec` growth — advisory gate via code review + resource_budget handler lint (no automated vec lint)
