# SPEC-041 — Code Is Law

**Baseline:** v0.13.2 (buggy) → v0.13.3 (fixed)  
**Issue:** [#273](https://github.com/raphaelmansuy/edgequake/issues/273)

---

## Bug — invalid operator

| Claim | Law (file:line) | Status (pre-fix) |
| ----- | --------------- | ---------------- |
| `->>>` on workspace_id index | `migrations/078_age_child_workspace_stats.sql:51` | ❌ Invalid |
| `->>>` on tenant_id index | `migrations/078_age_child_workspace_stats.sql:63` | ❌ Invalid |
| Concurrent script same typo | `migrations/support/078/concurrent.sql:27,36` | ❌ Invalid |
| Edge indexes OK | `078_age_child_workspace_stats.sql:75-88` | ✅ `start_id::text` |

---

## Fix — corrected operator

| Claim | Law (file:line) | Status (post-fix) |
| ----- | --------------- | ----------------- |
| `->>` workspace_id | `078_age_child_workspace_stats.sql:51` | ✅ |
| `->>` tenant_id | `078_age_child_workspace_stats.sql:63` | ✅ |
| Concurrent workspace_id | `support/078/concurrent.sql:27` | ✅ |
| Concurrent tenant_id | `support/078/concurrent.sql:36` | ✅ |
| Checksum lock | `migrations/checksums.lock` (078 entry) | ✅ Updated |

---

## SSOT — correct pattern elsewhere (DRY reference)

| File | Line | Expression |
| ---- | ---- | ------------ |
| `graph_lifecycle.rs` | 164-177 | `agtype_to_json(properties)->>'tenant_id'` / `workspace_id` |
| `014_add_graph_indexes.sql` | 43-55 | `->>'tenant_id'`, `->>'workspace_id'` |
| `036_add_edge_property_indexes.sql` | 65-78 | `->>'workspace_id'`, `->>'tenant_id'` |
| `support/046/apply.sql` | 28-33 | `->>'tenant_id'`, `->>'workspace_id'` |
| `074_native_upsert_unique_indexes.sql` | 105 | `->>'node_id'` |

**Law:** M078 was the **only** file using `->>>` in the entire repository (verified by grep).

---

## Migration bootstrap path

| Step | Law | Role |
| ---- | --- | ---- |
| Startup calls migrations | `migration_bootstrap/mod.rs:609-659` | `MIGRATOR.run(pool)` |
| M078 embedded | sqlx `migrations/078_age_child_workspace_stats.sql` | Auto-apply |
| Failure aborts startup | sqlx transaction rollback | No partial v78 marker on CREATE failure |
| AGE skip guard | `078:26-28` | `pg_extension extname = 'age'` |
| Node table guard | `078:39-42` | `to_regclass(... "Node")` |

---

## Idempotency guards (M078)

| Guard | Law | Purpose |
| ----- | --- | ------- |
| Index exists check | `078:45-48`, `57-60` | Skip CREATE if present |
| Per-graph loop | `078:36-97` | All AGE graphs |
| ANALYZE after indexes | `078:92-95` | Planner stats |

---

## Test coverage map

| Requirement | Proof | Path |
| ----------- | ----- | ---- |
| REQ-041-04 No `->>>` | Static grep | `e2e/verify_no_invalid_json_operators.sh` |
| REQ-041-05 AGE+Node apply | SQL E2E | `e2e/apply_m078_with_age_graph.sql` |
| REQ-041-06 AGE absent skip | SQL E2E | `e2e/run_all.sh` GROUP 3 |
| REQ-041-07 Index definition | pg_get_indexdef | `e2e/verify_m078_indexes.sql` |
| REQ-041-09 Checksum | CI script | `scripts/check_migration_checksums.sh` |
| Bootstrap idempotent | Rust test | `migration_bootstrap_proof.rs` (existing) |

---

## Related defect (documented, not fixed)

| Claim | Law | Issue |
| ----- | --- | ----- |
| M071 unconditional HNSW | `071_hnsw_optimize.sql` | dim > 2000 fails startup |
| Runtime DDL swallows HNSW error | `vector/ddl.rs` `.execute().await.ok()` | Silent no-index until M071 |

Tracked in [006-cross-reference-matrix.md](./006-cross-reference-matrix.md) as **SPEC-041-B** (future spec).
