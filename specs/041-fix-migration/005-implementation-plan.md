# SPEC-041 — Implementation Plan

**Target:** v0.13.3 patch (or v0.13.2.1 hotfix)  
**Issue:** [#273](https://github.com/raphaelmansuy/edgequake/issues/273)

---

## Phase 0 — Evidence lock ✅

- [x] Grep entire repo for `->>>` — 4 hits, all M078
- [x] Confirm SSOT uses `->>` in M014, M046, `graph_lifecycle.rs`
- [x] Reproduce error signature from issue body

---

## Phase 1 — Code fix ✅

| Task | File | Change |
| ---- | ---- | ------ |
| Fix workspace_id operator | `078_age_child_workspace_stats.sql:51` | `->>>` → `->>` |
| Fix tenant_id operator | `078_age_child_workspace_stats.sql:63` | `->>>` → `->>` |
| Fix concurrent workspace | `support/078/concurrent.sql:27` | `->>>` → `->>` |
| Fix concurrent tenant | `support/078/concurrent.sql:36` | `->>>` → `->>` |
| Bump version comment | `078` header | v1.0.1 + SPEC-041 ref |
| Update checksum lock | `checksums.lock` | `./scripts/update_migration_checksums.sh` |

**DRY:** No new abstraction layer — fix aligns with existing SSOT; concurrent script mirrors inline migration.

---

## Phase 2 — E2E proof suite ✅

| Test | Script | Edge case |
| ---- | ------ | --------- |
| G1 Static operator grep | `verify_no_invalid_json_operators.sh` | Any future typo |
| G2 Index definition audit | `verify_m078_indexes.sql` | Expression shape |
| G3 AGE+Node migration apply | `apply_m078_with_age_graph.sql` | **Prod failure path** |
| G4 Idempotent re-apply | `run_all.sh` GROUP 4 | Second run no error |
| G5 Checksum lock | `check_migration_checksums.sh` | CI immutability |
| G6 Checksum repair doc | `repair_migration_078_checksum.sh` | v0.13.2 no-graph installs |

---

## Phase 3 — Release gates

```bash
# Must all pass before tag
./specs/041-fix-migration/e2e/run_all.sh
./scripts/check_migration_checksums.sh
cargo test -p edgequake-api migration_bootstrap_proof --features postgres  # if DATABASE_URL set
```

---

## Edge case matrix

| # | Scenario | Pre-fix | Post-fix | Proof |
| - | -------- | ------- | -------- | ----- |
| EC-01 | AGE + Node graph, fresh upgrade | **FAIL** startup | **PASS** | G3 |
| EC-02 | AGE absent | PASS (skip) | PASS | G3 skip branch |
| EC-03 | AGE present, zero graphs | PASS | PASS | Empty loop |
| EC-04 | Graph without Node label | PASS (continue) | PASS | CONTINUE guard |
| EC-05 | Graph with Node, no EDGE | PASS partial | PASS | Node indexes only |
| EC-06 | Indexes already exist | PASS (skip) | PASS | G4 idempotent |
| EC-07 | v0.13.2 applied v78 (no Node) | N/A | Checksum mismatch on upgrade | repair script |
| EC-08 | v0.13.2 blocked at v78 | N/A | Retry applies fixed v78 | G3 |
| EC-09 | Concurrent ops script | **FAIL** if run | **PASS** | G2 on concurrent path |

---

## Rollback

| Action | Risk |
| ------ | ---- |
| Revert M078 file to v0.13.2 | Re-introduces startup blocker |
| DROP INDEX on child tables | Safe — no data loss; `graph_lifecycle.rs` recreates |

---

## Definition of Done

- [x] All REQ-041-xx satisfied
- [x] `run_all.sh` exits 0 with evidence in `e2e/evidence/`
- [x] Zero `->>>` in `edgequake/migrations/`
- [x] Cross-ref docs complete in `specs/041-fix-migration/`
- [ ] GitHub #273 closed with E2E evidence link (release step)
