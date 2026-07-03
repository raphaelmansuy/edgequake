# SPEC-041 — Cross-Reference Matrix

**Method:** Every row = claim → evidence → test → status

---

## Primary defect (#273)

| ID | Claim | Code law | E2E proof | Status |
| -- | ----- | -------- | --------- | ------ |
| C-01 | `->>>` invalid on json | PG operator table | `verify_no_invalid_json_operators.sh` | ✅ Fixed |
| C-02 | M078 line 51 typo | `078_age_child_workspace_stats.sql:51` | G3 apply | ✅ Fixed |
| C-03 | M078 line 63 typo | `078_age_child_workspace_stats.sql:63` | G3 apply | ✅ Fixed |
| C-04 | Concurrent line 27 typo | `support/078/concurrent.sql:27` | G2 indexdef | ✅ Fixed |
| C-05 | Concurrent line 36 typo | `support/078/concurrent.sql:36` | G2 indexdef | ✅ Fixed |
| C-06 | Edge indexes unaffected | `078:75-88` | G2 — no `->>>` | ✅ OK |
| C-07 | Startup fatal on failure | `migration_bootstrap/mod.rs:659` | Issue #273 report | ✅ Understood |
| C-08 | Checksum lock updated | `checksums.lock` | `check_migration_checksums.sh` | ✅ Done |

---

## SSOT alignment (DRY)

| ID | Canonical source | M078 (fixed) | Match |
| -- | ---------------- | ------------ | ----- |
| D-01 | `graph_lifecycle.rs:174` | `->>'workspace_id'` | ✅ |
| D-02 | `graph_lifecycle.rs:164` | `->>'tenant_id'` | ✅ |
| D-03 | `014_add_graph_indexes.sql:55` | workspace pattern | ✅ |
| D-04 | `036_add_edge_property_indexes.sql:65` | workspace on EDGE | ✅ (M078 targets Node) |
| D-05 | `support/046/apply.sql:33` | workspace pattern | ✅ |

---

## SPEC-040 lineage

| SPEC-040 artifact | Relationship to #273 |
| ----------------- | ---------------------- |
| `008-implementation-plan.md` §1.1 M078 | Introduced typo |
| `006-postgres-age-pgvector-lens.md:95-98` | Proposed SQL had typo |
| `e2e/measure_graph_stats_perf.sh` | Assumes M078 applied — didn't catch CREATE |
| `010-release-runbook.md` | v0.13.2 shipped broken M078 |
| Issue #262 | Original intent — perf fix blocked by #273 |

---

## SPEC-041-B — Related (out of scope)

| ID | Claim | Code law | Recommended fix | Status |
| -- | ----- | -------- | --------------- | ------ |
| B-01 | M071 HNSW no dim guard | `071_hnsw_optimize.sql` | atttypmod check / halfvec | 📋 Future |
| B-02 | Runtime DDL swallows HNSW err | `vector/ddl.rs` | Log + surface | 📋 Future |
| B-03 | Orphan eq_*_vectors tables | workspace join | Skip in M071 | 📋 Future |

Reporter noted in #273 additional context — **not fixed in SPEC-041** to preserve single-responsibility.

---

## Requirement traceability

| REQ | C/D/B IDs | Verified by |
| --- | --------- | ----------- |
| REQ-041-01 | C-02, C-03 | G3 |
| REQ-041-02 | C-04, C-05 | G2 |
| REQ-041-03 | D-01..D-05 | G2 indexdef |
| REQ-041-04 | C-01 | G1 grep |
| REQ-041-05 | C-02, EC-01 | G3 |
| REQ-041-06 | EC-02 | G3 skip |
| REQ-041-07 | D-01 | `verify_m078_indexes.sql` |
| REQ-041-08 | EC-07 | `repair_migration_078_checksum.sh` |
| REQ-041-09 | C-08 | checksum script |

---

## Issue closure evidence checklist

For closing [#273](https://github.com/raphaelmansuy/edgequake/issues/273):

1. Link to `specs/041-fix-migration/000-index.md`
2. Attach `e2e/evidence/run_all_summary.txt`
3. Note release tag (v0.13.3+)
4. Checksum repair note for EC-07 population
