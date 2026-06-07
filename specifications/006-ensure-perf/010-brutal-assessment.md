# SPEC-006 — Brutal Honest Assessment (Post P9)

**Date:** 2026-06-06  
**Grade:** **A** (production-ready; honest residual debt)

---

## What Actually Works Now

| Area | Verdict | Evidence |
|------|---------|----------|
| Zero `get_all_*` in API `src/` | ✅ | `spec006_no_get_all_api.sh` |
| DRY resource budget on `AppState` | ✅ | `spec006_no_adhoc_resource_budget.sh` |
| DRY graph materialization guard | ✅ **P8** | `services/graph_materialization.rs` + proof 018 |
| Semaphore on all materialization endpoints | ✅ **P8** | graph, popular, search, stream |
| Server upload SSOT | ✅ **P8** | `server.rs` → `resource_budget().max_upload_bytes` |
| Bounded delete/lineage/relationships | ✅ | `document_graph_cascade.rs` |
| Migration 038 + `/ready` | ✅ | proofs 016–017 |
| Clippy/fmt `-D warnings` | ✅ | workspace lib |
| CI `resource-proof` | ✅ | `ci.yml` |

**Exit 137 from routine API flows:** Should not recur when indexes applied, memory limits set, ops follow checklist.

---

## What Still Can Kill You (Honest)

### 1. Large graph — ops must run CONCURRENTLY

Bootstrap defers; `/ready` stays 503 until `apply_038.sh --concurrent`.

### 2. Residual `get_all_*` (non-API)

- `e2e_document_deletion.rs` (52+ calls on tiny mock graphs — test debt only)
- `community.rs` (intentional, post-guard)
- Test mocks in `edgequake-core/tests/sc2_sc5_ingestion.rs`

**Closed P9:** `orchestrator/deletion.rs` now uses `GraphScanOps` (`spec006_no_get_all_orchestrator.sh`).

### 3. `EDGEQUAKE_MEM_LIMIT` warn-only

Not enforced in-process.

### 4. PDF / vision RAM

Separate incident class.

### 5. `search_labels` unguarded

Lightweight label search — intentionally no materialization guard (no graph load).

### 6. Pipeline worker env vs runbook

**Closed P9:** `spec006_runbook_env_sync.sh` verifies 9 high-signal vars in code + 009 §2.2.

---

## First-Principles Scorecard (P8)

| Invariant | P7 | P8 | Target |
|-----------|----|----|--------|
| Single budget authority | partial | **✅ handlers + server** | ✅ |
| Graph materialize cap | 2/4 endpoints | **✅ all materialization** | ✅ |
| Materialization busy proven | no | **✅ proof 018** | ✅ |
| Upload env override wired | no | **✅ server SSOT** | ✅ |
| Storage full-scan trap | deprecated | deprecated | ⚠️ adapters remain |

---

## Grade Justification: A

P9 closes orchestrator deletion + runbook sync + HTTP graph 503 proof ([019](e2e/019-p9-production-delivery-proof.md), [012](012_production_delivery.md)). **A+** still blocked by: e2e test `get_all_*` debt, no in-process mem cap enforcement.

---

## Bottom Line

**Ship-ready** for exit-137 class when:

1. `make resource-proof-postgres` before prod
2. `/ready` + `/health.schema.source_ids_indexes` after deploy
3. `apply_038.sh --concurrent` on large graphs if degraded
4. `EDGEQUAKE_MEM_LIMIT` / Docker `mem_limit` ≥ 4g
5. Never add `ResourceBudgetConfig::default()` in handlers — use `state.resource_budget()`
