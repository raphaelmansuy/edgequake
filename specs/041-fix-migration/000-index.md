# SPEC-041 — Migration 078 JSON Operator Fix (GitHub #273)

**Spec:** `041-fix-migration`  
**Date:** 2026-07-03  
**Status:** `IMPLEMENTED` — hotfix for v0.13.2 startup blocker  
**Method:** Code is law — every claim maps to file, migration, or E2E proof  
**Trigger:** [Issue #273](https://github.com/raphaelmansuy/edgequake/issues/273) — `operator does not exist: json ->>> unknown`

---

## TL;DR

Migration **078** (`078_age_child_workspace_stats.sql`) shipped in **v0.13.2** with a typo: `->>>` (triple `>`) instead of PostgreSQL's valid JSON text extraction operator `->>`. On any install with Apache AGE and a graph containing a `"Node"` label table, backend startup **hard-fails** at migration 78 — the entire stack is down.

**Fix:** Three-layer repair — (L1) automatic checksum repair at bootstrap; (L2) fixed M078 + M079 safety-net migration; (L3) post-bootstrap reconcile via `support/078/apply.sql`. See [008-upgrade-path-matrix.md](./008-upgrade-path-matrix.md).

---

## Scope

| Item | Severity | Fix state |
| ---- | -------- | --------- |
| [#273](https://github.com/raphaelmansuy/edgequake/issues/273) | P0 startup blocker | **Fixed** — M078 operator typo |
| Related #271 context (M071 HNSW dim guard) | P1 separate track | **Out of scope** — document only |

---

## Documents

| File | Lens |
| ---- | ---- |
| [001-five-whys.md](./001-five-whys.md) | Root cause (5 WHY) |
| [002-first-principles.md](./002-first-principles.md) | First principles |
| [003-code-is-law.md](./003-code-is-law.md) | Evidence map (file:line) |
| [004-postgres-age-operator-lens.md](./004-postgres-age-operator-lens.md) | PostgreSQL JSON / AGE operators |
| [005-implementation-plan.md](./005-implementation-plan.md) | Battle-tested fix plan |
| [006-cross-reference-matrix.md](./006-cross-reference-matrix.md) | Cross-ref matrix |
| [007-release-runbook.md](./007-release-runbook.md) | Release + checksum repair |
| [008-upgrade-path-matrix.md](./008-upgrade-path-matrix.md) | All-version upgrade paths |

---

## Requirements (REQ-041-xx)

| ID | Requirement |
| -- | ----------- |
| REQ-041-01 | M078 uses `->>` not `->>>` for `workspace_id` and `tenant_id` expression indexes |
| REQ-041-02 | `support/078/concurrent.sql` uses same corrected operators (DRY with M078) |
| REQ-041-03 | Canonical operator pattern matches M014, M046, `graph_lifecycle.rs` (SSOT) |
| REQ-041-04 | No `->>>` anywhere under `edgequake/migrations/` (CI grep gate) |
| REQ-041-05 | M078 applies idempotently on AGE + Node graph (E2E proof) |
| REQ-041-06 | M078 no-ops gracefully when AGE extension absent (E2E proof) |
| REQ-041-07 | Created indexes use expression containing `->>'workspace_id'` (pg_get_indexdef proof) |
| REQ-041-08 | Checksum repair for v0.13.2 skip-path (automatic L1 + manual script) |
| REQ-041-09 | `checksums.lock` updated for M078 + M079 |
| REQ-041-10 | Bootstrap L1: `repair_migration_078_checksum_if_needed()` before sqlx |
| REQ-041-11 | M079 idempotent safety-net migration |
| REQ-041-12 | Bootstrap L3: `reconcile_migration_078()` via `support/078/apply.sql` |

---

## E2E proof

```bash
./specs/041-fix-migration/e2e/run_all.sh
./specs/041-fix-migration/e2e/simulate_upgrade_paths.sh
./scripts/check_migration_checksums.sh
```

---

## Related

| Spec / Issue | Relationship |
| ------------ | ------------ |
| [SPEC-040](../040-edgequake-issues/000-index.md) | M078 authored for #262 — introduced typo |
| [Issue #273](https://github.com/raphaelmansuy/edgequake/issues/273) | Reporter: akashs-devops (ECS Fargate prod) |
| `edgequake/docs/migrations/078-age-child-workspace-stats.md` | Ops doc (update post-fix) |
